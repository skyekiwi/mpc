use libp2p::request_response::ProtocolSupport;
use libp2p::{
    identity, mplex, noise, yamux, core,
    tcp, PeerId, Transport,
    Swarm, InboundUpgradeExt, OutboundUpgradeExt, 
    request_response,
};

use futures::channel::mpsc;
use skw_mpc_payload::{PayloadHeader};

use crate::behavior::SkwMpcP2pProtocol;
use crate::behavior::skw_mpc_p2p_behavior::SkwMpcP2pCodec;
use crate::{
    behavior::MpcNodeBahavior, 
    client::{MpcNodeClient}, 
    error::MpcNodeError,
    event_loop::MpcNodeEventLoop
};

pub fn new_full_node() -> Result<(
    PeerId, // local peer id
    
    MpcNodeClient, 
    MpcNodeEventLoop, 

    mpsc::Receiver< PayloadHeader >, // new job assignment channel - receiver side
    mpsc::Receiver< Vec<u8> >, // main message incoming channel
), MpcNodeError> {
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
        // let mdns = mdns::async_io::Behaviour::new(mdns::Config::default(), local_peer_id)
        //     .map_err(|_| MpcNodeError::FailToListenMDNS)?;
        let behaviour = MpcNodeBahavior { 
            // gossipsub, 
            // mdns,
            request_response,
            // keep_alive: keep_alive::Behaviour::default(),
        };
        Swarm::with_async_std_executor(transport, behaviour, local_peer_id)
    };

    // the main message INCOMING channel 
    let (node_incoming_message_sender, node_incoming_message_receiver) = mpsc::channel(0);
    
    // the new job notifier
    let (node_incoming_job_sender, node_incoming_job_receiver) = mpsc::channel(0);

    // the main outgoing channel
    let (command_sender, command_receiver) = mpsc::channel(0);

    Ok( (
        local_peer_id, 
        MpcNodeClient { command_sender },
        MpcNodeEventLoop::new(
            swarm, 
            node_incoming_message_sender,
            node_incoming_job_sender, 
            command_receiver
        ),

        node_incoming_job_receiver,
        node_incoming_message_receiver,
    ))
}
