use serde::{Serialize, de::DeserializeOwned};
use skw_mpc_payload::Payload;

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
