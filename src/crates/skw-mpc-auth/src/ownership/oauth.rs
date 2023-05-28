use blake2::{Blake2s256, Digest};
use serde::{Serialize, Deserialize};
use super::{ProofOfOwnership, OwnershipProofError};

use crate::{
    ProofSystem, SelfProveableSystem,
    JweProofSystem, Ed25519SelfProveableSystem, Ed25519ProverConfig,
};
use crate::types::CryptoHash;

pub struct OAuthTokenProofOfOwnership();

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthCredential {
    provider: String,
    email: String,
}
impl OAuthCredential {
    pub fn new(provider: String, email: String) -> Self {
        Self { provider, email }
    }
}

#[derive(Clone, Debug)]
pub struct OAuthTokenProofOfOwnershipConfig {
    client_side_secret: String,
    signature_secret_key: [u8; 32],
}

impl Into<Ed25519ProverConfig> for OAuthTokenProofOfOwnershipConfig {
    fn into(self) -> Ed25519ProverConfig {
        self.signature_secret_key.into()
    }
}

impl OAuthTokenProofOfOwnershipConfig {
    pub fn new(client_side_secret: String, signature_secret_key: [u8; 32]) -> Self {
        Self {client_side_secret, signature_secret_key}
    }
}

impl ProofOfOwnership for OAuthTokenProofOfOwnership {
    type Credential = OAuthCredential;
    type Config = OAuthTokenProofOfOwnershipConfig;

    type Proof = JweProofSystem;
    type OwnershipProof = Ed25519SelfProveableSystem;

    fn get_credential_hash(_config: &Self::Config, credential: &Self::Credential) -> Result<
            CryptoHash, 
            OwnershipProofError<Self::Proof, Self::OwnershipProof>
        > {
        let mut credential_hasher = Blake2s256::new();
        credential_hasher.update(credential.provider.as_bytes());
        credential_hasher.update(credential.email.as_bytes());
        let credential_hash = credential_hasher.finalize();
        
        Ok(credential_hash.into())
    }

    fn generate_challenge(config: &Self::Config, _credential: &Self::Credential) -> Result< 
        <Self::Proof as ProofSystem>::Verifier,
        OwnershipProofError<Self::Proof, Self::OwnershipProof>
    > {
        let verifier = Self::Proof::generate_verifier((), config.client_side_secret.as_str().into())
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
        let payload = Self::Proof::verify_proof(proof, verifier)
            .map_err(|e| OwnershipProofError::ValidationError(e))?;
        let credential_hash = Self::get_credential_hash(config, credential)?;

        if payload != credential_hash {
            return Err(OwnershipProofError::CredentialMismatch);
        }
        let proof = Self::OwnershipProof::generate_proof(
            &config.signature_secret_key.into(),
            credential_hash,
        ).map_err(|e| OwnershipProofError::ProofIssuanceError(e))?;

        Ok(proof)
    }
}

#[test]
fn smoke_test() {
    let config = OAuthTokenProofOfOwnershipConfig::new("chokowallet".to_string(), [1u8; 32]);
    let verifier = OAuthTokenProofOfOwnership::generate_challenge(
        &config,
        &OAuthCredential::new("google".to_string(), "hello@skye.kiwi".to_string())
    ).unwrap();

    let proof = "eyJhbGciOiJkaXIiLCJlbmMiOiJBMjU2R0NNIn0.._SGRCB7xJDGjt2Og.fnk4QWVusvnMAdaRwvcEtKul9ZWFa994mjMh8D8nEoeG3D8l8Y2TlC0U8hTj-N0YkljTKOg7p0r6v2tk2KYUPCIEGwEarpC_UlADmwTtAJubpCRiQwUnpUYdQ0tRpYFV_bNGDkr-OkUfIe-8iagTTnmoIwBE6ZWTV-ZcF4qxgOWAr45jwFIQS3yNwpF0MLWR3lnzjAOcfya_5ZfOxNVqMEc_wp_4Fmn2myU8878Hhld-u5Zcz5TXfYeQcQYryFcJAfCulKrUXrb-GsGFyYzw0ZbpLMD3NJ4gx0wA6UOUaFJC9mAKipxHf7GNF9WEiMhmD1FC17SEmME-V1-wDaC9z3lwI53p334NLHss0BJGdtWAqCgx-14.FY4Isf4EnB5NpMXRuni-OA".to_string();

    let certification = OAuthTokenProofOfOwnership::issue_proof(
        &config, 
        &OAuthCredential::new("google".to_string(), "hello@skye.kiwi".to_string()),
        &proof, 
        &verifier
    ).unwrap();

    let verifier_config = Ed25519SelfProveableSystem::derive_verifier_config(&config.signature_secret_key.into()).unwrap();

    Ed25519SelfProveableSystem::verify_proof(&verifier_config, &certification).unwrap();
}