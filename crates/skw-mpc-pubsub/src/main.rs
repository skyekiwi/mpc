use futures::{StreamExt, SinkExt, AsyncBufReadExt, FutureExt};
use skw_mpc_pubsub::node::MpcPubsub;
use anyhow::Result;

use async_std::io::{BufReader, stdin};
use futures::{pin_mut, select};

#[async_std::main]
async fn main() -> Result<()> {
    let mut node = MpcPubsub::new().await?;

    node.start(0)?;

    let (incoming, outgoing) = node.process("test")?;
    pin_mut!(incoming);
    pin_mut!(outgoing);

    // let mut stdin = BufReader::new(stdin()).lines();

    let stdin = BufReader::new(stdin());
    let mut lines_from_stdin = futures::StreamExt::fuse(stdin.lines());
    loop {
        select! {
            received = incoming.next().fuse() => {
                if let Some(Some(msg)) = received {
                    eprintln!("Received {:?}", msg);
                }
            },
            line = lines_from_stdin.next().fuse() => {
                outgoing.send(line.expect("Stdin not to close").unwrap().as_bytes().to_vec()).await?;
            },
        }
    }

}
