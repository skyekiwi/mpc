mod client_outcome;
mod client_request;
mod client;
mod job_manager;

#[cfg(feature = "full-node")]
mod full;

#[cfg(feature = "light-node")]
mod light;

// re-exports 
#[cfg(feature = "full-node")]
pub use full::full_node_event_loop;

#[cfg(feature = "light-node")]
pub use light::light_node_event_loop;

pub use client_request::ClientRequest;
pub use client::NodeClient;
pub use client_outcome::ClientOutcome;