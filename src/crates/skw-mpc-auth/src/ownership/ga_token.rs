use blake2::{Blake2s256, Digest};
use super::{ProofOfOwnership, OwnershipProofError};

use crate::{
    ProofSystem, SelfProveableSystem,
    Ed25519SelfProveableSystem, Ed25519ProverConfig,
    GAProofSystem,
};
use crate::types::{Timestamp, CryptoHash};

pub struct GATokenProofOfOwnership();

#[derive(Clone, Debug)]
pub struct GATokenProofOfOwnershipConfig {
    code_expiration_time: Timestamp,
    signature_secret_key: [u8; 32],
}

impl GATokenProofOfOwnershipConfig {
    pub fn new(code_expiration_time: Timestamp,
        signature_secret_key: [u8; 32]) -> Self {
        Self { code_expiration_time, signature_secret_key }
    }
}

impl Default for GATokenProofOfOwnershipConfig {
    fn default() -> Self {
        Self {
            code_expiration_time: 30,
            signature_secret_key: [0u8; 32],
        } // default to 5mins
    }
}

impl Into<Ed25519ProverConfig> for GATokenProofOfOwnershipConfig {
    fn into(self) -> Ed25519ProverConfig {
        self.signature_secret_key.into()
    }
}

impl ProofOfOwnership for GATokenProofOfOwnership {
    type Credential = [u8; 32];
    type Config = GATokenProofOfOwnershipConfig;

    type Proof = GAProofSystem;
    type OwnershipProof = Ed25519SelfProveableSystem;

    fn get_credential_hash(
        config: &Self::Config,
        credential: &Self::Credential
    ) -> Result<
            CryptoHash, 
            OwnershipProofError<Self::Proof, Self::OwnershipProof>
    > {
        let verifier = Self::generate_challenge(config, credential)?;
        let mut credential_hasher = Blake2s256::new();
        credential_hasher.update(verifier.try_to_string().unwrap().as_bytes());
        let credential_hash = credential_hasher.finalize();
        
        Ok(credential_hash.into())
    }

    fn generate_challenge(config: &Self::Config, credential: &Self::Credential) -> Result<
        <Self::Proof as ProofSystem>::Verifier,
        OwnershipProofError<Self::Proof, Self::OwnershipProof>
    > {
        let verifier = Self::Proof::generate_verifier(credential.clone().into(), config.code_expiration_time.into())
            .map_err(|e| OwnershipProofError::ValidationError(e))?;

        Ok(verifier)
    }

    fn issue_proof(
        config: &Self::Config,
        credential: &Self::Credential,
        proof: &<Self::Proof as ProofSystem>::Proof,
        verifier: &<Self::Proof as ProofSystem>::Verifier
    ) -> Result<
        <Self::OwnershipProof as SelfProveableSystem>::Proof,
        OwnershipProofError<Self::Proof, Self::OwnershipProof>
    > {
        Self::Proof::verify_proof(proof, verifier)
            .map_err(|e| OwnershipProofError::ValidationError(e))?;

        let credential_hash = Self::get_credential_hash(config, credential)?;

        let proof = Self::OwnershipProof::generate_proof(
            &config.signature_secret_key.into(),
            credential_hash,
        ).map_err(|e| OwnershipProofError::ProofIssuanceError(e))?;

        Ok(proof)
    }
}

#[test]
fn smoke_test() {
    let default_config = GATokenProofOfOwnershipConfig::default();
    let verifier = GATokenProofOfOwnership::generate_challenge(
        &default_config,
        &[1u8; 32]
    ).unwrap();

    let proof = GAProofSystem::generate_proof(&verifier, &0).unwrap();

    let certification = GATokenProofOfOwnership::issue_proof(
        &default_config,
        &[1u8; 32],
        &proof,
        &verifier
    ).unwrap();

    println!("{:?}", serde_json::to_string(&certification));

    let verifier_config = Ed25519SelfProveableSystem::derive_verifier_config(
        &default_config.into(),
    ).unwrap();

    Ed25519SelfProveableSystem::verify_proof(&verifier_config, &certification).unwrap();
}
