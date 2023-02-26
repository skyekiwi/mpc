use super::ProofSystem;

use hmacsha1::hmac_sha1;
use serde::{Serialize, Deserialize};

use crate::types::{Timestamp, SECRET_LEN, CODE_LEN};
use crate::utils::{get_time};

#[derive(Debug)]
pub struct GAProofSystem();

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GAVerfier {
    secret_key: [u8; SECRET_LEN],
    time: u64,
    time_discrepancy: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GAConfig {
    time_discrepancy: u64,
}
impl From<Timestamp> for GAConfig {
    fn from(value: Timestamp) -> Self {
        Self { time_discrepancy: value }
    }
}
impl Default for GAConfig {
    fn default() -> Self {
        Self {
            time_discrepancy: 30
        }
    }
}

pub type GAProof = [u8; CODE_LEN];
pub type GARandomMaterial = [u8; SECRET_LEN];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GAError {
    BadBase32Encoding,
    BadSecretKeyLength,
    InvalidProof,
}

impl GAVerfier {
    pub fn try_to_string(&self) -> Result<String, GAError> {
        String::from_utf8(self.secret_key.to_vec())
            .map_err(|_| GAError::BadBase32Encoding)
    }

    pub fn from_str(secret: &str, time: Timestamp, time_discrepancy: Timestamp) -> Result<Self, GAError> {
        if secret.len() < 32 {
            return Err(GAError::BadSecretKeyLength)
        }
        Ok(Self {
            secret_key: secret.as_bytes()[..32].try_into().expect("size must match"),
            time,
            time_discrepancy,
        })
    }
}

impl ProofSystem for GAProofSystem {
    type Proof = GAProof;
    type Verifier = GAVerfier;
    type RandomMaterial = GARandomMaterial;
    type Config = GAConfig;
    type Output = ();

    type Err = GAError;

    fn generate_verifier(random_material: Self::RandomMaterial, config: Self::Config) -> Result<Self::Verifier, Self::Err> {
        let encoded = base32::encode(base32::Alphabet::RFC4648 { padding: true }, &random_material[..]);
        let t = get_time(0);
        Ok(GAVerfier::from_str(&encoded, t, config.time_discrepancy)?)
    }

    fn generate_proof(verifier: &Self::Verifier) -> Result<Self::Proof, Self::Err> {
        let t = get_time(verifier.time);
        match base32::decode(base32::Alphabet::RFC4648 { padding: true }, &verifier.try_to_string()?) {
            Some(secret_key) => {
                let hash = hmac_sha1(&secret_key, &t.to_be_bytes());
                let offset = hash[hash.len() - 1] & 0x0F;
                let mut truncated_hash: [u8; 4] = Default::default();

                truncated_hash.copy_from_slice(&hash[offset as usize..(offset + 4) as usize]);
                let mut code = i32::from_be_bytes(truncated_hash);
                code &= 0x7FFFFFFF;
                code %= 10_i32.checked_pow(
                    u32::try_from(CODE_LEN).expect("code is small enough to fit u32. qed.")
                ).expect("code length overflow");

                // NOTE: sCODE_LEN is hardcoded here
                let res: Vec<u8> = format!("{:0>6}", code)
                    .to_string()
                    .chars()
                    .map(|c| u8::try_from(
                            c.to_digit(10).expect("digit conversion error")
                        ).expect("all digits should be within u8 size")
                    )
                    .collect();
                
                Ok(res.try_into().expect("output code should always be 6 digits long"))
            },
            _ => Err(GAError::BadBase32Encoding),
        }
    }

    fn verify_proof(proof: &Self::Proof, verifier: &Self::Verifier) -> Result<Self::Output, Self::Err> {
        let current_time = get_time(0);

        let lower_bound = current_time.saturating_sub(verifier.time_discrepancy);
        let upper_bound = current_time.saturating_add(verifier.time_discrepancy);

        for _time in lower_bound..upper_bound {
            if let Ok(c) = Self::generate_proof(verifier) {
                if c == *proof {
                    return Ok(())
                }
            }
        }

        Err(GAError::InvalidProof)
    }
}


#[test]
fn smoke_test() {

    let random: GARandomMaterial = [1u8; 32];
    let verifier = GAProofSystem::generate_verifier(random, GAConfig::default()).unwrap();
    let proof = GAProofSystem::generate_proof(&verifier).unwrap();

    GAProofSystem::verify_proof(&proof, &verifier).unwrap();

    assert!(GAProofSystem::verify_proof(&proof, &verifier).is_ok(), "valid proof");
}