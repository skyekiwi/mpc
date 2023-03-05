use ed25519_dalek::{SecretKey, Keypair, Signer, PublicKey};
use serde::{self, Serialize, Deserialize};
use serde_hex::{SerHex, Strict};

use super::SelfProveableSystem;

#[derive(Debug)]
pub struct Ed25519SelfProveableSystem();
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ed25519Proof {
    #[serde(with = "SerHex::<Strict>")]
    payload: [u8; 32],
    #[serde(with = "SerHex::<Strict>")]
    signature: [u8; 64],
}

impl Default for Ed25519Proof {
    fn default() -> Self {
        Self {
            payload: [0u8; 32],
            signature: [0u8; 64],
        }
    }
}

impl<'a> Ed25519Proof {
    pub fn payload(&'a self) -> &'a [u8; 32] {
        &self.payload
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ed25519ProverConfig {
    secret_key: [u8; 32]
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ed25519VerfierConfig {
    public_key: [u8; 32]
}

impl From<[u8; 32]> for Ed25519ProverConfig {
    fn from(value: [u8; 32]) -> Self {
        Self { secret_key: value }
    }
}

impl From<[u8; 32]> for Ed25519VerfierConfig {
    fn from(value: [u8; 32]) -> Self {
        Self { public_key: value }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Ed25519Error {
    SecretKeyError,
    PublicKeyError,
    FailedToParseSignature,
    ValidationFailed,
}

impl SelfProveableSystem for Ed25519SelfProveableSystem {
    type ProverConfig = Ed25519ProverConfig;
    type VerifierConfig = Ed25519VerfierConfig;
    
    type Payload = [u8; 32];
    type Proof = Ed25519Proof;
    type Output = [u8; 32];
    
    type Err = Ed25519Error;

    fn generate_proof(config: &Self::ProverConfig, payload: Self::Payload) -> Result<Self::Proof, Self::Err> {
        let secret = SecretKey::from_bytes(&config.secret_key[..])
            .map_err(|_| Ed25519Error::SecretKeyError)?;
        let public = (&secret).into();
        let keypair = Keypair { secret, public };
        let signature = keypair.sign(&payload[..]).to_bytes().try_into().expect("signature should always be 64 bytes");

        Ok(Ed25519Proof{ payload, signature })
    }

    fn derive_verifier_config(config: &Self::ProverConfig) -> Result<Self::VerifierConfig, Self::Err> {
        let secret = SecretKey::from_bytes(&config.secret_key)
            .map_err(|_| Ed25519Error::SecretKeyError)?;
        let public_key: PublicKey = (&secret).try_into()
            .map_err(|_| Ed25519Error::PublicKeyError)?;
        Ok(public_key.to_bytes().into())
    }

    fn verify_proof(config: &Self::VerifierConfig, proof: &Self::Proof) -> Result<Self::Output, Self::Err> {
        let public_key = PublicKey::from_bytes(&config.public_key)
            .map_err(|_| Ed25519Error::PublicKeyError)?;
        public_key.verify_strict(
            &proof.payload[..], 
            &proof.signature[..].try_into().map_err(|_| Ed25519Error::FailedToParseSignature)?
        ).map_err(|_| Ed25519Error::ValidationFailed)?;

        Ok(proof.payload)
    }
}

#[test]
fn smoke_test() {
    let message = [0u8; 32];

    let prover_config = [1u8; 32].into();
    let verifier_config = Ed25519SelfProveableSystem::derive_verifier_config(&prover_config).unwrap();

    let proof = Ed25519SelfProveableSystem::generate_proof(&prover_config, message).unwrap();
    println!("{:?}", serde_json::to_string(&proof));

    Ed25519SelfProveableSystem::verify_proof(&verifier_config, &proof).unwrap();
}