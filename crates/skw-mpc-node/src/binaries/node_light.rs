use std::fmt::format;

use futures::{StreamExt, channel::mpsc};

use skw_mpc_node::{
    node::new_full_node,
    error::MpcNodeError,
    job_manager::JobManager,
};
use skw_mpc_payload::header::PayloadType;
use skw_mpc_storage::db::{default_mpc_storage_opt, run_db_server};

#[async_std::main]
async fn main() -> Result<(), MpcNodeError> {
    let (
        local_peer_id,
        
        mut client,
        p2p_node_event_loop,
        
        mut addr_receiver,
        mut job_assignment_receiver,
        mut main_message_receiver,
    ) = new_full_node()?; 
    
    // Spin up the Swarm event loop
    let _event_loop_jh = async_std::task::spawn(p2p_node_event_loop.run());

    client.start_listening("/ip4/10.0.0.3/tcp/0".parse().expect("address need to be valid"))
        .await
        .expect("Listen not to fail.");
    
    let _local_addr = addr_receiver.select_next_some().await;

    // spin up the DB server event loop
    let (storage_config, storage_in_sender) = default_mpc_storage_opt(
        format!("mpc_storage-{:?}", local_peer_id), false
    );
    run_db_server(storage_config);

    let mut job_manager = JobManager::new(
        local_peer_id, &mut client, storage_in_sender
    );

    loop {
        futures::select! {
            payload_header = job_assignment_receiver.select_next_some() => {
                match payload_header.payload_type {
                    PayloadType::KeyGen(_maybe_existing_key) => {
                        job_manager.keygen_accept_new_job(
                            payload_header.clone(), 
                        );
                    },
                    PayloadType::Signing(_) => {
                        unimplemented!()
                    },
                    PayloadType::KeyRefresh => {
                        unimplemented!()
                    }
                }
            },
            payload = job_manager.main_outgoing_receiver.select_next_some() => {
                println!("Client Handling outgoing start, msg to {:?}", payload.body.receiver);
                job_manager.handle_outgoing(payload).await;
                println!("Handling outgoing done");
            },
            raw_payload = main_message_receiver.select_next_some() => {
                println!("cleint Handling incoming start", );
                job_manager.handle_incoming(&raw_payload).await;
                println!("client Handling incoming done");
            },
        }
    }
}
