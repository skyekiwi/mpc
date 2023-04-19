use serde::{Serialize, de::DeserializeOwned};
use skw_mpc_payload::Payload;
use skw_mpc_protocol::gg20::{
    state_machine::{keygen::LocalKey}, 
    party_i::SignatureRecid
};

use skw_crypto_curv::elliptic::curves::secp256_k1::Secp256k1;

use crate::error::{MpcNodeError, SerdeError};

pub fn encode_payload<M>(payload: &Payload<M>) -> Vec<u8>
    where M: Serialize + DeserializeOwned 
{
    serde_json::to_vec(payload)
        .expect("a valid outgoing payload")
}

pub fn decode_payload<M>(payload: &[u8]) -> Result<M, MpcNodeError>
    where M: Serialize + DeserializeOwned 
{
    serde_json::from_slice(payload)
        .map_err(|_| MpcNodeError::SerdeError(SerdeError::DeserializePayload))
}

pub fn encode_key(key: &LocalKey<Secp256k1>) -> Vec<u8> {
    serde_json::to_vec(key)
        .expect("a valid outgoing payload")
}

pub fn decode_key(raw_key: &[u8]) -> Result<LocalKey<Secp256k1>, MpcNodeError> {
    serde_json::from_slice(raw_key)
        .map_err(|_| MpcNodeError::SerdeError(SerdeError::DeserializeLocalKey))
}

pub fn encode_signature(sig: &SignatureRecid) -> Vec<u8> {
    serde_json::to_vec(sig)
        .expect("a valid partial sig")
}

pub fn decode_signature(raw_sig: &[u8]) -> Result<SignatureRecid, MpcNodeError> {
    serde_json::from_slice(raw_sig)
    .map_err(|_| MpcNodeError::SerdeError(SerdeError::DeserializeSignature))
}