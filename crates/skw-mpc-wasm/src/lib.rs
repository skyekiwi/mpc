use skw_mpc_client::{swarm::{new_swarm_node, MpcP2pRequest}, async_executor};
use wasm_bindgen::prelude::*;
use skw_mpc_payload::{PayloadHeader, header::PayloadType, AuthHeader};
use std::panic;

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);

    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}


macro_rules! console_log {
    // Note that this is using the `log` function imported above during
    // `bare_bones`
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

#[wasm_bindgen]
pub async fn run(name: &str) {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
    alert(name);
    
    let client_node = (
        "12D3KooWPT98FXMfDQYavZm66EeVjTqP9Nnehn1gyaydqV8L8BQw".parse().unwrap(), 
        "/ip4/10.0.0.3/tcp/2621/ws/p2p/12D3KooWPT98FXMfDQYavZm66EeVjTqP9Nnehn1gyaydqV8L8BQw".parse().unwrap()
    );

    let node1 = (
        "12D3KooWRndVhVZPCiQwHBBBdg769GyrPUW13zxwqQyf9r3ANaba".parse().unwrap(), 
        "/ip4/10.0.0.3/tcp/2620/ws/p2p/12D3KooWRndVhVZPCiQwHBBBdg769GyrPUW13zxwqQyf9r3ANaba".parse().unwrap()
    );

    let node2 = (
        "12D3KooWK99VoVxNE7XzyBwXEzW7xhK7Gpv85r9F3V3fyKSUKPH5".parse().unwrap(), 
        "/ip4/10.0.0.3/tcp/2619/ws/p2p/12D3KooWK99VoVxNE7XzyBwXEzW7xhK7Gpv85r9F3V3fyKSUKPH5".parse().unwrap()
    );

    let node3 = (
        "12D3KooWJWoaqZhDaoEFshF7Rh1bpY9ohihFhzcW6d69Lr2NASuq".parse().unwrap(), 
        "/ip4/10.0.0.3/tcp/2618/ws/p2p/12D3KooWJWoaqZhDaoEFshF7Rh1bpY9ohihFhzcW6d69Lr2NASuq".parse().unwrap()
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

    let (
        _,
        mut client,
        event_loop,
        _termination_sender,
    ) = new_swarm_node( None ).unwrap();
    async_executor(event_loop.run());

    let _ = client
        .dial(client_node.0, client_node.1)
        .await;
    
    console_log!("Sending keygen Req");
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
    
    console_log!("KeyGen Res {:?}", local_key);
    console_log!("Sending Sign Req");
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
    console_log!("Result {:?}", sig_payload);
}
