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
use std::hash::{Hash, Hasher};
use std::time::Duration;
use anyhow::{Result};
use futures::{Sink, Stream, StreamExt, future, stream_select, stream};

#[derive(NetworkBehaviour)]
pub struct MpcPubsubBahavior {
    pub gossipsub: Gossipsub,
    pub mdns: mdns::async_io::Behaviour,
    pub keep_alive: keep_alive::Behaviour,
}

pub struct MpcPubsub {}
impl MpcPubsub {

    pub async fn new_node() -> Result<Swarm<MpcPubsubBahavior>> {
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

        // let topic = Topic::new(topic);
        let gossipsub = Gossipsub::new(MessageAuthenticity::Signed(local_key.clone()), gossipsub_config)
            .expect("Correct configuration");
        // gossipsub.subscribe(&topic)?;

        let swarm = {
            let mdns = mdns::async_io::Behaviour::new(mdns::Config::default())?;
            let behaviour = MpcPubsubBahavior { gossipsub, mdns, keep_alive: keep_alive::Behaviour::default() };
            Swarm::with_async_std_executor(transport, behaviour, local_peer_id)
        };

        Ok(swarm)
    }

    pub fn start<'node> (node: &'node mut Swarm<MpcPubsubBahavior>) -> Result<()> {
        node.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;
        Ok(())
    }

    pub fn incoming<'node> (node: &'node mut Swarm<MpcPubsubBahavior>, topic: &'node str) -> Result<
        impl Stream<Item = Option<Vec<u8>> > + 'node, //incoming
    > {
        node
            .behaviour_mut()
            .gossipsub
            .subscribe(&Topic::new(topic))?;

        let incoming = async_stream::stream!{
            loop {
                tokio::select! {
                    event = node.select_next_some() => {
                        match event {
                            SwarmEvent::Behaviour(MpcPubsubBahaviorEvent::Mdns(mdns::Event::Discovered(list))) => {
                                println!("Connected {:?}", list);
                                yield None
                            },
                            SwarmEvent::Behaviour(MpcPubsubBahaviorEvent::Mdns(mdns::Event::Expired(list))) => {
                                println!("Left {:?}", list);
        
                                yield None
                            },
                            SwarmEvent::Behaviour(MpcPubsubBahaviorEvent::Gossipsub(GossipsubEvent::Message {
                                propagation_source: peer_id,
                                message_id: id,
                                message,
                            })) => {
                                println!("Got message From {:?}, with ID {:?}", peer_id, id);
                                yield Some(message.data.clone())
                            },
                            _ => {
                                yield None
                            }
                        }
                    }
                }
            }
        };

        Ok(incoming)
    }

    pub fn outgoing<'node>(node: &'node mut Swarm<MpcPubsubBahavior>, topic: &'node str) -> Result<
        impl Sink<Vec<u8>, Error = anyhow::Error> + 'node, // outgoing
    > {
        let outgoing = futures::sink::unfold(
            node, move |n, message: Vec<u8> | async move {
            n
                .behaviour_mut().gossipsub
                .publish(Topic::new(topic.to_owned()), message)?;
            Ok::<_, anyhow::Error>(n)
        });

        Ok(outgoing)
    }

}
