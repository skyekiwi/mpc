use std::{fs, io::Write};

use futures::channel::mpsc;
use skw_mpc_client::{
    swarm::{new_swarm_node},
    async_executor,
};
use skw_mpc_node::{
    node::{NodeClient, light_node_event_loop},
};

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let (client_request_sender, client_request_receiver) = mpsc::channel(0);
    async_executor(light_node_event_loop(client_request_receiver));
    let mut light_node_client = NodeClient::new(client_request_sender);

    light_node_client
        .bootstrap_node(
            None, 
            "/ip4/0.0.0.0/tcp/2622/ws".to_string(),
            "mpc-storage-db-light-node".to_string()
        ).await;

    let peer_id = light_node_client.peer_id();

    let (
        local_peer_id,
        mut client,
        event_loop,
        _termination_sender,
    ) = new_swarm_node( light_node_client, Some([4u8; 32]) );
    async_executor(event_loop.run());

    client
        .start_listening("/ip4/0.0.0.0/tcp/2619/ws".parse().expect("multiaddr should be valid"))
        .await
        .unwrap();

    let env_file_node1 = format!("LIGHT_NODE_ID = {}\n", peer_id.to_string());
    let env_file_node2 = format!("CLIENT_NODE_ID = {}\n", local_peer_id.to_string());

    let mut file = fs::OpenOptions::new()
        .append(true)
        .open("./.env.test")
        .expect("able to open a file");

    file.write_all(env_file_node1.as_bytes()).expect("abe to write");
    file.write_all(env_file_node2.as_bytes()).expect("abe to write");
    
    loop {}
}