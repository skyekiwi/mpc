pub mod event_loop;

use crate::event_loop::MpcSwarmEventLoop;
use libp2p::{ identity, PeerId, Multiaddr };

use futures::channel::mpsc;

use skw_mpc_node::{MpcSwarmClient, build_swarm};

pub fn new_swarm_node(
    local_key: Option<[u8; 32]>
) -> (
    PeerId, // local peer id
    
    MpcSwarmClient, 
    MpcSwarmEventLoop, 

    mpsc::Receiver< Multiaddr >,
    mpsc::UnboundedReceiver< Vec<u8> >, // main message incoming channel

    mpsc::Sender<()>, // swarm termination
) {
    let local_key = match local_key {
        None => identity::Keypair::generate_ed25519(),
        Some(key) => {
            identity::Keypair::ed25519_from_bytes(key).unwrap()
        }
    };

    let local_peer_id = PeerId::from(local_key.public());
    // eprintln!("Local peer id: {local_peer_id}");

    let swarm = build_swarm(local_key);
    
    // the new job notifier
    let (swarm_incoming_message_sender, swarm_incoming_message_receiver) = mpsc::unbounded();

    // the main outgoing channel
    // we give it one buffer so that outgoing can be synced
    let (command_sender, command_receiver) = mpsc::unbounded();

    let (addr_sender, addr_receiver) = mpsc::channel(0);

    let (swarm_termination_sender, swarm_termination_receiver) = mpsc::channel(0);
    (
        local_peer_id, 
        MpcSwarmClient { command_sender },
        MpcSwarmEventLoop::new(
            swarm, 
            swarm_incoming_message_sender,
            command_receiver,
            addr_sender,
            swarm_termination_receiver
        ),

        addr_receiver,
        swarm_incoming_message_receiver,
        swarm_termination_sender,
    )
}
