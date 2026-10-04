use crate::error::{ConnexaResult, Error};
use crate::handle::Connexa;
use crate::prelude::PeerId;
use crate::types::BlacklistCommand;
use futures::channel::oneshot;

pub struct ConnexaBlacklist<'a, T, K> {
    connexa: &'a Connexa<T, K>,
}

impl<'a, T, K> Copy for ConnexaBlacklist<'a, T, K> {}

impl<'a, T, K> Clone for ConnexaBlacklist<'a, T, K> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, T, K> ConnexaBlacklist<'a, T, K>
where
    T: Send + Sync + 'static,
{
    pub(crate) fn new(connexa: &'a Connexa<T, K>) -> Self {
        Self { connexa }
    }

    /// Adds a peer to the blacklist.
    pub async fn add(&self, peer_id: PeerId) -> ConnexaResult<()> {
        let (tx, rx) = oneshot::channel();
        self.connexa
            .to_task
            .clone()
            .send(BlacklistCommand::Add { peer_id, resp: tx }.into())
            .await
            .map_err(|_| Error::ChannelClosed)?;
        rx.await.map_err(|_| Error::ChannelClosed)?
    }

    /// Removes a peer from the blacklist.
    pub async fn remove(&self, peer_id: PeerId) -> ConnexaResult<()> {
        let (tx, rx) = oneshot::channel();
        self.connexa
            .to_task
            .clone()
            .send(BlacklistCommand::Remove { peer_id, resp: tx }.into())
            .await
            .map_err(|_| Error::ChannelClosed)?;
        rx.await.map_err(|_| Error::ChannelClosed)?
    }

    /// Retrieves the list of blacklisted peers.
    pub async fn list(&self) -> ConnexaResult<Vec<PeerId>> {
        let (tx, rx) = oneshot::channel();
        self.connexa
            .to_task
            .clone()
            .send(BlacklistCommand::List { resp: tx }.into())
            .await
            .map_err(|_| Error::ChannelClosed)?;
        rx.await.map_err(|_| Error::ChannelClosed)?
    }
}
