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
            Err(Ed25519Error::ValidationFailed)
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
        let (verifier, credential_hash) = GATokenProofOfOwnership::generate_challenge(
            &default_config, 
            &[1u8; 32]
        ).unwrap();

        let proof = GAProofSystem::generate_proof(&verifier).unwrap();

        let proof_of_ownership = GATokenProofOfOwnership::issue_proof(
            &default_config, 
            credential_hash.clone(), 
            &proof, 
            &verifier
        ).unwrap();

        let verifier_config = Ed25519SelfProveableSystem::derive_verifier_config(
            &default_config.into(),
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
            &credential_hash,
            &usage_verif_config,
            &p
        ).unwrap();

        println!("Proof {:?}", p);
    }

    #[test]
    fn oauth_smoke_test() {
        let config = OAuthTokenProofOfOwnershipConfig::new(
            "chokowallet".to_string(), 
            [0u8; 32]
        );
        let (verifier, credential_hash) = OAuthTokenProofOfOwnership::generate_challenge(
            &config,
            &OAuthCredential::new("google".to_string(), "hello@skye.kiwi".to_string())
        ).unwrap();

        let proof = "eyJhbGciOiJkaXIiLCJlbmMiOiJBMjU2R0NNIn0.._SGRCB7xJDGjt2Og.fnk4QWVusvnMAdaRwvcEtKul9ZWFa994mjMh8D8nEoeG3D8l8Y2TlC0U8hTj-N0YkljTKOg7p0r6v2tk2KYUPCIEGwEarpC_UlADmwTtAJubpCRiQwUnpUYdQ0tRpYFV_bNGDkr-OkUfIe-8iagTTnmoIwBE6ZWTV-ZcF4qxgOWAr45jwFIQS3yNwpF0MLWR3lnzjAOcfya_5ZfOxNVqMEc_wp_4Fmn2myU8878Hhld-u5Zcz5TXfYeQcQYryFcJAfCulKrUXrb-GsGFyYzw0ZbpLMD3NJ4gx0wA6UOUaFJC9mAKipxHf7GNF9WEiMhmD1FC17SEmME-V1-wDaC9z3lwI53p334NLHss0BJGdtWAqCgx-14.FY4Isf4EnB5NpMXRuni-OA".to_string();

        let proof_of_ownership = OAuthTokenProofOfOwnership::issue_proof(
            &config, 
            credential_hash, 
            &proof, 
            &verifier
        ).unwrap();

        let verifier_config = Ed25519SelfProveableSystem::derive_verifier_config(
            &config.into(),
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
            &credential_hash,
            &usage_verif_config,
            &p
        ).unwrap();

        println!("Proof {:?}", p);
    }
}
