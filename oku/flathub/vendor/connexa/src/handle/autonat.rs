use crate::error::{ConnexaResult, Error};
use crate::handle::Connexa;
use crate::types::AutonatCommand;
use futures::channel::oneshot;
use libp2p::autonat::NatStatus;
use libp2p::{Multiaddr, PeerId};

pub struct ConnexaAutonat<'a, T, K> {
    connexa: &'a Connexa<T, K>,
}

impl<'a, T, K> Copy for ConnexaAutonat<'a, T, K> {}

impl<'a, T, K> Clone for ConnexaAutonat<'a, T, K> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, T, K> ConnexaAutonat<'a, T, K>
where
    T: Send + Sync + 'static,
{
    pub(crate) fn new(connexa: &'a Connexa<T, K>) -> Self {
        Self { connexa }
    }

    /// Returns the assumed public address of the local node.
    pub async fn public_address(&self) -> ConnexaResult<Option<Multiaddr>> {
        let (tx, rx) = oneshot::channel();
        self.connexa
            .to_task
            .clone()
            .send(AutonatCommand::PublicAddress { resp: tx }.into())
            .await
            .map_err(|_| Error::ChannelClosed)?;
        rx.await.map_err(|_| Error::ChannelClosed)?
    }

    /// Returns the current NAT status of the local node
    pub async fn nat_status(&self) -> ConnexaResult<NatStatus> {
        let (tx, rx) = oneshot::channel();
        self.connexa
            .to_task
            .clone()
            .send(AutonatCommand::NatStatus { resp: tx }.into())
            .await
            .map_err(|_| Error::ChannelClosed)?;
        rx.await.map_err(|_| Error::ChannelClosed)?
    }

    /// Adds a peer to the list of servers used for probes.
    pub async fn add_server(
        &self,
        peer_id: PeerId,
        address: Option<Multiaddr>,
    ) -> ConnexaResult<()> {
        let (tx, rx) = oneshot::channel();
        self.connexa
            .to_task
            .clone()
            .send(
                AutonatCommand::AddServer {
                    peer: peer_id,
                    address,
                    resp: tx,
                }
                .into(),
            )
            .await
            .map_err(|_| Error::ChannelClosed)?;
        rx.await.map_err(|_| Error::ChannelClosed)?
    }

    /// Remove a peer from the list of servers.
    pub async fn remove_server(&self, peer_id: PeerId) -> ConnexaResult<()> {
        let (tx, rx) = oneshot::channel();
        self.connexa
            .to_task
            .clone()
            .send(
                AutonatCommand::RemoveServer {
                    peer: peer_id,
                    resp: tx,
                }
                .into(),
            )
            .await
            .map_err(|_| Error::ChannelClosed)?;
        rx.await.map_err(|_| Error::ChannelClosed)?
    }

    /// Probes a specific address for external reachability
    pub async fn probe(&self, address: Multiaddr) -> ConnexaResult<()> {
        let (tx, rx) = oneshot::channel();
        self.connexa
            .to_task
            .clone()
            .send(AutonatCommand::Probe { address, resp: tx }.into())
            .await
            .map_err(|_| Error::ChannelClosed)?;
        rx.await.map_err(|_| Error::ChannelClosed)?
    }
}
