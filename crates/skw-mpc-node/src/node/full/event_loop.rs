use std::collections::HashMap;

use futures::{channel::{oneshot, mpsc}, StreamExt, SinkExt, stream::FuturesUnordered};
use libp2p::PeerId;
use skw_crypto_curv::elliptic::curves::Secp256k1;
use skw_mpc_payload::{header::PayloadType, PayloadHeader, CryptoHash};
use skw_mpc_protocol::gg20::state_machine::keygen::LocalKey;
use skw_mpc_storage::{default_mpc_storage_opt, run_db_server, DBOpIn, DBOpOut};

use crate::{error::MpcNodeError, swarm::{ new_full_swarm_node}, serde_support::{decode_key, decode_signature}, async_executor};
use crate::{
    node::client_request::{ClientRequest},
    node::client_outcome::ClientOutcome
};

use super::job_manager::JobManager;

async fn get_local_key(db_in: &mut mpsc::Sender<DBOpIn>, keygen_id: CryptoHash) -> LocalKey<Secp256k1> {
    let (result_sender, result_receiver) = oneshot::channel();

    db_in
        .send(DBOpIn::ReadFromDB { key: keygen_id, result_sender })
        .await
        .expect("db channel must remain open");
    
    let raw_local_key = result_receiver
        .await
        .expect("db read to be success"); // TODO;
    
    let raw_local_key = match raw_local_key {
        DBOpOut::ReadFromDB { status } => status.unwrap(),
        _ => unreachable!(),
    };
    decode_key(&raw_local_key)
}

async fn assign_job(
    payload_header: PayloadHeader, 
    result_sender: oneshot::Sender<Result< ClientOutcome, MpcNodeError>>,
    db_in_channel: &mut mpsc::Sender<DBOpIn>,
    job_manager: &mut JobManager<'_>
) {
    match payload_header.clone().payload_type {
        PayloadType::KeyGen(_maybe_existing_key) => {
            job_manager.keygen_accept_new_job(
                payload_header.clone(), 
                result_sender
            );
        },
        PayloadType::SignOffline {
            message, keygen_id, keygen_peers
        }=> {

            let local_key = get_local_key(db_in_channel, keygen_id).await;
            job_manager.sign_accept_new_job(
                payload_header.clone(), 

                local_key, 
                keygen_peers, 
                message,

                result_sender
            ).await;
        },
        PayloadType::KeyRefresh => {
            unimplemented!()
        },
        PayloadType::SignFinalize => {
            // nop
        }
    }
}

pub async fn full_node_event_loop(
    mut client_in: mpsc::Receiver<ClientRequest>
) {
    let mut shutdown_channels: HashMap<PeerId, mpsc::Sender<bool>> = HashMap::new();
    let mut db_in_channels: HashMap<PeerId, mpsc::Sender<DBOpIn>> = HashMap::new();

    loop {
        let client_request = client_in.select_next_some().await;

        match client_request {
            ClientRequest::BootstrapNode { local_key, listen_addr, db_name, result_sender } => {
                let (shutdown_sender, mut shutdown_receiver) = mpsc::channel(0);

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
                    ) = new_full_swarm_node(local_key)
                        .unwrap(); // TODO: handle this unwrap

                    async_executor(swarm_event_loop.run());
                    let mut interal_results = FuturesUnordered::new();
                    
                    swarm_client.start_listening(listen_addr.parse().expect("address need to be valid"))
                        .await
                        .map_err(|e| println!("Failed To Listen {:?}", e))
                        .expect("Listen not to fail.");// TODO: actually .. listen can fail
                    let local_addr = addr_receiver.select_next_some().await;
                    peer_id_sender
                        .send((local_peer_id, local_addr))
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
                                    println!("{:?} Received job assignment {:?}", local_peer_id, payload_header);

                                    let (result_sender, result_receiver) = oneshot::channel();
                                
                                    assign_job(
                                        payload_header, result_sender, &mut storage_in_sender, &mut job_manager
                                    ).await;

                                    interal_results.push(result_receiver);
                                }
                            },

                            payload = keygen_outgoing_receiver.select_next_some() => {
                                job_manager.handle_outgoing(payload).await;
                            },
                            payload = sign_offline_outgoing_receiver.select_next_some() => {
                                job_manager.handle_outgoing(payload).await;
                            },
                            payload = sign_fianlize_partial_signature_outgoing_receiver.select_next_some() => {
                                job_manager.handle_outgoing(payload).await;
                            }

                            raw_payload = swarm_message_receiver.select_next_some() => {
                                job_manager.handle_incoming(&raw_payload).await;
                            },

                            outcome = interal_results.select_next_some() => {
                                let outcome = outcome.unwrap().unwrap();
                                match outcome {
                                    ClientOutcome::KeyGen {
                                        payload_id, local_key, ..
                                    } => {
                                        let (res_sender, res_receiver) = oneshot::channel();
                                        storage_in_sender.send(
                                            DBOpIn::WriteToDB { 
                                                key: payload_id, 
                                                value: local_key, 
                                                result_sender: res_sender
                                            }
                                        )
                                            .await
                                            .expect("DB Write should not fail");

                                        let res = res_receiver.await;
                                        println!("DB Result {:?}", res);
                                    },
                                    ClientOutcome::Sign {
                                        peer_id, payload_id, sig
                                    } => {
                                        println!("Sign Result {:?} {:?} {:?}", peer_id, payload_id, decode_signature(&sig));
                                    }
                                };
                            },

                            _ = shutdown_receiver.select_next_some() => {
                                // 1. shutdown the swarm
                                swarm_termination_sender
                                    .send(true)
                                    .await
                                    .expect("shutdown swarm should not fail");

                                // 2. shutdown the db server
                                let (result_sender, result_receiver) = oneshot::channel();
                                storage_in_sender
                                    .send(DBOpIn::Shutdown { result_sender })
                                    .await
                                    .expect("shutdown swarm should not fail");
                                let _ = result_receiver.await;
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
                    .send(Ok(local_swarm_info))
                    .expect("result_receiver should not be dropped for client_reuqest");
            },

            ClientRequest::Shutdown { node, result_sender} => {
                shutdown_channels
                    .get_mut(&node)
                    .expect("shutdown channel not found")
                    .send(true)
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
                if let DBOpOut::WriteToDB { status: Ok(_) } = x {
                    result_sender
                        .send(true)
                        .expect("result receiver not to be dropped");
                } else {
                    result_sender
                        .send(false)
                        .expect("result receiver not to be dropped");
                }
            }
        }
    }
}
