pub mod header;
pub mod types;

use serde::{Serialize, Deserialize};
use crate::types::{IdentityKey};
use crate::header::PayloadHeader;

#[derive(Debug, Serialize, Deserialize)]
pub struct Payload<B> {
    payload_header: PayloadHeader,

    from: IdentityKey,
    to: Option<IdentityKey>,

    body: B,
}

#[cfg(test)]
mod test {
    use skw_mpc_auth::{AuthCode};
    use crate::header::PayloadType;
    use super::{PayloadHeader, Payload};

    #[test]
    fn serde() {
        let header = PayloadHeader::new(
            [0u8; 32],
            PayloadType::KeyGen(None),
            AuthCode::default()
        );

        let msg = Payload {
            payload_header: header,
            
            from: [0u8; 32],
            to: None,

            body: "test_msg"
        };

        let encoded = bincode::serialize(&msg).unwrap();
        println!("{:?}", encoded);

        let restructred: Payload<&str> = bincode::deserialize(&encoded).unwrap();

        println!("{:?}", restructred);
    }
}
