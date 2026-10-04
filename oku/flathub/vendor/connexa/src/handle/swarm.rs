use crate::error::{ConnexaResult, Error};
use crate::handle::Connexa;
use crate::types::{ConnexaSwarmEvent, SwarmCommand};
use futures::StreamExt;
use futures::channel::oneshot;
use futures::stream::BoxStream;
use libp2p::core::transport::ListenerId;
use libp2p::swarm::ConnectionId;
use libp2p::swarm::dial_opts::DialOpts;
use libp2p::{Multiaddr, PeerId};
use std::str::FromStr;

pub struct ConnexaSwarm<'a, T, K> {
    connexa: &'a Connexa<T, K>,
}

impl<'a, T, K> Copy for ConnexaSwarm<'a, T, K> {}

impl<'a, T, K> Clone for ConnexaSwarm<'a, T, K> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, T, K> ConnexaSwarm<'a, T, K>
where
    T: Send + Sync + 'static,
{
    pub(crate) fn new(connexa: &'a Connexa<T, K>) -> Self {
        Self { connexa }
    }

    /// Initiates a dial attempt to connect to a remote peer using provided dial options
    /// Returns the ConnectionId upon successful connection
    pub async fn dial(&self, target: impl Into<DialOpts>) -> ConnexaResult<ConnectionId> {
        let target = target.into();
        let (tx, rx) = oneshot::channel();
        self.connexa
            .to_task
            .clone()
            .send(
                SwarmCommand::Dial {
                    opt: target,
                    resp: tx,
                }
                .into(),
            )
            .await
            .map_err(|_| Error::ChannelClosed)?;

        rx.await.map_err(|_| Error::ChannelClosed)?
    }

    /// Disconnects from a peer specified by either PeerId or ConnectionId
    pub async fn disconnect(&self, target_type: impl Into<ConnectionTarget>) -> ConnexaResult<()> {
        let target_type = target_type.into();
        let (tx, rx) = oneshot::channel();
        self.connexa
            .to_task
            .clone()
            .send(
                SwarmCommand::Disconnect {
                    target_type,
                    resp: tx,
                }
                .into(),
            )
            .await
            .map_err(|_| Error::ChannelClosed)?;

        rx.await.map_err(|_| Error::ChannelClosed)?
    }

    /// Checks if we are connected to a specific peer
    pub async fn is_connected(&self, peer_id: PeerId) -> ConnexaResult<bool> {
        let (tx, rx) = oneshot::channel();
        self.connexa
            .to_task
            .clone()
            .send(SwarmCommand::IsConnected { peer_id, resp: tx }.into())
            .await
            .map_err(|_| Error::ChannelClosed)?;
        rx.await.map_err(|_| Error::ChannelClosed)
    }

    /// Returns a list of all currently connected peers
    pub async fn connected_peers(&self) -> ConnexaResult<Vec<PeerId>> {
        let (tx, rx) = oneshot::channel();
        self.connexa
            .to_task
            .clone()
            .send(SwarmCommand::ConnectedPeers { resp: tx }.into())
            .await
            .map_err(|_| Error::ChannelClosed)?;

        rx.await.map_err(|_| Error::ChannelClosed)
    }

    /// Start listening for incoming connections on the given multiaddress
    pub async fn listen_on(&self, addr: impl ToMultiaddr) -> ConnexaResult<ListenerId> {
        let address = addr.to_multiaddr()?;
        let (tx, rx) = oneshot::channel();
        self.connexa
            .to_task
            .clone()
            .send(SwarmCommand::ListenOn { address, resp: tx }.into())
            .await
            .map_err(|_| Error::ChannelClosed)?;

        rx.await.map_err(|_| Error::ChannelClosed)?
    }

    /// Get a listening address by an existing [`ListenerId`]
    pub async fn get_listening_addresses(&self, id: ListenerId) -> ConnexaResult<Vec<Multiaddr>> {
        let (tx, rx) = oneshot::channel();
        self.connexa
            .to_task
            .clone()
            .send(SwarmCommand::GetListeningAddress { id, resp: tx }.into())
            .await
            .map_err(|_| Error::ChannelClosed)?;

        rx.await.map_err(|_| Error::ChannelClosed)?
    }

    /// Stops listening to the address associated with the given [`ListenerId`]
    pub async fn remove_listener(&self, listener_id: ListenerId) -> ConnexaResult<()> {
        let (tx, rx) = oneshot::channel();
        self.connexa
            .to_task
            .clone()
            .send(
                SwarmCommand::RemoveListener {
                    listener_id,
                    resp: tx,
                }
                .into(),
            )
            .await
            .map_err(|_| Error::ChannelClosed)?;

        rx.await.map_err(|_| Error::ChannelClosed)?
    }

    /// Adds an external address that other peers can use to reach us
    pub async fn add_external_address(&self, address: impl ToMultiaddr) -> ConnexaResult<()> {
        let address = address.to_multiaddr()?;
        let (tx, rx) = oneshot::channel();
        self.connexa
            .to_task
            .clone()
            .send(SwarmCommand::AddExternalAddress { address, resp: tx }.into())
            .await
            .map_err(|_| Error::ChannelClosed)?;

        rx.await.map_err(|_| Error::ChannelClosed)?
    }

    /// Removes an external address
    pub async fn remove_external_address(&self, address: Multiaddr) -> ConnexaResult<()> {
        let (tx, rx) = oneshot::channel();
        self.connexa
            .to_task
            .clone()
            .send(SwarmCommand::RemoveExternalAddress { address, resp: tx }.into())
            .await
            .map_err(|_| Error::ChannelClosed)?;

        rx.await.map_err(|_| Error::ChannelClosed)?
    }

    /// Returns a list of all external addresses that other peers can use to reach us
    pub async fn external_addresses(&self) -> ConnexaResult<Vec<Multiaddr>> {
        let (tx, rx) = oneshot::channel();
        self.connexa
            .to_task
            .clone()
            .send(SwarmCommand::ListExternalAddresses { resp: tx }.into())
            .await
            .map_err(|_| Error::ChannelClosed)?;

        rx.await.map_err(|_| Error::ChannelClosed)
    }

    /// Returns a list of all addresses we are currently listening on
    pub async fn listening_addresses(&self) -> ConnexaResult<Vec<Multiaddr>> {
        let (tx, rx) = oneshot::channel();
        self.connexa
            .to_task
            .clone()
            .send(SwarmCommand::ListListeningAddresses { resp: tx }.into())
            .await
            .map_err(|_| Error::ChannelClosed)?;

        rx.await.map_err(|_| Error::ChannelClosed)
    }

    /// Associates an address with a PeerId in the local address book
    pub async fn add_peer_address(&self, peer_id: PeerId, address: Multiaddr) -> ConnexaResult<()> {
        let (tx, rx) = oneshot::channel();
        self.connexa
            .to_task
            .clone()
            .send(
                SwarmCommand::AddPeerAddress {
                    peer_id,
                    address,
                    resp: tx,
                }
                .into(),
            )
            .await
            .map_err(|_| Error::ChannelClosed)?;

        rx.await.map_err(|_| Error::ChannelClosed)?
    }

    /// Subscribes to swarm connection events.
    pub async fn listener(&self) -> ConnexaResult<BoxStream<'static, ConnexaSwarmEvent>> {
        let (tx, rx) = oneshot::channel();
        self.connexa
            .to_task
            .clone()
            .send(SwarmCommand::Listener { resp: tx }.into())
            .await
            .map_err(|_| Error::ChannelClosed)?;

        rx.await
            .map_err(|_| Error::ChannelClosed)
            .map(|rx| rx.boxed())
    }
}

pub trait ToMultiaddr {
    fn to_multiaddr(self) -> std::io::Result<Multiaddr>;
}

impl ToMultiaddr for Multiaddr {
    fn to_multiaddr(self) -> std::io::Result<Multiaddr> {
        Ok(self)
    }
}

impl ToMultiaddr for &Multiaddr {
    fn to_multiaddr(self) -> std::io::Result<Multiaddr> {
        Ok(self.clone())
    }
}

impl ToMultiaddr for &String {
    fn to_multiaddr(self) -> std::io::Result<Multiaddr> {
        Multiaddr::from_str(self).map_err(std::io::Error::other)
    }
}

impl ToMultiaddr for &'static str {
    fn to_multiaddr(self) -> std::io::Result<Multiaddr> {
        Multiaddr::from_str(self).map_err(std::io::Error::other)
    }
}

impl ToMultiaddr for String {
    fn to_multiaddr(self) -> std::io::Result<Multiaddr> {
        Multiaddr::from_str(&self).map_err(std::io::Error::other)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ConnectionTarget {
    PeerId(PeerId),
    ConnectionId(ConnectionId),
}

impl From<PeerId> for ConnectionTarget {
    fn from(peer_id: PeerId) -> Self {
        Self::PeerId(peer_id)
    }
}

impl From<ConnectionId> for ConnectionTarget {
    fn from(connection_id: ConnectionId) -> Self {
        Self::ConnectionId(connection_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_ADDR: &str = "/ip4/127.0.0.1/tcp/8080";
    const INVALID_ADDR: &str = "not-a-valid-multiaddr";

    #[test]
    fn to_multiaddr_from_multiaddr() {
        let addr = Multiaddr::from_str(VALID_ADDR).unwrap();
        let result = addr.clone().to_multiaddr().unwrap();
        assert_eq!(result, addr);
    }

    #[test]
    fn to_multiaddr_from_multiaddr_ref() {
        let addr = Multiaddr::from_str(VALID_ADDR).unwrap();
        let result = (&addr).to_multiaddr().unwrap();
        assert_eq!(result, addr);
    }

    #[test]
    fn to_multiaddr_from_static_str() {
        let result = VALID_ADDR.to_multiaddr().unwrap();
        assert_eq!(result, Multiaddr::from_str(VALID_ADDR).unwrap());
    }

    #[test]
    fn to_multiaddr_from_static_str_invalid() {
        let result = INVALID_ADDR.to_multiaddr();
        assert!(result.is_err());
    }

    #[test]
    fn to_multiaddr_from_string() {
        let addr = String::from(VALID_ADDR);
        let result = addr.to_multiaddr().unwrap();
        assert_eq!(result, Multiaddr::from_str(VALID_ADDR).unwrap());
    }

    #[test]
    fn to_multiaddr_from_string_invalid() {
        let addr = String::from(INVALID_ADDR);
        let result = addr.to_multiaddr();
        assert!(result.is_err());
    }

    #[test]
    fn to_multiaddr_from_string_ref() {
        let addr = String::from(VALID_ADDR);
        let result = (&addr).to_multiaddr().unwrap();
        assert_eq!(result, Multiaddr::from_str(VALID_ADDR).unwrap());
    }

    #[test]
    fn to_multiaddr_from_string_ref_invalid() {
        let addr = String::from(INVALID_ADDR);
        let result = (&addr).to_multiaddr();
        assert!(result.is_err());
    }
}
