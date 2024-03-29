use std::collections::HashMap;

use futures::{channel::{oneshot, mpsc}, StreamExt, SinkExt};
use libp2p::PeerId;
use skw_mpc_payload::{header::PayloadType, PayloadHeader, AuthHeader, CryptoHash};

use crate::{
    async_executor,
    node::client_request::{ClientRequest},
    node::client_outcome::ClientOutcome,
    error::{MpcNodeError, NodeError}, 
    swarm::{ new_light_swarm_node }, 
    serde_support::decode_key, 
    
    wire_outgoing_pipe,
};

use super::job_manager::JobManager;

async fn assign_job(
    key_shard_id: CryptoHash,
    payload_header: PayloadHeader, 
    maybe_local_key: Option<Vec<u8>>,
    result_sender: oneshot::Sender<Result< ClientOutcome, MpcNodeError>>,
    job_manager: &mut JobManager<'_>
) -> Result<(), MpcNodeError> {
    match payload_header.clone().payload_type {
        PayloadType::KeyGen => {
            job_manager.keygen_accept_new_job( key_shard_id, payload_header.clone(), result_sender );
        },
        PayloadType::SignOffline { message, .. } => {
            if maybe_local_key.is_none() {
                return Err(MpcNodeError::NodeError(NodeError::LocalKeyMissing));
            }        
            job_manager.sign_accept_new_job(
                key_shard_id,
                payload_header.clone(), 
                decode_key( &maybe_local_key.unwrap() )?,
                message, result_sender
            ).await;
        },
        PayloadType::KeyRefresh { .. } => { 
            job_manager.key_refresh_accept_new_job(
                key_shard_id,
                payload_header.clone(), 
                None,
                result_sender
            ).await;
        },
        PayloadType::SignFinalize => { /* nop */ }
    }
    Ok(())
}

pub async fn light_node_event_loop(
    mut client_in: mpsc::Receiver<ClientRequest>
) {
    let mut external_request_channels: HashMap<PeerId, mpsc::Sender<(
        PayloadHeader,  AuthHeader, Option<Vec<u8>>,
        oneshot::Sender<Result<ClientOutcome, MpcNodeError>>
    )>> = HashMap::new();

    let mut shutdown_channels: HashMap<PeerId, mpsc::Sender<()>> = HashMap::new();

    loop {
        let client_request = client_in.select_next_some().await;
        match client_request {
            ClientRequest::BootstrapNode { local_key, listen_addr, mut result_sender, .. } => {    
                // Wire up this node to receive external request
                let (external_request_sender, mut external_request_receiver) = mpsc::channel::<(
                    PayloadHeader, AuthHeader, Option<Vec<u8>>,
                    oneshot::Sender<Result<ClientOutcome, MpcNodeError>>
                )>(0);
                let (shutdown_sender, mut shutdown_receiver) = mpsc::channel(0);
                let mut result_sender_inside = result_sender.clone();

                // wire up this node to emit PeerId & Listening Addr
                let (peer_id_sender, peer_id_receiver) = oneshot::channel();            

                async_executor(async move {
                    let (
                        local_peer_id,
                        
                        mut swarm_client,
                        swarm_event_loop,
                        
                        mut addr_receiver,
                        mut swarm_message_receiver,
                        mut swarm_termination_sender,
                    ) = new_light_swarm_node(local_key);

                    async_executor(swarm_event_loop.run());                    
                    swarm_client.start_listening(listen_addr.parse().expect("address need to be valid"))
                        .await
                        .map_err(|e| { log::error!("Failed To Listen {:?}", e); })
                        .expect("shutting down node if listen fail");

                    peer_id_sender
                        .send((local_peer_id, addr_receiver.select_next_some().await ))
                        .expect("peer_id receiver not to be droppped");
 
                    let (keygen_outgoing_sender, mut keygen_outgoing_receiver) = mpsc::unbounded();
                    let (sign_offline_outgoing_sender, mut sign_offline_outgoing_receiver) = mpsc::unbounded();
                    let (sign_fianlize_partial_signature_outgoing_sender, mut sign_fianlize_partial_signature_outgoing_receiver) = mpsc::unbounded();

                    let (key_refresh_join_message_outgoing_sender, mut key_refresh_join_message_outgoing_receiver) = mpsc::unbounded();
                    let (key_refresh_refresh_message_outgoing_sender, mut key_refresh_refresh_message_outgoing_receiver) = mpsc::unbounded();

                    let mut job_manager = JobManager::new(
                        local_peer_id, &mut swarm_client,
                        keygen_outgoing_sender, sign_offline_outgoing_sender,
                        sign_fianlize_partial_signature_outgoing_sender,
                        key_refresh_join_message_outgoing_sender,
                        key_refresh_refresh_message_outgoing_sender,
                    );

                    loop {
                        futures::select! {
                            request = external_request_receiver.select_next_some() => {
                                let payload_header = request.0;
                                let auth_header = request.1;
                                let maybe_local_key = request.2;
                                let request_result_sender = request.3;

                                match job_manager.init_new_job( auth_header.clone(), payload_header.clone()).await {
                                    Ok(_) => {
                                        match assign_job( 
                                            auth_header.key_shard_id(), 
                                            payload_header, maybe_local_key, request_result_sender, &mut job_manager
                                        ).await {
                                            Ok(_) => {  } // job assignment success
                                            Err(e) => { 
                                                log::error!("FATAL ERROR: Assigning Job Failed {:?}", e); 
                                                result_sender_inside
                                                    .send(Err(e)).await
                                                    .expect("bootstrapping result sender not to be dropped");
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        log::error!("Catch Error on init_new_job {:?}", e);
                                        request_result_sender
                                            .send(Err(e.clone()))
                                            .expect("request result sender not to be dropped");
                                    }
                                };
                            },

                            payload = keygen_outgoing_receiver.select_next_some() => wire_outgoing_pipe!(payload, job_manager, result_sender_inside),
                            payload = sign_offline_outgoing_receiver.select_next_some() => wire_outgoing_pipe!(payload, job_manager, result_sender_inside),
                            payload = sign_fianlize_partial_signature_outgoing_receiver.select_next_some() => wire_outgoing_pipe!(payload, job_manager, result_sender_inside),
                            payload = key_refresh_join_message_outgoing_receiver.select_next_some() => wire_outgoing_pipe!(payload, job_manager, result_sender_inside),
                            payload = key_refresh_refresh_message_outgoing_receiver.select_next_some() => wire_outgoing_pipe!(payload, job_manager, result_sender_inside),

                            raw_payload = swarm_message_receiver.select_next_some() => {
                                match job_manager.handle_incoming(&raw_payload).await {
                                    Ok(_) => {},
                                    Err(e) => result_sender_inside
                                        .send(Err(e)).await
                                        .expect("bootstrapping result sender not to be dropped")
                                }
                            },

                            _ = shutdown_receiver.select_next_some() => {
                                // 1. shutdown the swarm
                                swarm_termination_sender.send(()).await
                                    .expect("swarm node should not be dropped");
                                // 2. shutdown node event loop for node
                                break;
                            }
                        }
                    }
                });

                let local_swarm_info = peer_id_receiver.await.expect("cannot be canceled");                
                external_request_channels.insert(local_swarm_info.0, external_request_sender);
                shutdown_channels.insert(local_swarm_info.0, shutdown_sender);
                result_sender
                    .send(Ok(local_swarm_info)).await
                    .expect("result_receiver should not be dropped for client_reuqest");
            },

            ClientRequest::MpcRequest { 
                from, 
                payload_header, 
                auth_header,
                maybe_local_key,
                result_sender 
            } => {
                let external_request_channel = external_request_channels
                    .get_mut(&from).expect("peer must be valid");
                external_request_channel.send((
                    payload_header, auth_header,
                    maybe_local_key, result_sender
                ))
                    .await.expect("external request receiver not to be dropped.");
            },

            ClientRequest::Shutdown { node, result_sender} => {
                shutdown_channels
                    .get_mut(&node)
                    .expect("shutdown channel not found")
                    .send(())
                    .await
                    .expect("shutdown receiver not to be dropped");
                result_sender
                    .send(Ok(()))
                    .expect("result receiver not to be dropped");
            }
        }
    }
}
