use std::fmt::Debug;

use crate::proof::SelfProveableSystem;

pub mod mpc;

pub trait UsageCertification {
    type OwnershipProof: SelfProveableSystem;
    type Certification: SelfProveableSystem;

    type OwnershipVerificationConfig;

    type UsageCertificationProverConfig;
    type UsageCertificationVerifierConfig;

    type KeyGenId;
    type Err: Debug;

    fn issue_usage_certification(
        keygen_id: &Self::KeyGenId, 
        ownership_verification_config: &Self::OwnershipVerificationConfig,
        usage_proof_config: &Self::UsageCertificationProverConfig,
        ownership_proof: &<Self::OwnershipProof as SelfProveableSystem>::Proof
    ) -> Result<
        <Self::Certification as SelfProveableSystem>::Proof,
        Self::Err
    >;

    fn verify_usage_certification(
        keygen_id: &Self::KeyGenId,
        credential_hash: &[u8; 32],
        usage_verification_config: &Self::UsageCertificationVerifierConfig,
        usage_certification: &<Self::Certification as SelfProveableSystem>::Proof
    ) -> Result<(), Self::Err>;
}
