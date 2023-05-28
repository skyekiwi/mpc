use serde::{Serialize, Deserialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Error)]
pub enum MpcStorageError {
    #[error("Storage: failed to open DB")]
    FailToOpenDB,
    #[error("Storage: failed to write to DB")]
    FailToWriteDB,
    #[error("Storage: failed to delete from DB")]
    FailToDeleteDB,
    #[error("Storage: failed to flush to DB")]
    FailToFlushDB,
    #[error("Storage: failed to close DB")]
    FailToCloseDB,
    #[error("Storage: failed to find key in DB")]
    KeyNotInDB,
    #[error("Storage: no payload for this type of op")]
    NoPayload,
}