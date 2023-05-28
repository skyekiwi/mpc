use futures::channel::{oneshot, mpsc};
use libp2p::{PeerId, Multiaddr};

use crate::error::MpcNodeError;

use super::client_outcome::ClientOutcome;
use skw_mpc_payload::{PayloadHeader, AuthHeader};

#[derive(Debug)]
pub enum ClientRequest {
    BootstrapNode {
        local_key: Option<[u8; 32]>,
        listen_addr: String,
        db_name: String,

        // the node might keep emitting errors
        result_sender: mpsc::Sender< 
            Result<
                (PeerId, Multiaddr) // node peer_id and listening addr
            , MpcNodeError>
        >
    },

    PersisteResult {
        node: PeerId,
        key: [u8; 32],
        value: Vec<u8>,

        result_sender: oneshot::Sender<Result<(), MpcNodeError>>,
    },

    MpcRequest {
        from: PeerId,
        payload_header: PayloadHeader,
        auth_header: AuthHeader,
        maybe_local_key: Option<Vec<u8>>,
        result_sender: oneshot::Sender<Result<ClientOutcome, MpcNodeError>>,
    },

    Shutdown {
        node: PeerId,
        result_sender: oneshot::Sender<Result<(), MpcNodeError>>,
    }
}
