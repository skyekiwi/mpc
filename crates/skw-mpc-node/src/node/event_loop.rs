use std::collections::{HashMap, hash_map::Entry};

use libp2p::{
    swarm::{SwarmEvent, ConnectionHandlerUpgrErr}, PeerId,
    Swarm,
    multiaddr, 
    request_response::{self, RequestId,}, Multiaddr, 
};
use futures::{StreamExt, FutureExt, SinkExt};
use futures::channel::{oneshot, mpsc};

use skw_mpc_payload::{PayloadHeader};
use super::{
    behavior::{MpcNodeBahavior, MpcNodeBahaviorEvent, MpcP2pRequest, MpcP2pResponse}, 
    client::MpcNodeCommand,
};

use crate::error::MpcNodeError;

pub struct MpcNodeEventLoop {
    node: Swarm<MpcNodeBahavior>,

    node_incoming_message_sender: mpsc::UnboundedSender< Vec<u8> >,
    node_incoming_job_sender: mpsc::Sender <PayloadHeader>,
    command_receiver: mpsc::UnboundedReceiver<MpcNodeCommand>,

    pending_dial: HashMap<PeerId, oneshot::Sender<Result<(), MpcNodeError>>>,
    pending_request: HashMap<RequestId, oneshot::Sender<Result<MpcP2pResponse, MpcNodeError>>>,
    
    listen_to_addr_sender: mpsc::Sender< Multiaddr >,
}

impl MpcNodeEventLoop {
    pub fn new(
        node: Swarm<MpcNodeBahavior>,

        node_incoming_message_sender: mpsc::UnboundedSender< Vec<u8> >,
        node_incoming_job_sender: mpsc::Sender <PayloadHeader>,
    
        command_receiver: mpsc::UnboundedReceiver<MpcNodeCommand>,
        
        listen_to_addr_sender: mpsc::Sender< Multiaddr >,
    ) -> Self {
        Self {
            node,

            node_incoming_message_sender, node_incoming_job_sender,
            
            command_receiver, 

            pending_dial: Default::default(),
            pending_request: Default::default(),
            listen_to_addr_sender,
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
        match event {
            // general network
            SwarmEvent::NewListenAddr { address, .. } => {
                let local_peer_id = self.node.local_peer_id().clone();
                self.listen_to_addr_sender
                    .send(address.clone().with(multiaddr::Protocol::P2p(local_peer_id.into())))
                    .await
                    .unwrap();
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
            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                // println!("{:?} Disconnected", peer_id);
                // TODO: handle connect close
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
                    match request {
                        MpcP2pRequest::StartJob { auth_header, job_header } => {
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
                            } else {
                                for (peer, address) in job_header.peers.iter() {
                                    self.node
                                        .behaviour_mut()
                                        .request_response
                                        .add_address(peer, address.clone());
                                }

                                self.node
                                    .behaviour_mut()
                                    .request_response
                                    .send_response(channel, MpcP2pResponse::StartJob { 
                                        status: Ok(())
                                    })
                                    .unwrap(); // TODO: this unwrap is not correct

                                self.node_incoming_job_sender
                                    .try_send(job_header )
                                    .expect("node_incoming_job_sender should not be dropped. qed.");
                            }
                        },

                        MpcP2pRequest::RawMessage { payload } => {
                            // println!("Received Request RawMesage");
                            self.node_incoming_message_sender
                                .unbounded_send( payload )
                                .expect("node_incoming_job_sender should not be dropped. qed.");

                            self.node
                                .behaviour_mut()
                                .request_response
                                .send_response(channel, MpcP2pResponse::RawMessage { 
                                    status: Ok(())
                                })
                                .unwrap(); // TODO: this unwrap is not correct
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
                    request_id, error, peer,
                },
            )) => {

                eprintln!("p2p outbound request failure {:?} {:?}", error, peer);
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
        match request {
            MpcNodeCommand::StartListening { addr, result_sender } => {
                match self.node.listen_on(addr) {
                    Ok(_) => {
                        result_sender.send(Ok(()))
                            .expect("sender should not be dropped");
                        Ok(())
                    },
                    Err(_e )=> result_sender.send(Err(MpcNodeError::FailToListenOnPort)),
                }.map_err(|_| MpcNodeError::FailToSendViaChannel)
            },
            MpcNodeCommand::Dial { peer_id, peer_addr, result_sender } => {
                if let Entry::Vacant(e) = self.pending_dial.entry(peer_id) {
                    self.node
                        .behaviour_mut()
                        .request_response
                        .add_address(&peer_id, peer_addr.clone());

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
