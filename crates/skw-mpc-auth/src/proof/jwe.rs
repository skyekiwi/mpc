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
    let verifier = JweProofSystem::generate_verifier((), "ee9bcd27ed0d2bbca5f0c620e0dcc01a7d4cc76bfa8fa2a8e2de964848a9d8b8".into()).unwrap();
    let res = JweProofSystem::verify_proof(
        &"eyJhbGciOiJkaXIiLCJlbmMiOiJBMjU2R0NNIn0..1OF2hGAtCq5cpYp9.k6sGKaEEGf2ZIaXNrvprNeR0JrnTYXfIf2tqgobJrsQr65IFpBduX0oL3xSRnCqyhNCrk-1pmL2G0eA9IXIAKsue-WpGRZcYtVU5SyeJOUHgqk8Q7y3XHT4rLKg2Mg3CrkNvUJgjCWieZt8niZ_6r4WDnVv2Gh8bPNf9s4ijjBwkFEClB9cRQh-V06LsWLnE-I051Mo_bdK25TNp5JER7yoT20jSkfOZhqFU7cufpngDQUG3E3x37l9L9bIXLKtOZvoGMp056x9Z2TUg-4cgB20WuBYY3oqdtepp4c68tuLCZ2qT9Bp27pMOPvhYM06ppRojbFIoJVP_YWW9dDas8yqWVAfQYONMnZLJhZB86ucZEO-A-H8.UnEZb2Vad-4aLP8Ev2Cyeg".to_string(), 
        &verifier,
    );

    println!("{:?}", res);

    assert!(res.is_ok(), "valid proof");
}
