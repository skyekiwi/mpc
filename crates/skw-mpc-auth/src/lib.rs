pub mod auth;
pub mod code;
pub mod email;
pub mod types;
pub mod utils;

// re-exports
pub use crate::code::AuthCode;
pub use crate::email::EmailAuth;