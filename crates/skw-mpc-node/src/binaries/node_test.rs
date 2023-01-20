use std::collections::HashMap;

use futures::{StreamExt, SinkExt, AsyncBufReadExt, FutureExt, channel::mpsc};

use libp2p::PeerId;
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

    client.start_listening("/ip4/0.0.0.0/tcp/0".parse().expect("address need to be valid"))
        .await
        .expect("Listen not to fail.");

    client.dial(
        "12D3KooWM8oPXYPMFBqVoQGGgTuy5vUfW2CuSZoPNorVrSoHMVm4".parse().unwrap(), 
        "/ip4/100.104.199.31/tcp/64769/p2p/12D3KooWM8oPXYPMFBqVoQGGgTuy5vUfW2CuSZoPNorVrSoHMVm4".parse().unwrap()
    )
        .await
        .expect("dailing to be not failed");

    client.dial(
        "12D3KooWJFLENTzhKpkRkKKGb5jabkmhjydaswssTraehzAZqU5p".parse().unwrap(), 
        "/ip4/100.104.199.31/tcp/64786/p2p/12D3KooWJFLENTzhKpkRkKKGb5jabkmhjydaswssTraehzAZqU5p".parse().unwrap()
    )
        .await
        .expect("dailing to be not failed");

    client.send_request(
        "12D3KooWJFLENTzhKpkRkKKGb5jabkmhjydaswssTraehzAZqU5p".parse().expect("right peer id"), 
        MpcP2pRequest::StartJob { 
            auth_header: AuthHeader::default(), 
            job_header: PayloadHeader::new(
                [0u8; 32], 
                PayloadType::KeyGen(None), 
                1, 3,
            ), 
            nodes: vec![
                local_peer_id.to_string(), 
                "12D3KooWM8oPXYPMFBqVoQGGgTuy5vUfW2CuSZoPNorVrSoHMVm4".to_string(), 
                "12D3KooWJFLENTzhKpkRkKKGb5jabkmhjydaswssTraehzAZqU5p".to_string(),
            ],
        }
    )
        .await
        .expect("request should be taken");

    /* Internal Memory state to keep in heap and stay alive the whole time */
    // channel handler of all spin up tasks
    let mut channel_map = HashMap::<
        CryptoHash, mpsc::Sender<Result<KeyGenMessage, anyhow::Error>>, // protocol incoming
    >::new();

    // channel to allocate a new job and spin up a new thread for it
    let (mut main_outgoing_sender, mut main_outgoing_receiver) = mpsc::channel(0);
    
    /* spin up all event loops */    
    // the job channel never closes - same lifetime as the binary
    loop {
        futures::select! {
            (job_header, peers) = job_assignment_receiver.select_next_some() => {

                println!("{:?} {:?}", job_header, peers);
                match job_header.payload_type {
                    PayloadType::KeyGen(maybe_existing_key) => {
                        eprintln!("maybe_existing_key {:?}", maybe_existing_key);
    
                        // The keygen protocol IO - they are useful for one specific job
                        // We dont attach these channels to the main event channels yet
                        // in the job creation stream - just creating those are good enough
                        let (protocol_in_sender, protocol_in_receiver) = mpsc::channel::<Result<KeyGenMessage, anyhow::Error>>(0);
                        let (protocol_out_sender, protocol_out_receiver) = mpsc::channel(0);
    
                        channel_map.insert(job_header.clone().payload_id, protocol_in_sender);
                        main_outgoing_sender.send((
                            peers, job_header, protocol_out_receiver
                        ))
                            .await
                            .expect("main outgoing handler should not be dropped");

                        async_std::task::spawn(async move {
                            let keygen_sm = keygen::Keygen::new(1u16, 1u16, 3u16)
                                .map_err(|e| { println!("Protocl Error {:?}", e) })
                                .unwrap();
                            let output = AsyncProtocol::new(keygen_sm, protocol_in_receiver, protocol_out_sender)
                                .run()
                                .await; // discard all error?

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
            (header, body) = main_message_receiver.select_next_some() => {
                let pipe = channel_map.get_mut(&header.payload_id).unwrap();
                pipe.send(Ok( bincode::deserialize(&body).unwrap()) )
                    .await
                    .expect("protocol income sender should not be dropped .. yet");
            },
            (peers, job_header, mut protocol_out_receiver) = main_outgoing_receiver.select_next_some() => {
                loop { 
                    let msg = protocol_out_receiver.select_next_some().await;
                    match msg.receiver {
                        // this is a p2p message - only one receiver is assigned
                        Some(to) => {
                            assert!(to >= 1 && to <= peers.len() as u16, "wrong receiver index");
    
                            let payload = Payload {
                                payload_header: job_header.clone(),
                                from: local_peer_id.to_string(),
                                to: peers[(to - 1) as usize].to_string(),
                                body: bincode::serialize(&msg).unwrap(), // TODO: make this unwrap better handled
                            };
    
                            client
                                .send_request(peers[(to - 1) as usize], MpcP2pRequest::RawMessage { payload })
                                .await
                                .expect("client should not be dropped, node should take in this request");
                        },
                        // this is a broadcast message
                        None => {
                            for peer in peers.clone() {
                                if peer.to_string() != local_peer_id.to_string() {
                                    let payload = Payload {
                                        payload_header: job_header.clone(),
                                        from: local_peer_id.to_string(),
                                        to: peer.to_string(),
                                        body: bincode::serialize(&msg).unwrap(), // TODO: make this unwrap better handled
                                    };
                
                                    client
                                        .send_request(peer.clone(), MpcP2pRequest::RawMessage { payload })
                                        .await
                                        .expect("node should take in these requests");
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
