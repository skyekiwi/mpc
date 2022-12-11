use async_std::io;
use futures::{prelude::*, select};
use libp2p::gossipsub::MessageId;
use libp2p::gossipsub::{
    Gossipsub, GossipsubEvent, GossipsubMessage, IdentTopic as Topic, MessageAuthenticity,
    ValidationMode,
};
use libp2p::swarm::keep_alive;
use libp2p::{
    gossipsub, identity, mdns, swarm::NetworkBehaviour, swarm::SwarmEvent, PeerId, Swarm,
};
use std::collections::hash_map::DefaultHasher;
use std::error::Error;
use std::hash::{Hash, Hasher};
use std::time::Duration;

#[derive(NetworkBehaviour)]
struct MpcNodeBahavior {
    gossipsub: Gossipsub,
    mdns: mdns::async_io::Behaviour,
    keep_alive: keep_alive::Behaviour,
}

pub struct MpcNode {
    local_key: identity::Keypair,
    swarm: Swarm<MpcNodeBahavior>,
}

impl MpcNode {

    pub async fn new() -> Result<Self, std::io::Error> {
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());
        println!("Local peer id: {local_peer_id}");

        let transport = libp2p::development_transport(local_key.clone()).await?;

        // Placeholder
        let message_id_fn = |message: &GossipsubMessage| {
            let mut s = DefaultHasher::new();
            message.data.hash(&mut s);
            MessageId::from(s.finish().to_string())
        };

        let gossipsub_config = gossipsub::GossipsubConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(10)) // This is set to aid debugging by not cluttering the log space
            .validation_mode(ValidationMode::Strict) // This sets the kind of message validation. The default is Strict (enforce message signing)
            .message_id_fn(message_id_fn) // content-address messages. No two messages of the same content will be propagated.
            .build()
            .expect("Valid config");

        let mut gossipsub = Gossipsub::new(MessageAuthenticity::Signed(local_key.clone()), gossipsub_config)
            .expect("Correct configuration");
    
        let topic = Topic::new("test-net");

        let mut swarm = {
            let mdns = mdns::async_io::Behaviour::new(mdns::Config::default())?;
            let behaviour = MpcNodeBahavior { gossipsub, mdns, keep_alive: keep_alive::Behaviour::default() };
            Swarm::with_async_std_executor(transport, behaviour, local_peer_id)
        };

        Ok(Self {
            local_key,
            swarm
        })
    }


}

#[async_std::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let node = MpcNode::new().await?;

    Ok(())
}