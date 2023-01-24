use std::collections::{HashMap, hash_map::Entry};

use libp2p::{
    swarm::{SwarmEvent, ConnectionHandlerUpgrErr}, PeerId,
    Swarm,
    multiaddr, Multiaddr, 
    request_response::{self, RequestId,}, 
};
use futures::{StreamExt, FutureExt, SinkExt};
use futures::channel::{oneshot, mpsc};

use skw_mpc_payload::{PayloadHeader};
use crate::{
    behavior::{MpcNodeBahavior, MpcNodeBahaviorEvent, MpcP2pRequest, MpcP2pResponse}, 
    client::MpcNodeCommand, 
    error::MpcNodeError,
};

pub struct MpcNodeEventLoop {
    // the p2p node
    node: Swarm<MpcNodeBahavior>,

    // The incoming message channel
    // Sender: MpcNodeEventLoop
    // Receiver: MpcNode
    node_incoming_message_sender: mpsc::Sender< Vec<u8> >,
    node_incoming_job_sender: mpsc::Sender <PayloadHeader>,

    // the command receiver
    // Sender: MpcNodeClient
    // Receiver: MpcNodeEventLoop
    command_receiver: mpsc::Receiver<MpcNodeCommand>,

    pub known_peers: HashMap<PeerId, (Multiaddr, bool)>, // PeerId -> (Address, if_in_use)
    
    pending_dial: HashMap<PeerId, oneshot::Sender<Result<(), MpcNodeError>>>,
    pending_request: HashMap<RequestId, oneshot::Sender<Result<MpcP2pResponse, MpcNodeError>>>,
}

impl MpcNodeEventLoop {
    pub fn new(
        node: Swarm<MpcNodeBahavior>,

        node_incoming_message_sender: mpsc::Sender< Vec<u8> >,
        node_incoming_job_sender: mpsc::Sender <PayloadHeader>,
    
        command_receiver: mpsc::Receiver<MpcNodeCommand>,
    ) -> Self {
        Self {
            node,

            node_incoming_message_sender, node_incoming_job_sender,
            
            command_receiver, 

            known_peers: Default::default(),

            pending_dial: Default::default(),
            pending_request: Default::default(),
        }
    }

    pub async fn run(mut self) -> Result<(), MpcNodeError> {
        loop {
            futures::select! {
                // events are INCOMING Streams for the node raw events
                event = self.node.next().fuse() => {
                    self.handle_event(event.expect("always have an event")).await;
                },

                // commands are OUTGOING Sink of events 
                command = self.command_receiver.select_next_some() => {
                    match self.handle_command(command).await {
                        Ok(()) => {},
                        Err(e) => eprintln!("{:?}", e)
                    }
                }
            }
        }
    }

    async fn handle_event(
        &mut self,
        event: SwarmEvent<
            MpcNodeBahaviorEvent, ConnectionHandlerUpgrErr<std::io::Error>,
        >,
    ) {

        // eprintln!("{:?}", event);

        match event {
            // general network
            SwarmEvent::NewListenAddr { address, .. } => {
                let local_peer_id = *self.node.local_peer_id();
                eprintln!(
                    "Local node is listening on {:?}",
                    address.with(multiaddr::Protocol::P2p(local_peer_id.into()))
                );
            }
            SwarmEvent::IncomingConnection { .. } => {}
            SwarmEvent::ConnectionEstablished {
                peer_id, endpoint, ..
            } => {
                println!("Established Connection {:?} {:?}", peer_id, endpoint);
                if endpoint.is_dialer() {
                    if let Some(sender) = self.pending_dial.remove(&peer_id) {
                        let _ = sender.send(Ok(()));
                    }
                } else {
                    self.known_peers.insert(
                        peer_id, (endpoint.get_remote_address().clone(), false)
                    );
                    println!("{:?}", self.known_peers);
                }
            }
            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                println!("{:?} Disconnected", peer_id);
                self.known_peers.remove(&peer_id);
            }
            SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                if let Some(peer_id) = peer_id {
                    if let Some(sender) = self.pending_dial.remove(&peer_id) {

                        eprintln!("Dialing Error {:?}", error);
                        let _ = sender.send(Err(MpcNodeError::FailToDial));
                    }
                }
            }
            SwarmEvent::IncomingConnectionError { .. } => {}
            SwarmEvent::Dialing(peer_id) => eprintln!("Dialing {peer_id}"),

            // p2p events
            SwarmEvent::Behaviour(MpcNodeBahaviorEvent::RequestResponse(
                request_response::Event::Message { message, .. },
            )) => match message {
                
                // p2p message request hanlder
                request_response::Message::Request {
                    request, channel, ..
                } => {

                    println!("Request Received {:?}", request);

                    match request {
                        MpcP2pRequest::StartJob { auth_header, job_header } => {
                            
                            let validate_nodes = |_nodes: Vec<PeerId>| -> bool {
                                // nodes.iter()
                                //     .all(|peer| {
                                //         self.known_peers.contains_key(peer)
                                //     })

                                true
                            };
                            println!("{:?}", validate_nodes(job_header.peers.clone()));


                            // if the auth_header is invalid - send error
                            // if !auth_header.validate() {
                            if !true {
                                self.node
                                    .behaviour_mut()
                                    .request_response
                                    .send_response(channel, MpcP2pResponse::StartJob { 
                                        status: Err(MpcNodeError::P2pBadAuthHeader)
                                    })
                                    .unwrap(); // TODO: this unwrap is not correct
                            } else if validate_nodes(job_header.peers.clone()) {
                                self.node
                                    .behaviour_mut()
                                    .request_response
                                    .send_response(channel, MpcP2pResponse::StartJob { 
                                        status: Ok(())
                                    })
                                    .unwrap(); // TODO: this unwrap is not correct

                                self.node_incoming_job_sender
                                    .send(job_header )
                                    .await
                                    .expect("node_incoming_job_sender should not be dropped. qed.");
                            } else {
                                self.node
                                    .behaviour_mut()
                                    .request_response
                                    .send_response(channel, MpcP2pResponse::StartJob { 
                                        status: Err(MpcNodeError::P2pUnknownPeers)
                                    })
                                    .unwrap(); // TODO: this unwrap is not correct
                            }
                        },

                        MpcP2pRequest::RawMessage { payload } => {
                            // TODO: Handle errors correctly
                            self.node_incoming_message_sender
                                .send( payload ) // TODO: this is an unsafe unwrap
                                .await
                                .expect("node_incoming_job_sender should not be dropped. qed.");
                        },
                    }
                }

                // p2p message response hanlder
                request_response::Message::Response {
                    request_id,
                    response,
                } => {
                    println!("Sending Response {:?}", response);
                    // TODO: handle Err from `let Err(e) = response.status`
                    let _ = self
                        .pending_request
                        .remove(&request_id)
                        .expect("Request to still be pending.")
                        .send(Ok(response));
                }
            },

            // p2p message misc handler
            SwarmEvent::Behaviour(MpcNodeBahaviorEvent::RequestResponse(
                request_response::Event::OutboundFailure {
                    request_id, error, ..
                },
            )) => {

                eprintln!("p2p outbound request failure {:?}", error);
                let _ = self
                    .pending_request
                    .remove(&request_id)
                    .expect("Request to still be pending.")
                    .send(Err(MpcNodeError::P2pOutboundFailure));
            }
            SwarmEvent::Behaviour(MpcNodeBahaviorEvent::RequestResponse(
                request_response::Event::ResponseSent { .. },
            )) => {},
            
            _ => {}
        }
    }

    async fn handle_command(&mut self, request: MpcNodeCommand) -> Result<(), MpcNodeError> {

        // eprintln!("Command {:?}", request);
        match request {
            MpcNodeCommand::StartListening { addr, result_sender } => {
                match self.node.listen_on(addr) {
                    Ok(_) => {
                        result_sender.send(Ok(()))
                    },
                    Err(_e )=> result_sender.send(Err(MpcNodeError::FailToListenOnPort)),
                }.map_err(|_| MpcNodeError::FailToSendViaChannel)
            },
            MpcNodeCommand::Dial { peer_id, peer_addr, result_sender } => {
                let mut dial = |peer_id, peer_addr: Multiaddr, result_sender| {
                    if let Entry::Vacant(e) = self.pending_dial.entry(peer_id) {
                        match self
                            .node
                            .dial(peer_addr.with(multiaddr::Protocol::P2p(peer_id.into())))
                        {
                            Ok(()) => {
                                e.insert(result_sender);
                                Ok(())
                            }
                            Err(_) => {
                                result_sender.send(Err(MpcNodeError::FailToDial))
                                    .map_err(|_| MpcNodeError::FailToSendViaChannel)
                            }
                        }
                    } else {
                        todo!("Already dialing peer.");
                    }
                };
                
                match peer_addr {
                    Some(peer_addr) => {
                        dial(peer_id, peer_addr, result_sender)
                    },
                    None => {
                        let peer_record = self.known_peers.get(&peer_id);
                        match peer_record {
                            Some((addr, _)) => {

                                println!("Got a NOP {:?}", addr.clone());
                                result_sender.send(Ok(())).expect("Sender not to be drooped");

                                Ok(()) // Do Nothing
                                // dial(peer_id, addr.clone(), result_sender)
                            },
                            None => Err(MpcNodeError::P2pUnknownPeers)
                        }
                    }
                }
            },
            MpcNodeCommand::SendP2pRequest { to, request, result_sender } => {
                let request_id = self.node
                    .behaviour_mut()
                    .request_response
                    .send_request(&to, request.clone());
                self.pending_request.insert(request_id, result_sender);

                Ok(())
            }
        }
    }
}
