use crate::error::{ConnexaResult, Error};
use crate::handle::Connexa;
use crate::prelude::PeerId;
use crate::types::PeerstoreCommand;
use futures::channel::oneshot;
use libp2p::Multiaddr;

pub struct ConnexaPeerstore<'a, T, K> {
    connexa: &'a Connexa<T, K>,
}

impl<'a, T, K> Copy for ConnexaPeerstore<'a, T, K> {}

impl<'a, T, K> Clone for ConnexaPeerstore<'a, T, K> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, T, K> ConnexaPeerstore<'a, T, K>
where
    T: Send + Sync + 'static,
{
    pub(crate) fn new(connexa: &'a Connexa<T, K>) -> Self {
        Self { connexa }
    }

    /// Adds a new address for a peer to the peer store.
    pub async fn add_address(&self, peer_id: PeerId, addr: Multiaddr) -> ConnexaResult<()> {
        let (tx, rx) = oneshot::channel();
        self.connexa
            .to_task
            .clone()
            .send(
                PeerstoreCommand::Add {
                    peer_id,
                    addr,
                    resp: tx,
                }
                .into(),
            )
            .await
            .map_err(|_| Error::ChannelClosed)?;
        rx.await.map_err(|_| Error::ChannelClosed)?
    }

    /// Removes a specific address for a peer from the peer store.
    pub async fn remove_address(&self, peer_id: PeerId, addr: Multiaddr) -> ConnexaResult<()> {
        let (tx, rx) = oneshot::channel();
        self.connexa
            .to_task
            .clone()
            .send(
                PeerstoreCommand::RemoveAddress {
                    peer_id,
                    addr,
                    resp: tx,
                }
                .into(),
            )
            .await
            .map_err(|_| Error::ChannelClosed)?;
        rx.await.map_err(|_| Error::ChannelClosed)?
    }

    /// Removes a peer and all its associated addresses from the peer store.
    pub async fn remove_peer(&self, peer_id: PeerId) -> ConnexaResult<Vec<Multiaddr>> {
        let (tx, rx) = oneshot::channel();
        self.connexa
            .to_task
            .clone()
            .send(PeerstoreCommand::Remove { peer_id, resp: tx }.into())
            .await
            .map_err(|_| Error::ChannelClosed)?;
        rx.await.map_err(|_| Error::ChannelClosed)?
    }

    /// Lists all addresses associated with a specific peer.
    pub async fn list(&self, peer_id: PeerId) -> ConnexaResult<Vec<Multiaddr>> {
        let (tx, rx) = oneshot::channel();
        self.connexa
            .to_task
            .clone()
            .send(PeerstoreCommand::List { peer_id, resp: tx }.into())
            .await
            .map_err(|_| Error::ChannelClosed)?;
        rx.await.map_err(|_| Error::ChannelClosed)?
    }

    /// Lists all peers and their associated addresses in the peer store.
    pub async fn list_all(&self) -> ConnexaResult<Vec<(PeerId, Vec<Multiaddr>)>> {
        let (tx, rx) = oneshot::channel();
        self.connexa
            .to_task
            .clone()
            .send(PeerstoreCommand::ListAll { resp: tx }.into())
            .await
            .map_err(|_| Error::ChannelClosed)?;
        rx.await.map_err(|_| Error::ChannelClosed)?
    }
}
