#[derive(Debug)]
pub enum MpcPubSubError {
    FailToParseMultiaddr,

    FailToListenMDNS,

    FailToListenOnPort,
    FailToDial,
    FailToSubscribeToTopic,

    FailToSendViaChannel,
}