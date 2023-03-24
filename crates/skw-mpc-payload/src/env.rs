pub struct EnvironmentVar {
    pub usage_verify_key: [u8; 32],
}

impl EnvironmentVar {
    pub fn load() -> Self {
        let usage_verify_key = hex::decode(
            dotenv::var("USAGE_VERIFY_KEY")
                .expect("USAGE_VERIFY_KEY in env")
            )
            .expect("expect valid hex")
            .try_into()
            .expect("valid length");

        Self { usage_verify_key }
    }
}

pub struct TestEnvironmentVar {
    pub usage_prover_key: [u8; 32],
}

impl TestEnvironmentVar {
    pub fn load() -> Self {
        let usage_prover_key = hex::decode(
            dotenv::var("USAGE_CERT_KEY")
                .expect("USAGE_CERT_KEY in env")
            )
            .expect("expect valid hex")
            .try_into()
            .expect("valid length");

        Self { usage_prover_key }
    }
}