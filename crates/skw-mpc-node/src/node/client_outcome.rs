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
}