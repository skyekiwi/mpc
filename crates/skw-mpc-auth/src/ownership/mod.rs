use std::fmt::Debug;

use serde::{de::DeserializeOwned, Serialize};

use crate::{
    proof::{ProofSystem, SelfProveableSystem}, 
    types::CryptoHash
};

pub mod email;
pub mod ga_token;
pub mod oauth;

#[derive(Debug)]
pub enum OwnershipProofError<PE, SPE> 
    where 
        PE: ProofSystem, 
        SPE: SelfProveableSystem,
{
    ValidationError(PE::Err),
    ProofIssuanceError(SPE::Err),
    CredentialMismatch,
}

pub trait ProofOfOwnership {
    type Credential;
    type Config;

    type Proof: ProofSystem;
    type OwnershipProof: SelfProveableSystem;

    fn generate_challenge(config: &Self::Config, credential: &Self::Credential) -> Result< 
        (<Self::Proof as ProofSystem>::Verifier, CryptoHash),
        OwnershipProofError<Self::Proof, Self::OwnershipProof>
    >
        where 
            Self::Credential: Serialize + DeserializeOwned,
            <Self::Proof as ProofSystem>::Verifier: Serialize + DeserializeOwned;

    fn issue_proof(config: &Self::Config,
            credential_hash: CryptoHash,
            proof: &<Self::Proof as ProofSystem>::Proof, 
            verifier: &<Self::Proof as ProofSystem>::Verifier
    ) -> Result<
        <Self::OwnershipProof as SelfProveableSystem>::Proof, 
        OwnershipProofError<Self::Proof, Self::OwnershipProof>
    >;
}
