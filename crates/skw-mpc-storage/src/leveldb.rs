use futures::{channel::mpsc, StreamExt};
use rusty_leveldb::{DB, Options};

use crate::{DBOpIn, DBOpOut, MpcStorageConfig, MpcStorageError};

pub fn default_mpc_storage_opt(
    db_name_or_path: String,
    in_memory: bool
) -> (
    MpcStorageConfig,
    mpsc::Sender<DBOpIn>,
) {
    // we want the db op to be executed as long as they are avalaible
    let (db_in_sender, db_in_receiver) = mpsc::channel(0);
    (
        MpcStorageConfig::new(db_name_or_path, in_memory, db_in_receiver),
        db_in_sender, 
    )
}

pub fn run_db_server(
    mut config: MpcStorageConfig
) {
    let opt = {
        match config.is_in_memory() {
            false => Options::default(),
            true => rusty_leveldb::in_memory()
        }
    };

    // TODO: this unwrap is not correct
    let mut db = DB::open(config.db_name_or_path(), opt)
        .map_err(|_| MpcStorageError::FailToOpenDB)
        .unwrap();

    async_std::task::spawn(async move {
        let mut graceful_terminate = false;
        loop {
            if graceful_terminate {
                break;
            }
            let db_opt_in = config.db_pending_ops().select_next_some().await;
            match db_opt_in {
                DBOpIn::WriteToDB { key, value, result_sender } => {
                    let status = db.put(&key[..], &value[..])
                        .map_err(|_| MpcStorageError::FailToWriteDB);
                    
                    let flush_status = db.flush()
                        .map_err(|_| MpcStorageError::FailToFlushDB);

                    result_sender
                        .send(DBOpOut::WriteToDB { status: status.and(flush_status) })
                        .expect("db out receiver should not been dropped")
                },
                DBOpIn::ReadFromDB { key, result_sender } => {
                    let v = db.get(&key);
                    let status = match v {
                        Some(v) => Ok(v),
                        None => Err(MpcStorageError::KeyNotInDB)
                    };
                    result_sender
                        .send(DBOpOut::ReadFromDB { status })
                        .expect("db out receiver should not been dropped")
                },
                DBOpIn::DeleteFromDB { key, result_sender } => {
                    let status = db.delete(&key)
                        .map_err(|_| MpcStorageError::KeyNotInDB);
                    result_sender
                        .send(DBOpOut::DeleteFromDB { status })
                        .expect("db out receiver should not been dropped")
                },
                DBOpIn::Shutdown { result_sender } => {
                    let flush_status = db.flush()
                        .map_err(|_| MpcStorageError::FailToFlushDB);
                    let shutdown_status = db.close()
                        .map_err(|_| MpcStorageError::FailToCloseDB);
    
                    // TODO: make sure no err before shutdown
                    graceful_terminate = true;
                    result_sender
                        .send(DBOpOut::Shutdown { status: flush_status.and(shutdown_status) })
                        .expect("db out receiver should not been dropped")
                },
                DBOpIn::ForceFlush { result_sender } => {
                    let status = db.flush()
                        .map_err(|_| MpcStorageError::FailToFlushDB);
                    result_sender
                        .send(DBOpOut::ForceFlush { status })
                        .expect("db out receiver should not been dropped")
                },
            }
        }
    });
}

#[cfg(test)]
mod test {
    use super::*;
    use futures::SinkExt;
    use futures::channel::oneshot;
    
    #[async_std::test]
    async fn in_memory() {
        let (config, mut in_pipe) = default_mpc_storage_opt("in_memory".to_string(), true);
        // async_std::task::spawn();
        run_db_server(config);
    
        { 
            let (i, o) = oneshot::channel();
            in_pipe
                .send(DBOpIn::WriteToDB {
                    key: [0u8; 32],
                    value: vec![1, 2, 3],
                    result_sender: i,
                })
                .await
                .expect("receiver not dropped");
            let res = o.await;
            println!("{:?}", res.unwrap());
        }

        {
            let (i, o) = oneshot::channel();
            in_pipe.send(DBOpIn::WriteToDB {
                key: [1u8; 32],
                value: vec![4, 5, 6],
                result_sender: i,
            })
                .await
                .expect("receiver not dropped");
            let res = o.await;
            println!("{:?}", res.unwrap());
        }

        {
            let (i, o) = oneshot::channel();
            in_pipe.send(DBOpIn::ReadFromDB {
                key: [0u8; 32],
                result_sender: i,
            })
                .await
                .expect("receiver not dropped");
            let res = o.await;
            println!("{:?}", res.unwrap());
        }

        {
            let (i, o) = oneshot::channel();
            in_pipe.send(DBOpIn::Shutdown {
                result_sender: i,
            })
                .await
                .expect("receiver not dropped");
            let res = o.await;
            println!("{:?}", res.unwrap());
        }
    }

    #[async_std::test]
    async fn on_disk() {
        // Run #1
        {
            let (config, mut in_pipe) = default_mpc_storage_opt("mock".to_string(), false);
            run_db_server(config);

            { 
                let (i, o) = oneshot::channel();
                in_pipe
                    .send(DBOpIn::WriteToDB {
                        key: [0u8; 32],
                        value: vec![1, 2, 3],
                        result_sender: i,
                    })
                    .await
                    .expect("receiver not dropped");
                let res = o.await;
                println!("{:?}", res.unwrap());
            }
    
            {
                let (i, o) = oneshot::channel();
                in_pipe.send(DBOpIn::WriteToDB {
                    key: [1u8; 32],
                    value: vec![4, 5, 6],
                    result_sender: i,
                })
                    .await
                    .expect("receiver not dropped");
                let res = o.await;
                println!("{:?}", res.unwrap());
            }
    
            {
                let (i, o) = oneshot::channel();
                in_pipe.send(DBOpIn::ReadFromDB {
                    key: [0u8; 32],
                    result_sender: i,
                })
                    .await
                    .expect("receiver not dropped");
                let res = o.await;
                println!("{:?}", res.unwrap());
            }
    
            {
                let (i, o) = oneshot::channel();
                in_pipe.send(DBOpIn::Shutdown {
                    result_sender: i,
                })
                    .await
                    .expect("receiver not dropped");
                let res = o.await;
                println!("{:?}", res.unwrap());
            }
        }

        {
            let (config, mut in_pipe) = default_mpc_storage_opt("mock".to_string(), false);
            run_db_server(config);
            {
                let (i, o) = oneshot::channel();
                in_pipe.send(DBOpIn::ReadFromDB {
                    key: [0u8; 32],
                    result_sender: i,
                })
                    .await
                    .expect("receiver not dropped");
                let res = o.await;
                println!("{:?}", res.unwrap());
            }
    
            {
                let (i, o) = oneshot::channel();
                in_pipe.send(DBOpIn::Shutdown {
                    result_sender: i,
                })
                    .await
                    .expect("receiver not dropped");
                let res = o.await;
                println!("{:?}", res.unwrap());
            }
        }
    }
}
