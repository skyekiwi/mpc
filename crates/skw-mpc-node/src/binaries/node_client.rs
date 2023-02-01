use futures::StreamExt;

use skw_mpc_node::{
    node::{new_full_node},
    error::MpcNodeError,
    job_manager::JobManager,
};
use skw_mpc_payload::{PayloadHeader, header::PayloadType, AuthHeader};
use skw_mpc_storage::db::{default_mpc_storage_opt, run_db_server};

#[async_std::main]
async fn main() -> Result<(), MpcNodeError> {
    let (
        local_peer_id,
        
        mut client,
        p2p_node_event_loop,

        mut addr_receiver,
        _job_assignment_receiver,
        mut main_message_receiver,
    ) = new_full_node()?; 
    
    // Spin up the Swarm event loop
    let _event_loop_jh = async_std::task::spawn(p2p_node_event_loop.run());

    client.start_listening("/ip4/10.0.0.3/tcp/0".parse().expect("address need to be valid"))
        .await
        .expect("Listen not to fail.");
    
    let local_addr = addr_receiver.select_next_some().await;

    let sample_auth_header = AuthHeader::default();
    let sample_payload_header = PayloadHeader::new(
        [1u8; 32], 
        PayloadType::KeyGen(None), 
        vec![
            (local_peer_id, local_addr.clone()),
            ("12D3KooWLRFXzFnCtiQhkAYc1Gr166wLZyUqz4kzHuN4RjoizXyM".parse().unwrap(), "/ip4/10.0.0.3/tcp/52202".parse().unwrap()),
            ("12D3KooWBJNxkf5aMN39UD2ADsLZv5HJdZhBKyMLL38fzEGNEE3Q".parse().unwrap(), "/ip4/10.0.0.3/tcp/52201".parse().unwrap()),
        ],
        local_peer_id,
        2, 3,
    );

    // spin up the DB server event loop
    let (storage_config, storage_in_sender) = default_mpc_storage_opt(
        format!("mpc_storage-{:?}", local_peer_id), false
    );    
    run_db_server(storage_config);

    let mut job_manager = JobManager::new(
        local_peer_id, &mut client, storage_in_sender
    );
    // Finally, we spin up the job locally
    job_manager.keygen_init_new_job(sample_auth_header, sample_payload_header).await;

    loop {
        futures::select! {
            payload = job_manager.main_outgoing_receiver.select_next_some() => {
                job_manager.handle_outgoing(payload).await;
            },
            raw_payload = main_message_receiver.select_next_some() => {
                job_manager.handle_incoming(&raw_payload).await;
            },
        }
    }
}
