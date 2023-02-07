use futures::channel::oneshot;
use libp2p::{PeerId, Multiaddr};
use skw_mpc_payload::{PayloadHeader};

use crate::error::MpcNodeError;

use super::client_outcome::ClientOutcome;

pub enum ClientRequest {
    BootstrapNode {
        local_key: Option<[u8; 32]>,
        listen_addr: String,
        db_name: String,

        result_sender: oneshot::Sender< 
            Result<
                (PeerId, Multiaddr) // node peer_id and listening addr
            , MpcNodeError>
        >
    },

    // TODO: we don't really like this, keep it here until we are done with the wasm version of the node
    WriteToDB {
        node: PeerId,
        key: [u8; 32],
        value: Vec<u8>,

        result_sender: oneshot::Sender<bool>,
    },

    MpcRequest {
        from: PeerId,
        payload_header: PayloadHeader, 
        result_sender: oneshot::Sender<Result<ClientOutcome, MpcNodeError>>,
    },

    Shutdown {
        node: PeerId,
        result_sender: oneshot::Sender<Result<(), MpcNodeError>>,
    }
}
