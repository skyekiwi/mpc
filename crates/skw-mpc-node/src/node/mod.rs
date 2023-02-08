mod client_outcome;
mod client_request;
mod client;

#[cfg(feature = "full")]
mod full;

#[cfg(feature = "light")]
mod light;

// re-exports 
#[cfg(feature = "full")]
pub use full::full_node_event_loop;

#[cfg(feature = "light")]
pub use light::light_node_event_loop;

pub use client_request::ClientRequest;
pub use client::NodeClient;
pub use client_outcome::ClientOutcome;