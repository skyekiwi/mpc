use std::fmt::Debug;

use skw_mpc_auth::{SelfProveableSystem, Ed25519SelfProveableSystem, Ed25519Proof};
use libp2p::{PeerId, Multiaddr};
use serde::{Serialize, Deserialize};
use crate::types::{CryptoHash, SecertKey};
use crate::env::EnvironmentVar;

// TODO: a const for well-known pub key of auth provider
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthHeader {
    proof: Ed25519Proof,
}

impl AuthHeader {
    pub fn new(
        proof: Ed25519Proof
    ) -> Self {
        Self { proof }
    }

    pub fn validate(&self) -> bool {
        let verifier_config = EnvironmentVar::load().usage_verify_key;

        Ed25519SelfProveableSystem::verify_proof(
            &verifier_config.into(), 
            &self.proof
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
    KeyGen(Option<SecertKey>),
    
    // instruct all nodes to refresh keys
    KeyRefresh,
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
            payload_type: PayloadType::KeyGen(None),
            peers: peers.clone(),
            sender: peers[0].0,

            t: 2, n: 3,
        }
    }
}

#[cfg(test)]
mod test {
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

    // #[test]
    // fn serde_auth_header() {

        
    //     let header = AuthHeader::new(
    //         auth.get_code(None).unwrap(),
    //         [0u8; 64].to_vec(), // TODO: replace with real sig on ed25519
    //     );

    //     println!("{:?}", header);
    //     let encoded = bincode::serialize(&header).unwrap();
    //     println!("{:?}", encoded);

    //     let restructred: AuthHeader = bincode::deserialize(&encoded).unwrap();

    //     println!("{:?}", restructred);
    // }
}
