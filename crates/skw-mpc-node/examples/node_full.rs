use futures::channel::mpsc;
use skw_mpc_node::{
    node::{full_node_event_loop, NodeClient},
    async_executor
};

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    let (client_request_sender, client_request_receiver) = mpsc::channel(0);

    async_executor(full_node_event_loop(client_request_receiver));

    let mut client = NodeClient::new(client_request_sender);
    client
        .bootstrap_node(
            Some([1u8; 32]), 
            "/ip4/100.104.199.31/tcp/2620/ws".to_string(), 
            "mpc-storage-db-12D3KooWK99VoVxNE7XzyBwXEzW7xhK7Gpv85r9F3V3fyKSUKPH5".to_string()
        ).await;

    client
        .bootstrap_node(
            Some([2u8; 32]), 
            "/ip4/100.104.199.31/tcp/2621/ws".to_string(), 
            "mpc-storage-db-12D3KooWJWoaqZhDaoEFshF7Rh1bpY9ohihFhzcW6d69Lr2NASuq".to_string()
        ).await;

    loop {}
    // client.shutdown(node1.0).await.expect("shutdown not to be failed");
    // client.shutdown(node2.0).await.expect("shutdown not to be failed");
    // client.shutdown(node3.0).await.expect("shutdown not to be failed");
}
