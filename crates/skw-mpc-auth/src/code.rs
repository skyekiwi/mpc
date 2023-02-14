use std::env;
use serde::{Serialize, Deserialize};
use crate::types::{SECRET_LEN, CODE_LEN};
use ed25519_dalek::{SIGNATURE_LENGTH};

use ed25519_dalek::{Keypair, PublicKey, SecretKey};
use ed25519_dalek::{Signature, Signer};

use crate::auth::BaseAuth;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AuthCode {

    pub secret_key: [u8; SECRET_LEN],
    pub code: [u8; CODE_LEN],

    pub time: u64,
    pub time_discrepancy: u64,

	pub signature: Vec<u8>,
}

impl AuthCode {
    pub fn new (
        secret: &str,
        code: [u8; CODE_LEN],
        time: u64,
        time_discrepancy: u64,
    ) -> Self {
		let mut auth_code = Self {
            secret_key: secret.as_bytes().try_into().expect("wrong size"),
            code, time, time_discrepancy,
			signature: vec![],
        };

		let public_key_binding = env::var("SERVER_PUBLIC_KEY").unwrap();
		let secret_key_binding = env::var("SERVER_SECRET_KEY").unwrap();
		let public_key_bytes = hex::decode(&public_key_binding).unwrap();
		let secret_key_bytes = hex::decode(&secret_key_binding).unwrap();

		let public_key: PublicKey = PublicKey::from_bytes(public_key_bytes.as_slice()).unwrap();
		let secret_key: SecretKey = SecretKey::from_bytes(secret_key_bytes.as_slice()).unwrap();
		let keypair: Keypair = Keypair { secret: secret_key, public: public_key };

		let auth_code_binding = serde_json::to_vec(&auth_code).unwrap();
		let auth_code_bytes = auth_code_binding.as_slice();
		let signature: Signature = keypair.sign(auth_code_bytes);
		auth_code.signature = Vec::from(signature.to_bytes());
		return auth_code
    }

    pub fn validate(&self) -> bool {
        BaseAuth::verify_code_raw(&self.secret_key, &self.code, self.time_discrepancy, self.time)
    }
}

#[test]
pub fn validate_works() {
    // generate on the client side
    let code = BaseAuth::get_code("H6ORCEULNB4LSP2XXYZFPC4NPADXEEC6", 0).unwrap();

    let a = AuthCode::new(
        "H6ORCEULNB4LSP2XXYZFPC4NPADXEEC6",
        code, 0, 30
    );

    println!("{:?}", a);

    assert_eq!(a.validate(), true)
}
