#![cfg(target_arch = "wasm32")]

use skw_mpc_client::{swarm::{new_swarm_node, MpcP2pRequest}, async_executor};
use wasm_bindgen::prelude::*;
use skw_mpc_payload::{PayloadHeader, header::PayloadType, AuthHeader};
use std::panic;

use futures::sink::SinkExt;

#[wasm_bindgen]
pub async fn ext_run_keygen(auth_header: &str, payload: &str, client_identity: &str, client_addr: &str, enable_log: bool) -> String {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
    if enable_log {
        console_log::init_with_level(log::Level::Debug);
    }

    let request: PayloadHeader = serde_json::from_str(payload).unwrap();
    let auth_header: AuthHeader = serde_json::from_str(auth_header).unwrap();

    let ( _, mut client, event_loop, mut shutdown_handler) = new_swarm_node( None );
    async_executor(event_loop.run());

    let client_node = (
        client_identity.parse().unwrap(), 
        client_addr.parse().unwrap()
    );

    client
        .dial(client_node.0, client_node.1)
        .await
        .unwrap();
    
    let res = client
        .send_request(
            client_node.0, 
            MpcP2pRequest::Mpc { 
                auth_header, 
                job_header: request,
                maybe_local_key: None,
            }
        ).await;
    
    shutdown_handler.send(()).await;
    String::from_utf8(res.unwrap().payload()).unwrap()
}

#[wasm_bindgen]
pub async fn ext_run_sign(auth_header: &str, payload: &str, local_key: &str, client_identity: &str, client_addr: &str, enable_log: bool) -> String {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init_with_level(log::Level::Debug);

    let request: PayloadHeader = serde_json::from_str(payload).unwrap();
    let auth_header: AuthHeader = serde_json::from_str(auth_header).unwrap();
    let local_key = local_key.as_bytes();

    let ( _, mut client, event_loop, mut shutdown_handler) = new_swarm_node( None );
    async_executor(event_loop.run());

    let client_node = (
        client_identity.parse().unwrap(), 
        client_addr.parse().unwrap()
    );

    client
        .dial(client_node.0, client_node.1)
        .await
        .unwrap();
    
    let res = client
        .send_request(
            client_node.0, 
            MpcP2pRequest::Mpc { 
                auth_header, 
                job_header: request,
                maybe_local_key: Some(local_key.to_vec()),
            }
        ).await;
    
    shutdown_handler.send(()).await;
    String::from_utf8(res.unwrap().payload()).unwrap()
}
