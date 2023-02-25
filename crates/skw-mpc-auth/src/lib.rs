pub mod types;
pub mod utils;

pub mod ownership;
pub mod proof;
pub mod usage;

// re-exports - traits
pub use proof::{ProofSystem, SelfProveableSystem};
pub use ownership::ProofOfOwnership;
pub use usage::UsageCertification;

// re-exports
pub use ownership::email::{EmailProofOfOwnership, EmailProofOfOwnershipConfig};
pub use ownership::ga_token::{GATokenProofOfOwnership, GATokenProofOfOwnershipConfig};

pub use proof::ga::{GAProofSystem, GAError, GAConfig, GARandomMaterial, GAVerfier, GAProof};
pub use proof::ed25519::{Ed25519SelfProveableSystem, Ed25519Error, Ed25519ProverConfig, Ed25519VerfierConfig, Ed25519Proof};

pub use usage::mpc::MpcUsageCertification;