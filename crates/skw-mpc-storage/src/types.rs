pub const PENDING_TX_THRESHOLD: u64 = 100;

#[derive(Debug, PartialEq)]
pub enum MpcStorageError {
    FailToOpenDB,
    FailToWriteDB,
    FailToDeleteDB,
    FailToFlushDB,
    FailToCloseDB,

    KeyNotInDB,
}