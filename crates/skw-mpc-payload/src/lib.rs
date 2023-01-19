pub mod header;
pub mod types;

use serde::{Serialize, Deserialize};

// re-export
pub use crate::header::{PayloadHeader, AuthHeader}; 
pub use crate::types::{CryptoHash, SecertKey};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Payload<B> {
    pub payload_header: PayloadHeader,

    pub from: String, // PeerId
    pub to: String, // PeerId

    pub body: B,
}

#[cfg(test)]
mod test {
    use crate::header::PayloadType;
    use super::{PayloadHeader, Payload};

    #[test]
    fn serde() {
        let header = PayloadHeader::new(
            [0u8; 32],
            PayloadType::KeyGen(None),
            1, 3,
        );

        let msg = Payload {
            payload_header: header,

            from: "one".to_string(),
            to: "two".to_string(),
            
            body: "test_msg"
        };

        let encoded = bincode::serialize(&msg).unwrap();
        println!("{:?}", encoded);

        let restructred: Payload<&str> = bincode::deserialize(&encoded).unwrap();

        println!("{:?}", restructred);
    }
}
