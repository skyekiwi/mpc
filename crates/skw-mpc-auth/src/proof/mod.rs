use std::fmt::Debug;

pub mod ga;
pub mod ed25519;

pub mod jwe;

pub trait ProofSystem {
    type Proof;
    type Verifier;
    type RandomMaterial;
    type Config;
    type Salt;

    type Output;

    type Err: Debug;

    fn generate_verifier(random_material: Self::RandomMaterial, config: Self::Config) -> Result<Self::Verifier, Self::Err>;
    fn generate_proof(verifier: &Self::Verifier, salt: &Self::Salt) -> Result<Self::Proof, Self::Err>;
    fn verify_proof(proof: &Self::Proof, verifier: &Self::Verifier) -> Result<Self::Output, Self::Err>;
}

pub trait SelfProveableSystem {
    /// ProverConfig & VerifierConfig are considered hardcoded on the node
    type ProverConfig;
    type VerifierConfig;

    type Payload;
    type Proof;

    type Output;

    type Err: Debug;

    fn generate_proof(config: &Self::ProverConfig, payload: Self::Payload) -> Result<Self::Proof, Self::Err>;
    fn derive_verifier_config(config: &Self::ProverConfig) -> Result<Self::VerifierConfig, Self::Err>;
    fn verify_proof(config: &Self::VerifierConfig, proof: &Self::Proof) -> Result<Self::Output, Self::Err>;
}