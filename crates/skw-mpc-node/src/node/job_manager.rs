use std::collections::HashMap;

use futures::{channel::{mpsc, oneshot}, StreamExt, TryStreamExt};
use libp2p::{PeerId, Multiaddr};
use serde::{Serialize, de::DeserializeOwned};

use skw_crypto_curv::elliptic::curves::secp256_k1::Secp256k1;
use skw_crypto_curv::{BigInt, arithmetic::Converter};

use skw_mpc_payload::{CryptoHash, PayloadHeader, Payload, header::PayloadType};
use skw_round_based::{async_runtime::AsyncProtocol, Msg};
use skw_mpc_protocol::gg20::state_machine::{keygen::{self, LocalKey}, sign::{self, SignManual, PartialSignature}};

use crate::{
    async_executor,
    swarm::{MpcSwarmClient, MpcP2pRequest, MpcP2pResponse}, 
    serde_support::{decode_payload, encode_payload, encode_key, encode_signature}, 
    error::{MpcNodeError, MpcProtocolError, NodeError}, 
};

use crate::node::client_outcome::ClientOutcome;

type KeyGenMessage = Msg<keygen::ProtocolMessage>;
type SignOfflineMessage = Msg<sign::OfflineProtocolMessage>;
type PartialSignatureMessage = Msg<PartialSignature>;

#[cfg(feature = "light-node")]
use skw_mpc_payload::AuthHeader;

// 'node should be the same as 'static for most of the time
pub struct JobManager<'node> {
    local_peer_id: PeerId,
    client: &'node mut MpcSwarmClient,

    // Protocol IO For KeyGen
    keygen_protocol_incoming_channel: HashMap<CryptoHash, mpsc::Sender<Result<Payload<KeyGenMessage>, std::io::Error>>>,
    keygen_outgoing_sender: mpsc::UnboundedSender<Payload<KeyGenMessage>>,

    // Protocol IO For SignOffline
    sign_offline_protocol_incoming_channel: HashMap<CryptoHash, mpsc::Sender<Result<Payload<SignOfflineMessage>, std::io::Error>>>,
    sign_offline_outgoing_sender: mpsc::UnboundedSender<Payload<SignOfflineMessage>>,

    sign_fianlize_partial_signature_incoming_channel: HashMap<CryptoHash, mpsc::Sender<Result<Payload<PartialSignatureMessage>, std::io::Error>>>,
    sign_fianlize_partial_signature_outgoing_sender: mpsc::UnboundedSender<Payload<PartialSignatureMessage>>,
}

impl<'node> JobManager<'node> {
    pub fn new(
        local_peer_id: PeerId,
        client: &'node mut MpcSwarmClient,

        keygen_outgoing_sender: mpsc::UnboundedSender<Payload<KeyGenMessage>>,
        sign_offline_outgoing_sender: mpsc::UnboundedSender<Payload<SignOfflineMessage>>,
        sign_fianlize_partial_signature_outgoing_sender: mpsc::UnboundedSender<Payload<PartialSignatureMessage>>,
    ) -> Self {
        Self {
            local_peer_id,

            client,

            keygen_protocol_incoming_channel: Default::default(),            
            keygen_outgoing_sender,

            sign_offline_protocol_incoming_channel: Default::default(),
            sign_offline_outgoing_sender,
            sign_fianlize_partial_signature_incoming_channel: Default::default(),
            sign_fianlize_partial_signature_outgoing_sender,
        }
    }

    #[cfg(feature = "light-node")]
    pub async fn init_new_job(&mut self, 
        new_auth_header: AuthHeader, 
        new_header: PayloadHeader,
    ) -> Result<(), MpcNodeError> {
        for (peer, peer_addr) in new_header.clone().peers.iter() {    
            if peer.clone() != self.local_peer_id.clone() {
                self.client.dial(peer.clone(), peer_addr.clone()).await?;
                self.client.send_request( peer.clone(), 
                    MpcP2pRequest::StartJob { 
                        auth_header: new_auth_header.clone(),
                        job_header: new_header.clone(), 
                    }
                ).await?;
            }
        }
        Ok(())
    }

    pub fn keygen_accept_new_job(&mut self, 
        new_header: PayloadHeader,
        result_sender: oneshot::Sender<Result<ClientOutcome, MpcNodeError>>,
    ) {
        let job_id = new_header.clone().payload_id;
        let local_peer_id = self.local_peer_id.clone();
        let (incoming_sender, incoming_receiver) = mpsc::channel(2);
        let outgoing_sender = self.keygen_outgoing_sender.clone();
        self.keygen_protocol_incoming_channel.insert(job_id, incoming_sender.clone());

        // spin up the thread to handle these tasks
        async_executor(async move {
            let local_index = new_header.peers.iter()
                .position(|p| p.0.clone() == local_peer_id)
                .unwrap()
                .saturating_add(1);

            match keygen::Keygen::new(
                local_index.try_into().unwrap(), 
                new_header.t.saturating_sub(1), // we need to sub t by 1 - ref to kzen-curv's VSS impl
                new_header.n
            ) {
                Ok(keygen_sm) => {
                    match AsyncProtocol::new(keygen_sm, 
                        incoming_receiver, outgoing_sender,
                        new_header.clone()
                    )
                        .run()
                        .await
                    {
                        Ok(local_key) => {
                            result_sender
                            .send(Ok(ClientOutcome::KeyGen {
                                peer_id: local_peer_id,
                                payload_id: new_header.payload_id,
                                local_key: encode_key(&local_key)
                            }))
                            .expect("result_receiver not to be dropped")
                        },
                        Err(e) => {
                            result_sender
                                .send(Err(MpcNodeError::MpcProtocolError(MpcProtocolError::KeyGenError(e.to_string()))))
                                .expect("result_receiver not to be dropped");
                        }
                    }
                },
                Err(e) => {
                    result_sender
                        .send(Err(MpcNodeError::MpcProtocolError(MpcProtocolError::KeyGenError(e.to_string()))))
                        .expect("result_receiver not to be dropped");
                }
            }
        });
    }

    pub async fn sign_accept_new_job(&mut self, 
        new_header: PayloadHeader, 
        
        local_key: LocalKey<Secp256k1>,
        keygen_peers: Vec<(PeerId, Multiaddr)>,

        message: CryptoHash,
        
        result_sender: oneshot::Sender<Result<ClientOutcome, MpcNodeError>>,
    ) {
        let job_id = new_header.clone().payload_id;
        let local_peer_id = self.local_peer_id.clone();

        let (incoming_sender, incoming_receiver) = mpsc::channel(2);
        let (incoming_partial_sig_sender, incoming_partial_sig_receiver) = mpsc::channel(2);

        let outgoing_sender = self.sign_offline_outgoing_sender.clone();
        let sign_fianlize_partial_signature_outgoing_sender = self.sign_fianlize_partial_signature_outgoing_sender.clone();

        self.sign_fianlize_partial_signature_incoming_channel.insert(job_id, incoming_partial_sig_sender.clone());
        self.sign_offline_protocol_incoming_channel.insert(job_id, incoming_sender.clone());

        // spin up the thread to handle these tasks
        async_executor(async move {
            let local_index: u16 = keygen_peers.iter()
                .position(|p| p.0.clone() == local_peer_id)
                .unwrap()
                .saturating_add(1)
                .try_into().unwrap();
            
            let mut peers_index = Vec::<u16>::new();
            for current_peer in new_header.peers.iter() {
                let peer_index = keygen_peers.iter()
                    .position(|p| p.0.clone() == current_peer.0)
                    .unwrap()
                    .saturating_add(1)
                    .try_into().unwrap();
                peers_index.push(peer_index);
            }

            match sign::OfflineStage::new(
                local_index, peers_index, local_key
            ) {
                Ok(offline_sign_sm) => {
                    match AsyncProtocol::new(offline_sign_sm, 
                        incoming_receiver, outgoing_sender,
                        new_header.clone()
                    )
                        .run()
                        .await
                    {
                        Ok(completed_offline_stage) => {
                            match SignManual::new(
                                BigInt::from_bytes(&message[..]), 
                                completed_offline_stage
                            ) {
                                Ok((signing, partial_signature)) => {
                                    let mut sign_fianlize_header = new_header.clone();
                                    sign_fianlize_header.payload_type = PayloadType::SignFinalize;
                        
                                    sign_fianlize_partial_signature_outgoing_sender
                                        .unbounded_send(Payload { 
                                            payload_header: sign_fianlize_header, 
                                            body: Msg {
                                                sender: local_index, receiver: None,
                                                body: partial_signature
                                            }
                                        })
                                        .expect("sign_fianlize_partial_signature_outgoing_sender channel should not be dropped");
                                    
                                    // let partial_sigs_payload: Vec<Payload<PartialSignatureMessage>> = 
                                    match incoming_partial_sig_receiver
                                        .take(new_header.clone().peers.len() - 1)
                                        .try_collect::<Vec<Payload<PartialSignatureMessage>>>()
                                        .await
                                    {
                                        Ok(partial_sigs_payload) => {
                                            let partial_sigs: Vec<PartialSignature> = partial_sigs_payload
                                                .iter()
                                                .map(|p| p.body.clone().body)
                                                .collect();
                                            match signing
                                                .complete(&partial_sigs)
                                            {
                                                Ok(sig) => {
                                                    result_sender
                                                        .send(Ok(ClientOutcome::Sign {
                                                            peer_id: local_peer_id,
                                                            payload_id: new_header.payload_id,
                                                            sig: encode_signature(&sig),
                                                        }))
                                                        .expect("result_receiver not to be dropped");
                                                },
                                                Err(e) => {
                                                    result_sender
                                                        .send(Err(MpcNodeError::MpcProtocolError(MpcProtocolError::SignError(e.to_string()))))
                                                        .expect("result_receiver not to be dropped")
                                                }
                                            }
                                        },
                                        Err(e) => result_sender
                                            .send(Err(MpcNodeError::MpcProtocolError(MpcProtocolError::SignError(e.to_string()))))
                                            .expect("result_receiver not to be dropped")
                                    }; 
                                },
                                Err(e) => result_sender
                                    .send(Err(MpcNodeError::MpcProtocolError(MpcProtocolError::SignError(e.to_string()))))
                                    .expect("result_receiver not to be dropped")
                            };
                        },
                        Err(e) => result_sender
                                .send(Err(MpcNodeError::MpcProtocolError(MpcProtocolError::SignError(e.to_string()))))
                                .expect("result_receiver not to be dropped")
                    }
                },
                Err(e) => result_sender
                    .send(Err(MpcNodeError::MpcProtocolError(MpcProtocolError::SignError(e.to_string()))))
                    .expect("result_receiver not to be dropped")
            };
        });
    }

    pub async fn handle_incoming(&mut self,
        raw_payload: &[u8],
    ) -> Result<(), MpcNodeError> {
        // Note: currently - we try to guess the type of the payload ... there might be another way
        let maybe_payload_keygen: Result<Payload<KeyGenMessage>, MpcNodeError> = decode_payload(raw_payload.clone());
        let maybe_payload_sign_offline: Result<Payload<SignOfflineMessage>, MpcNodeError> = decode_payload(raw_payload.clone());
        let maybe_payload_partial_sig: Result<Payload<PartialSignatureMessage>, MpcNodeError> = decode_payload(raw_payload.clone());

        if maybe_payload_keygen.is_ok() {
            let payload = maybe_payload_keygen.unwrap();
            let job_id = &payload.payload_header.payload_id;
            let channel = self.keygen_protocol_incoming_channel.get_mut(job_id);
            match channel {
                Some(pipe) => {
                    pipe.try_send(Ok(payload))
                        .expect("protocol_incoming_channels should not be dropped");
                },
                None => {
                    panic!("unknown job");
                }
            }
        } else if maybe_payload_sign_offline.is_ok() {
            let payload = maybe_payload_sign_offline.unwrap();
            let job_id = &payload.payload_header.payload_id;
            let channel = self.sign_offline_protocol_incoming_channel.get_mut(job_id);
            match channel {
                Some(pipe) => {
                    pipe.try_send(Ok(payload))
                        .expect("protocol_incoming_channels should not be dropped");
                },
                None => {
                    panic!("unknown job");
                }
            }
        } else if maybe_payload_partial_sig.is_ok() {
            let payload = maybe_payload_partial_sig.unwrap();
            let job_id = &payload.payload_header.payload_id;
            let channel = self.sign_fianlize_partial_signature_incoming_channel.get_mut(job_id);
            match channel {
                Some(pipe) => {
                    pipe.try_send(Ok(payload))
                        .expect("protocol_incoming_channels should not be dropped");
                },
                None => {
                    panic!("unknown job");
                }
            }
        } else {
            return Err(MpcNodeError::NodeError(NodeError::InputUnknown));
        }

        Ok(())
    }


    pub async fn handle_outgoing<M>(&mut self, 
        payload: Payload<Msg<M>>,
    ) -> Result<(), MpcNodeError>
        where M: Clone + Serialize + DeserializeOwned
    {
        let local_peer_id = self.local_peer_id.clone();

        match payload.body.receiver {
            // this is a p2p message - only one receiver is assigned
            Some(to) => {
                if to < 1 && to > payload.payload_header.peers.len() as u16 {
                    return Err(MpcNodeError::NodeError(NodeError::InvalidOutgoingParameter));
                }
                let to_peer = payload.payload_header.peers[(to - 1) as usize].clone();

                self.client
                    .dial(to_peer.0, to_peer.1)
                    .await?;
                
                let mut payload_out = payload.clone();
                payload_out.payload_header.sender = local_peer_id;
                if let MpcP2pResponse::RawMessage { status } = self.client
                    .send_request(to_peer.0, MpcP2pRequest::RawMessage { 
                        payload: encode_payload(&payload_out)
                        })
                    .await? 
                {
                    status?;
                } else {
                    unreachable!()
                }
            },
            // this is a broadcast message
            None => {
                for peer in payload.clone().payload_header.peers {
                    if peer.0.to_string() != self.local_peer_id.to_string() {
                        self.client
                            .dial(peer.0, peer.1)
                            .await?;

                        let mut payload_out = payload.clone();
                        payload_out.payload_header.sender = local_peer_id;
                        if let MpcP2pResponse::RawMessage { status } = self.client
                            .send_request(peer.0, MpcP2pRequest::RawMessage { 
                                payload: encode_payload(&payload_out)
                                })
                            .await? 
                        {
                            status?;
                        } else {
                            unreachable!()
                        }
                    }
                }
            }
        }
    
        Ok(())
    }

}