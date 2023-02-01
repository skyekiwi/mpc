use serde::{Serialize, de::DeserializeOwned};
use skw_mpc_payload::Payload;
use skw_mpc_protocol::gg20::state_machine::keygen::LocalKey;

use curv::elliptic::curves::secp256_k1::Secp256k1;

pub fn encode_payload<M>(payload: &Payload<M>) -> Vec<u8>
    where M: Serialize + DeserializeOwned 
{
    serde_json::to_vec(payload)
        .expect("a valid outgoing payload")
}

pub fn decode_payload<M>(payload: &[u8]) -> M 
    where M: Serialize + DeserializeOwned 
{
    serde_json::from_slice(payload)
        .expect("incoming payload not valid")
}

pub fn encode_key(key: &LocalKey<Secp256k1>) -> Vec<u8> {
    serde_json::to_vec(key)
        .expect("a valid outgoing payload")
}

pub fn decode_key(raw_key: &[u8]) -> LocalKey<Secp256k1> {
    serde_json::from_slice(raw_key)
        .expect("incoming payload not valid")
}