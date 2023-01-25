use std::collections::HashMap;

use futures::{StreamExt, SinkExt, AsyncBufReadExt, FutureExt, channel::mpsc};

use libp2p::{PeerId, Multiaddr};
use skw_mpc_node::{
    node::{new_full_node},
    error::MpcNodeError, behavior::MpcP2pRequest
};
use skw_mpc_payload::{CryptoHash, PayloadHeader, header::PayloadType, AuthHeader, Payload};
use skw_round_based::{async_runtime::AsyncProtocol, Msg};

use skw_mpc_protocol::gg20::state_machine::{keygen, sign};

type KeyGenMessage = Msg<keygen::ProtocolMessage>;

#[async_std::main]
async fn main() -> Result<(), MpcNodeError> {
    let (
        local_peer_id,
        
        mut client,
        event_loop,

        mut job_assignment_receiver,
        mut main_message_receiver,
    ) = new_full_node()?; 
    
    let _event_loop_jh = async_std::task::spawn(event_loop.run());

    client.start_listening("/ip4/10.0.0.3/tcp/0".parse().expect("address need to be valid"))
        .await
        .expect("Listen not to fail.");

    client.dial(
        "12D3KooWR1XwiHNUgpPXmjEnXGAscHM3ELMGNv2DKbWtANga1NUM".parse().unwrap(), 
        Some("/ip4/10.0.0.3/tcp/53079/p2p/12D3KooWR1XwiHNUgpPXmjEnXGAscHM3ELMGNv2DKbWtANga1NUM".parse().unwrap())
    )
        .await
        .expect("dailing to be not failed");

    client.dial(
        "12D3KooWPLCNVJ8hN7abW65bag7CY5gr4PLYQxqVzDphw4bdGXpf".parse().unwrap(), 
        Some("/ip4/10.0.0.3/tcp/53080/p2p/12D3KooWPLCNVJ8hN7abW65bag7CY5gr4PLYQxqVzDphw4bdGXpf".parse().unwrap())
    )
        .await
        .expect("dailing to be not failed");

    client.send_request(
        "12D3KooWR1XwiHNUgpPXmjEnXGAscHM3ELMGNv2DKbWtANga1NUM".parse().expect("right peer id"), 
        MpcP2pRequest::StartJob { 
            auth_header: AuthHeader::default(), 
            job_header: PayloadHeader::new(
                [0u8; 32], 
                PayloadType::KeyGen(None), 
                vec![
                    local_peer_id,
                    "12D3KooWR1XwiHNUgpPXmjEnXGAscHM3ELMGNv2DKbWtANga1NUM".parse().unwrap(),
                    "12D3KooWPLCNVJ8hN7abW65bag7CY5gr4PLYQxqVzDphw4bdGXpf".parse().unwrap(),
                ],
                local_peer_id,
                1, 3,
            ), 
        }
    )
        .await
        .expect("request should be taken");
    
    client.send_request(
        "12D3KooWPLCNVJ8hN7abW65bag7CY5gr4PLYQxqVzDphw4bdGXpf".parse().expect("right peer id"), 
        MpcP2pRequest::StartJob { 
            auth_header: AuthHeader::default(), 
            job_header: PayloadHeader::new(
                [0u8; 32], 
                PayloadType::KeyGen(None), 
                vec![
                    local_peer_id,
                    "12D3KooWR1XwiHNUgpPXmjEnXGAscHM3ELMGNv2DKbWtANga1NUM".parse().unwrap(),
                    "12D3KooWPLCNVJ8hN7abW65bag7CY5gr4PLYQxqVzDphw4bdGXpf".parse().unwrap(),
                ],
                local_peer_id,
                1, 3,
            ),
        }
    )
        .await
        .expect("request should be taken");

        let mut channel_map = HashMap::<
        CryptoHash, mpsc::Sender<Result<Payload<KeyGenMessage>, std::io::Error>>,  // protocol incoming
    >::new();
    
    let (main_outgoing_sender, mut main_outgoing_receiver) = mpsc::channel::<Payload<KeyGenMessage>>(0);

    /* spin up all event loops */    
    // the job channel never closes - same lifetime as the binary
    println!("starting event loops");
    
    loop {
        futures::select! {
            payload_header = job_assignment_receiver.select_next_some() => {
                match payload_header.payload_type {
                    PayloadType::KeyGen(maybe_existing_key) => {
                        eprintln!("maybe_existing_key {:?}", maybe_existing_key);
    
                        // The keygen protocol IO - they are useful for one specific job
                        // We dont attach these channels to the main event channels yet
                        // in the job creation stream - just creating those are good enough
                        let (protocol_in_sender, protocol_in_receiver) = mpsc::channel(0);
                        let protocol_outgoing_sender = main_outgoing_sender.clone();
                        channel_map.insert(
                            payload_header.clone().payload_id, protocol_in_sender
                        );

                        eprintln!("Starting local keygen process");
                        async_std::task::spawn(async move {
                            let keygen_sm = keygen::Keygen::new(1u16, 1u16, 3u16)
                                .map_err(|e| { println!("Protocl Error {:?}", e) })
                                .unwrap();
                            let output = AsyncProtocol::new(keygen_sm, 
                                protocol_in_receiver, 
                                protocol_outgoing_sender, 
                                payload_header.clone()
                            )
                                .run()
                                .await; // TODO: discard all error?

                            println!("{:?}", output);
                        });
                    },
                    PayloadType::Signing(_) => {
                        unimplemented!()
                    },
                    PayloadType::KeyRefresh => {
                        unimplemented!()
                    }
                }
            },
            payload = main_outgoing_receiver.select_next_some() => {
                // println!("Outgoing sender msg received {:?}", payload);
                match payload.body.receiver {
                    // this is a p2p message - only one receiver is assigned
                    Some(to) => {
                        assert!(to >= 1 && to <= payload.payload_header.peers.len() as u16, "wrong receiver index");
                        let to_peer = payload.payload_header.peers[(to - 1) as usize];
                        client
                            .dial(to_peer, None)
                            .await
                            .expect("client should not be dropped");
                        
                        let mut payload_out = payload.clone();
                        payload_out.payload_header.sender = local_peer_id;
                        client
                            .send_request(to_peer, MpcP2pRequest::RawMessage { 
                                payload: bincode::serialize( &payload_out ).unwrap()
                             })
                            .await
                            .expect("client should not be dropped, node should take in this request");
                    },
                    // this is a broadcast message
                    None => {
                        for peer in payload.clone().payload_header.peers {
                            if peer.to_string() != local_peer_id.to_string() {
                                client
                                    .dial(peer, None)
                                    .await
                                    .expect("client should not be dropped");
                                
                                let mut payload_out = payload.clone();
                                payload_out.payload_header.sender = local_peer_id;
                                let x = client
                                    .send_request(peer.clone(), MpcP2pRequest::RawMessage { 
                                        payload: bincode::serialize(&payload_out).unwrap() 
                                    })
                                    .await;
                                println!("Got Respobnse {:?}", x);
                                    // .expect("node should take in these requests");
                            }
                        }
                    }
                }
            },
            payload = main_message_receiver.select_next_some() => {
                let payload = bincode::deserialize::<Payload<KeyGenMessage>>(&payload).unwrap();
                println!("Received Payload {:?} {:?}", payload.payload_header, channel_map);
                // let pipe = channel_map.get_mut(&payload.payload_header.payload_id).unwrap();
                // pipe.send( Ok(payload) )
                //     .await
                //     .expect("protocol income sender should not be dropped .. yet");
            },
        }
    }
}
