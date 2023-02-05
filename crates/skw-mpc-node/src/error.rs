use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MpcNodeError {
    FailToParseMultiaddr,

    FailToListenMDNS,

    FailToListenOnPort,
    FailToDial,
    FailToSubscribeToTopic,
    
    P2pOutboundFailure,
    P2pBadAuthHeader,
    P2pUnknownPeers,
    P2pBadPayload,

    FailToSendViaChannel,
    FailToDeserilaizePayload,
}