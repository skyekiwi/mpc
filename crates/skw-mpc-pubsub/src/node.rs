use libp2p::{
    identity, mdns, mplex, noise,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, PeerId, Transport,
    floodsub::{Floodsub, Topic, FloodsubEvent},
    Swarm,
};

use async_notify::Notify;

use anyhow::{Result};
use futures::{Sink, Stream, StreamExt, FutureExt};
use std::{cell::RefCell, rc::Rc};

#[derive(NetworkBehaviour)]
pub struct MpcPubsubBahavior {
    floodsub: Floodsub,
    mdns: mdns::async_io::Behaviour,
}

pub struct MpcPubsub {
    node: Rc<RefCell<Swarm<MpcPubsubBahavior>>>,

    notifier: Notify,
}

impl MpcPubsub {
    pub async fn new() -> Result<Self> {
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());
        println!("Local peer id: {local_peer_id}");

        let transport = tcp::async_io::Transport::new(tcp::Config::default().nodelay(true))
            .upgrade(libp2p::core::upgrade::Version::V1)
            .authenticate(
                noise::NoiseAuthenticated::xx(&local_key)
                    .expect("Signing libp2p-noise static DH keypair failed."),
            )
            .multiplex(mplex::MplexConfig::new())
            .boxed();

        let swarm = {
            let mdns = mdns::async_io::Behaviour::new(mdns::Config::default())?;
            let behaviour = MpcPubsubBahavior { 
                floodsub: Floodsub::new(local_peer_id),
                mdns, 
            };
            Swarm::with_async_std_executor(transport, behaviour, local_peer_id)
        };

        Ok(Self {
            node: Rc::new(RefCell::new(swarm)),
            notifier: Notify::new(),
        })
    }

    pub fn start(&mut self, port: i32) -> Result<()> {
        self.node
            .borrow_mut()
            .listen_on(format!("/ip4/0.0.0.0/tcp/{}", port).parse()?)?;
        Ok(())
    }

    pub fn process (&mut self, topic: &str) -> Result<(
        impl Stream<Item = Option<Vec<u8>> > + '_, //incoming
        impl Sink<Vec<u8>, Error = anyhow::Error> + '_, // outgoing
    )> {

        let mut original_stream = self.node.borrow_mut();
        original_stream.behaviour_mut().floodsub.subscribe(Topic::new(topic));

        let notifier = &self.notifier;

        let q = Rc::new(RefCell::new(Vec::new()));
        let outgoing = futures::sink::unfold(
            q.clone(),
            |v, message: Vec<u8> | {
                eprintln!("New Message {:?}", message);
                v.borrow_mut().push(message);
                self.notifier.notify();
                futures::future::ready(Ok(v))
            }
        );

        let incoming = async_stream::stream!{
            loop {
                futures::select! {
                    event = original_stream.select_next_some().fuse() => {
                        match event {
                            SwarmEvent::Behaviour(MpcPubsubBahaviorEvent::Mdns(mdns::Event::Discovered(list))) => {
                                for (peer_id, _multiaddr) in list {
                                    println!("mDNS discovered a new peer: {peer_id}");
                                    original_stream.behaviour_mut().floodsub.add_node_to_partial_view(peer_id);
                                }
                                
                                yield None
                            },
                            SwarmEvent::Behaviour(MpcPubsubBahaviorEvent::Mdns(mdns::Event::Expired(list))) => {
                                println!("Left {:?}", list);
                                for (peer_id, _multiaddr) in list {
                                    println!("mDNS discover peer has expired: {peer_id}");
                                    original_stream.behaviour_mut().floodsub.remove_node_from_partial_view(&peer_id);
                                }
        
                                yield None
                            },
                            SwarmEvent::Behaviour(MpcPubsubBahaviorEvent::Floodsub(FloodsubEvent::Message(message))) => {
                                yield Some(message.data.clone())
                            },
                            _ => {
                                yield None
                            }
                        }
                    }
                    _ = notifier.notified().fuse() => {
                        for msg in q.borrow().iter() {
                            original_stream
                                .behaviour_mut()
                                .floodsub
                                .publish_any(Topic::new("test"), msg.to_owned());
                        }

                        *q.borrow_mut() = Vec::new();
                    }
                }
            }
        };

        Ok((incoming, outgoing))
    }
}
