use libp2p::{
    floodsub, mdns, swarm::{NetworkBehaviour},
};


#[derive(NetworkBehaviour)]
pub struct MpcPubsubBahavior {
    pub floodsub: floodsub::Floodsub,
    pub mdns: mdns::async_io::Behaviour,
}
