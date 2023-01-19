use clap::{self, Parser};

use futures::{StreamExt, SinkExt, AsyncBufReadExt, FutureExt};
use async_std::{io::{BufReader, stdin}};

use skw_mpc_storage::db::MpcStorage;
use skw_mpc_protocol::gg20::state_machine::keygen::Keygen;
use skw_round_based::async_runtime::AsyncProtocol;
use skw_mpc_pubsub::error::MpcPubSubError;

#[derive(Parser, Debug)]
pub struct ClapArgs {
	#[clap(long)]
	index: u16,
}

#[async_std::main]
async fn main() -> Result<(), MpcPubSubError> {
	let cli_args = ClapArgs::parse();

	let store = MpcStorage::new(
        &format!("examples_db_{}", cli_args.index), 
        false
    ).expect("cannot fail to create db");

	let (
        mut client,
        event_loop, 
        incoming, 
        outgoing
    ) = skw_mpc_pubsub::node::new_node().await?;

    let incoming = incoming.fuse();
    async_std::task::spawn(event_loop.run("mpc-keygen".to_string()));

    futures::pin_mut!(incoming);
    futures::pin_mut!(outgoing);

    client.start_listening("/ip4/0.0.0.0/tcp/0".parse().map_err(|_| MpcPubSubError::FailToParseMultiaddr)?)
        .await
        .expect("Listen not to fail.");
    client.subscribe_to_topic("mpc-keygen".to_string()).await.expect("Listen to topic cannot fail");

    // let stdin = BufReader::new(stdin());
    // let mut lines_from_stdin = futures::StreamExt::fuse(stdin.lines());
    
    // loop {
    //     futures::select! {
    //         received = incoming.next() => {
    //             if let Some(msg) = received {
    //                 eprintln!("Received {:?}", msg);
    //             }
    //         },
    //         line = lines_from_stdin.next() => {
    //             outgoing.send(line.expect("Stdin not to close").unwrap().as_bytes().to_vec())
    //                 .await
    //                 .map_err(|_| MpcPubSubError::FailToSendViaChannel)?;
    //         },
    //     }
    // }

	let keygen = Keygen::new(cli_args.index, 1u16, 3u16).map_err(|e| {
        println!("{:?}", e);
    }).unwrap();


	let output = AsyncProtocol::new(keygen, incoming, outgoing)
        .run()
        .await
        .map_err(|_| MpcPubSubError::FailToSendViaChannel)?;
    
	println!("{:?}", output);

    Ok(())
}