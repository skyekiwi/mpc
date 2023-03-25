#[cfg(feature = "light-node")]

use libp2p::{PeerId, Multiaddr};
use skw_mpc_client::{
    async_executor,
    swarm::{new_swarm_node, MpcP2pRequest}
};
use skw_mpc_node::serde_support::{decode_signature, decode_key};
use skw_mpc_payload::{PayloadHeader, header::PayloadType, AuthHeader};

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let client_node: (PeerId, Multiaddr) = (
        "12D3KooWPT98FXMfDQYavZm66EeVjTqP9Nnehn1gyaydqV8L8BQw".parse().unwrap(), 
        "/ip4/100.104.199.31/tcp/2619/ws/p2p/12D3KooWPT98FXMfDQYavZm66EeVjTqP9Nnehn1gyaydqV8L8BQw".parse().unwrap()
    );

    let node1 = (
        "12D3KooWRndVhVZPCiQwHBBBdg769GyrPUW13zxwqQyf9r3ANaba".parse().unwrap(), 
        "/ip4/100.104.199.31/tcp/2622/ws/p2p/12D3KooWRndVhVZPCiQwHBBBdg769GyrPUW13zxwqQyf9r3ANaba".parse().unwrap()
    );

    let node2 = (
        "12D3KooWJWoaqZhDaoEFshF7Rh1bpY9ohihFhzcW6d69Lr2NASuq".parse().unwrap(), 
        "/ip4/100.104.199.31/tcp/2621/ws/p2p/12D3KooWJWoaqZhDaoEFshF7Rh1bpY9ohihFhzcW6d69Lr2NASuq".parse().unwrap()
    );

    let node3 = (
        "12D3KooWK99VoVxNE7XzyBwXEzW7xhK7Gpv85r9F3V3fyKSUKPH5".parse().unwrap(), 
        "/ip4/100.104.199.31/tcp/2620/ws/p2p/12D3KooWK99VoVxNE7XzyBwXEzW7xhK7Gpv85r9F3V3fyKSUKPH5".parse().unwrap()  
    );

    let keygen_request = PayloadHeader {
        payload_id: [0u8; 32],
        payload_type: PayloadType::KeyGen,
        peers: vec![node1.clone(), node2.clone(), node3.clone()],
        sender: node1.0,

        t: 2, n: 3
    };

    let keygen2_request = PayloadHeader {
        payload_id: [4u8; 32],
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


    println!("KeyGen {:?}", serde_json::to_string(&keygen_request));
    println!("Sign {:?}", serde_json::to_string(&sign_request));
    println!("KeyRefresh {:?}", serde_json::to_string(&key_refresh_request));
    println!("AuthHeader {:?}", serde_json::to_string(&AuthHeader::test_auth_header()));


    let ( _, mut client, event_loop, _) = new_swarm_node( None );
    async_executor(event_loop.run());

    let _ = client
        .dial(client_node.0, client_node.1)
        .await;
    
    println!("Sending keygen Req");
    let local_key = client
        .send_request(
            client_node.0, 
            MpcP2pRequest::Mpc { 
                auth_header: AuthHeader::test_auth_header(), 
                job_header: keygen_request,
                maybe_local_key: None,
            }
        ).await;

    let local_key = local_key.unwrap().payload().unwrap();
    
    println!("KeyGen Res {:?}", decode_key(&local_key));
    println!("Sending Sign Req");

    let res = client
        .send_request(
            client_node.0, 
            MpcP2pRequest::Mpc { 
                auth_header: AuthHeader::test_auth_header(),
                job_header: sign_request,
                maybe_local_key: Some(local_key),
            }
        ).await;

    let sig_payload = res.unwrap().payload().unwrap();
    let sig = decode_signature(&sig_payload);
    println!("Result {:?}", sig);


    let new_key = client
    .send_request(
        client_node.0, 
        MpcP2pRequest::Mpc { 
            auth_header: AuthHeader::test_auth_header(), 
            job_header: key_refresh_request,
            maybe_local_key: None,
        }
    ).await;

    let new_key = new_key.unwrap().payload().unwrap();

    println!("New Key {:?}", decode_key(&new_key));

    // wait for 1 seconds?
    let ten_millis = std::time::Duration::from_millis(3000);    
    std::thread::sleep(ten_millis);

    let res = client
        .send_request(
            client_node.0, 
            MpcP2pRequest::Mpc { 
                auth_header: AuthHeader::test_auth_header(),
                job_header: sign2_request,
                maybe_local_key: Some(new_key),
            }
        ).await;

    let sig_payload = res.unwrap().payload().unwrap();
    let sig = decode_signature(&sig_payload);
    println!("Result {:?}", sig);


    let abort_overwriting_key = client
        .send_request(
            client_node.0, 
            MpcP2pRequest::Mpc { 
                auth_header: AuthHeader::test_auth_header(), 
                job_header: keygen2_request,
                maybe_local_key: None,
            }
        ).await;

    println!("Result {:?}", abort_overwriting_key);

}
