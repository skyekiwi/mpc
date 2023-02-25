use std::fmt::Debug;

pub mod ga;
pub mod ed25519;

pub trait ProofSystem {
    type Proof;
    type Verifier;
    type RandomMaterial;
    type Config;

    type Err: Debug;

    fn generate_verifier(random_material: Self::RandomMaterial, config: Self::Config) -> Result<Self::Verifier, Self::Err>;
    fn generate_proof(verifier: &Self::Verifier) -> Result<Self::Proof, Self::Err>;
    fn verify_proof(proof: &Self::Proof, verifier: &Self::Verifier) -> Result<(), Self::Err>;
}

pub trait SelfProveableSystem {
    /// ProverConfig & VerifierConfig are considered hardcoded on the node
    type ProverConfig;
    type VerifierConfig;

    type Payload;
    type Proof;

    type Err: Debug;

    fn generate_proof(config: &Self::ProverConfig, payload: Self::Payload) -> Result<Self::Proof, Self::Err>;
    fn derive_verifier_config(config: &Self::ProverConfig) -> Result<Self::VerifierConfig, Self::Err>;
    fn verify_proof(config: &Self::VerifierConfig, proof: &Self::Proof) -> Result<(), Self::Err>;
}