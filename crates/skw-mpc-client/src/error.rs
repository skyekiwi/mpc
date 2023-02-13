use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SwarmError {
    FailToListenToAddress,
    FailToDailPeer,
    AlreadyDailingPeer,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SwarmP2pError {
    ResponseChannelClose,
    OutboundFailure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MpcClientError {
    SwarmError(SwarmError),
    SwarmP2pError(SwarmP2pError),
    MpcNodeError(String),
}
