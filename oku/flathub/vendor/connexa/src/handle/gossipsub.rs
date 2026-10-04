use crate::error::{ConnexaResult, Error};
use crate::handle::Connexa;
use crate::types::{GossipsubCommand, GossipsubEvent};
use bytes::Bytes;
use futures::StreamExt;
use futures::channel::oneshot;
use futures::stream::BoxStream;
use libp2p::PeerId;
use libp2p::gossipsub::{Hasher, IdentTopic, MessageAcceptance, MessageId, Topic, TopicHash};

pub struct ConnexaGossipsub<'a, T, K> {
    connexa: &'a Connexa<T, K>,
}

impl<'a, T, K> Copy for ConnexaGossipsub<'a, T, K> {}

impl<'a, T, K> Clone for ConnexaGossipsub<'a, T, K> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, T, K> ConnexaGossipsub<'a, T, K>
where
    T: Send + Sync + 'static,
{
    pub(crate) fn new(connexa: &'a Connexa<T, K>) -> Self {
        Self { connexa }
    }

    /// Subscribes to a specified topic in the gossipsub network.
    pub async fn subscribe(&self, topic: impl IntoTopic) -> ConnexaResult<()> {
        let topic = topic.into_topic();
        let (tx, rx) = oneshot::channel();

        self.connexa
            .to_task
            .clone()
            .send(GossipsubCommand::Subscribe { topic, resp: tx }.into())
            .await
            .map_err(|_| Error::ChannelClosed)?;

        rx.await.map_err(|_| Error::ChannelClosed)?
    }

    /// Creates a listener for a specified gossipsub topic.
    pub async fn listener(
        &self,
        topic: impl IntoTopic,
    ) -> ConnexaResult<BoxStream<'static, GossipsubEvent>> {
        let topic = topic.into_topic();
        let (tx, rx) = oneshot::channel();

        self.connexa
            .to_task
            .clone()
            .send(GossipsubCommand::GossipsubListener { topic, resp: tx }.into())
            .await
            .map_err(|_| Error::ChannelClosed)?;

        rx.await
            .map_err(|_| Error::ChannelClosed)?
            .map(|rx| rx.boxed())
    }

    /// Unsubscribes from a specified gossipsub topic.
    pub async fn unsubscribe(&self, topic: impl IntoTopic) -> ConnexaResult<()> {
        let topic = topic.into_topic();
        let (tx, rx) = oneshot::channel();

        self.connexa
            .to_task
            .clone()
            .send(GossipsubCommand::Unsubscribe { topic, resp: tx }.into())
            .await
            .map_err(|_| Error::ChannelClosed)?;

        rx.await.map_err(|_| Error::ChannelClosed)?
    }

    /// Retrieves a list of peers that are subscribed to a specified topic.
    pub async fn peers(&self, topic: impl IntoTopic) -> ConnexaResult<Vec<PeerId>> {
        let topic = topic.into_topic();
        let (tx, rx) = oneshot::channel();

        self.connexa
            .to_task
            .clone()
            .send(GossipsubCommand::Peers { topic, resp: tx }.into())
            .await
            .map_err(|_| Error::ChannelClosed)?;

        rx.await.map_err(|_| Error::ChannelClosed)?
    }

    /// Publishes a message to a specified gossipsub topic.
    pub async fn publish(
        &self,
        topic: impl IntoTopic,
        message: impl Into<Bytes>,
    ) -> ConnexaResult<()> {
        let topic = topic.into_topic();
        let data = message.into();
        let (tx, rx) = oneshot::channel();

        self.connexa
            .to_task
            .clone()
            .send(
                GossipsubCommand::Publish {
                    topic,
                    data,
                    resp: tx,
                }
                .into(),
            )
            .await
            .map_err(|_| Error::ChannelClosed)?;

        rx.await.map_err(|_| Error::ChannelClosed)?
    }

    /// Reports validation results to the gossipsub system for a received message
    pub async fn report_message(
        &self,
        peer_id: PeerId,
        message_id: MessageId,
        message_acceptance: MessageAcceptance,
    ) -> ConnexaResult<bool> {
        let (tx, rx) = oneshot::channel();

        self.connexa
            .to_task
            .clone()
            .send(
                GossipsubCommand::ReportMessage {
                    peer_id,
                    message_id,
                    accept: message_acceptance,
                    resp: tx,
                }
                .into(),
            )
            .await
            .map_err(|_| Error::ChannelClosed)?;

        rx.await.map_err(|_| Error::ChannelClosed)?
    }
}

pub trait IntoTopic {
    fn into_topic(self) -> TopicHash;
}

impl<H: Hasher> IntoTopic for Topic<H> {
    fn into_topic(self) -> TopicHash {
        self.hash()
    }
}

impl<H: Hasher> IntoTopic for &Topic<H> {
    fn into_topic(self) -> TopicHash {
        self.hash()
    }
}

impl IntoTopic for TopicHash {
    fn into_topic(self) -> TopicHash {
        self
    }
}

impl IntoTopic for &TopicHash {
    fn into_topic(self) -> TopicHash {
        self.clone()
    }
}

impl IntoTopic for String {
    fn into_topic(self) -> TopicHash {
        IdentTopic::new(self).hash()
    }
}

impl IntoTopic for &String {
    fn into_topic(self) -> TopicHash {
        IdentTopic::new(self).hash()
    }
}

impl IntoTopic for &str {
    fn into_topic(self) -> TopicHash {
        IdentTopic::new(self).hash()
    }
}

impl IntoTopic for Vec<u8> {
    fn into_topic(self) -> TopicHash {
        let topic = String::from_utf8_lossy(&self);
        IdentTopic::new(topic).hash()
    }
}

impl IntoTopic for &[u8] {
    fn into_topic(self) -> TopicHash {
        let topic = String::from_utf8_lossy(self);
        IdentTopic::new(topic).hash()
    }
}

impl IntoTopic for Bytes {
    fn into_topic(self) -> TopicHash {
        let topic = String::from_utf8_lossy(&self);
        IdentTopic::new(topic).hash()
    }
}

impl IntoTopic for &Bytes {
    fn into_topic(self) -> TopicHash {
        let topic = String::from_utf8_lossy(self);
        IdentTopic::new(topic).hash()
    }
}

impl IntoTopic for Vec<String> {
    fn into_topic(self) -> TopicHash {
        let topic = self.join("/");
        IntoTopic::into_topic(topic)
    }
}

impl IntoTopic for &[String] {
    fn into_topic(self) -> TopicHash {
        let topic = self.join("/");
        IntoTopic::into_topic(topic)
    }
}

impl IntoTopic for &[&str] {
    fn into_topic(self) -> TopicHash {
        let topic = self.join("/");
        IntoTopic::into_topic(topic)
    }
}

impl IntoTopic for Vec<&str> {
    fn into_topic(self) -> TopicHash {
        let topic = self.join("/");
        IntoTopic::into_topic(topic)
    }
}

impl<F> IntoTopic for F
where
    F: FnOnce() -> TopicHash,
{
    fn into_topic(self) -> TopicHash {
        self()
    }
}
