pub mod error;
pub mod swarm;
pub mod node;

pub mod serde_support;

pub fn async_executor<F>(future: F) 
    where F: futures::Future<Output = ()> + 'static + std::marker::Send,
{
    #[cfg(target_arch = "wasm32")]
    wasm_bindgen_futures::spawn_local(future);

    #[cfg(not(target_arch = "wasm32"))]
    tokio::spawn(future);
}

pub use crate::swarm::behavior::{
    MpcP2pRequest, MpcP2pResponse, MpcSwarmBahavior, MpcSwarmBahaviorEvent, SkwMpcP2pCodec, SkwMpcP2pProtocol
};
pub use crate::swarm::client::{MpcSwarmClient, MpcSwarmCommand};

#[cfg(feature = "tcp-ws-transport")]
pub use crate::swarm::build_swarm;

pub use crate::node::{ClientRequest, ClientOutcome};
