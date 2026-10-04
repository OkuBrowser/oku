use crate::behaviour::peer_store::store::Store;
use crate::task::ConnexaTask;
use libp2p::dcutr::Event as DcutrEvent;
use libp2p::swarm::NetworkBehaviour;
use std::fmt::Debug;

impl<X, C: NetworkBehaviour, S, T, K> ConnexaTask<X, C, S, T, K>
where
    X: Default + Send + 'static,
    C: Send,
    C::ToSwarm: Debug,
    S: Store,
{
    pub fn process_dcutr_event(&mut self, event: DcutrEvent) {
        let DcutrEvent {
            remote_peer_id,
            result,
        } = event;
        match result {
            Ok(connection_id) => {
                tracing::info!(%remote_peer_id, %connection_id, "dcutr success");
            }
            Err(e) => {
                tracing::error!(%remote_peer_id, %e, "dcutr failed");
            }
        }
    }
}
