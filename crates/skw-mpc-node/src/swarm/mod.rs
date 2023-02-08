mod behavior;
mod client;
mod event_loop;

use libp2p::request_response::ProtocolSupport;
use libp2p::{
    identity, mplex, noise, yamux, core,
    tcp, PeerId, Transport,
    Swarm, InboundUpgradeExt, OutboundUpgradeExt, 
    request_response, Multiaddr,
};

use futures::channel::mpsc;
use skw_mpc_payload::{PayloadHeader};

use behavior::{SkwMpcP2pCodec, SkwMpcP2pProtocol, MpcSwarmBahavior};
use crate::error::MpcNodeError;

// re-export
pub use client::MpcSwarmClient;
pub use event_loop::MpcSwarmEventLoop;
pub use behavior::{MpcP2pRequest, MpcP2pResponse};

pub fn new_full_swarm_node(
    local_key: Option<[u8; 32]>
) -> Result<(
    PeerId, // local peer id
    
    MpcSwarmClient, 
    MpcSwarmEventLoop, 

    mpsc::Receiver< Multiaddr >,
    mpsc::Receiver< PayloadHeader >, // new job assignment channel - receiver side
    mpsc::UnboundedReceiver< Vec<u8> >, // main message incoming channel

    mpsc::Sender<bool>, // swarm termination
), MpcNodeError> {
    let local_key = match local_key {
        None => identity::Keypair::generate_ed25519(),
        Some(mut key) => {
            identity::Keypair::Ed25519(
                identity::ed25519::SecretKey::from_bytes(&mut key[..]).unwrap().into()
            )
        }
    };

    let local_peer_id = PeerId::from(local_key.public());
    // eprintln!("Local peer id: {local_peer_id}");

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
        let behaviour = MpcSwarmBahavior {  request_response, };
        Swarm::with_async_std_executor(transport, behaviour, local_peer_id)
    };

    // the main message INCOMING channel 
    let (swarm_incoming_message_sender, swarm_incoming_message_receiver) = mpsc::unbounded();
    
    // the new job notifier
    let (swarm_incoming_job_sender, swarm_incoming_job_receiver) = mpsc::channel(0);

    // the main outgoing channel
    // we give it one buffer so that outgoing can be synced
    let (command_sender, command_receiver) = mpsc::unbounded();

    let (addr_sender, addr_receiver) = mpsc::channel(0);

    let (swarm_termination_sender, swarm_termination_receiver) = mpsc::channel(0);
    Ok( (
        local_peer_id, 
        MpcSwarmClient { command_sender },
        MpcSwarmEventLoop::new(
            swarm, 
            swarm_incoming_message_sender,
            swarm_incoming_job_sender, 
            command_receiver,
            addr_sender,
            swarm_termination_receiver
        ),

        addr_receiver,
        swarm_incoming_job_receiver,
        swarm_incoming_message_receiver,
        swarm_termination_sender,
    ))
}
