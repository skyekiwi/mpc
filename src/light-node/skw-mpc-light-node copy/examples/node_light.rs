use futures::{channel::mpsc, StreamExt};
use skw_mpc_node::{
    node::{NodeClient, light_node_event_loop},
    async_executor, serde_support::decode_key
};
use skw_mpc_payload::{PayloadHeader, header::PayloadType, AuthHeader};
use std::{thread, time};

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    let (client_request_sender, client_request_receiver) = mpsc::channel(0);
    async_executor(light_node_event_loop(client_request_receiver));
    let mut client = NodeClient::new(client_request_sender);
    
    let mut clinet_node = client
        .bootstrap_node(
            Some([3u8; 32]), 
            "/ip4/100.104.199.31/tcp/2619/ws".to_string(),
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
        "/ip4/100.104.199.31/tcp/2619/ws/p2p/12D3KooWRndVhVZPCiQwHBBBdg769GyrPUW13zxwqQyf9r3ANaba".parse().unwrap()
    );

    let node2 = (
        "12D3KooWK99VoVxNE7XzyBwXEzW7xhK7Gpv85r9F3V3fyKSUKPH5".parse().unwrap(), 
        "/ip4/100.104.199.31/tcp/2620/ws/p2p/12D3KooWK99VoVxNE7XzyBwXEzW7xhK7Gpv85r9F3V3fyKSUKPH5".parse().unwrap()
    );

    let node3 = (
        "12D3KooWJWoaqZhDaoEFshF7Rh1bpY9ohihFhzcW6d69Lr2NASuq".parse().unwrap(), 
        "/ip4/100.104.199.31/tcp/2621/ws/p2p/12D3KooWJWoaqZhDaoEFshF7Rh1bpY9ohihFhzcW6d69Lr2NASuq".parse().unwrap()
    );

    let keygen_request = PayloadHeader {
        payload_id: [0u8; 32],
        payload_type: PayloadType::KeyGen,
        peers: vec![node1.clone(), node2.clone(), node3.clone()],
        sender: node1.0,

        t: 2, n: 3
    };

    let sign_request = PayloadHeader {
        payload_id: [1u8; 32],
        payload_type: PayloadType::SignOffline { message: [2u8; 32] },
        peers: vec![node1.clone(), node2.clone()],
        sender: node1.0,

        t: 2, n: 3
    };

    let key_refresh_request = PayloadHeader {
        payload_id: [2u8; 32],
        payload_type: PayloadType::KeyRefresh,
        peers: vec![node1.clone(), node2.clone(), node3.clone()],
        sender: node1.0,

        t: 2, n: 3
    };

    let sign2_request = PayloadHeader {
        payload_id: [3u8; 32],
        payload_type: PayloadType::SignOffline { message: [2u8; 32] },
        peers: vec![node1.clone(), node2.clone()],
        sender: node1.0,

        t: 2, n: 3
    };

    let local_key = client
        .send_request(
            keygen_request,
            AuthHeader::test_auth_header(),
            None,
        ).await;
    
    println!("KeyGen {:?}", decode_key(&local_key.clone().unwrap().payload()));

    let sign_res = client
        .send_request(
            sign_request,
            AuthHeader::test_auth_header(),
            Some(local_key.unwrap().payload())
        ).await;
    println!("Sign {:?}", sign_res);


    // Now we lost the key ... and want a key refresh then sign the same thing again
    let new_key = client
        .send_request(
            key_refresh_request, 
            AuthHeader::test_auth_header(), 
            None
        ).await;
    
    println!("KeyRefresh {:?}", decode_key(&new_key.clone().unwrap().payload()));

    let ten_millis = time::Duration::from_millis(5000);    
    thread::sleep(ten_millis);
    
    let sign_new_res = client
        .send_request(
            sign2_request,
            AuthHeader::test_auth_header(),
            Some(new_key.unwrap().payload())
        ).await;
    println!("Sign {:?}", sign_new_res);


    client
        .shutdown(node1.0)
        .await.expect("shutdown not to be failed");
}
