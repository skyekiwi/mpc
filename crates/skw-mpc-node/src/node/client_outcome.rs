use libp2p::PeerId;
use serde::{Serialize, Deserialize};
use skw_mpc_payload::CryptoHash;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ClientOutcome {
    KeyGen{ 
        peer_id: PeerId,
        payload_id: CryptoHash,
        local_key: Vec<u8> 
    },
    Sign {
        peer_id: PeerId,
        payload_id: CryptoHash,
        sig: Vec<u8>
    },
    KeyRefresh {
        peer_id: PeerId,
        payload_id: CryptoHash,
        new_key: Vec<u8>,
    }
}
impl ClientOutcome {
    pub fn payload(&self) -> Vec<u8> {
        match self {
            Self::KeyGen {local_key, ..} => local_key,
            Self::Sign {sig, ..} => sig,
            Self::KeyRefresh { new_key, .. } => new_key,
        }.clone()
    }
}