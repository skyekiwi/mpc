pub mod behavior;
pub mod client;

use libp2p::request_response::ProtocolSupport;
use libp2p::{
    identity, PeerId, Swarm,
    request_response,
    noise, yamux, Transport,
};

use self::behavior::{MpcSwarmBahavior, SkwMpcP2pCodec, SkwMpcP2pProtocol};

#[cfg(feature = "tcp-ws-transport")]
pub fn build_swarm(local_key: identity::Keypair) -> Swarm<MpcSwarmBahavior> {
    use std::time::Duration;

    use libp2p::{websocket, tcp, dns, swarm::SwarmBuilder};
    let local_peer_id = PeerId::from(local_key.public());

    let transport = {
        let mut yamux_config = yamux::YamuxConfig::default();
        // Enable proper flow-control: window updates are only sent when
        // buffered data has been consumed.
        yamux_config.set_window_update_mode(yamux::WindowUpdateMode::on_read());

        websocket::WsConfig::new(dns::TokioDnsConfig::system(
            tcp::tokio::Transport::new(tcp::Config::default()),
        ).unwrap())
            .upgrade(libp2p::core::upgrade::Version::V1)
            .authenticate(
                noise::NoiseAuthenticated::xx(&local_key)
                    .expect("Signing libp2p-noise static DH keypair failed."),
            )
            .multiplex(yamux_config)
            .timeout(Duration::from_secs(10))
            .boxed()
    };

    let request_response = request_response::Behaviour::<SkwMpcP2pCodec>::new(
        SkwMpcP2pCodec(),
        std::iter::once((SkwMpcP2pProtocol(), ProtocolSupport::Full)),
        Default::default(),
    );
    let behaviour = MpcSwarmBahavior {  request_response, };
    SwarmBuilder::with_tokio_executor(transport, behaviour, local_peer_id).build()
}
