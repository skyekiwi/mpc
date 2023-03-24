use serde::{Serialize, Deserialize};
use thiserror::Error;

#[cfg(feature = "full-node")]
use skw_mpc_storage::MpcStorageError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Error)]
pub enum SwarmError {
    #[error("Swarm: failed to listen to address")]
    FailToListenToAddress,
    #[error("Swarm: failed to dail the peer")]
    FailToDailPeer,
    #[error("Swarm: already dailing the peer")]
    AlreadyDailingPeer,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Error)]
pub enum SwarmP2pError {
    #[error("SwarmP2p: invalid autentication header, validation failed.")]
    BadAuthHeader,
    #[error("UNEXPECTED SwarmP2p: request response channel closed. ")]
    ResponseChannelClose,
    #[error("SwarmP2p: outbound failure. Peer closed?")]
    OutboundFailure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Error)]
pub enum SerdeError {
    #[error("SerdeError: failed to deserialize Payload<M>")]
    DeserializePayload,
    #[error("SerdeError: failed to deserialize LocalKey<Secp256k1>")]
    DeserializeLocalKey,
    #[error("SerdeError: failed to deserialize SignatureRecid")]
    DeserializeSignature,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Error)]
pub enum NodeError {
    #[error("NodeError: fail to recognize Swarm incoming message type")]
    InputUnknown,
    #[error("NodeError: outgoing parameter (i, t, n) failed. ")]
    InvalidOutgoingParameter,
    #[error("NodeError: local key must be provided when Signing")]
    LocalKeyMissing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Error)]
pub enum MpcProtocolError {
    #[error("MpcProtocolError: KeyGenError {0}")]
    KeyGenError(String),
    #[error("MpcProtocolError: SignError {0}")]
    SignError(String),
    #[error("MpcProtocolError: KeyRefreshError {0}")]
    KeyRefreshError(String),
}


#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Error)]
pub enum MpcNodeError {
    #[error("SwarmError: SwarmError {0}")]
    SwarmError(SwarmError),
    #[error("SwarmP2pError: SwarmP2pError {0}")]
    SwarmP2pError(SwarmP2pError),
    #[error("SerdeError: SerdeError {0}")]
    SerdeError(SerdeError),
    #[error("NodeError: NodeError {0}")]
    NodeError(NodeError),

    #[cfg(feature = "full-node")]
    #[error("StorageError: StorageError {0}")]
    StorageError(MpcStorageError),

    #[error("MpcProtocolError: MpcProtocolError {0}")]
    MpcProtocolError(MpcProtocolError),
}
