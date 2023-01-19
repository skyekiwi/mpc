use ::core::panic;
use std::collections::HashMap;
use futures::{StreamExt, SinkExt, AsyncBufReadExt, FutureExt};

use libp2p::request_response::ProtocolSupport;
use libp2p::{
    identity, mdns, mplex, noise, yamux, core,
    tcp, PeerId, Transport,
    Swarm, InboundUpgradeExt, OutboundUpgradeExt, 
    request_response,
};

use futures::{Sink, Stream, channel::mpsc};
use skw_mpc_payload::header::PayloadType;
use skw_mpc_payload::{CryptoHash, PayloadHeader, Payload};

use skw_round_based::async_runtime::AsyncProtocol;
use skw_round_based::Msg;

use skw_mpc_protocol::gg20::state_machine::{keygen, sign};

use crate::behavior::{SkwMpcP2pProtocol, MpcP2pRequest};
use crate::behavior::skw_mpc_p2p_behavior::SkwMpcP2pCodec;
use crate::{
    behavior::MpcNodeBahavior, 
    client::{MpcNodeClient}, 
    error::MpcNodeError,
    event_loop::MpcNodeEventLoop
};

pub struct MpcNode {

    local_peer_id: PeerId,
    
    // used for sending commands
    pub client: MpcNodeClient, 

    // event loop
    pub event_loop: MpcNodeEventLoop,

    // job creation
    new_job_from_network_receiver: mpsc::Receiver< (PayloadHeader, Vec<PeerId>) >,

    // main message channel 
    main_message_receiver: mpsc::Receiver< (PayloadHeader, Vec<u8>) >,

    node_start_outgoing_sender: mpsc::Sender<CryptoHash>,
    node_start_outgoing_receiver: mpsc::Receiver<CryptoHash>,

    // job message channels
    keygen_protocol_runs: HashMap<CryptoHash, (
        Vec<PeerId>, // peers 
        PayloadHeader,

        mpsc::Sender<Result<Msg<keygen::ProtocolMessage>, anyhow::Error>>, // incoming - sender,
        mpsc::Receiver<Msg<keygen::ProtocolMessage>>, // outgoing - receiver
    )>,
}

impl MpcNode {
    pub fn new() -> Result<Self, MpcNodeError> {
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());
        eprintln!("Local peer id: {local_peer_id}");

        let transport = {
            let multiplexing_config = {
                let mut mplex_config = mplex::MplexConfig::new();
                mplex_config.set_max_buffer_behaviour(mplex::MaxBufferBehaviour::Block);
                mplex_config.set_max_buffer_size(usize::MAX);
                
                let mut yamux_config = yamux::YamuxConfig::default();
                // Enable proper flow-control: window updates are only sent when
                // buffered data has been consumed.
                yamux_config.set_window_update_mode(yamux::WindowUpdateMode::on_read());

                core::upgrade::SelectUpgrade::new(yamux_config, mplex_config)
                    .map_inbound(core::muxing::StreamMuxerBox::new)
                    .map_outbound(core::muxing::StreamMuxerBox::new)
            };

            tcp::async_io::Transport::new(tcp::Config::default().nodelay(true))
                .upgrade(libp2p::core::upgrade::Version::V1)
                .authenticate(
                    noise::NoiseAuthenticated::xx(&local_key)
                        .expect("Signing libp2p-noise static DH keypair failed."),
                )
                .multiplex(multiplexing_config)
                .boxed()
        };

        let request_response = request_response::Behaviour::<SkwMpcP2pCodec>::new(
            SkwMpcP2pCodec(),
            std::iter::once((SkwMpcP2pProtocol(), ProtocolSupport::Full)),
            Default::default(),
        );

        let swarm = {
            let mdns = mdns::async_io::Behaviour::new(mdns::Config::default(), local_peer_id)
                .map_err(|_| MpcNodeError::FailToListenMDNS)?;
            let behaviour = MpcNodeBahavior { 
                // gossipsub, 
                mdns,
                request_response,
                // keep_alive: keep_alive::Behaviour::default(),
            };
            Swarm::with_async_std_executor(transport, behaviour, local_peer_id)
        };

        // the main message INCOMING channel 
        let (node_incoming_message_sender, node_incoming_message_receiver) = mpsc::channel(0);
        
        // the new job notifier
        let (new_job_from_network_sender, new_job_from_network_receiver) = mpsc::channel(0);

        // the main outgoing channel
        let (command_sender, command_receiver) = mpsc::channel(0);

        // the outgoing mesasge thread spawner channel
        let (node_start_outgoing_sender, node_start_outgoing_receiver) = mpsc::channel(0);
        Ok(Self {
            local_peer_id, 
            client: MpcNodeClient { command_sender },
            event_loop: MpcNodeEventLoop::new(
                swarm, 
                node_incoming_message_sender,
                new_job_from_network_sender, 
                command_receiver
            ),

            new_job_from_network_receiver,

            main_message_receiver: node_incoming_message_receiver,

            node_start_outgoing_sender, node_start_outgoing_receiver,

            keygen_protocol_runs: Default::default(),
        })
    }
    
    pub async fn run(&mut self) {

        loop {
            futures::select!{
                _ = self.event_loop.run().fuse() => {},

                // handle job creation
                (job_header, peers) = self.new_job_from_network_receiver.select_next_some() => {
                    self.node_start_outgoing_sender.send(job_header.clone().payload_id).await.expect("receiver not to be dropped");

                    match job_header.payload_type {
                        PayloadType::KeyGen(maybe_existing_key) => {
                            eprintln!("maybe_existing_key {:?}", maybe_existing_key);

                            // The keygen protocol IO - they are useful for one specific job
                            // We dont attach these channels to the main event channels yet
                            // in the job creation stream - just creating those are good enough
                            let (protocol_in_sender, protocol_in_receiver) = mpsc::channel(0);
                            let (protocol_out_sender, protocol_out_receiver) = mpsc::channel(0);

                            self.keygen_protocol_runs.insert(job_header.payload_id, (
                                peers,
                                job_header,
                                protocol_in_sender,
                                protocol_out_receiver,
                            ));

                            async_std::task::spawn(async move {
                                let keygen_sm = keygen::Keygen::new(1u16, 1u16, 3u16)
                                    .map_err(|e| {})
                                    .unwrap();
                                let output = AsyncProtocol::new(keygen_sm, protocol_in_receiver, protocol_out_sender)
                                    .run()
                                    .await;
                                
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

                // handle_main_incoming_mesasge
                (header, body) = self.main_message_receiver.select_next_some() => {
                    let job_id = header.payload_id;
                    if let Some(( 
                        _peers, 
                        _payload_header, 
                        incoming_sender, 
                        _outgoing_receiver, 
                    )) = self.keygen_protocol_runs.get_mut(&job_id) {
                        incoming_sender.send(Ok(
                            bincode::deserialize(&body).unwrap()
                        ))
                        .await
                        .expect("sender should not be dropped yet.");
                    } else {
                        unimplemented!()
                    }
                },

                // handle_keygen_outgoing_msg
                job_id = self.node_start_outgoing_receiver.select_next_some() => {
                    if let Some((
                        peers, 
                        payload_header, 
                        _incoming, 
                        outgoing
                    )) = self.keygen_protocol_runs.get_mut(&job_id) {
                        
                        // TODO: maybe spawn in new thread?
                        while let Some(msg) = outgoing.next().await {
                            match msg.receiver {
                                // this is a p2p message - only one receiver is assigned
                                Some(to) => {
                                    assert!(to >= 1 && to <= peers.len() as u16, "wrong receiver index");
            
                                    let payload = Payload {
                                        payload_header: payload_header.clone(),
                                        from: self.local_peer_id.to_string(),
                                        to: peers[(to - 1) as usize].to_string(),
                                        body: bincode::serialize(&msg).unwrap(), // TODO: make this unwrap better handled
                                    };
            
                                    self.client
                                        .send_request(peers[(to - 1) as usize], MpcP2pRequest::RawMessage { payload })
                                        .await
                                        .expect("node should take in this request");
                                },
                                // this is a broadcast message
                                None => {
                                    for peer in peers.clone() {
                                        if peer.to_string() != self.local_peer_id.to_string() {
                                            let payload = Payload {
                                                payload_header: payload_header.clone(),
                                                from: self.local_peer_id.to_string(),
                                                to: peer.to_string(),
                                                body: bincode::serialize(&msg).unwrap(), // TODO: make this unwrap better handled
                                            };
                        
                                            self.client
                                                .send_request(peer.clone(), MpcP2pRequest::RawMessage { payload })
                                                .await
                                                .expect("node should take in these requests");
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        panic!("job should exists");
                        // unexpected   
                    }
                }

            }
        }

    }
}
