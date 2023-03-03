use std::{fs, io::Write};

use futures::channel::mpsc;
use skw_mpc_node::{
    node::{full_node_event_loop, NodeClient},
    async_executor
};

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    // Full Node 
    let (client_request_sender, client_request_receiver) = mpsc::channel(0);

    async_executor(full_node_event_loop(client_request_receiver));

    let mut client = NodeClient::new(client_request_sender);
    client
        .bootstrap_node(
            None, 
            "/ip4/0.0.0.0/tcp/2620/ws".to_string(), 
            "mpc-storage-db-fullnode1".to_string()
        ).await;
    
    let peer_id_1 = client.peer_id();

    client
        .bootstrap_node(
            None, 
            "/ip4/0.0.0.0/tcp/2621/ws".to_string(), 
            "mpc-storage-db-fullnode2".to_string()
        ).await;
    
    let peer_id_2 = client.peer_id();
        
    let env_file_node1 = format!("FULL_NODE1_ID = {}\n", peer_id_1.to_string());
    let env_file_node2 = format!("FULL_NODE2_ID = {}\n", peer_id_2.to_string());

    let mut file = fs::OpenOptions::new()
        .append(true)
        .open("./.env.test")
        .expect("able to open a file");

    file.write_all(env_file_node1.as_bytes()).expect("abe to write");
    file.write_all(env_file_node2.as_bytes()).expect("abe to write");

    loop {}
}