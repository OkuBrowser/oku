use libp2p::gossipsub::{PublishError, SubscriptionError, TopicHash};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("already subscribed to topic {0}")]
    AlreadySubscribed(TopicHash),
    #[error("not subscribed to topic {0}")]
    NotSubscribed(TopicHash),
    #[error(transparent)]
    Subscription(#[from] SubscriptionError),
    #[error(transparent)]
    Publish(#[from] PublishError),
}
