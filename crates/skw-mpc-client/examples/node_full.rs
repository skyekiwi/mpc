use futures::channel::mpsc;
use skw_mpc_client::swarm::{new_swarm_node};
use skw_mpc_node::{
    node::{NodeClient, light_node_event_loop},
    async_executor
};

#[tokio::main]
async fn main() {
    let (client_request_sender, client_request_receiver) = mpsc::channel(0);
    async_executor(light_node_event_loop(client_request_receiver));
    let mut light_node_client = NodeClient::new(client_request_sender);

    let _ = light_node_client
        .bootstrap_node(
            Some([3u8; 32]), 
            "/ip4/10.0.0.3/tcp/2620/ws".to_string(),
            "mpc-storage-db-12D3KooWK99VoVxNE7XzyBwXEzW7xhK7Gpv85r9F3V3fyKSUKPH5".to_string()
        )
            .await
            .expect("creating not should not fail");

    let (
        local_peer_id,
        mut client,
        event_loop,
        _termination_sender,
    ) = new_swarm_node(
        light_node_client, Some([4u8; 32])
    ).unwrap();

    println!("Local Peer Id {:?}", local_peer_id);

    async_executor(event_loop.run());

    client
        .start_listening("/ip4/10.0.0.3/tcp/2621/ws".parse().expect("multiaddr should be valid"))
        .await
        .unwrap();

    loop {}
}
