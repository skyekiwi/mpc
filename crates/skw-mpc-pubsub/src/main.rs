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
struct MpcPubsubBahavior {
    gossipsub: Gossipsub,
    mdns: mdns::async_io::Behaviour,
    keep_alive: keep_alive::Behaviour,
}

pub struct MpcPubsub {
    local_key: identity::Keypair,
    swarm: Swarm<MpcPubsubBahavior>,
}

impl MpcPubsub {

    pub async fn new(topic: &str) -> Result<Self, Box<dyn Error>> {
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

        let topic = Topic::new(topic);
        let mut gossipsub = Gossipsub::new(MessageAuthenticity::Signed(local_key.clone()), gossipsub_config)
            .expect("Correct configuration");
        gossipsub.subscribe(&topic)?;

        let mut swarm = {
            let mdns = mdns::async_io::Behaviour::new(mdns::Config::default())?;
            let behaviour = MpcPubsubBahavior { gossipsub, mdns, keep_alive: keep_alive::Behaviour::default() };
            Swarm::with_async_std_executor(transport, behaviour, local_peer_id)
        };

        Ok(Self {
            local_key,
            swarm
        })
    }

    pub async fn start(&mut self, topic: &str) -> Result<(), Box<dyn Error>> {
        let mut stdin = io::BufReader::new(io::stdin()).lines().fuse();
        self.swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;
        let topic = Topic::new(topic);

        loop {
            select! {
                line = stdin.select_next_some() => {
                    if let Err(e) = self.swarm
                        .behaviour_mut().gossipsub
                        .publish(topic.clone(), line.expect("Stdin not to close").as_bytes()) {
                        println!("Publish error: {e:?}");
                    }
                },
                event = self.swarm.select_next_some() => match event {
                    SwarmEvent::Behaviour(MpcPubsubBahaviorEvent::Mdns(mdns::Event::Discovered(list))) => {
                        for (peer_id, _multiaddr) in list {
                            println!("mDNS discovered a new peer: {peer_id}");
                            self.swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);

                            for peer in self.swarm.behaviour().gossipsub.all_peers() {
                                println!("Peers {:?}", peer);
                            }
                        }
                    },
                    SwarmEvent::Behaviour(MpcPubsubBahaviorEvent::Mdns(mdns::Event::Expired(list))) => {
                        for (peer_id, _multiaddr) in list {
                            println!("mDNS discover peer has expired: {peer_id}");
                            self.swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer_id);
                        }
                    },
                    SwarmEvent::Behaviour(MpcPubsubBahaviorEvent::Gossipsub(GossipsubEvent::Message {
                        propagation_source: peer_id,
                        message_id: id,
                        message,
                    })) => println!(
                            "Got message: '{}' with id: {id} from peer: {peer_id}",
                            String::from_utf8_lossy(&message.data),
                        ),
                    _ => {}
                }
            }
        }
    }

}

#[async_std::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut node = MpcPubsub::new("test").await?;
    node.start("test").await?;

    Ok(())
}