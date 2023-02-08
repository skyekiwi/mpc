pub mod error;
pub mod swarm;
pub mod node;

mod serde_support;

pub fn async_executor<F>(future: F) 
    where F: futures::Future<Output = ()> + 'static + std::marker::Send,
{
    #[cfg(feature = "full")]
    async_std::task::spawn(future);

    #[cfg(feature = "light")]
    wasm_bindgen_futures::spawn_local(future)
}