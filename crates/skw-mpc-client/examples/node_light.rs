use core::panic;

use futures::channel::mpsc;
use libp2p::{PeerId, Multiaddr};
use skw_mpc_client::{
    async_executor,
    swarm::{new_swarm_node, MpcP2pRequest, MpcP2pResponse}
};
use skw_mpc_node::serde_support::{decode_signature, decode_key};
use skw_mpc_payload::{PayloadHeader, header::PayloadType, AuthHeader};

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let client_node: (PeerId, Multiaddr) = (
        "12D3KooWPT98FXMfDQYavZm66EeVjTqP9Nnehn1gyaydqV8L8BQw".parse().unwrap(), 
        "/ip4/10.0.0.3/tcp/2622/ws/p2p/12D3KooWPT98FXMfDQYavZm66EeVjTqP9Nnehn1gyaydqV8L8BQw".parse().unwrap()
    );

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

    let ( _, mut client, event_loop, _) = new_swarm_node( None );
    async_executor(event_loop.run());

    let _ = client
        .dial(client_node.0, client_node.1)
        .await;
    
    println!("Sending keygen Req");
    let res = client
        .send_request(
            client_node.0, 
            MpcP2pRequest::Mpc { 
                auth_header: AuthHeader::default(), 
                job_header: keygen_request,
                maybe_local_key: None,
            }
        ).await;
    let local_key = res.unwrap().payload();
    
    println!("KeyGen Res {:?}", decode_key(&local_key));
    println!("Sending Sign Req");

    let res = client
        .send_request(
            client_node.0, 
            MpcP2pRequest::Mpc { 
                auth_header: AuthHeader::default(), 
                job_header: sign_request,
                maybe_local_key: Some(local_key),
            }
        ).await;

    let sig_payload = res.unwrap().payload();
    let sig = decode_signature(&sig_payload);
    println!("Result {:?}", sig);
}
