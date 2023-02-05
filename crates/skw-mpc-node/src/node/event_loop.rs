use std::collections::HashMap;

use futures::{channel::{oneshot, mpsc}, StreamExt, SinkExt};
use libp2p::PeerId;
use skw_mpc_payload::{header::PayloadType, PayloadHeader};
use skw_mpc_storage::db::{default_mpc_storage_opt, run_db_server};

use crate::{error::MpcNodeError, swarm::{ new_full_swarm_node}};

use super::{client_request::{ClientRequest}, job_manager::JobManager};

async fn assign_job(
    payload_header: PayloadHeader, 
    result_sender: Option<oneshot::Sender<Result< Vec<u8>, MpcNodeError>>>,
    job_manager: &mut JobManager<'_>
) {
    match payload_header.clone().payload_type {
        PayloadType::KeyGen(_maybe_existing_key) => {
            job_manager.keygen_accept_new_job(
                payload_header.clone(), 
                result_sender
            );
        },
        PayloadType::Signing {
            message, keygen_id, keygen_peers
        }=> {
            job_manager.sign_accept_new_job(
                payload_header.clone(), 
                keygen_id, 
                keygen_peers, 
                message,
                result_sender
            ).await;
        },
        PayloadType::KeyRefresh => {
            unimplemented!()
        }
    }
}

pub async fn node_main_event_loop(
    mut client_in: mpsc::Receiver<ClientRequest>
) {

    let mut external_request_channels: HashMap<PeerId, mpsc::Sender<(
        PayloadHeader, oneshot::Sender<Result<Vec<u8>, MpcNodeError>>
    )>> = HashMap::new();

    // Setup Phase
    loop {
        let client_request = client_in.select_next_some().await;

        match client_request {
            ClientRequest::BootstrapNode { local_key, listen_addr, db_name, result_sender } => {
                let (external_request_sender, mut external_request_receiver) = mpsc::channel::<(
                    PayloadHeader, oneshot::Sender<Result<Vec<u8>, MpcNodeError>>
                )>(0);

                let (peer_id_sender, peer_id_receiver) = oneshot::channel();            
                async_std::task::spawn(async move {
                    let (
                        local_peer_id,
                        
                        mut swarm_client,
                        swarm_event_loop,
                        
                        mut addr_receiver,
                        mut job_assignment_receiver,
                        mut swarm_message_receiver,
                    ) = new_full_swarm_node(local_key)
                        .unwrap(); // TODO: handle this unwrap

                    let _event_loop_jh = async_std::task::spawn(swarm_event_loop.run());
                
                    swarm_client.start_listening(listen_addr.parse().expect("address need to be valid"))
                        .await
                        .expect("Listen not to fail.");
                    let local_addr = addr_receiver.select_next_some().await;
                    peer_id_sender.send((local_peer_id, local_addr));

                    
                    let (storage_config, storage_in_sender) = default_mpc_storage_opt(
                        db_name, false
                    );
                    run_db_server(storage_config);

                    let (keygen_outgoing_sender, mut keygen_outgoing_receiver) = mpsc::unbounded();
                    let (sign_offline_outgoing_sender, mut sign_offline_outgoing_receiver) = mpsc::unbounded();
                
                    let mut job_manager = JobManager::new(
                        local_peer_id, &mut swarm_client, storage_in_sender,

                        keygen_outgoing_sender, sign_offline_outgoing_sender
                    );

                    loop {
                        futures::select! {
                            payload_header = job_assignment_receiver.select_next_some() => {
                                assign_job(
                                    payload_header, None, &mut job_manager
                                );
                            },
                            payload = keygen_outgoing_receiver.select_next_some() => {
                                job_manager.handle_outgoing(payload).await;
                            },
                            payload = sign_offline_outgoing_receiver.select_next_some() => {
                                job_manager.handle_outgoing(payload).await;
                            },
                            raw_payload = swarm_message_receiver.select_next_some() => {
                                job_manager.handle_incoming(&raw_payload).await;
                            },
                            request = external_request_receiver.select_next_some() => {
                                let payload_header = request.0;
                                let result_sender = request.1;

                                assign_job(
                                    payload_header, Some(result_sender), &mut job_manager
                                );
                            }
                        }
                    }
                });

                let local_swarm_info = peer_id_receiver.await.expect("cannot be canceled");
                external_request_channels.insert(local_swarm_info.0, external_request_sender);
                result_sender
                    .send(Ok(local_swarm_info))
                    .expect("result_receiver should not be dropped for client_reuqest");
            },

            ClientRequest::MpcRequest { 
                from, 
                payload_header, 
                result_sender 
            } => {
                let external_request_channel = external_request_channels
                    .get_mut(&from)
                    .expect("peer must be valid");
                external_request_channel.send((
                    payload_header, result_sender
                )).await;
            }
        }
    }

}