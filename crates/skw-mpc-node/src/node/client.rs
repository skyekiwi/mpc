use futures::{channel::{mpsc, oneshot}, SinkExt};
use libp2p::{PeerId, Multiaddr};

use crate::error::MpcNodeError;

use super::ClientRequest;

#[cfg(feature = "light")]
use super::client_outcome::ClientOutcome;
#[cfg(feature = "light")]
use skw_mpc_payload::{PayloadHeader, AuthHeader};

pub struct NodeClient {
    external_request_sender: mpsc::Sender<ClientRequest>
}

impl NodeClient {
    pub fn new(external_request_sender: mpsc::Sender<ClientRequest>) -> Self {
        Self {
            external_request_sender,
        }
    }

    pub async fn bootstrap_node(
        &mut self,
        local_key: Option<[u8; 32]>,
        listen_addr: String, 
        db_name: String,
    ) -> Result<(PeerId, Multiaddr) , MpcNodeError> {
        let (result_sender, result_receiver) = oneshot::channel();
        self.external_request_sender
            .send(ClientRequest::BootstrapNode {
            local_key, listen_addr, db_name,  result_sender,
        })
            .await
            .expect("receiver not to be droppped");

        result_receiver
            .await
            .expect("sender not to dropped")
    }

    #[cfg(feature = "light")]
    pub async fn send_request(
        &mut self,
        from: PeerId,
        payload_header: PayloadHeader,
        auth_header: AuthHeader,
        maybe_local_key: Option<Vec<u8>>,
    ) -> Result<ClientOutcome, MpcNodeError> {

        let (result_sender, result_receiver) = oneshot::channel();
        self.external_request_sender
            .send(ClientRequest::MpcRequest { from, payload_header, auth_header, maybe_local_key, result_sender})
            .await
            .expect("receiver not to be droppped");

        result_receiver
            .await
            .expect("result_receiver not to be dropped")
    }

    pub async fn shutdown(&mut self, node: PeerId) -> Result<(), MpcNodeError> {
        let (result_sender, result_receiver) = oneshot::channel();
        self.external_request_sender
            .send(ClientRequest::Shutdown { node, result_sender })
            .await
            .expect("receiver not to be droppped");

        result_receiver
            .await
            .expect("sender not to dropped")
    }

    #[cfg(feature = "full")]
    pub async fn write_to_db(&mut self, node: PeerId, key: [u8; 32], value: Vec<u8>) -> Result<bool, MpcNodeError> {
        let (result_sender, result_receiver) = oneshot::channel();
        self.external_request_sender
            .send(ClientRequest::WriteToDB { node, key, value, result_sender })
            .await
            .expect("receiver not to be droppped");

        Ok(result_receiver
            .await
            .expect("sender not to dropped"))
    }
}