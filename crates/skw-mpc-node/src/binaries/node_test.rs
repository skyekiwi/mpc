use futures::{StreamExt, SinkExt, AsyncBufReadExt, FutureExt};

use skw_mpc_node::{
    node::MpcNode,
    error::MpcNodeError
};

#[async_std::main]
async fn main() -> Result<(), MpcNodeError> {
    let mut node = MpcNode::new()?; 
    
    loop {

        futures::select! {
            _ = node.event_loop.run().fuse() => {},
            _ = node.job_creation_handler().fuse() => {},
            _ = node.handle_main_incoming_mesasge().fuse() => {}
            _ = node.handle_keygen_outgoing_msg().fuse() => {}        }
    }
    // async_std::task::spawn(node.borrow_mut().job_creation_handler());

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

    Ok(())
}
