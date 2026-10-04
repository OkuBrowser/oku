use crate::error::{ConnexaResult, Error};
use crate::handle::Connexa;
use crate::types::RendezvousCommand;
use bytes::Bytes;
use futures::channel::oneshot;
use libp2p::rendezvous::{Cookie, Namespace};
use libp2p::{Multiaddr, PeerId};
use std::io;

pub struct ConnexaRendezvous<'a, T, K> {
    connexa: &'a Connexa<T, K>,
}

impl<'a, T, K> Copy for ConnexaRendezvous<'a, T, K> {}

impl<'a, T, K> Clone for ConnexaRendezvous<'a, T, K> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, T, K> ConnexaRendezvous<'a, T, K>
where
    T: Send + Sync + 'static,
{
    pub(crate) fn new(connexa: &'a Connexa<T, K>) -> Self {
        Self { connexa }
    }

    /// Registers a peer in a namespace with an optional time-to-live (TTL).
    pub async fn register(
        &self,
        peer_id: PeerId,
        namespace: impl IntoNamespace,
        ttl: Option<u64>,
    ) -> ConnexaResult<()> {
        let namespace = namespace
            .into_namespace()?
            .ok_or(Error::InvalidConfig("namespace is not provided".into()))?;

        let (tx, rx) = oneshot::channel();
        self.connexa
            .to_task
            .clone()
            .send(
                RendezvousCommand::Register {
                    namespace,
                    peer_id,
                    ttl,
                    resp: tx,
                }
                .into(),
            )
            .await
            .map_err(|_| Error::ChannelClosed)?;

        rx.await.map_err(|_| Error::ChannelClosed)?
    }

    /// Unregisters a peer from a namespace.
    pub async fn unregister(
        &self,
        peer_id: PeerId,
        namespace: impl IntoNamespace,
    ) -> ConnexaResult<()> {
        let namespace = namespace
            .into_namespace()?
            .ok_or(Error::InvalidConfig("namespace is not provided".into()))?;

        let (tx, rx) = oneshot::channel();
        self.connexa
            .to_task
            .clone()
            .send(
                RendezvousCommand::Unregister {
                    namespace,
                    peer_id,
                    resp: tx,
                }
                .into(),
            )
            .await
            .map_err(|_| Error::ChannelClosed)?;

        rx.await.map_err(|_| Error::ChannelClosed)?
    }

    /// Discovers peers in a namespace.
    pub async fn discovery(
        &self,
        peer_id: PeerId,
        namespace: impl IntoNamespace,
        ttl: Option<u64>,
        cookie: Option<Cookie>,
    ) -> ConnexaResult<(Cookie, Vec<(PeerId, Vec<Multiaddr>)>)> {
        let namespace = namespace.into_namespace()?;
        let (tx, rx) = oneshot::channel();
        self.connexa
            .to_task
            .clone()
            .send(
                RendezvousCommand::Discover {
                    namespace,
                    peer_id,
                    cookie,
                    ttl,
                    resp: tx,
                }
                .into(),
            )
            .await
            .map_err(|_| Error::ChannelClosed)?;

        rx.await.map_err(|_| Error::ChannelClosed)?
    }
}

pub trait IntoNamespace {
    fn into_namespace(self) -> io::Result<Option<Namespace>>;
}

impl IntoNamespace for String {
    fn into_namespace(self) -> io::Result<Option<Namespace>> {
        Namespace::new(self).map_err(io::Error::other).map(Some)
    }
}

impl IntoNamespace for &String {
    fn into_namespace(self) -> io::Result<Option<Namespace>> {
        Namespace::new(self.clone())
            .map_err(io::Error::other)
            .map(Some)
    }
}

impl IntoNamespace for Namespace {
    fn into_namespace(self) -> io::Result<Option<Namespace>> {
        Ok(Some(self))
    }
}

impl IntoNamespace for &Namespace {
    fn into_namespace(self) -> io::Result<Option<Namespace>> {
        Ok(Some(self.clone()))
    }
}

impl IntoNamespace for &str {
    fn into_namespace(self) -> io::Result<Option<Namespace>> {
        Namespace::new(self.to_string())
            .map_err(io::Error::other)
            .map(Some)
    }
}

impl IntoNamespace for () {
    fn into_namespace(self) -> io::Result<Option<Namespace>> {
        Ok(None)
    }
}

impl IntoNamespace for Vec<u8> {
    fn into_namespace(self) -> io::Result<Option<Namespace>> {
        let str = String::from_utf8_lossy(&self);
        str.into_namespace()
    }
}

impl IntoNamespace for &[u8] {
    fn into_namespace(self) -> io::Result<Option<Namespace>> {
        let str = String::from_utf8_lossy(self);
        str.into_namespace()
    }
}

impl IntoNamespace for Bytes {
    fn into_namespace(self) -> io::Result<Option<Namespace>> {
        let str = String::from_utf8_lossy(&self);
        str.into_namespace()
    }
}

impl IntoNamespace for &Bytes {
    fn into_namespace(self) -> io::Result<Option<Namespace>> {
        let str = String::from_utf8_lossy(self);
        str.into_namespace()
    }
}

impl<N: IntoNamespace> IntoNamespace for Option<N> {
    fn into_namespace(self) -> io::Result<Option<Namespace>> {
        match self {
            Some(n) => n.into_namespace(),
            None => Ok(None),
        }
    }
}
