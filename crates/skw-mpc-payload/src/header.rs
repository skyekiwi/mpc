use serde::{Serialize, Deserialize};
use crate::types::{CryptoHash, SecertKey};
use skw_mpc_auth::{AuthCode};

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
    payload_id: CryptoHash,
    payload_type: PayloadType,

    auth_code: AuthCode,
}

impl PayloadHeader {
    pub fn new(
        payload_id: CryptoHash,
        payload_type: PayloadType,

        auth_code: AuthCode
    ) -> Self {
        Self {
            payload_id, payload_type, auth_code
        }
    }

    pub fn validate(&self) -> bool {
        self.auth_code.validate()
    }
}

#[cfg(test)]
mod test {
    
    use skw_mpc_auth::{EmailAuth};
    use super::{PayloadHeader, PayloadType};

    #[test]
    fn serde() {

        let auth = EmailAuth::new(
            "test@skye.kiwi",
            [0u8; 32],
            0
        );
        
        let header = PayloadHeader::new(
            [0u8; 32], 
            PayloadType::KeyGen(None), 
            auth.get_code(None).unwrap()
        );

        println!("{:?}", header);
        let encoded = bincode::serialize(&header).unwrap();
        println!("{:?}", encoded);

        let restructred: PayloadHeader = bincode::deserialize(&encoded).unwrap();

        println!("{:?}", restructred);
    }
}
