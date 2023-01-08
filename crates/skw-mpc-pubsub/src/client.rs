use libp2p::{PeerId, Multiaddr};
use futures::{SinkExt};
use futures::channel::{mpsc, oneshot};

use crate::error::MpcPubSubError;

pub enum MpcPubSubRequest {
    StartListening {
        addr: Multiaddr,
        result_sender: oneshot::Sender<Result<(), MpcPubSubError>>,
    },
    Dial {
        peer_id: PeerId,
        peer_addr: Multiaddr,
        result_sender: oneshot::Sender<Result<(), MpcPubSubError>>,
    },
    SubscribeToTopic {
        topic: String,
        result_sender: oneshot::Sender<Result<(), MpcPubSubError>>,
    },

    // Send Message should also be handled here 
    // - but we do have a Sink for it ... so no need for now
}

pub struct MpcPubSubClient {
    pub request_sender: mpsc::Sender<MpcPubSubRequest>
}

impl MpcPubSubClient {

    /// Listen for incoming connections on the given address.
    pub async fn start_listening(
        &mut self,
        addr: Multiaddr,
    ) -> Result<(), MpcPubSubError> {
        let (result_sender, result_receiver) = oneshot::channel();
        self.request_sender
            .send(MpcPubSubRequest::StartListening { addr, result_sender })
            .await
            .expect("MpcPubSubRequest receiver not to be dropped.");
        result_receiver
            .await
            .expect("Sender not to be dropped.")
    }

    /// Dial the given peer at the given address.
    pub async fn dial(
        &mut self,
        peer_id: PeerId,
        peer_addr: Multiaddr,
    ) -> Result<(), MpcPubSubError> {
        let (result_sender, result_receiver) = oneshot::channel();
        self.request_sender
            .send(MpcPubSubRequest::Dial {
                peer_id,
                peer_addr,
                result_sender,
            })
            .await
            .expect("Command receiver not to be dropped.");
        result_receiver.await.expect("Sender not to be dropped.")
    }

    pub async fn subscribe_to_topic(&mut self, topic: String) -> Result<(),  MpcPubSubError> {
        let (result_sender, result_receiver) = oneshot::channel();
        self.request_sender
            .send(MpcPubSubRequest::SubscribeToTopic {
                topic,
                result_sender,
            })
            .await
            .expect("Command receiver not to be dropped.");
        result_receiver.await.expect("Sender not to be dropped.")
    }

}