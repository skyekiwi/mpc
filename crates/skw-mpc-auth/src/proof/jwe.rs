use hkdf::Hkdf;
use serde::{Serialize, Deserialize};
use sha2::Sha256;
use blake2::{Blake2s256, Digest};

use josekit::{jwe::Dir, jwt};

use crate::ProofSystem;

#[derive(Debug)]
pub struct JweProofSystem();
pub type JweToken = String; // the jwe token

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JweConfig {
    client_side_secret: String
}
impl From<&str> for JweConfig {
    fn from(value: &str) -> Self {
        Self { client_side_secret: value.to_string() }
    }
}
impl Into<String> for JweConfig {
    fn into(self) -> String {
        self.client_side_secret
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JweError {
    InvalidJWE,
    UnMatchedCredentialHash,
    MissingProviderClaim,
    MissingEmailClaim,
    InvalidProviderClaim,
    InvalidEmailClaim,
}

impl ProofSystem for JweProofSystem {
    type Proof = JweToken;
    type Verifier = [u8; 32];
    type RandomMaterial = ();
    type Config = JweConfig;
    type Salt = ();

    type Output = [u8; 32];

    type Err = JweError;

    fn generate_verifier(_random_material: Self::RandomMaterial, config: Self::Config) -> Result<Self::Verifier, Self::Err> {
        let hk = Hkdf::<Sha256>::new(None, Into::<String>::into(config.clone()).as_bytes());
        let mut okm = [0u8; 32];
        hk.expand(&"NextAuth.js Generated Encryption Key".as_bytes(), &mut okm).expect("cannot fail");

        Ok(okm)
    }

    /// JWE token will always be generated on the client side 
    fn generate_proof(_verifier: &Self::Verifier, _salt: &Self::Salt) -> Result<Self::Proof, Self::Err> { unreachable!() }

    fn verify_proof(proof: &Self::Proof, verifier: &Self::Verifier) -> Result<Self::Output, Self::Err> {
        let decrypter = Dir.decrypter_from_bytes(&verifier).expect("cannot fail");
        let (payload, _header) = jwt::decode_with_decrypter(&proof, &decrypter)
            .map_err(|_| Self::Err::InvalidJWE)?;

        // TODO: additional validation needed for email & acceptable provider
        let provider = payload.claim("provider")
            .ok_or_else(|| Self::Err::MissingProviderClaim)?
            .as_str()
            .ok_or_else(|| Self::Err::InvalidProviderClaim)?;
        let email = payload.claim("email")
            .ok_or_else(|| Self::Err::MissingEmailClaim)?
            .as_str()
            .ok_or_else(|| Self::Err::InvalidEmailClaim)?;

        let mut credential_hasher = Blake2s256::new();
        credential_hasher.update(provider.as_bytes());
        credential_hasher.update(email.as_bytes());
        let credential_hash = credential_hasher.finalize();

        Ok(credential_hash.into())
    }
}

#[test]
fn test() {
    let verifier = JweProofSystem::generate_verifier((), "chokowallet".into()).unwrap();
    let res = JweProofSystem::verify_proof(
        &"eyJhbGciOiJkaXIiLCJlbmMiOiJBMjU2R0NNIn0.._SGRCB7xJDGjt2Og.fnk4QWVusvnMAdaRwvcEtKul9ZWFa994mjMh8D8nEoeG3D8l8Y2TlC0U8hTj-N0YkljTKOg7p0r6v2tk2KYUPCIEGwEarpC_UlADmwTtAJubpCRiQwUnpUYdQ0tRpYFV_bNGDkr-OkUfIe-8iagTTnmoIwBE6ZWTV-ZcF4qxgOWAr45jwFIQS3yNwpF0MLWR3lnzjAOcfya_5ZfOxNVqMEc_wp_4Fmn2myU8878Hhld-u5Zcz5TXfYeQcQYryFcJAfCulKrUXrb-GsGFyYzw0ZbpLMD3NJ4gx0wA6UOUaFJC9mAKipxHf7GNF9WEiMhmD1FC17SEmME-V1-wDaC9z3lwI53p334NLHss0BJGdtWAqCgx-14.FY4Isf4EnB5NpMXRuni-OA".to_string(), 
        &verifier,
    );

    println!("{:?}", res);

    assert!(res.is_ok(), "valid proof");
}
