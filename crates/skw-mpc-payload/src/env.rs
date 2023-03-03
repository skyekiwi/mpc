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