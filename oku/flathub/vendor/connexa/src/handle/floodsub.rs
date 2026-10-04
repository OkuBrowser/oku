use crate::error::{ConnexaResult, Error};
use crate::handle::Connexa;
use crate::types::{FloodsubCommand, FloodsubEvent, PubsubFloodsubPublish};
use bytes::Bytes;
use futures::StreamExt;
use futures::channel::oneshot;
use futures::stream::BoxStream;
use libp2p::PeerId;
use libp2p::floodsub::Topic;

pub struct ConnexaFloodsub<'a, T, K> {
    connexa: &'a Connexa<T, K>,
}

impl<'a, T, K> Copy for ConnexaFloodsub<'a, T, K> {}

impl<'a, T, K> Clone for ConnexaFloodsub<'a, T, K> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, T, K> ConnexaFloodsub<'a, T, K>
where
    T: Send + Sync + 'static,
{
    pub(crate) fn new(connexa: &'a Connexa<T, K>) -> Self {
        Self { connexa }
    }

    /// Subscribes to a topic in the floodsub network
    pub async fn subscribe(&self, topic: impl IntoTopic) -> ConnexaResult<()> {
        let topic = topic.into_topic();
        let (tx, rx) = oneshot::channel();

        self.connexa
            .to_task
            .clone()
            .send(FloodsubCommand::Subscribe { topic, resp: tx }.into())
            .await
            .map_err(|_| Error::ChannelClosed)?;

        rx.await.map_err(|_| Error::ChannelClosed)?
    }

    /// Adds a new node to the partial view of the floodsub network.
    pub async fn add_node_to_partial_view(&self, peer_id: PeerId) -> ConnexaResult<()> {
        let (tx, rx) = oneshot::channel();

        self.connexa
            .to_task
            .clone()
            .send(FloodsubCommand::AddNodeToPartialView { peer_id, resp: tx }.into())
            .await
            .map_err(|_| Error::ChannelClosed)?;

        rx.await.map_err(|_| Error::ChannelClosed)?
    }

    /// Removes a node from the partial view of the floodsub network.
    pub async fn remove_node_from_partial_view(&self, peer_id: PeerId) -> ConnexaResult<()> {
        let (tx, rx) = oneshot::channel();

        self.connexa
            .to_task
            .clone()
            .send(FloodsubCommand::RemoveNodeFromPartialView { peer_id, resp: tx }.into())
            .await
            .map_err(|_| Error::ChannelClosed)?;

        rx.await.map_err(|_| Error::ChannelClosed)?
    }

    /// Creates a listener for a specific topic that returns a stream of pubsub events
    pub async fn listener(
        &self,
        topic: impl IntoTopic,
    ) -> ConnexaResult<BoxStream<'static, FloodsubEvent>> {
        let topic = topic.into_topic();
        let (tx, rx) = oneshot::channel();

        self.connexa
            .to_task
            .clone()
            .send(FloodsubCommand::FloodsubListener { topic, resp: tx }.into())
            .await
            .map_err(|_| Error::ChannelClosed)?;

        rx.await
            .map_err(|_| Error::ChannelClosed)?
            .map(|rx| rx.boxed())
    }

    /// Unsubscribes from a topic in the floodsub network
    pub async fn unsubscribe(&self, topic: impl IntoTopic) -> ConnexaResult<()> {
        let topic = topic.into_topic();
        let (tx, rx) = oneshot::channel();

        self.connexa
            .to_task
            .clone()
            .send(FloodsubCommand::Unsubscribe { topic, resp: tx }.into())
            .await
            .map_err(|_| Error::ChannelClosed)?;

        rx.await.map_err(|_| Error::ChannelClosed)?
    }

    /// Publishes a message to a single topic in the floodsub network
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
                FloodsubCommand::Publish(PubsubFloodsubPublish::Publish { topic, data }, tx).into(),
            )
            .await
            .map_err(|_| Error::ChannelClosed)?;

        rx.await.map_err(|_| Error::ChannelClosed)?
    }

    /// Publishes a message to any peers in a single topic, regardless if they're subscribed
    pub async fn publish_any(
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
                FloodsubCommand::Publish(PubsubFloodsubPublish::PublishAny { topic, data }, tx)
                    .into(),
            )
            .await
            .map_err(|_| Error::ChannelClosed)?;

        rx.await.map_err(|_| Error::ChannelClosed)?
    }

    /// Publishes the same message to multiple topics in the floodsub network
    pub async fn publish_many(
        &self,
        topics: impl IntoIterator<Item = impl IntoTopic>,
        message: impl Into<Bytes>,
    ) -> ConnexaResult<()> {
        let topics = topics
            .into_iter()
            .map(|t| t.into_topic())
            .collect::<Vec<_>>();
        let data = message.into();
        let (tx, rx) = oneshot::channel();

        self.connexa
            .to_task
            .clone()
            .send(
                FloodsubCommand::Publish(PubsubFloodsubPublish::PublishMany { topics, data }, tx)
                    .into(),
            )
            .await
            .map_err(|_| Error::ChannelClosed)?;

        rx.await.map_err(|_| Error::ChannelClosed)?
    }

    /// Publishes the same message to any peers in multiple topics, regardless if they're subscribed
    pub async fn publish_many_any(
        &self,
        topics: impl IntoIterator<Item = impl IntoTopic>,
        message: impl Into<Bytes>,
    ) -> ConnexaResult<()> {
        let topics = topics
            .into_iter()
            .map(|t| t.into_topic())
            .collect::<Vec<_>>();
        let data = message.into();
        let (tx, rx) = oneshot::channel();

        self.connexa
            .to_task
            .clone()
            .send(
                FloodsubCommand::Publish(
                    PubsubFloodsubPublish::PublishManyAny { topics, data },
                    tx,
                )
                .into(),
            )
            .await
            .map_err(|_| Error::ChannelClosed)?;

        rx.await.map_err(|_| Error::ChannelClosed)?
    }
}

pub trait IntoTopic {
    fn into_topic(self) -> Topic;
}

impl IntoTopic for String {
    fn into_topic(self) -> Topic {
        Topic::new(self)
    }
}

impl IntoTopic for &String {
    fn into_topic(self) -> Topic {
        Topic::new(self)
    }
}

impl IntoTopic for &str {
    fn into_topic(self) -> Topic {
        Topic::new(self)
    }
}

impl IntoTopic for Topic {
    fn into_topic(self) -> Topic {
        self
    }
}

impl IntoTopic for &Topic {
    fn into_topic(self) -> Topic {
        self.clone()
    }
}

impl IntoTopic for Vec<u8> {
    fn into_topic(self) -> Topic {
        let topic = String::from_utf8_lossy(&self);
        Topic::new(topic)
    }
}

impl IntoTopic for &[u8] {
    fn into_topic(self) -> Topic {
        let topic = String::from_utf8_lossy(self);
        Topic::new(topic)
    }
}

impl IntoTopic for Bytes {
    fn into_topic(self) -> Topic {
        let topic = String::from_utf8_lossy(&self);
        Topic::new(topic)
    }
}

impl IntoTopic for &Bytes {
    fn into_topic(self) -> Topic {
        let topic = String::from_utf8_lossy(self);
        Topic::new(topic)
    }
}

impl IntoTopic for Vec<String> {
    fn into_topic(self) -> Topic {
        let topic = self.join("/");
        Topic::new(topic)
    }
}

impl IntoTopic for &[String] {
    fn into_topic(self) -> Topic {
        let topic = self.join("/");
        Topic::new(topic)
    }
}

impl IntoTopic for &[&str] {
    fn into_topic(self) -> Topic {
        let topic = self.join("/");
        Topic::new(topic)
    }
}

impl IntoTopic for Vec<&str> {
    fn into_topic(self) -> Topic {
        let topic = self.join("/");
        Topic::new(topic)
    }
}

impl<F> IntoTopic for F
where
    F: FnOnce() -> Topic,
{
    fn into_topic(self) -> Topic {
        self()
    }
}
