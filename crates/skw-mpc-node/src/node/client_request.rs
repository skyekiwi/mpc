use futures::channel::oneshot;
use libp2p::{PeerId, Multiaddr};

use crate::error::MpcNodeError;

#[cfg(feature = "light-node")]
use super::client_outcome::ClientOutcome;
#[cfg(feature = "light-node")]
use skw_mpc_payload::{PayloadHeader, AuthHeader};

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

    #[cfg(feature = "full-node")]
    WriteToDB {
        node: PeerId,
        key: [u8; 32],
        value: Vec<u8>,

        result_sender: oneshot::Sender<bool>,
    },

    #[cfg(feature = "light-node")]
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
