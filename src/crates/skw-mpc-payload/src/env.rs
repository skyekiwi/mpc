pub struct EnvironmentVar {
    pub ownership_verify_key: [u8; 32],
}

impl EnvironmentVar {
    pub fn load() -> Self {
        let ownership_verify_key = hex::decode(
            dotenv::var("OWNERSHIP_VERIFY_KEY")
                .expect("OWNERSHIP_VERIFY_KEY in env")
            )
            .expect("expect valid hex")
            .try_into()
            .expect("valid length");

        Self { ownership_verify_key }
    }
}

pub struct TestEnvironmentVar {
    pub ownership_prover_key: [u8; 32],
}

impl TestEnvironmentVar {
    pub fn load() -> Self {
        let ownership_prover_key = hex::decode(
            dotenv::var("OWNERSHIP_PROOF_KEY")
                .expect("OWNERSHIP_PROOF_KEY in env")
            )
            .expect("expect valid hex")
            .try_into()
            .expect("valid length");

        Self { ownership_prover_key }
    }
}