use crate::types::{MpcStorageError, PENDING_TX_THRESHOLD};

use std::cell::RefCell;
use std::rc::Rc;

use rusty_leveldb::{DB, Options};

/// Open up a levelDB instance from multiple locations
/// db_in_memory - a levelDB in memory
/// db_on_disk - a levelDB pointed to some local file

pub struct MpcStorage {
    db: Rc<RefCell<rusty_leveldb::DB>>,
    pending_tx: u64,
}

impl MpcStorage {
    pub fn new(
        db_name_or_path: &str,
        in_memory: bool,
    ) -> Result<Self, MpcStorageError> {
        match in_memory {
            false => {
                Ok(Self {
                    db: Rc::new(RefCell::new(DB::open(db_name_or_path, Options::default())
                        .map_err(|_| MpcStorageError::FailToOpenDB)?)),
                    pending_tx: 0,
                })
            },
            true => {
                Ok(Self {
                    db: Rc::new(RefCell::new(DB::open(db_name_or_path, rusty_leveldb::in_memory())
                        .map_err(|_| MpcStorageError::FailToOpenDB)?)),
                    pending_tx: 0,
                })
            }

        }
    }

    pub fn put(&mut self, k: &[u8], v: &[u8]) -> Result<(), MpcStorageError> {
        self.pending_tx += 1;
        self.db.borrow_mut().put(k, v).map_err(|_| MpcStorageError::FailToWriteDB)?;

        if self.pending_tx >= PENDING_TX_THRESHOLD {
            self.flush()?;
            self.pending_tx = 0;
        }

        Ok(())

    }

    pub fn delete(&mut self, k: &[u8]) -> Result<(), MpcStorageError> {
        self.pending_tx += 1;

        self.db.borrow_mut().delete(k).map_err(|_| MpcStorageError::FailToDeleteDB)?;
        if self.pending_tx >= PENDING_TX_THRESHOLD {
            self.flush()?;
            self.pending_tx = 0;
        }

        Ok(())
    }

    pub fn flush(&mut self) -> Result<(), MpcStorageError> {
        self.db.borrow_mut().flush().map_err(|_| MpcStorageError::FailToFlushDB)
    }

    pub fn get(&mut self, k: &[u8]) -> Result<Vec<u8>, MpcStorageError> {
        match self.db.borrow_mut().get(k) {
            Some(v) => Ok(v),
            None => Err(MpcStorageError::KeyNotInDB)
        }
    }

    pub fn close(&mut self) -> Result<(), MpcStorageError> {
        self.db.borrow_mut().close().map_err(|_| MpcStorageError::FailToCloseDB)
    }
}
 
#[test]
fn in_memory() {
    let mut db = MpcStorage::new("test", true).unwrap();

    db.put(b"test", b"value").unwrap();
    assert_eq!(db.get(b"test"), Ok(b"value".to_vec()));

    db.delete(b"test").unwrap();
    assert_eq!(db.get(b"test"), Err(MpcStorageError::KeyNotInDB));

    db.flush().expect("cannot fail");
}

#[test]
fn on_disk() {

    // OP1
    {
        let mut db = MpcStorage::new("mock", false).unwrap();

        db.put(b"test", b"value").unwrap();
        db.put(b"test2", b"value2").unwrap();
        
        assert_eq!(db.get(b"test"), Ok(b"value".to_vec()));
        assert_eq!(db.get(b"test2"), Ok(b"value2".to_vec()));


        db.delete(b"test").unwrap();
        assert_eq!(db.get(b"test"), Err(MpcStorageError::KeyNotInDB));
        assert_eq!(db.get(b"test2"), Ok(b"value2".to_vec()));

        // When DB go out of scope, it will be automatically dropped & flush to DB
    }

    // OP2
    {
        let mut db = MpcStorage::new("mock", false).unwrap();

        assert_eq!(db.get(b"test"), Err(MpcStorageError::KeyNotInDB));
        assert_eq!(db.get(b"test2"), Ok(b"value2".to_vec()));
    }
}