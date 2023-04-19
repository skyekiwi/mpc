use futures::{channel::mpsc, StreamExt};
use rusty_leveldb::{DB, Options};

use skw_mpc_storage::{DBOpIn, DBOpOut, MpcStorageConfig, MpcStorageError};
use tokio::task::JoinHandle;

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
) -> Result<JoinHandle<()>, MpcStorageError> {
    let opt = {
        match config.is_in_memory() {
            false => Options::default(),
            true => rusty_leveldb::in_memory()
        }
    };

    // TODO: this unwrap is not correct
    let mut db = DB::open(config.db_name_or_path(), opt)
        .map_err(|_| MpcStorageError::FailToOpenDB)?;

    Ok(tokio::task::spawn(async move {
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
    }))
}

#[cfg(test)]
mod test {
    use super::*;
    use futures::SinkExt;
    use skw_mpc_storage::{write_to_db, read_from_db, shutdown_db};
    
    #[tokio::test]
    async fn in_memory() {
        let (config, mut in_pipe) = default_mpc_storage_opt("in_memory".to_string(), true);
        let jh = run_db_server(config).unwrap();
    
        write_to_db!( in_pipe, [0u8; 32], vec![1, 2, 3] ).unwrap();
        write_to_db!( in_pipe, [1u8; 32], vec![4, 5, 6] ).unwrap();
        let v = read_from_db!( in_pipe, [1u8; 32] ).unwrap();
        assert_eq!(v, vec![4, 5, 6]);

        let v = read_from_db!( in_pipe, [0u8; 32] ).unwrap();
        assert_eq!(v, vec![1, 2, 3]);

        shutdown_db!( in_pipe ).unwrap();
        jh.await.unwrap();
    }

    #[tokio::test]
    async fn on_disk() {
        // Run #1
        {
            let (config, mut in_pipe) = default_mpc_storage_opt("mock".to_string(), false);
            let jh = run_db_server(config).unwrap();

            write_to_db!( in_pipe, [0u8; 32], vec![1, 2, 3] ).unwrap();
            write_to_db!( in_pipe, [1u8; 32], vec![4, 5, 6] ).unwrap();
            let v = read_from_db!( in_pipe, [1u8; 32] ).unwrap();
            assert_eq!(v, vec![4, 5, 6]);
    
            let v = read_from_db!( in_pipe, [0u8; 32] ).unwrap();
            assert_eq!(v, vec![1, 2, 3]);
    
            shutdown_db!( in_pipe ).unwrap();
            jh.await.unwrap();
        }

        {
            let (config, mut in_pipe) = default_mpc_storage_opt("mock".to_string(), false);
            let jh = run_db_server(config).unwrap();

            let v = read_from_db!( in_pipe, [1u8; 32] ).unwrap();
            assert_eq!(v, vec![4, 5, 6]);
    
            let v = read_from_db!( in_pipe, [0u8; 32] ).unwrap();
            assert_eq!(v, vec![1, 2, 3]);
    
            shutdown_db!( in_pipe ).unwrap();
            jh.await.unwrap();
        }
    }
}
