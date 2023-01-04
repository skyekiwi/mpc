use futures::{StreamExt, SinkExt};
use skw_mpc_pubsub::node::MpcPubsub;
use anyhow::Result;

use tokio::io::{BufReader, stdin, AsyncBufReadExt};
#[tokio::main]
async fn main() -> Result<()> {
    let mut node = MpcPubsub::new().await?;

    node.start(0)?;

    let (incoming, outgoing) = node.process("test")?;
    tokio::pin!(incoming);
    tokio::pin!(outgoing);

    let mut stdin = BufReader::new(stdin()).lines();
    loop {
        tokio::select! {
            received = incoming.next() => {
                if let Some(Some(msg)) = received {
                    eprintln!("Received {:?}", msg);
                }
            },
            line = stdin.next_line() => {
                outgoing.send(line.expect("Stdin not to close").unwrap().as_bytes().to_vec()).await?;
            },
        }
    }

}
