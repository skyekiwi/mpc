use futures::StreamExt;

use skw_mpc_node::{
    node::{new_full_node, MpcP2pRequest},
    error::MpcNodeError,
    job_manager::JobManager,
};
use skw_mpc_payload::{PayloadHeader, header::PayloadType, AuthHeader};

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
    
    let local_addr = addr_receiver.select_next_some().await;
    println!("GOT {:?}", local_addr);

    let sample_auth_header = AuthHeader::default();
    let sample_payload_header = PayloadHeader::new(
        [0u8; 32], 
        PayloadType::KeyGen(None), 
        vec![
            (local_peer_id, local_addr.clone()),
            ("12D3KooWJh5o5FZYSGgtcdfpkEn7qhRH9yyBb8fmNLUPmCZEGSgK".parse().unwrap(), "/ip4/10.0.0.3/tcp/53459".parse().unwrap()),
            ("12D3KooWDwjLtB3XDHGy9BRyeaFEcJehzPRKFXaDjg86XJFBLnum".parse().unwrap(), "/ip4/10.0.0.3/tcp/53457".parse().unwrap()),
        ],
        local_peer_id,
        1, 3,
    );

    for (peer, peer_addr) in sample_payload_header.peers.iter() {
        if peer_addr.clone() != local_addr.clone() {
            client.dial(peer.clone(), peer_addr.clone())
                .await
                .expect("dailing to be not failed");
            client.send_request( peer.clone(), 
                MpcP2pRequest::StartJob { 
                    auth_header: sample_auth_header.clone(),
                    job_header: sample_payload_header.clone(), 
                }
            )
                .await
                .expect("request should be taken");
        }
    }

    let mut job_manager = JobManager::new(
        local_peer_id, &mut client
    );
    
    // Finally, we spin up the job locally
    job_manager.keygen_accept_new_job(sample_payload_header.clone());

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
                // println!("Outgoing sender msg received {:?}", payload);
                job_manager.handle_outgoing(payload).await;
            },
            payload = main_message_receiver.select_next_some() => {
                let payload = bincode::deserialize(&payload).unwrap();
                job_manager.handle_incoming(payload).await;
            },
        }
    }
}
