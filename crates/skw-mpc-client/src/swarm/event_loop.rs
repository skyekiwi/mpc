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
use skw_mpc_node::{serde_support::decode_key, error::MpcNodeError};

use super::{
    behavior::{MpcSwarmBahavior, MpcSwarmBahaviorEvent, MpcP2pRequest, MpcP2pResponse}, 
    client::MpcSwarmCommand,
};

pub struct MpcSwarmEventLoop {
    #[cfg(feature = "full-node")]
    light_node_client: NodeClient,

    swarm: Swarm<MpcSwarmBahavior>,
    command_receiver: mpsc::UnboundedReceiver<MpcSwarmCommand>,

    pending_dial: HashMap<PeerId, oneshot::Sender<Result<(), MpcNodeError>>>,
    pending_request: HashMap<RequestId, oneshot::Sender<Result<MpcP2pResponse, MpcNodeError>>>,
    swarm_termination_receiver: mpsc::Receiver<bool>,
}

impl MpcSwarmEventLoop {
    pub fn new(
        // Assume node is bootstrapped and running well
        #[cfg(feature = "full-node")]
        light_node_client: NodeClient,

        swarm: Swarm<MpcSwarmBahavior>,
        command_receiver: mpsc::UnboundedReceiver<MpcSwarmCommand>,
        swarm_termination_receiver: mpsc::Receiver<bool>,
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
                event = self.swarm.next().fuse() => {
                    self.handle_event(event.expect("always have an event")).await;
                },

                // commands are OUTGOING Sink of events 
                command = self.command_receiver.select_next_some() => {
                    match self.handle_command(command).await {
                        Ok(()) => {},
                        Err(e) => eprintln!("{:?}", e)
                    }
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
                eprintln!(
                    "Local node is listening on {:?}",
                    address.with(multiaddr::Protocol::P2p(local_peer_id.into()))
                );
            }
            SwarmEvent::ConnectionEstablished {
                peer_id, endpoint, ..
            } => {
                if endpoint.is_dialer() {
                    if let Some(sender) = self.pending_dial.remove(&peer_id) {
                        let _ = sender.send(Ok(()));
                    }
                }
            }
            SwarmEvent::ConnectionClosed { .. } => {
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
            SwarmEvent::Behaviour(MpcSwarmBahaviorEvent::RequestResponse(
                request_response::Event::Message { message, .. },
            )) => {
                match message {
                    // p2p message request hanlder
                    request_response::Message::Request {
                        request, channel, ..
                    } => {
                        #[cfg(feature = "full-node")]
                        match request {
                            MpcP2pRequest::Mpc { auth_header, job_header, maybe_local_key } => {
                                let client_outcome = self.light_node_client.send_request(
                                    job_header, auth_header, maybe_local_key
                                )
                                    .await
                                    .unwrap();

                                self.swarm
                                    .behaviour_mut()
                                    .request_response
                                    .send_response(channel, MpcP2pResponse::Mpc { 
                                        payload: client_outcome.payload() 
                                    })
                                    .unwrap();
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
                eprintln!("p2p outbound request failure {:?} {:?}", error, peer);
                let _ = self
                    .pending_request
                    .remove(&request_id)
                    .expect("Request to still be pending.")
                    .send(Err(MpcNodeError::P2pOutboundFailure));
            }
            SwarmEvent::Behaviour(MpcSwarmBahaviorEvent::RequestResponse(
                request_response::Event::ResponseSent { .. },
            )) => {},
            
            _ => {}
        }
    }

    async fn handle_command(&mut self, request: MpcSwarmCommand) -> Result<(), MpcNodeError> {
        match request {
            #[cfg(feature = "full-node")]
            MpcSwarmCommand::StartListening { addr, result_sender } => {
                match self.swarm.listen_on(addr) {
                    Ok(_) => {
                        result_sender.send(Ok(()))
                            .expect("sender should not be dropped");
                        Ok(())
                    },
                    Err(_e )=> {
                        println!("Failed To Listen {:?}", _e);
                        result_sender.send(Err(MpcNodeError::FailToListenOnPort))
                    },
                }.map_err(|_| MpcNodeError::FailToSendViaChannel)
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
            MpcSwarmCommand::SendP2pRequest { to, request, result_sender } => {
                let request_id = self.swarm
                    .behaviour_mut()
                    .request_response
                    .send_request(&to, request.clone());
                self.pending_request.insert(request_id, result_sender);

                Ok(())
            }
        }
    }
}
