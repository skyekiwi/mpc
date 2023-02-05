use futures::channel::oneshot;
use libp2p::{PeerId, Multiaddr};
use skw_mpc_payload::{PayloadHeader};

use crate::error::MpcNodeError;

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

    MpcRequest {
        from: PeerId,
        payload_header: PayloadHeader, 
        result_sender: oneshot::Sender<Result<Vec<u8>, MpcNodeError>>,
    }
}
