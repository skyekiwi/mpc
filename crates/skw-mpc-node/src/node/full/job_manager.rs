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
    swarm::{MpcSwarmClient, MpcP2pRequest}, 
    serde_support::{decode_payload, encode_payload, encode_key, encode_signature}, error::MpcNodeError, async_executor
};

use crate::node::client_outcome::ClientOutcome;

type KeyGenMessage = Msg<keygen::ProtocolMessage>;
type SignOfflineMessage = Msg<sign::OfflineProtocolMessage>;
type PartialSignatureMessage = Msg<PartialSignature>;


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

            let keygen_sm = keygen::Keygen::new(
                local_index.try_into().unwrap(), 
                new_header.t.saturating_sub(1), // we need to sub t by 1 - ref to kzen-curv's VSS impl
                new_header.n
            )
                .map_err(|e| { println!("Protocl Error {:?}", e) })
                .unwrap();
            let output = AsyncProtocol::new(keygen_sm, 
                incoming_receiver, 
                outgoing_sender,
                new_header.clone()
            )
                .run()
                .await; // TODO: discard all error?

            result_sender
                .send(Ok(ClientOutcome::KeyGen {
                    peer_id: local_peer_id,
                    payload_id: new_header.payload_id,
                    local_key: encode_key(&output.unwrap())
                }))
                .expect("result_receiver not to be dropped");
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

            let offline_sign = sign::OfflineStage::new(
                local_index,
                peers_index,
                local_key
            )
                .map_err(|e| { println!("Protocl Error {:?}", e) })
                .unwrap();

            let output = AsyncProtocol::new(offline_sign, 
                incoming_receiver, 
                outgoing_sender,
                new_header.clone()
            )
                .run()
                .await // TODO: discard all error?
                .unwrap();

            let (signing, partial_signature) = SignManual::new(
                BigInt::from_bytes(&message[..]), 
                output
            )
                .unwrap();

            let mut sign_fianlize_header = new_header.clone();
            sign_fianlize_header.payload_type = PayloadType::SignFinalize;

            sign_fianlize_partial_signature_outgoing_sender
                .unbounded_send(Payload { 
                    payload_header: sign_fianlize_header, 
                    body: Msg {
                        sender: local_index,
                        receiver: None,
                        body: partial_signature
                    }
                })
                .expect("sign_fianlize_partial_signature_outgoing_sender channel should not be dropped");
            
            let partial_sigs_payload: Vec<Payload<PartialSignatureMessage>> = incoming_partial_sig_receiver
                .take(new_header.clone().peers.len() - 1)
                .try_collect()
                .await
                .unwrap();
            let partial_sigs: Vec<PartialSignature> = partial_sigs_payload
                .iter()
                .map(|p| p.body.clone().body)
                .collect();
        
            let signature = signing
                .complete(&partial_sigs)
                .map_err(|e| {
                    println!("sign failure online {:?}", e);
                    MpcNodeError::P2pBadPayload
                })
                .unwrap(); // TODO

            result_sender
                .send(Ok(ClientOutcome::Sign {
                    peer_id: local_peer_id,
                    payload_id: new_header.payload_id,
                    sig: encode_signature(&signature),
                }))
                .expect("result_receiver not to be dropped");
        });
    }

    pub async fn handle_incoming(&mut self,
        raw_payload: &[u8],
    ) {
        // TODO: currently - we try to guess the type of the payload ... there might be another way
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
        } else 
        
        
        if maybe_payload_sign_offline.is_ok() {
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
        }

        if maybe_payload_partial_sig.is_ok() {
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
        }
    }


    pub async fn handle_outgoing<M>(&mut self, 
        payload: Payload<Msg<M>>,
    ) 
        where M: Clone + Serialize + DeserializeOwned
    {
        let local_peer_id = self.local_peer_id.clone();

        match payload.body.receiver {
            // this is a p2p message - only one receiver is assigned
            Some(to) => {
                assert!(to >= 1 && to <= payload.payload_header.peers.len() as u16, "wrong receiver index");
                let to_peer = payload.payload_header.peers[(to - 1) as usize].clone();
                
                self.client
                    .dial(to_peer.0, to_peer.1)
                    .await
                    .expect("client should not be dropped");
                
                let mut payload_out = payload.clone();
                payload_out.payload_header.sender = local_peer_id;
                self.client
                    .send_request(to_peer.0, MpcP2pRequest::RawMessage { 
                        payload: encode_payload(&payload_out)
                     })
                    .await
                    .expect("client should not be dropped, node should take in this request");
            },
            // this is a broadcast message
            None => {
                for peer in payload.clone().payload_header.peers {

                    println!("Outgoing to Peer {:?}", peer);
                    if peer.0.to_string() != self.local_peer_id.to_string() {
                        self.client
                            .dial(peer.0, peer.1)
                            .await
                            .expect("client should not be dropped");
                        
                        let mut payload_out = payload.clone();
                        payload_out.payload_header.sender = local_peer_id;
                        self.client
                            .send_request(peer.0, MpcP2pRequest::RawMessage { 
                                payload: encode_payload(&payload_out)
                            })
                            .await
                            .unwrap(); // TODO: 
                            // .expect("node should take in these requests");
                    }
                }
            }
        }
    }

}