use libp2p::{PeerId, Multiaddr};
use futures::{SinkExt};
use futures::channel::{mpsc, oneshot};

use crate::error::MpcNodeError;

use super::behavior::{MpcP2pRequest, MpcP2pResponse};

#[derive(Debug)]
pub enum MpcSwarmCommand {
    // Command to node
    StartListening {
        addr: Multiaddr,
        result_sender: oneshot::Sender<Result<(), MpcNodeError>>,
    },
    Dial {
        peer_id: PeerId,
        peer_addr: Multiaddr,
        result_sender: oneshot::Sender<Result<(), MpcNodeError>>,
    },
    // CORE: Command to ReqRes P2p sub-protocol 
    SendP2pRequest {
        to: PeerId,
        request: MpcP2pRequest,
        result_sender: oneshot::Sender<Result<MpcP2pResponse, MpcNodeError>>,
    },
}

pub struct MpcSwarmClient {
    pub command_sender: mpsc::UnboundedSender<MpcSwarmCommand>
}

impl MpcSwarmClient {
    /// Listen for incoming connections on the given address.
    pub async fn start_listening(
        &mut self,
        addr: Multiaddr,
    ) -> Result<(), MpcNodeError> {
        let (result_sender, result_receiver) = oneshot::channel();
        self.command_sender
            .send(MpcSwarmCommand::StartListening { addr, result_sender })
            .await
            .expect("MpcSwarmCommand receiver not to be dropped.");
        result_receiver
            .await
            .expect("Sender not to be dropped.")
    }

    /// Dial the given peer at the given address.
    pub async fn dial(
        &mut self,
        peer_id: PeerId,
        peer_addr: Multiaddr,
    ) -> Result<(), MpcNodeError> {
        let (result_sender, result_receiver) = oneshot::channel();
        
        self.command_sender
            .send(MpcSwarmCommand::Dial {
                peer_id,
                peer_addr,
                result_sender,
            })
            .await
            .expect("Command receiver not to be dropped.");
        result_receiver.await.expect("Sender not to be dropped.")
    }

    pub async fn send_request(&mut self, to: PeerId, request: MpcP2pRequest) -> Result<MpcP2pResponse,  MpcNodeError> {
        let (result_sender, result_receiver) = oneshot::channel();
        self.command_sender
            .send(MpcSwarmCommand::SendP2pRequest { to, request, result_sender })
            .await
            .expect("Command receiver not to be dropped.");
        let status = result_receiver.await.expect("Sender not to be dropped.");
        status
    }
}