pub mod db;
pub mod types;

// re-export
pub use db::{DBOpIn, DBOpOut, MpcStorageConfig};
pub use types::{MpcStorageError};

#[macro_export]
macro_rules! read_from_db {
    ($db_channel: expr, $key: expr) => {
        {
            let (i, o) = futures::channel::oneshot::channel();
            let op = DBOpIn::ReadFromDB { key: $key, result_sender: i };
            $db_channel.send(op).await.expect("db server must be running");
            let res = o.await.expect("db server must be running");

            res.payload()
        }
    };
}

#[macro_export]
macro_rules! write_to_db {
    ($db_channel: expr, $key: expr, $value: expr) => {
        {
            let (i, o) = futures::channel::oneshot::channel();
            let op = DBOpIn::WriteToDB { key: $key, value: $value, result_sender: i };
            $db_channel.send(op).await.expect("db server must be running");
            let res = o.await.expect("db server must be running");

            res.status()
        }
    };
}

#[macro_export]
macro_rules! delete_from_db {
    ($db_channel: expr, $key: expr) => {
        {
            let (i, o) = futures::channel::oneshot::channel();
            let op = DBOpIn::DeleteFromDB { key: $key, result_sender: i };
            $db_channel.send(op).await.expect("db server must be running");
            let res = o.await.expect("db server must be running");

            res.status()
        }
    };
}

#[macro_export]
macro_rules! shutdown_db {
    ($db_channel: expr) => {
        {
            let (i, o) = futures::channel::oneshot::channel();
            let op = DBOpIn::Shutdown { result_sender: i };
            $db_channel.send(op).await.expect("db server must be running");
            let res = o.await.expect("db server must be running");

            res.status()
        }
    };
}
