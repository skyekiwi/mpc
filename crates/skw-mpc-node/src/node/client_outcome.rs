use libp2p::PeerId;
use skw_mpc_payload::CryptoHash;

#[derive(Clone, Debug)]
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