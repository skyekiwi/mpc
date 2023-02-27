pub mod util;

pub mod routes;

use futures::{channel::{mpsc, oneshot}, SinkExt};
use skw_mpc_auth::ownership::OwnershipProofError;
use skw_mpc_storage::{db::{DBOpIn, DBOpOut}, types::MpcStorageError};

#[derive(Clone)]
pub struct ServerState {
    storage_in_sender: mpsc::Sender<DBOpIn>
}

impl ServerState {
    pub async fn write_to_db(&mut self, key: [u8; 32], value: Vec<u8>) -> Result<(), MpcStorageError> {
        let (i, o) = oneshot::channel();
        let op = DBOpIn::WriteToDB { key, value, result_sender: i};
        self.storage_in_sender.send(op).await;
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
        self.storage_in_sender.send(op).await;
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
        self.storage_in_sender.send(op).await;
        let res = o.await.expect("db server must be running");

        if let DBOpOut::Shutdown { status } = res {
            status
        } else {
            unreachable!()
        }
    }
}