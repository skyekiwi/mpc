use futures::{StreamExt, SinkExt, AsyncBufReadExt, FutureExt};

use async_std::{io::{BufReader, stdin}};
use skw_mpc_pubsub::error::MpcPubSubError;

#[async_std::main]
async fn main() -> Result<(), MpcPubSubError> {
    let (
        mut client,
        event_loop, 
        incoming, 
        outgoing
    ) = skw_mpc_pubsub::node::new_node().await?;
    
    async_std::task::spawn(event_loop.run());

    client.start_listening("/ip4/0.0.0.0/tcp/0".parse().map_err(|_| MpcPubSubError::FailToParseMultiaddr)?)
        .await
        .expect("Listen not to fail. ");
    
    client.subscribe_to_topic("test".to_string()).await.expect("Listen to topic cannot fail");

    // if let Some(addr) = opt.peer {
    //     let peer_id = match addr.iter().last() {
    //         Some(Protocol::P2p(hash)) => PeerId::from_multihash(hash).expect("Valid hash."),
    //         _ => return Err("Expect peer multiaddr to contain peer ID.".into()),
    //     };
    //     network_client
    //         .dial(peer_id, addr)
    //         .await
    //         .expect("Dial to succeed");
    // }

    futures::pin_mut!(incoming);
    futures::pin_mut!(outgoing);

    let stdin = BufReader::new(stdin());
    let mut lines_from_stdin = futures::StreamExt::fuse(stdin.lines());
    
    loop {
        futures::select! {
            received = incoming.next().fuse() => {
                if let Some(msg) = received {
                    eprintln!("Received {:?}", msg);
                }
            },
            line = lines_from_stdin.next().fuse() => {
                outgoing.send(line.expect("Stdin not to close").unwrap().as_bytes().to_vec())
                    .await
                    .map_err(|_| MpcPubSubError::FailToSendViaChannel)?;
            },
        }
    }
}
