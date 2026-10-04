use libp2p::floodsub::Topic;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("already subscribed to topic {0:?}")]
    AlreadySubscribed(Topic),
    #[error("not subscribed to topic {0:?}")]
    NotSubscribed(Topic),
}
