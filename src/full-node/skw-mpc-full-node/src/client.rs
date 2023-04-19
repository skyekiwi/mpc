use futures::{channel::{mpsc, oneshot}, SinkExt, StreamExt};
use libp2p::{PeerId, Multiaddr};

use skw_mpc_node::{
    ClientRequest,
    error::MpcNodeError
};

#[derive(Clone)]
pub struct NodeClient {
    self_peer_id: Option<PeerId>,
    external_request_sender: mpsc::Sender<ClientRequest>
}

impl NodeClient {
    pub fn new(external_request_sender: mpsc::Sender<ClientRequest>) -> Self {
        Self {
            self_peer_id: None,
            external_request_sender,
        }
    }

    pub fn peer_id(&self) -> PeerId {
        self.self_peer_id.unwrap()
    }

    pub async fn bootstrap_node(
        &mut self,
        local_key: Option<[u8; 32]>,
        listen_addr: String, 
        db_name: String,
    ) -> mpsc::Receiver<Result<(PeerId, Multiaddr), MpcNodeError>> {
        let (result_sender, mut result_receiver) = mpsc::channel(0);
        self.external_request_sender
            .send(ClientRequest::BootstrapNode { local_key, listen_addr, db_name, result_sender })
            .await
            .expect("mpc node exteranl request receiver not to be droppped");

        // Result on the initial bootstrapping
        let result = result_receiver.select_next_some().await;
        match result {
            Ok((peer_id, _peer_addr)) => { self.self_peer_id = Some(peer_id);  },
            Err(e) => { log::error!("Node Throw Error {:?}", e); }
        };
        result_receiver
    }

    pub async fn shutdown(&mut self, node: PeerId) -> Result<(), MpcNodeError> {
        let (result_sender, result_receiver) = oneshot::channel();
        self.external_request_sender
            .send(ClientRequest::Shutdown { node, result_sender })
            .await
            .expect("mpc node exteranl request receiver not to be droppped");

        result_receiver
            .await
            .expect("mpc node not to dropped")
    }

    pub async fn write_to_db(&mut self, node: PeerId, key: [u8; 32], value: Vec<u8>) -> Result<(), MpcNodeError> {
        let (result_sender, result_receiver) = oneshot::channel();
        self.external_request_sender
            .send(ClientRequest::PersisteResult { node, key, value, result_sender })
            .await
            .expect("mpc node exteranl request receiver not to be droppped");

        result_receiver
            .await
            .expect("mpc node not to dropped")
    }
}
