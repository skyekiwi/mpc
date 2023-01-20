use std::{collections::{HashMap, hash_map::Entry}, str::FromStr};

use libp2p::{
    mdns, core::either::EitherError,
    swarm::{SwarmEvent, ConnectionHandlerUpgrErr}, PeerId,
    Swarm,
    multiaddr, Multiaddr, 
    request_response::{self, RequestId,},
};
use futures::{StreamExt, FutureExt, SinkExt};
use futures::channel::{oneshot, mpsc};

use skw_mpc_payload::{PayloadHeader};
use void::Void;
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
    node_incoming_message_sender: mpsc::Sender< (PayloadHeader, Vec<u8>) >,
    node_incoming_job_sender: mpsc::Sender <(PayloadHeader, Vec<PeerId>)>,

    // the command receiver
    // Sender: MpcNodeClient
    // Receiver: MpcNodeEventLoop
    command_receiver: mpsc::Receiver<MpcNodeCommand>,

    /* other internla state */
    known_peers: HashMap<PeerId, (Multiaddr, bool)>, // PeerId -> (Address, if_in_use)
    
    pending_dial: HashMap<PeerId, oneshot::Sender<Result<(), MpcNodeError>>>,
    pending_request: HashMap<RequestId, oneshot::Sender<Result<MpcP2pResponse, MpcNodeError>>>,
    // outgoing_results: HashMap<RequestId, oneshot::Sender<Result<MpcP2pResponse, MpcNodeError>>>,
}

impl MpcNodeEventLoop {
    pub fn new(
        node: Swarm<MpcNodeBahavior>,

        node_incoming_message_sender: mpsc::Sender< (PayloadHeader, Vec<u8>) >,
        node_incoming_job_sender: mpsc::Sender <(PayloadHeader, Vec<PeerId>)>,
    
        command_receiver: mpsc::Receiver<MpcNodeCommand>,
    ) -> Self {
        Self {
            node,

            node_incoming_message_sender, node_incoming_job_sender,
            
            command_receiver, 

            known_peers: Default::default(),

            pending_dial: Default::default(),
            pending_request: Default::default(),
            // outgoing_results: Default::default(),
        }
    }

    pub async fn run(mut self) -> Result<(), MpcNodeError>{
        loop {
            futures::select! {
                // events are INCOMING Streams for the node raw events
                event = self.node.next().fuse() => {
                    self.handle_event(event.expect("always have an event")).await;
                },

                // commands are OUTGOING Sink of events 
                command = self.command_receiver.select_next_some() => {
                    println!("{:?}", command);
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
            MpcNodeBahaviorEvent,
            EitherError<Void, ConnectionHandlerUpgrErr<std::io::Error>>,
        >,
    ) {

        eprintln!("{:?}", event);

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
                if endpoint.is_dialer() {
                    if let Some(sender) = self.pending_dial.remove(&peer_id) {
                        let _ = sender.send(Ok(()));
                    }
                }
            }
            SwarmEvent::ConnectionClosed { .. } => {}
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

            // mDNS passive local node discovery
            SwarmEvent::Behaviour(MpcNodeBahaviorEvent::Mdns(mdns_event)) => {
                match mdns_event {
                    mdns::Event::Discovered(list) => {
                        for (peer_id, multiaddr) in list {
                            println!("mDNS discovered a new peer: {peer_id} {multiaddr}");
                            
                            // TODO: is this matching necessary?
                            match self.known_peers.insert(
                                peer_id, (multiaddr, false)
                            ) {
                                None => {},
                                Some(old_value) => {
                                    eprint!("old_value of known_peer {:?} - {:?}", peer_id, old_value);
                                    // unexpected!
                                }
                            };
                        }
                    },
                    mdns::Event::Expired(list) => {
                        // for (peer_id, multiaddr) in list {
                        //     println!("mDNS discover peer has expired: {peer_id} {multiaddr}");
                            
                        //     // TODO: is this matching necessary?
                        //     match self.known_peers.remove(&peer_id) {
                        //         Some(v) => {
                        //             // TODO: handle if the node is still in use
                        //             eprintln!("Value Removed from Known_Peers {:?}", v);
                        //         },
                        //         None => {
                        //             eprintln!("peer_id not found in known_peers {:?}", peer_id);
                        //         }
                        //     }
                        // }
                    }
                }
            },

            // p2p events
            SwarmEvent::Behaviour(MpcNodeBahaviorEvent::RequestResponse(
                request_response::Event::Message { message, .. },
            )) => match message {
                
                // p2p message request hanlder
                request_response::Message::Request {
                    request, channel, ..
                } => {
                    match request {
                        MpcP2pRequest::StartJob { auth_header, job_header, nodes } => {
                            let mut validate_nodes = |nodes: Vec<String>| -> Option<Vec<PeerId>> {
                                nodes.iter()
                                    .fold(Some(Vec::new()), |res, node| {
                                        
                                        // if we have not invalidate things yet. - therefore res is Some()
                                        if let Some(mut inner_vec) = res {
                                            let peer = PeerId::from_str(&node);

                                            // if the peer can be correctly parsed as PeerId
                                            if let Ok(peer) = peer {

                                                // if this peer is in our list of known_peers
                                                // if let Some((_, in_job)) = self.known_peers.get_mut(&peer) {
                                                    // mark the peer as in jobs
                                                    // *in_job = true;
                                                    inner_vec.push(peer);
                                                    Some(inner_vec)
                                                // } else { None }
                                            } else { None }
                                        } else { None }
                                    })
                            };


                            // if the auth_header is invalid - send error
                            // if !auth_header.validate() {
                            if false {
                                self.node
                                    .behaviour_mut()
                                    .request_response
                                    .send_response(channel, MpcP2pResponse::StartJob { 
                                        status: Err(MpcNodeError::P2pBadAuthHeader)
                                    })
                                    .unwrap(); // TODO: this unwrap is not correct
                            } else if let Some(peer_list) = validate_nodes(nodes.clone()) {
                                self.node_incoming_job_sender
                                    .send(( job_header, peer_list ))
                                    .await
                                    .expect("node_incoming_job_sender should not be dropped. qed.");

                                self.node
                                    .behaviour_mut()
                                    .request_response
                                    .send_response(channel, MpcP2pResponse::StartJob { 
                                        status: Ok(())
                                    })
                                    .unwrap(); // TODO: this unwrap is not correct
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
                            self.node_incoming_message_sender
                                .send( (payload.payload_header, payload.body) ) // payload.body is Vec<u8>
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

        eprintln!("Command {:?}", request);
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
