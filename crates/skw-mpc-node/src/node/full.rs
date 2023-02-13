use std::collections::HashMap;

use futures::{channel::{oneshot, mpsc}, StreamExt, SinkExt, stream::FuturesUnordered};
use libp2p::PeerId;
use skw_crypto_curv::elliptic::curves::Secp256k1;
use skw_mpc_payload::{header::PayloadType, PayloadHeader, CryptoHash};
use skw_mpc_protocol::gg20::state_machine::keygen::LocalKey;
use skw_mpc_storage::{default_mpc_storage_opt, run_db_server, DBOpIn, DBOpOut};

use crate::{
    async_executor,
    error::MpcNodeError, 
    swarm::{ new_full_swarm_node }, 
    serde_support::{decode_key, decode_signature}, 
    node::client_request::ClientRequest,
    node::client_outcome::ClientOutcome,
};

use super::job_manager::JobManager;

async fn get_local_key(db_in: &mut mpsc::Sender<DBOpIn>, keygen_id: CryptoHash) -> Result<LocalKey<Secp256k1>, MpcNodeError> {
    let (result_sender, result_receiver) = oneshot::channel();

    db_in
        .send(DBOpIn::ReadFromDB { key: keygen_id, result_sender })
        .await
        .expect("local db must remain open");
    
    let raw_local_key = result_receiver
        .await
        .expect("local db must remain open");
    
    let raw_local_key = match raw_local_key {
        DBOpOut::ReadFromDB { status } => 
            status.map_err(|e| MpcNodeError::StorageError(e))?,
        _ => unreachable!(),
    };
    decode_key(&raw_local_key)
}

async fn assign_job(
    payload_header: PayloadHeader, 
    result_sender: oneshot::Sender<Result< ClientOutcome, MpcNodeError>>,
    db_in_channel: &mut mpsc::Sender<DBOpIn>,
    job_manager: &mut JobManager<'_>
) -> Result<(), MpcNodeError> {
    match payload_header.clone().payload_type {
        PayloadType::KeyGen(_maybe_existing_key) => {
            job_manager.keygen_accept_new_job( payload_header.clone(), result_sender );
        },
        PayloadType::SignOffline { message, keygen_id, keygen_peers }=> {
            job_manager.sign_accept_new_job(
                payload_header.clone(), 
                get_local_key(db_in_channel, keygen_id).await?, 
                keygen_peers, message, result_sender
            ).await;
        },
        PayloadType::KeyRefresh => { unimplemented!(); },
        PayloadType::SignFinalize => { /* nop */ }
    }
    Ok(())
}

pub async fn full_node_event_loop(
    mut client_in: mpsc::Receiver<ClientRequest>
) {
    let mut shutdown_channels: HashMap<PeerId, mpsc::Sender<()>> = HashMap::new();
    let mut db_in_channels: HashMap<PeerId, mpsc::Sender<DBOpIn>> = HashMap::new();

    loop {
        let client_request = client_in.select_next_some().await;

        match client_request {
            ClientRequest::BootstrapNode { local_key, listen_addr, db_name, mut result_sender } => {                
                let (shutdown_sender, mut shutdown_receiver) = mpsc::channel(0);
                let mut result_sender_inside = result_sender.clone();

                // wire up this node to emit PeerId & Listening Addr
                let (peer_id_sender, peer_id_receiver) = oneshot::channel();            
                let (storage_config, mut storage_in_sender) = default_mpc_storage_opt(
                    db_name, false
                );
                run_db_server(storage_config);
                let db_in_chanel = storage_in_sender.clone();

                async_executor(async move {
                    let (
                        local_peer_id,
                        
                        mut swarm_client,
                        swarm_event_loop,
                        
                        mut addr_receiver,
                        mut job_assignment_receiver,
                        mut swarm_message_receiver,
                        mut swarm_termination_sender,
                    ) = new_full_swarm_node(local_key);

                    async_executor(swarm_event_loop.run());
                    let mut interal_results = FuturesUnordered::new();
                    
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

                    let mut job_manager = JobManager::new(
                        local_peer_id, &mut swarm_client,
                        keygen_outgoing_sender, sign_offline_outgoing_sender,
                        sign_fianlize_partial_signature_outgoing_sender,
                    );

                    loop {
                        futures::select! {
                            payload_header = job_assignment_receiver.select_next_some() => {
                                // For StartJob Swarm Request - sometimes the sender is not 100% correct
                                // Just in case - we filter out request address to ourselves
                                // TODO: To be removed in future
                                if payload_header.sender != local_peer_id {
                                    let (inner_result_sender, inner_result_receiver) = oneshot::channel();
                                    match assign_job(payload_header, inner_result_sender, &mut storage_in_sender, &mut job_manager).await {
                                        Ok(_) => { interal_results.push(inner_result_receiver); }
                                        Err(e) => { 
                                            log::error!("FATAL ERROR: Assigning Job Failed {:?}", e); 
                                            result_sender_inside
                                                .send(Err(e)).await
                                                .expect("bootstrapping result sender not to be dropped");
                                        }
                                    }
                                }
                            },

                            payload = keygen_outgoing_receiver.select_next_some() => {
                                match job_manager.handle_outgoing(payload).await {
                                    Ok(_) => {},
                                    Err(e) => result_sender_inside
                                        .send(Err(e)).await
                                        .expect("bootstrapping result sender not to be dropped")
                                }
                            },
                            payload = sign_offline_outgoing_receiver.select_next_some()  => {
                                match job_manager.handle_outgoing(payload).await {
                                    Ok(_) => {},
                                    Err(e) => result_sender_inside
                                        .send(Err(e)).await
                                        .expect("bootstrapping result sender not to be dropped")
                                }
                            },
                            payload = sign_fianlize_partial_signature_outgoing_receiver.select_next_some() => {
                                match job_manager.handle_outgoing(payload).await {
                                    Ok(_) => {},
                                    Err(e) => result_sender_inside
                                        .send(Err(e)).await
                                        .expect("bootstrapping result sender not to be dropped")
                                }
                            }

                            raw_payload = swarm_message_receiver.select_next_some() => {
                                match job_manager.handle_incoming(&raw_payload).await {
                                    Ok(_) => {},
                                    Err(e) => result_sender_inside
                                        .send(Err(e)).await
                                        .expect("bootstrapping result sender not to be dropped")
                                }
                            },

                            outcome = interal_results.select_next_some() => {
                                match outcome.expect("internal result sender not to be dropped") {
                                    Ok(outcome) => {
                                        match outcome {
                                            ClientOutcome::KeyGen { payload_id, local_key, .. } => {
                                                let (db_res_sender, db_res_receiver) = oneshot::channel();
                                                storage_in_sender.send(DBOpIn::WriteToDB { 
                                                    key: payload_id, 
                                                    value: local_key, 
                                                    result_sender: db_res_sender
                                                }).await.expect("DB must remain open");
        
                                                if let DBOpOut::WriteToDB { status } = db_res_receiver.await.expect("DB must remain open") {
                                                    match status {
                                                        Ok(_) => {},
                                                        Err(e) => { 
                                                            log::error!("Internal result write to db error {:?}", e); 
                                                            result_sender_inside
                                                                .send(Err(MpcNodeError::StorageError(e))).await
                                                                .expect("bootstrapping result sender not to be dropped");
                                                        }
                                                    }
                                                }
                                            },
                                            ClientOutcome::Sign { peer_id, payload_id, sig } => {
                                                log::info!("Sign Result {:?} {:?} {:?}", peer_id, payload_id, decode_signature(&sig));
                                            }
                                        };
                                    },
                                    Err(e) => { log::error!("Internal result error {:?}", e); }
                                }
                            },

                            _ = shutdown_receiver.select_next_some() => {
                                // 1. shutdown the swarm
                                swarm_termination_sender.send(()).await
                                    .expect("swarm node should not be dropped");

                                // 2. shutdown the db server
                                let (result_sender, result_receiver) = oneshot::channel();
                                storage_in_sender
                                    .send(DBOpIn::Shutdown { result_sender }).await
                                    .expect("shutdown swarm should not fail");
                                if let DBOpOut::Shutdown { status }= result_receiver.await.expect("DB must remain open") {
                                    match status {
                                        Ok(_) => {},
                                        Err(e) => { 
                                            log::error!("Internal result write to db error {:?}", e); 
                                            result_sender_inside
                                                .send(Err(MpcNodeError::StorageError(e))).await
                                                .expect("bootstrapping result sender not to be dropped");
                                        }
                                    }
                                }
                                // 3. shutdown node event loop for node
                                break;
                            }
                        }
                    }
                });

                let local_swarm_info = peer_id_receiver.await.expect("cannot be canceled");
                shutdown_channels.insert(local_swarm_info.0, shutdown_sender);
                db_in_channels.insert(local_swarm_info.0, db_in_chanel);
                result_sender
                    .send(Ok(local_swarm_info)).await
                    .expect("result_receiver should not be dropped for client_reuqest");
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
            ClientRequest::WriteToDB { node, key, value, result_sender } => {
                let (db_write_result_sender, db_write_result_receiver) = oneshot::channel();

                db_in_channels
                    .get_mut(&node)
                    .expect("shutdown channel not found")
                    .send(DBOpIn::WriteToDB { key, value, result_sender: db_write_result_sender })
                    .await
                    .expect("db_in_receiver should not be dropped");
                let x = db_write_result_receiver
                    .await
                    .expect("db_write_result_receiver is not dropped");
                if let DBOpOut::WriteToDB { status } = x {
                    match status {
                        Ok(_) => { result_sender.send(Ok(())).expect("result receiver not to be dropped"); }
                        Err(e) => { 
                            log::error!("Internal result write to db error {:?}", e); 
                            result_sender
                                .send(Err(MpcNodeError::StorageError(e)))
                                .expect("bootstrapping result sender not to be dropped");
                        }
                    }
                }
            }
        }
    }
}
