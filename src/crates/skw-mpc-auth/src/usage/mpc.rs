use blake2::{Blake2s256, Digest};

use super::UsageCertification;

use crate::{
    SelfProveableSystem,
    Ed25519SelfProveableSystem, Ed25519Error, Ed25519VerfierConfig, Ed25519ProverConfig,
};

pub struct MpcUsageCertification();

impl UsageCertification for MpcUsageCertification {
    type OwnershipProof = Ed25519SelfProveableSystem;
    type Certification = Ed25519SelfProveableSystem;

    type OwnershipVerificationConfig = Ed25519VerfierConfig;

    type UsageCertificationProverConfig = Ed25519ProverConfig;
    type UsageCertificationVerifierConfig = Ed25519VerfierConfig;

    type KeyGenId = [u8; 32];
    type Err = Ed25519Error;

    fn issue_usage_certification(
        keygen_id: &Self::KeyGenId, 
        ownership_verification_config: &Self::OwnershipVerificationConfig,
        usage_proof_config: &Self::UsageCertificationProverConfig,
        ownership_proof: &<Self::OwnershipProof as SelfProveableSystem>::Proof
    ) -> Result<
        <Self::Certification as crate::proof::SelfProveableSystem>::Proof,
        Self::Err
    > {
        // 1. verify ownership proof first
        Self::OwnershipProof::verify_proof(ownership_verification_config, ownership_proof)?;

        // 2. generate the linkage hash
        let mut linkage_hasher = Blake2s256::new();
        linkage_hasher.update(&ownership_proof.payload());
        linkage_hasher.update(keygen_id);
        let linkage_hash = linkage_hasher.finalize();

        // 3. issue the certification
        Self::Certification::generate_proof(usage_proof_config, linkage_hash.into())
    }

    fn verify_usage_certification(
        keygen_id: &Self::KeyGenId,
        credential_hash: &[u8; 32],
        usage_verification_config: &Self::UsageCertificationVerifierConfig,
        usage_certification: &<Self::Certification as SelfProveableSystem>::Proof
    ) -> Result<(), Self::Err> {

        let mut linkage_hasher = Blake2s256::new();
        linkage_hasher.update(credential_hash);
        linkage_hasher.update(keygen_id);
        let linkage_hash = linkage_hasher.finalize();

        if &Into::<[u8; 32]>::into(linkage_hash) == usage_certification.payload() {
            Self::Certification::verify_proof(usage_verification_config, usage_certification)?;
            Ok(())
        } else {
            Err(Self::Err::ValidationFailed)
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        ProofSystem, SelfProveableSystem, 
        
        ProofOfOwnership, 
        
        GATokenProofOfOwnershipConfig, GATokenProofOfOwnership, 
        
        GAProofSystem, 
        
        Ed25519SelfProveableSystem,
        
        UsageCertification, ownership::oauth::{OAuthTokenProofOfOwnershipConfig, OAuthTokenProofOfOwnership, OAuthCredential},
    };

    use super::MpcUsageCertification;

    #[test]
    fn ga_smoke_test() {
        let default_config = GATokenProofOfOwnershipConfig::default();
        let credential = [1u8; 32];
        let verifier = GATokenProofOfOwnership::generate_challenge( &default_config, &credential ).unwrap();

        let proof = GAProofSystem::generate_proof(&verifier, &0).unwrap();

        let proof_of_ownership = GATokenProofOfOwnership::issue_proof(
            &default_config, 
            &credential, 
            &proof, 
            &verifier
        ).unwrap();

        let verifier_config = Ed25519SelfProveableSystem::derive_verifier_config(
            &default_config.clone().into(),
        ).unwrap();


        let usage_proof_config = [3u8; 32].into();
        let p = MpcUsageCertification::issue_usage_certification(
            &[10u8; 32],
            &verifier_config,
            &usage_proof_config,
            &proof_of_ownership
        ).unwrap();

        let usage_verif_config = Ed25519SelfProveableSystem::derive_verifier_config(&usage_proof_config).unwrap();
        MpcUsageCertification::verify_usage_certification(
            &[10u8; 32],
            &GATokenProofOfOwnership::get_credential_hash(&default_config, &credential).unwrap(),
            &usage_verif_config,
            &p
        ).unwrap();

        println!("Proof {:?}", p);
    }

    #[test]
    fn oauth_smoke_test() {
        let config = OAuthTokenProofOfOwnershipConfig::new(
            "ee9bcd27ed0d2bbca5f0c620e0dcc01a7d4cc76bfa8fa2a8e2de964848a9d8b8".to_string(), 
            [0u8; 32]
        );
        let credential = OAuthCredential::new("google".to_string(), "hello@skye.kiwi".to_string());
        let verifier = OAuthTokenProofOfOwnership::generate_challenge(&config, &credential).unwrap();

        let proof = "eyJhbGciOiJkaXIiLCJlbmMiOiJBMjU2R0NNIn0..1OF2hGAtCq5cpYp9.k6sGKaEEGf2ZIaXNrvprNeR0JrnTYXfIf2tqgobJrsQr65IFpBduX0oL3xSRnCqyhNCrk-1pmL2G0eA9IXIAKsue-WpGRZcYtVU5SyeJOUHgqk8Q7y3XHT4rLKg2Mg3CrkNvUJgjCWieZt8niZ_6r4WDnVv2Gh8bPNf9s4ijjBwkFEClB9cRQh-V06LsWLnE-I051Mo_bdK25TNp5JER7yoT20jSkfOZhqFU7cufpngDQUG3E3x37l9L9bIXLKtOZvoGMp056x9Z2TUg-4cgB20WuBYY3oqdtepp4c68tuLCZ2qT9Bp27pMOPvhYM06ppRojbFIoJVP_YWW9dDas8yqWVAfQYONMnZLJhZB86ucZEO-A-H8.UnEZb2Vad-4aLP8Ev2Cyeg".to_string();

        let proof_of_ownership = OAuthTokenProofOfOwnership::issue_proof(
            &config, 
            &credential, 
            &proof, 
            &verifier
        ).unwrap();

        let verifier_config = Ed25519SelfProveableSystem::derive_verifier_config(
            &config.clone().into(),
        ).unwrap();

        let usage_proof_config = [3u8; 32].into();
        let p = MpcUsageCertification::issue_usage_certification(
            &[10u8; 32],
            &verifier_config,
            &usage_proof_config,
            &proof_of_ownership
        ).unwrap();

        let usage_verif_config = Ed25519SelfProveableSystem::derive_verifier_config(&usage_proof_config).unwrap();
        MpcUsageCertification::verify_usage_certification(
            &[10u8; 32],
            &OAuthTokenProofOfOwnership::get_credential_hash(&config, &credential).unwrap(),
            &usage_verif_config,
            &p
        ).unwrap();

        println!("Proof {:?}", p);
    }
}
