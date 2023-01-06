use serde::{Serialize, Deserialize};
use crate::types::{CryptoHash, SecertKey};
use skw_mpc_auth::AuthCode;

/// message header between nodes

#[derive(Debug, Serialize, Deserialize)]
pub enum  PayloadType {
    
    // with the hash of the message to be signed. 
    Signing(CryptoHash),

    // with an option of the old keys
    // None -> generate a fresh key
    // Some(key) -> inject the old key to the mpc protocol
    KeyGen(Option<SecertKey>),
    
    // instruct all nodes to refresh keys
    KeyRefresh,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PayloadHeader {
    payload_uuid: CryptoHash,
    payload_type: PayloadType,

    auth_code: AuthCode,
}
