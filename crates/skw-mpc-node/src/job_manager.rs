use std::collections::HashMap;

use futures::{SinkExt, channel::mpsc};

use libp2p::{PeerId, Multiaddr};
use skw_mpc_payload::{CryptoHash, PayloadHeader, Payload, AuthHeader};
use skw_round_based::{async_runtime::AsyncProtocol, Msg};

use skw_mpc_protocol::gg20::state_machine::{keygen, sign};

use crate::node::{MpcNodeClient, MpcP2pRequest};

type KeyGenMessage = Msg<keygen::ProtocolMessage>;

// 'node should be the same as 'static for most of the time
pub struct JobManager<'node> {
    local_peer_id: PeerId,
    headers: HashMap<CryptoHash, PayloadHeader>, // Do we really need this?

    client: &'node mut MpcNodeClient,

    // Protocol IO
    protocol_incoming_channel: HashMap<CryptoHash, mpsc::Sender<Result<Payload<KeyGenMessage>, std::io::Error>>>,
    
    pub main_outgoing_sender: mpsc::UnboundedSender<Payload<KeyGenMessage>>,
    pub main_outgoing_receiver: mpsc::UnboundedReceiver<Payload<KeyGenMessage>>,
}

impl<'node> JobManager<'node> {
    pub fn new(
        local_peer_id: PeerId,
        client: &'node mut MpcNodeClient,
    ) -> Self {
        let (main_outgoing_sender, main_outgoing_receiver) = mpsc::unbounded();
        Self {
            local_peer_id,
            headers: Default::default(),

            client,

            protocol_incoming_channel: Default::default(),
            main_outgoing_sender,
            main_outgoing_receiver, 
        }
    }

    pub async fn keygen_init_new_job(&mut self, 
        new_auth_header: AuthHeader, 
        new_header: PayloadHeader,
    ) {
        for (peer, peer_addr) in new_header.clone().peers.iter() {

            // println!("Sending Out to {:?}", peer);
    
            if peer.clone() != self.local_peer_id.clone() {
                self.client.dial(peer.clone(), peer_addr.clone())
                    .await
                    .expect("dailing to be not failed");
                self.client.send_request( peer.clone(), 
                    MpcP2pRequest::StartJob { 
                        auth_header: new_auth_header.clone(),
                        job_header: new_header.clone(), 
                    }
                )
                    .await
                    .expect("request should be taken");
    
                println!("Sending Out to {:?} Done", peer);
            }
        }
        self.keygen_accept_new_job(new_header.clone());
    }

    pub fn keygen_accept_new_job(&mut self, new_header: PayloadHeader) {
        let job_id = new_header.clone().payload_id;
        let local_peer_id = self.local_peer_id.clone();

        let (incoming_sender, incoming_receiver) = mpsc::channel(2);
        let outgoing_sender = self.main_outgoing_sender.clone();
        self.headers.insert(job_id, new_header.clone());
        self.protocol_incoming_channel.insert(job_id, incoming_sender.clone());

        // spin up the thread to handle these tasks
        async_std::task::spawn(async move {
            let local_index = new_header.peers.iter()
                .position(|p| p.0.clone() == local_peer_id)
                .unwrap()
                .saturating_add(1);

            let keygen_sm = keygen::Keygen::new(local_index.try_into().unwrap(), new_header.t, new_header.n)
                .map_err(|e| { println!("Protocl Error {:?}", e) })
                .unwrap();
            let output = AsyncProtocol::new(keygen_sm, 
                incoming_receiver, 
                outgoing_sender,
                new_header.clone()
            )
                .run()
                .await; // TODO: discard all error?

            println!("{:?}", output);
        });
    }

    pub async fn handle_outgoing(&mut self, 
        payload: Payload<KeyGenMessage>,
    ) {

        // println!("Outgoing {:?} {:?} {:?}", payload.payload_header, payload.body, payload.body.receiver);
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
                        payload: bincode::serialize( &payload_out ).unwrap()
                     })
                    .await
                    .expect("client should not be dropped, node should take in this request");
            },
            // this is a broadcast message
            None => {
                for peer in payload.clone().payload_header.peers {
                    if peer.0.to_string() != self.local_peer_id.to_string() {
                        self.client
                            .dial(peer.0, peer.1)
                            .await
                            .expect("client should not be dropped");
                        
                        let mut payload_out = payload.clone();
                        payload_out.payload_header.sender = local_peer_id;
                        self.client
                            .send_request(peer.0, MpcP2pRequest::RawMessage { 
                                payload: bincode::serialize(&payload_out).unwrap() 
                            })
                            .await
                            .unwrap();
                            // .expect("node should take in these requests");
                    }
                }
            }
        }
    }

    pub async fn handle_incoming(&mut self,
        payload: Payload<KeyGenMessage>,
    ) {
        // println!("Incoming Payload {:?}", payload.payload_header);
        let job_id = &payload.payload_header.payload_id;
        let channel = self.protocol_incoming_channel.get_mut(job_id);
        match channel {
            Some(pipe) => {
                // println!("Incoming channel found, forwarding msg to state machine");

                pipe.try_send(Ok(payload))
                    .expect("protocol_incoming_channels should not be dropped");
                // println!("msg received by state machine");
            },
            None => {
                panic!("unknown job");
            }
        }
    }
}