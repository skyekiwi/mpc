use std::collections::{HashMap, hash_map::Entry};
use serde::{Serialize, de::DeserializeOwned};

use libp2p::{
    mdns, core::either::EitherError,
    swarm::{SwarmEvent, ConnectionHandlerUpgrErr}, PeerId,
    floodsub::{Topic, FloodsubEvent, protocol::CodecError},
    Swarm,
    multiaddr,
};
use futures::{StreamExt, FutureExt, SinkExt};
use futures::channel::{oneshot, mpsc};

use void::Void;
use crate::{
    behavior::{MpcPubsubBahavior, MpcPubsubBahaviorEvent}, 
    client::MpcPubSubRequest, 
    error::MpcPubSubError
};

pub struct MpcPubSubNodeEventLoop<M> {
    node: Swarm<MpcPubsubBahavior>,
    request_receiver: mpsc::Receiver<MpcPubSubRequest>,
    incoming_sender: mpsc::Sender<Result<M, anyhow::Error>>, //impl Stream<Item = M >, //incoming msg
    outgoing_receiver: mpsc::Receiver<M>, //impl Sink<M>, // outgoing msg

    // internal state
    pending_dial: HashMap<PeerId, oneshot::Sender<Result<(), MpcPubSubError>>>,
}

impl<M> MpcPubSubNodeEventLoop<M> 
    where M: Serialize + DeserializeOwned
{
    pub fn new(
        node: Swarm<MpcPubsubBahavior>,
        request_receiver: mpsc::Receiver<MpcPubSubRequest>,
        incoming_sender: mpsc::Sender<Result<M, anyhow::Error>>, //impl Stream<Item = M >, //incoming msg
        outgoing_receiver: mpsc::Receiver<M>, //impl Sink<M>, // outgoing msg
    ) -> Self {
        Self {
            node, request_receiver, incoming_sender, outgoing_receiver,

            pending_dial: Default::default(),
        }
    }

    pub async fn run(mut self) {
        loop {
            futures::select! {
                event = self.node.next().fuse() => {
                    self.handle_event(event.expect("always have an event")).await;
                },
                maybe_out = self.outgoing_receiver.next() => {
                    match maybe_out {
                        Some(msg) => self.handle_outgoing("test", &msg).await,
                        None => {}
                    }
                }
                request = self.request_receiver.next() => {
                    match request {
                        Some(request) => {
                            match self.handle_request(request).await {
                                Ok(()) => {},
                                Err(e) => eprintln!("{:?}", e)
                            }
                        },
                        None => return
                    }
                }
            }
        }
    }

    async fn handle_event(
        &mut self,
        event: SwarmEvent<
            MpcPubsubBahaviorEvent,
            EitherError<ConnectionHandlerUpgrErr<CodecError>, Void>,
        >,
    ) {
        match event {
            // mDNS passive local node discovery
            SwarmEvent::Behaviour(MpcPubsubBahaviorEvent::Mdns(mdns_event)) => {
                match mdns_event {
                    mdns::Event::Discovered(list) => {
                        for (peer_id, multiaddr) in list {
                            println!("mDNS discovered a new peer: {peer_id} {multiaddr}");
                            self.node
                                .behaviour_mut()
                                .floodsub
                                .add_node_to_partial_view(peer_id);
                        }
                    },
                    mdns::Event::Expired(list) => {
                        for (peer_id, _multiaddr) in list {
                            println!("mDNS discover peer has expired: {peer_id}");
                            self.node
                                .behaviour_mut()
                                .floodsub
                                .remove_node_from_partial_view(&peer_id);
                        }
                    }
                }
            },

            // floodsub message
            SwarmEvent::Behaviour(MpcPubsubBahaviorEvent::Floodsub(FloodsubEvent::Message(message))) => {
                self.incoming_sender.send(
                    // TODO: correctly handle this
                    Ok(bincode::deserialize::<M>(&message.data).unwrap())
                    // message.data.clone()
                ).await.map_err(|_| MpcPubSubError::FailToSendViaChannel);
            },

            _ => { }
        }
    }

    async fn handle_outgoing(&mut self, topic: &str, msg: &M) {
        self.node
            .behaviour_mut()
            .floodsub
            .publish_any(Topic::new(topic), bincode::serialize(msg).unwrap());
    }

    async fn handle_request(&mut self, request: MpcPubSubRequest) -> Result<(), MpcPubSubError> {
        match request {
            MpcPubSubRequest::StartListening { addr, result_sender } => {
                match self.node.listen_on(addr) {
                    Ok(_) => {
                        result_sender.send(Ok(()))
                    },
                    Err(_e )=> result_sender.send(Err(MpcPubSubError::FailToListenOnPort)),
                }.map_err(|_| MpcPubSubError::FailToSendViaChannel)
            },
            MpcPubSubRequest::Dial { peer_id, peer_addr, result_sender } => {
                if let Entry::Vacant(e) = self.pending_dial.entry(peer_id) {
                    self.node
                        .behaviour_mut()
                        .floodsub
                        .add_node_to_partial_view(peer_id);

                    match self
                        .node
                        .dial(peer_addr.with(multiaddr::Protocol::P2p(peer_id.into())))
                    {
                        Ok(()) => {
                            e.insert(result_sender);
                            Ok(())
                        }
                        Err(_) => {
                            result_sender.send(Err(MpcPubSubError::FailToDial))
                                .map_err(|_| MpcPubSubError::FailToSendViaChannel)
                        }
                    }
                } else {
                    todo!("Already dialing peer.");
                }
            },
            MpcPubSubRequest::SubscribeToTopic { topic, result_sender } => {
                if self.node
                    .behaviour_mut()
                    .floodsub
                    .subscribe(Topic::new(topic)) == true {

                    result_sender.send(Ok(()))
                        .map_err(|_| MpcPubSubError::FailToSendViaChannel)
                } else {
                    result_sender.send(Err(MpcPubSubError::FailToSubscribeToTopic))
                        .map_err(|_| MpcPubSubError::FailToSendViaChannel)
                }
            }
        }
    }

}
