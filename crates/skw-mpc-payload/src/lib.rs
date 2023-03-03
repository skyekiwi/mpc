pub mod header;
pub mod types;
mod env;
use serde::{Serialize, Deserialize};

// re-export
pub use crate::header::{PayloadHeader, AuthHeader}; 
pub use crate::types::{CryptoHash, SecertKey};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Payload<B> {
    pub payload_header: PayloadHeader,
    pub body: B,
}

#[cfg(test)]
mod test {
    use super::{PayloadHeader, Payload};

    #[test]
    fn serde() {
        let header = PayloadHeader::default();

        let msg = Payload {
            payload_header: header,
            body: "test_msg"
        };

        let encoded = bincode::serialize(&msg).unwrap();
        println!("{:?}", encoded);

        let restructred: Payload<&str> = bincode::deserialize(&encoded).unwrap();

        println!("{:?}", restructred);
    }
}
