use crate::error::{ConnexaResult, Error};
use crate::handle::Connexa;
use crate::prelude::{Multiaddr, PeerId};
use crate::types::AutoRelayCommand;
use futures::channel::oneshot;

pub struct ConnexaRelay<'a, T, K> {
    connexa: &'a Connexa<T, K>,
}

impl<'a, T, K> ConnexaRelay<'a, T, K>
where
    T: Send + Sync + 'static,
{
    pub(crate) fn new(connexa: &'a Connexa<T, K>) -> Self {
        Self { connexa }
    }

    pub async fn add_static_relay(&self, peer_id: PeerId, addr: Multiaddr) -> ConnexaResult<bool> {
        let (tx, rx) = oneshot::channel();
        self.connexa
            .to_task
            .clone()
            .send(
                AutoRelayCommand::AddStaticRelay {
                    peer_id,
                    relay_addr: addr,
                    resp: tx,
                }
                .into(),
            )
            .await
            .map_err(|_| Error::ChannelClosed)?;
        rx.await.map_err(|_| Error::ChannelClosed)?
    }

    pub async fn remove_static_relay(&self, peer_id: PeerId) -> ConnexaResult<bool> {
        let (tx, rx) = oneshot::channel();
        self.connexa
            .to_task
            .clone()
            .send(AutoRelayCommand::RemoveStaticRelay { peer_id, resp: tx }.into())
            .await
            .map_err(|_| Error::ChannelClosed)?;
        rx.await.map_err(|_| Error::ChannelClosed)?
    }

    pub async fn list_static_relays(&self) -> ConnexaResult<Vec<(PeerId, Vec<Multiaddr>)>> {
        let (tx, rx) = oneshot::channel();
        self.connexa
            .to_task
            .clone()
            .send(AutoRelayCommand::ListStaticRelays { resp: tx }.into())
            .await
            .map_err(|_| Error::ChannelClosed)?;
        rx.await.map_err(|_| Error::ChannelClosed)?
    }

    pub async fn get_static_relay(&self, peer_id: PeerId) -> ConnexaResult<Vec<Multiaddr>> {
        let (tx, rx) = oneshot::channel();
        self.connexa
            .to_task
            .clone()
            .send(AutoRelayCommand::GetStaticRelay { peer_id, resp: tx }.into())
            .await
            .map_err(|_| Error::ChannelClosed)?;
        rx.await.map_err(|_| Error::ChannelClosed)?
    }

    pub async fn enable_auto_relay(&self) -> ConnexaResult<()> {
        let (tx, rx) = oneshot::channel();
        self.connexa
            .to_task
            .clone()
            .send(AutoRelayCommand::EnableAutoRelay { resp: tx }.into())
            .await
            .map_err(|_| Error::ChannelClosed)?;
        rx.await.map_err(|_| Error::ChannelClosed)?
    }

    pub async fn disable_auto_relay(&self) -> ConnexaResult<()> {
        let (tx, rx) = oneshot::channel();
        self.connexa
            .to_task
            .clone()
            .send(AutoRelayCommand::DisableAutoRelay { resp: tx }.into())
            .await
            .map_err(|_| Error::ChannelClosed)?;
        rx.await.map_err(|_| Error::ChannelClosed)?
    }

    pub async fn disable_relays(&self) -> ConnexaResult<()> {
        let (tx, rx) = oneshot::channel();
        self.connexa
            .to_task
            .clone()
            .send(AutoRelayCommand::DisableRelays { resp: tx }.into())
            .await
            .map_err(|_| Error::ChannelClosed)?;
        rx.await.map_err(|_| Error::ChannelClosed)?
    }
}
