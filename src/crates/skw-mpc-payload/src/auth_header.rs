use std::fmt::Debug;

use blake2::{Blake2s256, Digest};
use skw_mpc_auth::{SelfProveableSystem, Ed25519SelfProveableSystem, Ed25519Proof};
use serde::{Serialize, Deserialize};
use crate::types::{CryptoHash};
use crate::env::{EnvironmentVar, TestEnvironmentVar};

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthHeader {
    primary: Ed25519Proof,
    secondary: Ed25519Proof,
    additional: Option<Ed25519Proof>,
}

impl AuthHeader {
    pub fn new(
        primary: Ed25519Proof,
        secondary: Ed25519Proof,
        additional: Option<Ed25519Proof>,
    ) -> Self {
        Self { primary, secondary, additional }
    }

    pub fn validate(&self) -> bool {
        let verifier_config = EnvironmentVar::load().ownership_verify_key;

        // basic verification
        let primary_verification = Ed25519SelfProveableSystem::verify_proof(
            &verifier_config.into(), 
            &self.primary
        ).is_ok();

        let secondary_verification = Ed25519SelfProveableSystem::verify_proof(
            &verifier_config.into(), 
            &self.secondary
        ).is_ok();

        let additional_verification = self.additional.is_none() || Ed25519SelfProveableSystem::verify_proof(
            &verifier_config.into(), 
            &self.additional.clone().unwrap_or_default()
        ).is_ok();
        
        // verify primary credential != secondary credential
        // let distinct_credential = self.primary.payload() != self.secondary.payload();
        
        primary_verification && secondary_verification && additional_verification // && distinct_credential
    }

    pub fn key_shard_id(&self) -> CryptoHash {
        let mut hasher = Blake2s256::new();
        hasher.update(self.primary.payload());
        hasher.update(self.secondary.payload());
        hasher.finalize().into()
    }

    /// For testing only
    pub fn test_auth_header() -> Self {
        let prover_key = TestEnvironmentVar::load().ownership_prover_key;

        let primary = Ed25519SelfProveableSystem::generate_proof(
            &prover_key.into(), [0u8; 32]
        ).unwrap();

        let secondary = Ed25519SelfProveableSystem::generate_proof(
            &prover_key.into(), [1u8; 32]
        ).unwrap();

        Self {
            primary, secondary, additional: None
        }
    }

}



#[cfg(test)]
mod test {
    use skw_mpc_auth::Ed25519Proof;

    use super::AuthHeader;

    #[test]
    fn serde_auth_header() {

        let proof1 = Ed25519Proof::default();
        let proof2 = Ed25519Proof::default();
        let header = AuthHeader::new( proof1, proof2, None );

        let encoded = serde_json::to_string(&header).unwrap();

        // let encoded = 
        // "{\"proof\":\"{\"payload\":\"7ba12a07689462486c916a03da194acd21422dcfcc6be8b101b1808d0b8b06f3\",\"signature\":\"8bcacf9a6a11c23d18c4cf93b10b094efcf3450e237fb61f29e2f4082d94c2598ca6fed6a0ea1d2afd0ead4c052cec132c3be935f64daccca0f80a3ce76ad701\"}\"}";
        
        // "{\"proof\":{\"payload\":\"0000000000000000000000000000000000000000000000000000000000000000\",\"signature\":\"00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000\"}}";
        // "{\"proof\":\"{\"payload\":\"7ba12a07689462486c916a03da194acd21422dcfcc6be8b101b1808d0b8b06f3\",\"signature\":\"8bcacf9a6a11c23d18c4cf93b10b094efcf3450e237fb61f29e2f4082d94c2598ca6fed6a0ea1d2afd0ead4c052cec132c3be935f64daccca0f80a3ce76ad701\"}\"}";
        
        let restructred: AuthHeader = serde_json::from_str(&encoded).unwrap();

        println!("{:?}", restructred);
    }
}
