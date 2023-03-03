use futures::{channel::mpsc, StreamExt};
use skw_mpc_node::{
    node::{NodeClient, light_node_event_loop},
    async_executor
};
use skw_mpc_payload::{PayloadHeader, header::PayloadType, AuthHeader};

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    let (client_request_sender, client_request_receiver) = mpsc::channel(0);
    async_executor(light_node_event_loop(client_request_receiver));
    let mut client = NodeClient::new(client_request_sender);
    
    let mut clinet_node = client
        .bootstrap_node(
            Some([3u8; 32]), 
            "/ip4/10.0.0.3/tcp/2619/ws".to_string(),
            "mpc-storage-db-12D3KooWK99VoVxNE7XzyBwXEzW7xhK7Gpv85r9F3V3fyKSUKPH5".to_string()
        ).await;
    async_executor(async move {
        loop {
            futures::select! {
                client_res = clinet_node.select_next_some() => {
                    match client_res {
                        Ok(_) => {},
                        Err(error) => {
                            log::error!("Node1 Encountered Error: {:?}", error);
                        }
                    }
                },
            }
        }
    });
    
    let node1 = (
        "12D3KooWRndVhVZPCiQwHBBBdg769GyrPUW13zxwqQyf9r3ANaba".parse().unwrap(), 
        "/ip4/10.0.0.3/tcp/2619/ws/p2p/12D3KooWRndVhVZPCiQwHBBBdg769GyrPUW13zxwqQyf9r3ANaba".parse().unwrap()
    );

    let node2 = (
        "12D3KooWK99VoVxNE7XzyBwXEzW7xhK7Gpv85r9F3V3fyKSUKPH5".parse().unwrap(), 
        "/ip4/10.0.0.3/tcp/2620/ws/p2p/12D3KooWK99VoVxNE7XzyBwXEzW7xhK7Gpv85r9F3V3fyKSUKPH5".parse().unwrap()
    );

    let node3 = (
        "12D3KooWJWoaqZhDaoEFshF7Rh1bpY9ohihFhzcW6d69Lr2NASuq".parse().unwrap(), 
        "/ip4/10.0.0.3/tcp/2621/ws/p2p/12D3KooWJWoaqZhDaoEFshF7Rh1bpY9ohihFhzcW6d69Lr2NASuq".parse().unwrap()
    );

    let keygen_request = PayloadHeader {
        payload_id: [0u8; 32],
        payload_type: PayloadType::KeyGen(None),
        peers: vec![node1.clone(), node2.clone(), node3.clone()],
        sender: node1.0,

        t: 2, n: 3
    };

    let sign_request = PayloadHeader {
        payload_id: [1u8; 32],
        payload_type: PayloadType::SignOffline {
            message: [2u8; 32],
            keygen_id: [0u8; 32],
            keygen_peers: vec![node1.clone(), node2.clone(), node3.clone()],
        },
        peers: vec![node1.clone(), node2.clone()],
        sender: node1.0,

        t: 2, n: 3
    };

    let res = client
        .send_request(
            keygen_request,
            AuthHeader::default(),
            None,
        ).await;
    
    println!("KeyGen {:?}", res);

    let res = client
        .send_request(
            sign_request,
            AuthHeader::default(),
            Some(res.unwrap().payload())
        ).await;
    println!("Sign {:?}", res);

    client
        .shutdown(node1.0)
        .await.expect("shutdown not to be failed");
}
