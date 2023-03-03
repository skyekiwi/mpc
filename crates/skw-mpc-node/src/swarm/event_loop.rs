use core::panic;
use std::collections::{HashMap, hash_map::Entry};

use libp2p::{
    swarm::{SwarmEvent, ConnectionHandlerUpgrErr}, PeerId,
    Swarm,
    multiaddr, 
    request_response::{self, RequestId,}, Multiaddr, 
};
use futures::{StreamExt, SinkExt};
use futures::channel::{oneshot, mpsc};

#[cfg(feature = "full-node")]
use skw_mpc_payload::{PayloadHeader};

use super::{
    behavior::{MpcSwarmBahavior, MpcSwarmBahaviorEvent, MpcP2pRequest, MpcP2pResponse}, 
    client::MpcSwarmCommand,
};

use crate::error::{ MpcNodeError, SwarmError, SwarmP2pError };

pub struct MpcSwarmEventLoop {
    swarm: Swarm<MpcSwarmBahavior>,

    swarm_incoming_message_sender: mpsc::UnboundedSender< Vec<u8> >,

    #[cfg(feature = "full-node")]
    swarm_incoming_job_sender: mpsc::Sender <PayloadHeader>,

    command_receiver: mpsc::UnboundedReceiver<MpcSwarmCommand>,

    pending_dial: HashMap<PeerId, oneshot::Sender<Result<(), MpcNodeError>>>,
    pending_request: HashMap<RequestId, oneshot::Sender<Result<MpcP2pResponse, MpcNodeError>>>,
    
    listen_to_addr_sender: mpsc::Sender< Multiaddr >,
    swarm_termination_receiver: mpsc::Receiver<()>,
}

impl MpcSwarmEventLoop {
    pub fn new(
        swarm: Swarm<MpcSwarmBahavior>,

        swarm_incoming_message_sender: mpsc::UnboundedSender< Vec<u8> >,

        #[cfg(feature = "full-node")]
        swarm_incoming_job_sender: mpsc::Sender <PayloadHeader>,
    
        command_receiver: mpsc::UnboundedReceiver<MpcSwarmCommand>,
        
        listen_to_addr_sender: mpsc::Sender< Multiaddr >,

        swarm_termination_receiver: mpsc::Receiver<()>,
    ) -> Self {
        Self {
            swarm,

            swarm_incoming_message_sender, 
            
            #[cfg(feature = "full-node")]
            swarm_incoming_job_sender,
            
            command_receiver, 

            pending_dial: Default::default(),
            pending_request: Default::default(),
            listen_to_addr_sender,
            swarm_termination_receiver,
        }
    }

    pub async fn run(mut self) {
        loop {
            futures::select! {
                // events are INCOMING Streams for the node raw events
                event = self.swarm.select_next_some() => {
                    self.handle_event(event).await;
                },

                // commands are OUTGOING Sink of events 
                command = self.command_receiver.select_next_some() => {
                    self.handle_command(command).await;
                },

                _ = self.swarm_termination_receiver.select_next_some() => {
                    break;
                }
            }
        }
    }

    async fn handle_event(
        &mut self,
        event: SwarmEvent<
            MpcSwarmBahaviorEvent, ConnectionHandlerUpgrErr<std::io::Error>,
        >,
    ) {
        match event {
            // general network
            SwarmEvent::NewListenAddr { address, .. } => {
                let local_peer_id = self.swarm.local_peer_id().clone();
                self.listen_to_addr_sender
                    .send(address.clone().with(multiaddr::Protocol::P2p(local_peer_id.into())))
                    .await
                    .expect("local address receiver not to be dropped");
                log::info!(
                    "Local node is listening on {:?}",
                    address.with(multiaddr::Protocol::P2p(local_peer_id.into()))
                );
            }
            SwarmEvent::ConnectionEstablished {
                peer_id, endpoint, ..
            } => {
                if endpoint.is_dialer() {
                    if let Some(sender) = self.pending_dial.remove(&peer_id) {
                        let _ = sender
                            .send(Ok(()))
                            .expect("dailing result receiver not to be dropped");
                    }
                }
            }
            SwarmEvent::ConnectionClosed { peer_id, cause, ..} => {
                log::info!("{:?} is disconnected because {:?}", peer_id, cause);
            }
            SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                if let Some(peer_id) = peer_id {
                    if let Some(sender) = self.pending_dial.remove(&peer_id) {
                        log::error!("OutgoingConnectionError Error {:?}", error);
                        sender
                            .send(Err(MpcNodeError::SwarmError(SwarmError::FailToDailPeer)))
                            .expect("dailing result receiver not to be dropped");
                    }
                }
            }
            SwarmEvent::IncomingConnectionError { .. } => {}
            SwarmEvent::Dialing(peer_id) => log::debug!("Dialing {peer_id}"),

            // p2p events
            SwarmEvent::Behaviour(MpcSwarmBahaviorEvent::RequestResponse(
                request_response::Event::Message { message, .. },
            )) => match message {
                
                // p2p message request hanlder
                request_response::Message::Request {
                    request, channel, request_id
                } => {
                    match request {
                        MpcP2pRequest::StartJob { 
                            #[cfg(feature = "full-node")]
                            job_header, 
                            #[cfg(feature = "full-node")]
                            auth_header,
                            ..
                        } => {

                            #[cfg(feature = "full-node")]
                            {
                                // if the auth_header is invalid - send error
                                if !auth_header.validate() {
                                    match self.swarm
                                        .behaviour_mut()
                                        .request_response
                                        .send_response(channel, MpcP2pResponse::StartJob { 
                                            status: Err(MpcNodeError::SwarmP2pError(SwarmP2pError::BadAuthHeader))
                                        }) 
                                    {
                                        Ok(_) => {}
                                        Err(response) => {
                                            log::debug!("Mpc StartJob Reponse channel closed {:?}", response);
                                            self
                                                .pending_request
                                                .remove(&request_id)
                                                .expect("client request channel to still be pending.")
                                                .send(Err(MpcNodeError::SwarmP2pError(SwarmP2pError::ResponseChannelClose)))
                                                .expect("p2p response receiver not to be dropped");
                                        }
                                    }
                                } else {
                                    for (peer, address) in job_header.peers.iter() {
                                        self.swarm
                                            .behaviour_mut()
                                            .request_response
                                            .add_address(peer, address.clone());
                                    }

                                    match self.swarm
                                        .behaviour_mut()
                                        .request_response
                                        .send_response(channel, MpcP2pResponse::StartJob { 
                                            status: Ok(())
                                        })
                                    {
                                        Ok(_) => self.swarm_incoming_job_sender
                                            .send(job_header)
                                            .await
                                            .expect("swarm_incoming_job_sender should not be dropped. qed."),
                                        Err(response) => {
                                            log::debug!("Mpc StartJob Reponse channel closed {:?}", response);
                                            self
                                                .pending_request
                                                .remove(&request_id)
                                                .expect("client request channel to still be pending.")
                                                .send(Err(MpcNodeError::SwarmP2pError(SwarmP2pError::ResponseChannelClose)))
                                                .expect("p2p response receiver not to be dropped");
                                        }
                                    }
                                };
                            }

                            // NOP for light node - light node client never receive StartJob Request
                            #[cfg(feature = "light-node")] 
                            {}
                        },

                        MpcP2pRequest::RawMessage { payload } => {
                            self.swarm_incoming_message_sender
                                .unbounded_send( payload )
                                .expect("swarm_incoming_message_sender should not be dropped. qed.");

                            match self.swarm
                                .behaviour_mut()
                                .request_response
                                .send_response(channel, MpcP2pResponse::RawMessage { 
                                    status: Ok(())
                                })
                            {
                                Ok(_) => {}
                                Err(response) => {
                                    log::debug!("Mpc RawMessage Reponse channel closed {:?}", response);
                                    self
                                        .pending_request
                                        .remove(&request_id)
                                        .expect("client request channel to still be pending.")
                                        .send(Err(MpcNodeError::SwarmP2pError(SwarmP2pError::ResponseChannelClose)))
                                        .expect("p2p response receiver not to be dropped");
                                }
                            }
                        },
                    }
                }

                // p2p message response hanlder
                request_response::Message::Response {
                    request_id,
                    response,
                } => {
                    self
                        .pending_request
                        .remove(&request_id)
                        .expect("Request to still be pending.")
                        .send(Ok(response))
                        .expect("p2p response receiver not to be dropped");
                }
            },

            // p2p message misc handler
            SwarmEvent::Behaviour(MpcSwarmBahaviorEvent::RequestResponse(
                request_response::Event::OutboundFailure {
                    request_id, error, peer,
                },
            )) => {
                log::error!("p2p outbound request failure to {peer} because {:?}", error);
                let _ = self
                    .pending_request
                    .remove(&request_id)
                    .expect("Request to still be pending.")
                    .send(Err(MpcNodeError::SwarmP2pError(SwarmP2pError::OutboundFailure)))
                    .expect("p2p response receiver not to be dropped");
            }
            SwarmEvent::Behaviour(MpcSwarmBahaviorEvent::RequestResponse(
                request_response::Event::ResponseSent { .. },
            )) => {},
            
            _ => {}
        }
    }

    async fn handle_command(&mut self, request: MpcSwarmCommand) {
        match request {
            MpcSwarmCommand::StartListening { addr, result_sender } => {
                let res = self.swarm.listen_on(addr)
                    .map_err(|e| {
                        log::error!("Listen Error {:?}", e);
                        MpcNodeError::SwarmError(SwarmError::FailToListenToAddress)
                    })
                    .map(|_| ());
                result_sender
                    .send(res)
                    .expect("swarm command result receiver not to be dropped");
            },
            MpcSwarmCommand::Dial { peer_id, peer_addr, result_sender } => {
                if let Entry::Vacant(e) = self.pending_dial.entry(peer_id) {
                    self.swarm
                        .behaviour_mut()
                        .request_response
                        .add_address(&peer_id, peer_addr.clone());

                    match self
                        .swarm
                        .dial(peer_addr.with(multiaddr::Protocol::P2p(peer_id.into())))
                    {
                        Ok(_) => { e.insert(result_sender); },
                        Err(error) => {
                            log::error!("Dailing Error {:?}", error);
                            result_sender.send(Err(MpcNodeError::SwarmError(SwarmError::FailToDailPeer)))
                                .expect("swarm command result receiver not to be dropped");
                        }
                    }
                } else {
                    result_sender
                        .send(Err(MpcNodeError::SwarmError(SwarmError::AlreadyDailingPeer)))
                        .expect("swarm command result receiver not to be dropped");
                }
            },
            MpcSwarmCommand::SendP2pRequest { to, request, result_sender } => {
                let request_id = self.swarm
                    .behaviour_mut()
                    .request_response
                    .send_request(&to, request.clone());
                self.pending_request.insert(request_id, result_sender);
            }
        }
    }
}
