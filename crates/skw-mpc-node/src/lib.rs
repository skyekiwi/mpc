pub mod error;
pub mod swarm;
pub mod node;

mod serde_support;

pub fn async_executor<F>(future: F) 
    where F: futures::Future<Output = ()> + 'static + std::marker::Send,
{
    #[cfg(target_arch = "wasm32")]
    wasm_bindgen_futures::spawn_local(future);

    #[cfg(not(target_arch = "wasm32"))]
    tokio::spawn(future);
}