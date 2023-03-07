use std::{fs, io::Write};

use futures::channel::mpsc;
use futures::StreamExt;

use skw_mpc_client::{
    swarm::{new_swarm_node},
    async_executor,
};
use skw_mpc_node::{
    node::{NodeClient, light_node_event_loop},
};

const LISTEN_ADDR: &str = "143.198.142.119";

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let (client_request_sender, client_request_receiver) = mpsc::channel(0);
    async_executor(light_node_event_loop(client_request_receiver));
    let mut light_node_client = NodeClient::new(client_request_sender);

    let mut light_client_node_res = light_node_client
    .bootstrap_node(
        None,
        format!("/ip4/{}/tcp/2622/ws", LISTEN_ADDR),
        "mpc-storage-db-12D3KooWK99VoVxNE7XzyBwXEzW7xhK7Gpv85r9F3V3fyKSUKPH5".to_string()
    ).await;

    async_executor(async move {
        loop {
            let res = light_client_node_res.select_next_some().await;
            log::error!("Result {:?}", res);
        }
    });

    let peer_id = light_node_client.peer_id();

    let (
        local_peer_id,
        mut client,
        event_loop,
        _termination_sender,
    ) = new_swarm_node( light_node_client, None );
    async_executor(event_loop.run());

    client
        .start_listening(format!("/ip4/{}/tcp/2619/ws", LISTEN_ADDR).parse().expect("multiaddr should be valid"))
        .await
        .unwrap();

    let env_file_node1 = format!("LIGHT_NODE_ID = {}\n", peer_id.to_string());
    let env_file_node2 = format!("CLIENT_NODE_ID = {}\n", local_peer_id.to_string());

    let mut file = fs::OpenOptions::new()
        .append(true)
        .open("./.env.peers")
        .expect("able to open a file");

    file.write_all(env_file_node1.as_bytes()).expect("able to write");
    file.write_all(env_file_node2.as_bytes()).expect("able to write");
    
    std::env::set_var("LIGHT_NODE_ID", peer_id.to_string());
    std::env::set_var("CLIENT_NODE_ID", local_peer_id.to_string());

    log::info!("Init done. PeerIds has been written to .env.peers");
    loop {}
}