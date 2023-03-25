use std::fmt::Debug;
use libp2p::{PeerId, Multiaddr};
use serde::{Serialize, Deserialize};
use serde_hex::{SerHex, Strict};

use crate::types::{CryptoHash};
/// message header between nodes
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum  PayloadType {
    
    // with the hash of the message to be signed. 
    SignOffline {
        #[serde(with = "SerHex::<Strict>")]
        message: CryptoHash 
    },

    SignFinalize,

    // with an option of the old keys
    // None -> generate a fresh key
    // Some(key) -> inject the old key to the mpc protocol
    KeyGen,
    
    // instruct all nodes to refresh keys
    KeyRefresh,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PayloadHeader {
    #[serde(with = "SerHex::<Strict>")]
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
}
