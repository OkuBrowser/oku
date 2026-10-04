use crate::behaviour::autorelay;
use crate::behaviour::peer_store::store::Store;
use crate::error::{Error, Protocol};
use crate::task::ConnexaTask;
use crate::types::AutoRelayCommand;
#[cfg(not(target_arch = "wasm32"))]
use crate::types::RelayServerCommand;
#[cfg(not(target_arch = "wasm32"))]
use libp2p::relay::Event as RelayServerEvent;
use libp2p::relay::client::Event as RelayClientEvent;
use libp2p::swarm::NetworkBehaviour;
use std::fmt::Debug;

#[allow(dead_code)]
pub const RELAY_NAMESPACE: &[u8] = b"/libp2p/relay";

impl<X, C: NetworkBehaviour, S, T, K> ConnexaTask<X, C, S, T, K>
where
    X: Default + Send + 'static,
    C: Send,
    C::ToSwarm: Debug,
    S: Store,
{
    pub fn process_autorelay_commands(&mut self, command: AutoRelayCommand) {
        let swarm = self.swarm.as_mut().expect("swarm is still valid");
        match command {
            AutoRelayCommand::AddStaticRelay {
                peer_id,
                relay_addr,
                resp,
            } => {
                let Some(autorelay) = swarm.behaviour_mut().autorelay.as_mut() else {
                    let _ = resp.send(Err(Error::Disabled {
                        protocol: Protocol::Autorelay,
                    }));
                    return;
                };

                let _ = resp.send(Ok(autorelay.add_static_relay(peer_id, relay_addr)));
            }
            AutoRelayCommand::RemoveStaticRelay { peer_id, resp } => {
                let Some(autorelay) = swarm.behaviour_mut().autorelay.as_mut() else {
                    let _ = resp.send(Err(Error::Disabled {
                        protocol: Protocol::Autorelay,
                    }));
                    return;
                };

                let _ = resp.send(Ok(autorelay.remove_static_relay(&peer_id)));
            }
            AutoRelayCommand::ListStaticRelays { resp } => {
                let Some(autorelay) = swarm.behaviour_mut().autorelay.as_mut() else {
                    let _ = resp.send(Err(Error::Disabled {
                        protocol: Protocol::Autorelay,
                    }));
                    return;
                };

                let list = autorelay
                    .static_relays()
                    .map(|(peer_id, addr)| (*peer_id, addr.to_vec()))
                    .collect::<Vec<_>>();
                let _ = resp.send(Ok(list));
            }
            AutoRelayCommand::GetStaticRelay { peer_id, resp } => {
                let Some(autorelay) = swarm.behaviour_mut().autorelay.as_mut() else {
                    let _ = resp.send(Err(Error::Disabled {
                        protocol: Protocol::Autorelay,
                    }));
                    return;
                };

                let addr = autorelay
                    .static_relays()
                    .find(|(p, _)| **p == peer_id)
                    .map(|(_, addr)| addr.to_vec())
                    .ok_or_else(|| Error::NotFound("static relay not found".into()));
                let _ = resp.send(addr);
            }
            AutoRelayCommand::EnableAutoRelay { resp } => {
                let Some(autorelay) = swarm.behaviour_mut().autorelay.as_mut() else {
                    let _ = resp.send(Err(Error::Disabled {
                        protocol: Protocol::Autorelay,
                    }));
                    return;
                };

                autorelay.set_status(Some(autorelay::Status::Enable));

                let _ = resp.send(Ok(()));
            }
            AutoRelayCommand::DisableAutoRelay { resp } => {
                let Some(autorelay) = swarm.behaviour_mut().autorelay.as_mut() else {
                    let _ = resp.send(Err(Error::Disabled {
                        protocol: Protocol::Autorelay,
                    }));
                    return;
                };

                autorelay.set_status(Some(autorelay::Status::Disable));

                let _ = resp.send(Ok(()));
            }
            AutoRelayCommand::DisableRelays { resp } => {
                let Some(autorelay) = swarm.behaviour_mut().autorelay.as_mut() else {
                    let _ = resp.send(Err(Error::Disabled {
                        protocol: Protocol::Autorelay,
                    }));
                    return;
                };

                autorelay.remove_all_reservations();

                let _ = resp.send(Ok(()));
            }
        }
    }

    pub fn process_relay_client_event(&mut self, event: RelayClientEvent) {
        match event {
            RelayClientEvent::ReservationReqAccepted {
                relay_peer_id,
                renewal,
                limit,
            } => {
                tracing::info!(%relay_peer_id, %renewal, ?limit, "relay client reservation request accepted");
            }
            RelayClientEvent::OutboundCircuitEstablished {
                relay_peer_id,
                limit,
            } => {
                tracing::info!(%relay_peer_id, ?limit, "relay client outbound circuit established");
            }
            RelayClientEvent::InboundCircuitEstablished { src_peer_id, limit } => {
                tracing::info!(%src_peer_id, ?limit, "relay client inbound circuit established");
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn process_relay_server_event(&mut self, event: RelayServerEvent) {
        match event {
            RelayServerEvent::ReservationReqAccepted {
                src_peer_id,
                renewed,
            } => {
                tracing::info!(%src_peer_id, %renewed, "relay server reservation request accepted");
            }
            RelayServerEvent::ReservationReqDenied {
                src_peer_id,
                status,
            } => {
                tracing::warn!(%src_peer_id, ?status, "relay server reservation request denied");
            }
            RelayServerEvent::ReservationTimedOut { src_peer_id } => {
                tracing::warn!(%src_peer_id, "relay server reservation timed out");
            }
            RelayServerEvent::CircuitReqDenied {
                src_peer_id,
                dst_peer_id,
                status,
            } => {
                tracing::warn!(%src_peer_id, %dst_peer_id, ?status, "relay server circuit request denied");
            }
            RelayServerEvent::CircuitReqAccepted {
                src_peer_id,
                dst_peer_id,
            } => {
                tracing::info!(%src_peer_id, %dst_peer_id, "relay server circuit request accepted");
            }
            RelayServerEvent::CircuitClosed {
                src_peer_id,
                dst_peer_id,
                error,
            } => {
                tracing::warn!(%src_peer_id, %dst_peer_id, ?error, "relay server circuit closed");
            }
            RelayServerEvent::StatusChanged { status } => {
                tracing::info!(?status, "relay server status changed");
            }
            _ => {}
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl<X, C: NetworkBehaviour, S, T, K> ConnexaTask<X, C, S, T, K>
where
    X: Default + Send + 'static,
    C: Send,
    C::ToSwarm: Debug,
    S: Store,
{
    pub fn process_relay_server_command(&mut self, command: RelayServerCommand) {
        let swarm = self.swarm.as_mut().expect("swarm is active");
        match command {
            RelayServerCommand::StatusChanged { status, resp } => {
                let Some(relay) = swarm.behaviour_mut().relay.as_mut() else {
                    let _ = resp.send(Err(std::io::Error::other(
                        "relay server protocol is not enabled",
                    )));
                    return;
                };

                relay.set_status(status);

                let _ = resp.send(Ok(()));
            }
        }
    }
}
