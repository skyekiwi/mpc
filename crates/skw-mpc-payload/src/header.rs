use serde::{Serialize, Deserialize};
use crate::types::{CryptoHash, SecertKey};
use skw_mpc_auth::{AuthCode};

// TODO: a const for well-known pub key of auth provider
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthHeader {
    auth_code: AuthCode, 
    auth_code_sig: Vec<u8>
}

impl AuthHeader {
    pub fn new(
        auth_code: AuthCode, 
        auth_code_sig: Vec<u8>,
    ) -> Self {
        Self {
            auth_code, auth_code_sig
        }
    }

    pub fn validate(&self) -> bool {
        // TODO: validate the sig first!
        self.auth_code.validate()
    }
}

/// message header between nodes
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum  PayloadType {
    
    // with the hash of the message to be signed. 
    // TODO: change the skw-mpc-protocol to not do hash on it
    Signing(CryptoHash),

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

    pub t: u16, 
    pub n: u16,
}

impl PayloadHeader {
    pub fn new(
        payload_id: CryptoHash,
        payload_type: PayloadType,

        t: u16, n: u16,
    ) -> Self {
        Self {
            payload_id, payload_type, t, n,
        }
    }
}

#[cfg(test)]
mod test {
    
    use skw_mpc_auth::{EmailAuth};
    use crate::header::AuthHeader;

    use super::{PayloadHeader, PayloadType};

    #[test]
    fn serde_payload_header() {
        let header = PayloadHeader::new(
            [0u8; 32], 
            PayloadType::KeyGen(None), 
            1, 3,
        );

        println!("{:?}", header);
        let encoded = bincode::serialize(&header).unwrap();
        println!("{:?}", encoded);

        let restructred: PayloadHeader = bincode::deserialize(&encoded).unwrap();

        println!("{:?}", restructred);
    }

    #[test]
    fn serde_auth_header() {

        let auth = EmailAuth::new(
            "test@skye.kiwi",
            [0u8; 32],
            0
        );
        
        let header = AuthHeader::new(
            auth.get_code(None).unwrap(),
            [0u8; 64].to_vec(), // TODO: replace with real sig on ed25519
        );

        println!("{:?}", header);
        let encoded = bincode::serialize(&header).unwrap();
        println!("{:?}", encoded);

        let restructred: AuthHeader = bincode::deserialize(&encoded).unwrap();

        println!("{:?}", restructred);
    }
}
