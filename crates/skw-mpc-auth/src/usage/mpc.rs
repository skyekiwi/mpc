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
            Self::Certification::verify_proof(usage_verification_config, usage_certification)
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
        
        UsageCertification,
    };

    use super::MpcUsageCertification;

    #[test]
    fn smoke_test() {
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
}
