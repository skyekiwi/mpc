use libp2p::{
    identity, mdns, mplex, noise, yamux, core,
    tcp, PeerId, Transport,
    floodsub::{Floodsub},
    Swarm, InboundUpgradeExt, OutboundUpgradeExt, 
};

use serde::{Serialize, de::DeserializeOwned};

use futures::{Sink, Stream, channel::mpsc};
use crate::{
    behavior::MpcPubsubBahavior, 
    client::{MpcPubSubClient, MpcPubSubRequest}, 
    error::MpcPubSubError,
    event_loop::MpcPubSubNodeEventLoop
};

pub async fn new_node<M>() -> Result<(
    MpcPubSubClient,
    MpcPubSubNodeEventLoop<M>,
    impl Stream<Item = Result<M, anyhow::Error> >, //incoming msg
    impl Sink<M>, // outgoing msg
), MpcPubSubError> 
    where M: Serialize + DeserializeOwned 
{
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    println!("Local peer id: {local_peer_id}");

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

    let swarm = {
        let mdns = mdns::async_io::Behaviour::new(mdns::Config::default(), local_peer_id)
            .map_err(|_| MpcPubSubError::FailToListenMDNS)?;
        let behaviour = MpcPubsubBahavior { 
            floodsub: Floodsub::new(local_peer_id),
            mdns, 
        };
        Swarm::with_async_std_executor(transport, behaviour, local_peer_id)
    };

    let (in_sender, in_receiver) = mpsc::channel::<Result<M, anyhow::Error>>(0);
    let (out_sender, out_receiver) = mpsc::channel::<M>(0);

    let (request_sender, request_receiver) = mpsc::channel::<MpcPubSubRequest>(0);

    Ok((
        MpcPubSubClient {
            request_sender
        },
        MpcPubSubNodeEventLoop::new(
            swarm,
            request_receiver,
            in_sender,
            out_receiver
        ),
        in_receiver, 
        out_sender
    ))
}
