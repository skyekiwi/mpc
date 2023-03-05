use core::panic;
use std::collections::{HashMap, hash_map::Entry};

use libp2p::{
    swarm::{SwarmEvent, ConnectionHandlerUpgrErr}, PeerId,
    Swarm,
    multiaddr, 
    request_response::{self, RequestId,}, 
};
use futures::{StreamExt, FutureExt};
use futures::channel::{oneshot, mpsc};

#[cfg(feature = "full-node")]
use skw_mpc_node::node::NodeClient;

use crate::error::{MpcClientError, SwarmError, SwarmP2pError};

use super::{
    behavior::{MpcSwarmBahavior, MpcSwarmBahaviorEvent, MpcP2pRequest, MpcP2pResponse}, 
    client::MpcSwarmCommand,
};

pub struct MpcSwarmEventLoop {
    #[cfg(feature = "full-node")]
    light_node_client: NodeClient,

    swarm: Swarm<MpcSwarmBahavior>,
    command_receiver: mpsc::UnboundedReceiver<MpcSwarmCommand>,

    pending_dial: HashMap<PeerId, oneshot::Sender<Result<(), MpcClientError>>>,
    pending_request: HashMap<RequestId, oneshot::Sender<Result<MpcP2pResponse, MpcClientError>>>,
    swarm_termination_receiver: mpsc::Receiver<()>,
}

impl MpcSwarmEventLoop {
    pub fn new(
        // Assume node is bootstrapped and running well
        #[cfg(feature = "full-node")]
        light_node_client: NodeClient,

        swarm: Swarm<MpcSwarmBahavior>,
        command_receiver: mpsc::UnboundedReceiver<MpcSwarmCommand>,
        swarm_termination_receiver: mpsc::Receiver<()>,
    ) -> Self {
        Self {
            #[cfg(feature = "full-node")]
            light_node_client,

            swarm,            
            command_receiver, 

            pending_dial: Default::default(),
            pending_request: Default::default(),
            swarm_termination_receiver,
        }
    }

    pub async fn run(mut self) {
        loop {
            futures::select! {
                // events are INCOMING Streams for the node raw events
                event = self.swarm.select_next_some().fuse() => {
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
                            .send(Err(MpcClientError::SwarmError(SwarmError::FailToDailPeer)))
                            .expect("dailing result receiver not to be dropped");
                    }
                }
            }
            SwarmEvent::IncomingConnectionError { .. } => {}
            SwarmEvent::Dialing(peer_id) => log::debug!("Dialing {peer_id}"),

            // p2p events
            SwarmEvent::Behaviour(MpcSwarmBahaviorEvent::RequestResponse(
                request_response::Event::Message { message, .. },
            )) => {
                match message {
                    // p2p message request hanlder
                    request_response::Message::Request {
                        #[cfg(feature = "full-node")]
                        request_id,
                        #[cfg(feature = "full-node")]
                        request, 
                        #[cfg(feature = "full-node")]
                        channel, ..
                    } => {
                        #[cfg(feature = "full-node")]
                        match request {
                            MpcP2pRequest::Mpc { auth_header, job_header, maybe_local_key } => {    
                                match self.light_node_client.send_request(
                                    job_header, auth_header, maybe_local_key
                                )
                                    .await
                                {
                                    Ok(client_outcome) => match self.swarm
                                        .behaviour_mut()
                                        .request_response
                                        .send_response(channel, MpcP2pResponse::Mpc { 
                                            payload: Ok(client_outcome.payload()) 
                                        })
                                    {
                                        Ok(_) => { } // let the - Response - section take over
                                        Err(response) => {
                                            log::debug!("Mpc StartJob Reponse channel closed {:?}", response);
                                            self
                                                .pending_request
                                                .remove(&request_id)
                                                .expect("client request channel to still be pending.")
                                                .send(Err(MpcClientError::SwarmP2pError(SwarmP2pError::ResponseChannelClose)))
                                                .expect("p2p response receiver not to be dropped");
                                        }
                                    },

                                    Err(e) => {
                                        match self.swarm
                                            .behaviour_mut()
                                            .request_response
                                            .send_response(channel, MpcP2pResponse::Mpc { payload: Err(MpcClientError::MpcNodeError(e.to_string()))}) 
                                        {
                                                Ok(_) => { } // let the - Response - section take over
                                                Err(response) => {
                                                    log::debug!("Mpc StartJob Reponse channel closed {:?}", response);
                                                    self
                                                        .pending_request
                                                        .remove(&request_id)
                                                        .expect("client request channel to still be pending.")
                                                        .send(Err(MpcClientError::SwarmP2pError(SwarmP2pError::ResponseChannelClose)))
                                                        .expect("p2p response receiver not to be dropped");
                                                }
                                        }
                                    }

                                }  
                            },
                        }

                        // Light Node never receive requests
                        #[cfg(feature = "light-node")]
                        {}
                    }

                    // p2p message response hanlder
                    request_response::Message::Response { request_id, response } => {
                        let _ = self
                            .pending_request
                            .remove(&request_id)
                            .expect("Request to still be pending.")
                            .send(Ok(response));
                    }
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
                    .send(Err(MpcClientError::SwarmP2pError(SwarmP2pError::OutboundFailure)))
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
            #[cfg(feature = "full-node")]
            MpcSwarmCommand::StartListening { addr, result_sender } => {
                let res = self.swarm.listen_on(addr)
                    .map_err(|e| {
                        log::error!("Listen Error {:?}", e);
                        MpcClientError::SwarmError(SwarmError::FailToListenToAddress)
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
                            result_sender.send(Err(MpcClientError::SwarmError(SwarmError::FailToDailPeer)))
                                .expect("swarm command result receiver not to be dropped");
                        }
                    }
                } else {
                    result_sender
                        .send(Err(MpcClientError::SwarmError(SwarmError::AlreadyDailingPeer)))
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
