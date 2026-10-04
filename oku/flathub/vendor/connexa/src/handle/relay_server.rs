use crate::handle::Connexa;
use crate::types::RelayServerCommand;
use libp2p::relay::Status as RelayServerStatus;

#[derive(Copy, Clone)]
pub struct ConnexaRelayServer<'a, T = (), K = crate::keystore::store::memory::MemoryKeystore> {
    connexa: &'a Connexa<T, K>,
}

impl<'a, T, K> ConnexaRelayServer<'a, T, K>
where
    T: Send + Sync + 'static,
{
    pub(crate) fn new(connexa: &'a Connexa<T, K>) -> Self {
        Self { connexa }
    }

    pub async fn change_status(
        &self,
        status: impl Into<Option<RelayServerStatus>>,
    ) -> std::io::Result<()> {
        let (tx, rx) = futures::channel::oneshot::channel();
        let status = status.into();
        self.connexa
            .to_task
            .clone()
            .send(RelayServerCommand::StatusChanged { status, resp: tx }.into())
            .await?;
        rx.await.map_err(std::io::Error::other)?
    }
}
