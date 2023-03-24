use std::fmt::Debug;

use skw_mpc_auth::{SelfProveableSystem, Ed25519SelfProveableSystem, Ed25519Proof};
use libp2p::{PeerId, Multiaddr};
use serde::{Serialize, Deserialize};
use crate::types::{CryptoHash};
use crate::env::EnvironmentVar;

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthHeader {
    primary: Ed25519Proof,
    secondary: Ed25519Proof,
}

impl AuthHeader {
    pub fn new(
        primary: Ed25519Proof,
        secondary: Ed25519Proof,
    ) -> Self {
        Self { primary, secondary }
    }

    pub fn validate(&self) -> bool {
        let verifier_config = EnvironmentVar::load().usage_verify_key;

        Ed25519SelfProveableSystem::verify_proof(
            &verifier_config.into(), 
            &self.primary
        ).is_ok() && Ed25519SelfProveableSystem::verify_proof(
            &verifier_config.into(), 
            &self.secondary
        ).is_ok()
    }
}

/// message header between nodes
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum  PayloadType {
    
    // with the hash of the message to be signed. 
    SignOffline {
        message: CryptoHash,
        keygen_id: CryptoHash,
        keygen_peers: Vec<(PeerId, Multiaddr)>
    },

    SignFinalize,

    // with an option of the old keys
    // None -> generate a fresh key
    // Some(key) -> inject the old key to the mpc protocol
    KeyGen,
    
    // instruct all nodes to refresh keys
    KeyRefresh { keygen_id: CryptoHash },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PayloadHeader {
    pub payload_id: CryptoHash,
    pub payload_type: PayloadType,

    pub peers: Vec<(PeerId, Multiaddr)>,
    pub sender: PeerId,

    pub t: u16, 
    pub n: u16,
}

impl PayloadHeader {
    pub fn new(
        payload_id: CryptoHash,
        payload_type: PayloadType,
        peers: Vec<(PeerId, Multiaddr)>,
        sender: PeerId,

        t: u16, n: u16,
    ) -> Self {
        Self {
            payload_id, payload_type, 
            peers, sender, 
            t, n,
        }
    }
}

impl Default for PayloadHeader {
    fn default() -> Self {
        let peers = vec![
            (PeerId::random(), "/ip4/127.0.0.1/tcp/5001".parse().unwrap()),
            (PeerId::random(), "/ip4/127.0.0.1/tcp/5001".parse().unwrap()),
            (PeerId::random(), "/ip4/127.0.0.1/tcp/5001".parse().unwrap())
        ];
        Self {
            payload_id: [0u8; 32],
            payload_type: PayloadType::KeyGen,
            peers: peers.clone(),
            sender: peers[0].0,

            t: 2, n: 3,
        }
    }
}

#[cfg(test)]
mod test {
    use skw_mpc_auth::Ed25519Proof;

    use crate::AuthHeader;

    use super::{PayloadHeader};

    #[test]
    fn serde_payload_header() {
        let header = PayloadHeader::default();

        println!("{:?}", header);
        let encoded = bincode::serialize(&header).unwrap();
        println!("{:?}", encoded);

        let restructred: PayloadHeader = bincode::deserialize(&encoded).unwrap();

        println!("{:?}", restructred);
    }

    #[test]
    fn serde_auth_header() {

        let proof1 = Ed25519Proof::default();
        let proof2 = Ed25519Proof::default();
        let header = AuthHeader::new( proof1, proof2 );

        let encoded = serde_json::to_string(&header).unwrap();

        // let encoded = 
        // "{\"proof\":\"{\"payload\":\"7ba12a07689462486c916a03da194acd21422dcfcc6be8b101b1808d0b8b06f3\",\"signature\":\"8bcacf9a6a11c23d18c4cf93b10b094efcf3450e237fb61f29e2f4082d94c2598ca6fed6a0ea1d2afd0ead4c052cec132c3be935f64daccca0f80a3ce76ad701\"}\"}";
        
        // "{\"proof\":{\"payload\":\"0000000000000000000000000000000000000000000000000000000000000000\",\"signature\":\"00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000\"}}";
        // "{\"proof\":\"{\"payload\":\"7ba12a07689462486c916a03da194acd21422dcfcc6be8b101b1808d0b8b06f3\",\"signature\":\"8bcacf9a6a11c23d18c4cf93b10b094efcf3450e237fb61f29e2f4082d94c2598ca6fed6a0ea1d2afd0ead4c052cec132c3be935f64daccca0f80a3ce76ad701\"}\"}";
        
        let restructred: AuthHeader = serde_json::from_str(&encoded).unwrap();

        println!("{:?}", restructred);
    }
}
