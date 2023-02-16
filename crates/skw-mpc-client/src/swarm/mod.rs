mod behavior;
mod client;
mod event_loop;

use libp2p::request_response::ProtocolSupport;
use libp2p::{
    identity, PeerId, Swarm,
    request_response,
    mplex, noise, yamux, core, InboundUpgradeExt, OutboundUpgradeExt, Transport,
};

use futures::channel::mpsc;

#[cfg(feature = "full-node")]
use skw_mpc_node::node::NodeClient;

use crate::error::MpcClientError;

use self::behavior::{MpcSwarmBahavior, SkwMpcP2pCodec, SkwMpcP2pProtocol};

// re-export
pub use client::MpcSwarmClient;
pub use event_loop::MpcSwarmEventLoop;
pub use behavior::{MpcP2pRequest, MpcP2pResponse};

#[cfg(feature = "tcp-ws-transport")]
fn build_swarm(local_key: identity::Keypair) -> Swarm<MpcSwarmBahavior> {
    use libp2p::{websocket, tcp};
    let local_peer_id = PeerId::from(local_key.public());

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

        websocket::WsConfig::new(tcp::tokio::Transport::new(tcp::Config::default().nodelay(true)))
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
    let behaviour = MpcSwarmBahavior {  request_response, };
    Swarm::with_tokio_executor(transport, behaviour, local_peer_id)
}


#[cfg(all(feature = "wasm-transport", target_arch = "wasm32"))]
fn build_swarm(local_key: identity::Keypair) -> Swarm<MpcSwarmBahavior> {
    use libp2p::wasm_ext;
    let local_peer_id = PeerId::from(local_key.public());

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

        let transport_base = wasm_ext::ffi::websocket_transport();
        let transport_base = wasm_ext::ExtTransport::new(transport_base);
        libp2p::Transport::upgrade(transport_base, libp2p::core::upgrade::Version::V1)
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
    let behaviour = MpcSwarmBahavior {  request_response, };
    Swarm::with_wasm_executor(transport, behaviour, local_peer_id)
}

pub fn new_swarm_node(
    #[cfg(feature = "full-node")] bootstrapped_client: NodeClient,
    local_key: Option<[u8; 32]>
) -> (
    PeerId, // local peer id
    MpcSwarmClient, 
    MpcSwarmEventLoop,
    mpsc::Sender<()>, // swarm termination
) {
    let local_key = match local_key {
        None => identity::Keypair::generate_ed25519(),
        Some(mut key) => {
            identity::Keypair::Ed25519(
                identity::ed25519::SecretKey::from_bytes(&mut key[..]).unwrap().into()
            )
        }
    };

    let local_peer_id = PeerId::from(local_key.public());    
    let swarm = build_swarm(local_key);

    // the main outgoing channel
    // we give it one buffer so that outgoing can be synced
    let (command_sender, command_receiver) = mpsc::unbounded();
    let (swarm_termination_sender, swarm_termination_receiver) = mpsc::channel(0);

    (
        local_peer_id, 
        MpcSwarmClient { command_sender },
        MpcSwarmEventLoop::new(
            #[cfg(feature = "full-node")] bootstrapped_client,
            swarm, 
            command_receiver,
            swarm_termination_receiver
        ),
        swarm_termination_sender,
    )
}
