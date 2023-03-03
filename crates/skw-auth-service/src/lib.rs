pub mod util;

pub mod routes;
mod env;

use futures::{channel::{mpsc, oneshot}, SinkExt};
use skw_mpc_storage::{db::{DBOpIn, DBOpOut}, types::MpcStorageError};

#[derive(Clone)]
pub struct ServerState {
    storage_in_sender: mpsc::Sender<DBOpIn>
}

impl ServerState {
    pub fn new(storage_in_sender: &mpsc::Sender<DBOpIn>) -> Self {
        Self { storage_in_sender: storage_in_sender.clone() }
    }

    pub async fn write_to_db(&mut self, key: [u8; 32], value: Vec<u8>) -> Result<(), MpcStorageError> {
        let (i, o) = oneshot::channel();
        let op = DBOpIn::WriteToDB { key, value, result_sender: i};
        self.storage_in_sender.send(op).await.expect("db server must be running");
        let res = o.await.expect("db server must be running");

        if let DBOpOut::WriteToDB { status } = res {
            status
        } else {
            unreachable!()
        }
    }

    pub async fn read_from_db(&mut self, key: [u8; 32]) -> Result<Vec<u8>, MpcStorageError> {
        let (i, o) = oneshot::channel();
        let op = DBOpIn::ReadFromDB { key, result_sender: i };
        self.storage_in_sender.send(op).await.expect("db server must be running");
        let res = o.await.expect("db server must be running");

        if let DBOpOut::ReadFromDB { status } = res {
            status
        } else {
            unreachable!()
        }
    }

    pub async fn shutdown_db(&mut self) -> Result<(), MpcStorageError> {
        let (i, o) = oneshot::channel();
        let op = DBOpIn::Shutdown { result_sender: i };
        self.storage_in_sender.send(op).await.expect("db server must be running");
        let res = o.await.expect("db server must be running");

        if let DBOpOut::Shutdown { status } = res {
            status
        } else {
            unreachable!()
        }
    }
}

pub async fn shutdown_db(mut db_in: mpsc::Sender<DBOpIn>) -> Result<(), MpcStorageError> {
    let (i, o) = oneshot::channel();
    let op = DBOpIn::Shutdown { result_sender: i };
    db_in.send(op).await.expect("db server must be running");
    let res = o.await.expect("db server must be running");

    if let DBOpOut::Shutdown { status } = res {
        status
    } else {
        unreachable!()
    }
}