use crate::behaviour::peer_store::store::Store;
use crate::task::ConnexaTask;
use libp2p::identify::Event as IdentifyEvent;
use libp2p::swarm::NetworkBehaviour;
use std::fmt::Debug;

impl<X, C: NetworkBehaviour, S, T, K> ConnexaTask<X, C, S, T, K>
where
    X: Default + Send + 'static,
    C: Send,
    C::ToSwarm: Debug,
    S: Store,
{
    pub fn process_identify_event(&mut self, event: IdentifyEvent) {
        #[allow(unused_variables)]
        let Some(swarm) = self.swarm.as_mut() else {
            return;
        };
        match event {
            IdentifyEvent::Received {
                peer_id,
                connection_id,
                info,
            } => {
                tracing::info!(%peer_id, %connection_id, ?info, "identify received");
                let libp2p::identify::Info {
                    listen_addrs,
                    protocols,
                    ..
                } = info;

                // TODO: Debate on if we should leave this in place or rely on the user to add the addresses manually
                //       or as needed using the callback to check this event.
                #[cfg(feature = "kad")]
                if let Some(kad) = swarm.behaviour_mut().kademlia.as_mut() {
                    let supports_kademlia = kad
                        .protocol_names()
                        .iter()
                        .any(|protocol| protocols.contains(protocol));

                    if supports_kademlia {
                        for addr in listen_addrs {
                            kad.add_address(&peer_id, addr.clone());
                        }
                    }
                }

                let _ = listen_addrs;
                let _ = protocols;
            }
            IdentifyEvent::Sent {
                peer_id,
                connection_id,
            } => {
                tracing::info!(%peer_id, %connection_id, "identify sent");
            }
            IdentifyEvent::Pushed {
                peer_id,
                connection_id,
                info,
            } => {
                tracing::info!(%peer_id, %connection_id, ?info, "identify pushed");
            }
            IdentifyEvent::Error {
                peer_id,
                connection_id,
                error,
            } => {
                tracing::error!(%peer_id, %connection_id, error=%error, "identify error");
            }
        }
    }
}
