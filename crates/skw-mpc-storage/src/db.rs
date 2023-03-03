use crate::types::{MpcStorageError};

use futures::channel::{mpsc, oneshot};

type CryptoHash = [u8; 32];

#[derive(Debug)]
pub enum DBOpIn  {
    WriteToDB {
        key: CryptoHash,
        value: Vec<u8>,
        
        result_sender: oneshot::Sender<DBOpOut>,
    },

    ReadFromDB {
        key: CryptoHash,

        result_sender: oneshot::Sender<DBOpOut>,
    },

    DeleteFromDB {
        key: CryptoHash,

        result_sender: oneshot::Sender<DBOpOut>,
    },

	ForceFlush {
		result_sender: oneshot::Sender<DBOpOut>,
    },

    Shutdown {
        result_sender: oneshot::Sender<DBOpOut>,
    },
}

#[derive(Debug, Clone)]
pub enum DBOpOut {
    WriteToDB {
        status: Result<(), MpcStorageError>,
    },

    ReadFromDB {
        status: Result<Vec<u8>, MpcStorageError>,
    },

    DeleteFromDB {
        status: Result<(), MpcStorageError>,
    },

	ForceFlush {
        status: Result<(), MpcStorageError>,
    },

    Shutdown {
        status: Result<(), MpcStorageError>,
    },
}

pub struct MpcStorageConfig {
    db_name_or_path: String,
    in_memory: bool,

    db_in_receiver: mpsc::Receiver<DBOpIn>,
}

impl MpcStorageConfig {

    pub fn new(
        db_name_or_path: String,
        in_memory: bool,

        db_in_receiver: mpsc::Receiver<DBOpIn>,
    ) -> Self {
        Self {
            db_name_or_path, in_memory, 
            db_in_receiver
        }
    }

    pub fn is_in_memory(&self) -> bool {
        self.in_memory
    }

    pub fn db_name_or_path(&self) -> String {
        self.db_name_or_path.clone()
    }

    pub fn db_pending_ops(&mut self) -> &mut mpsc::Receiver<DBOpIn> {
        &mut self.db_in_receiver
    }
}