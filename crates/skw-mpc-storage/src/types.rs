use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MpcStorageError {
    FailToOpenDB,
    FailToWriteDB,
    FailToDeleteDB,
    FailToFlushDB,
    FailToCloseDB,

    KeyNotInDB,
}