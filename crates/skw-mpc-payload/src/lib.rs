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
