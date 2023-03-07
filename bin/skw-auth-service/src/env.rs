use serde::{Serialize, Deserialize};

pub struct EnvironmentVar {
    pub ownership_prover_key: [u8; 32],
    pub usage_cert_key: [u8; 32],
    pub client_oauth_secret: String,
}

impl EnvironmentVar {
    pub fn load() -> Self {
        let ownership_prover_key = hex::decode(
            dotenv::var("OWNERSHIP_PROOF_KEY")
                .expect("OWNERSHIP_PROOF_KEY in env")
            )
            .expect("expect valid hex")
            .try_into()
            .expect("valid length");

        let usage_cert_key = hex::decode(
            dotenv::var("USAGE_CERT_KEY")
                .expect("USAGE_CERT_KEY in env")
            )
            .expect("expect valid hex")
            .try_into()
            .expect("valid length");

        let client_oauth_secret = dotenv::var("CLIENT_OAUTH_SECRET")
            .expect("CLIENT_OAUTH_SECRET in env");
        Self {
            ownership_prover_key,
            usage_cert_key, client_oauth_secret
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PeerIds {
    f1: String,
    f2: String,
    l: String,
    c: String, 
}

impl PeerIds {
    pub fn load() -> Self {

        dotenv::from_path("./.env.peers").expect(".env.peers to exist");

        let f1 = std::env::var("FULL_NODE1_ID").expect("FULL_NODE1_ID peer id in env");
        let f2= std::env::var("FULL_NODE2_ID").expect("FULL_NODE2_ID peer id in env");
        let l = std::env::var("LIGHT_NODE_ID").expect("LIGHT_NODE_ID peer id in env");
        let c = std::env::var("CLIENT_NODE_ID").expect("CLIENT_NODE_ID peer id in env");

        Self {
            f1, f2, l, c,
        }
    }
}