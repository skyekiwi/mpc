use serde::{Serialize, Deserialize};

pub const PENDING_TX_THRESHOLD: u64 = 100;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MpcStorageError {
    FailToOpenDB,
    FailToWriteDB,
    FailToDeleteDB,
    FailToFlushDB,
    FailToCloseDB,

    KeyNotInDB,
}