use biscuit::{JWE, Empty, jwk::JWK};
use hkdf::Hkdf;
use serde::{Serialize, Deserialize};
use sha2::Sha256;
use blake2::{Blake2s256, Digest};

use crate::{ProofSystem, types::Timestamp};

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
struct ClientSideClaims {
    name: String,
    pub email: String,
    pub provider: String,
    picture: String,
    sub: String,
    iat: Timestamp,
    exp: Timestamp,
    jti: String,
}

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
    InvalidTokenHeader,
    InvalidClaims,
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
        // TODO: additional validation needed for email & acceptable provider
        let token: JWE<ClientSideClaims, Empty, Empty> = JWE::new_encrypted(&proof);
    
        let key: JWK<Empty> = JWK::new_octet_key(verifier, Default::default());
        
        let decrypted = token.into_decrypted(
            &key,
            biscuit::jwa::KeyManagementAlgorithm::DirectSymmetricKey,
            biscuit::jwa::ContentEncryptionAlgorithm::A256GCM,
        )
            .map_err(|e| {
                println!("{:?} {:?}", e, verifier);
                JweError::InvalidTokenHeader
            })?
            .payload()
            .expect("should be decrypted")
            .clone()
            .unwrap_encoded() // should be encoded by now
            .encode();
    
        let claims = serde_json::from_str::<ClientSideClaims>(&decrypted)
            .map_err(|_| JweError::InvalidClaims)?;

        let mut credential_hasher = Blake2s256::new();
        credential_hasher.update(claims.provider.as_bytes());
        credential_hasher.update(claims.email.as_bytes());
        let credential_hash = credential_hasher.finalize();

        Ok(credential_hash.into())
    }
}

#[test]
fn test() {
    let verifier = JweProofSystem::generate_verifier((), "ee9bcd27ed0d2bbca5f0c620e0dcc01a7d4cc76bfa8fa2a8e2de964848a9d8b8".into()).unwrap();
    let res = JweProofSystem::verify_proof(
        &"eyJhbGciOiJkaXIiLCJlbmMiOiJBMjU2R0NNIn0..zwb0peuwOCmSzPvW.TUt37qdndS6Z9XZ_vqIYTidiE6VTqp7irPh5LGnwVXOKdCAK0jnKrs8XClwR7E6gp92CycD7blXhsOWd09EEW2DSKC-7qPNAP-LC80K8-JYfySrV1ZERwgWmZcZVkujAkEN1hizpckDVm8FcfXxOPFg8NN8-EsXLxCq8QAoBFSMNc7m8uR6B2HKitHErkUs1HqpzMYz5lTqsf1bbwU8W-YdWXeZZKJxQ8QaQaY6Hlgujr-NUmSw4uf7Qt1z_VRQBKUQ51HU3RG8rsjksGibBwK9Ngg8-extyRYoTIt6JxUoC4p15R4hCqPbpTgWDc5Zq1ShwwGRVGMuCmkuQOuk6xWsjWxmvPdNnMBZg3D31d5C7C2wPiw.73_C0v-Gn0ZjWCbCoRsaUg".to_string(), 
        &verifier,
    );

    println!("{:?}", res);

    assert!(res.is_ok(), "valid proof");
}
