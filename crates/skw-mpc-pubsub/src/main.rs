use futures::{AsyncBufReadExt, StreamExt, SinkExt};
use skw_mpc_pubsub::node::MpcPubsub;
use anyhow::Result;
use async_std::io;

#[async_std::main]
async fn main() -> Result<()> {
    let mut node = MpcPubsub::new().await?;

    node.start(0)?;

    let (incoming, outgoing) = node.process("test")?;
    tokio::pin!(incoming);
    tokio::pin!(outgoing);

    let mut stdin = io::BufReader::new(io::stdin()).lines().fuse();
    loop {
        tokio::select! {
            received = incoming.next() => {
                if let Some(Some(msg)) = received {
                    eprintln!("Received {:?}", msg);
                }
            },
            line = stdin.select_next_some() => {
                outgoing.send(line.expect("Stdin not to close").as_bytes().to_vec()).await?;
            },
        }
    }

}
