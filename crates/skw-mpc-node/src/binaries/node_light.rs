use futures::{channel::{mpsc, oneshot}, SinkExt};
use skw_mpc_node::{
    node::{node_main_event_loop, ClientRequest, NodeClient},
    error::MpcNodeError
};
use skw_mpc_payload::{PayloadHeader, header::PayloadType};

#[async_std::main]
async fn main() -> Result<(), MpcNodeError> {
    let (client_request_sender, client_request_receiver) = mpsc::channel(0);

    async_std::task::spawn(node_main_event_loop(client_request_receiver));
    let mut client = NodeClient::new(client_request_sender);
    let node1 = client
        .bootstrap_node(
            Some([1u8; 32]), 
            "/ip4/10.0.0.3/tcp/0".to_string(), 
            "mpc-storage-db-12D3KooWK99VoVxNE7XzyBwXEzW7xhK7Gpv85r9F3V3fyKSUKPH5".to_string()
        )
        .await
        .expect("creating not should not fail");

    let node2 = client
        .bootstrap_node(
            Some([2u8; 32]), 
            "/ip4/10.0.0.3/tcp/0".to_string(), 
            "mpc-storage-db-12D3KooWJWoaqZhDaoEFshF7Rh1bpY9ohihFhzcW6d69Lr2NASuq".to_string()
        )
        .await
        .expect("creating not should not fail");

    let node3 = client
        .bootstrap_node(
            Some([3u8; 32]), 
            "/ip4/10.0.0.3/tcp/0".to_string(), 
            "mpc-storage-db-12D3KooWRndVhVZPCiQwHBBBdg769GyrPUW13zxwqQyf9r3ANaba".to_string()
        )
        .await
        .expect("creating not should not fail");
    
    println!("{:?}", node1);
    println!("{:?}", node2);
    println!("{:?}", node3);

    let keygen_request = PayloadHeader {
        payload_id: [0u8; 32],
        payload_type: PayloadType::KeyGen(None),
        peers: vec![node1.clone(), node2, node3],
        sender: node1.0,

        t: 2, n: 3
    };

    let res = client
        .send_request(node1.0, keygen_request)
        .await
        .expect("not to be failed");
    
    
    println!("{:?}", res);

    Ok(())
}
