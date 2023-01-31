use serde::{Serialize, Deserialize};
use crate::types::{SECRET_LEN, CODE_LEN};

use crate::auth::BaseAuth;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AuthCode {

    secret_key: [u8; SECRET_LEN],
    pub code: [u8; CODE_LEN],

    time: u64,
    time_discrepancy: u64,
}

impl AuthCode {
    pub fn new (
        secret: &str,
        code: [u8; CODE_LEN],
        time: u64,
        time_discrepancy: u64,
    ) -> Self {
        Self {
            secret_key: secret.as_bytes().try_into().expect("wrong size"),
            code, time, time_discrepancy
        }
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