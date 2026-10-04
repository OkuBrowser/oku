use crate::behaviour::peer_store::store::Store;
use crate::error::{ConnexaResult, Error, Protocol};
use crate::task::ConnexaTask;
use crate::types::RendezvousCommand;
use libp2p::rendezvous::Registration;
use libp2p::rendezvous::client::Event as RendezvousClientEvent;
use libp2p::rendezvous::server::Event as RendezvousServerEvent;
use libp2p::swarm::NetworkBehaviour;
use std::fmt::Debug;

impl<X, C: NetworkBehaviour, S, T, K> ConnexaTask<X, C, S, T, K>
where
    X: Default + Send + 'static,
    C: Send,
    C::ToSwarm: Debug,
    S: Store,
{
    pub fn process_rendezvous_command(&mut self, command: RendezvousCommand) {
        match command {
            RendezvousCommand::Register {
                namespace,
                peer_id,
                ttl,
                resp,
            } => {
                let key = (peer_id, namespace);
                if let Some(queue) = self.pending_rendezvous_register.get_mut(&key) {
                    queue.push_back((ttl, resp));
                    return;
                }

                if let Err(error) = self.start_rendezvous_registration(&key, ttl) {
                    let _ = resp.send(Err(error));
                    return;
                }

                self.pending_rendezvous_register
                    .entry(key)
                    .or_default()
                    .push_back((ttl, resp));
            }
            RendezvousCommand::Unregister {
                namespace,
                peer_id,
                resp,
            } => {
                let swarm = self.swarm.as_mut().expect("swarm is active");
                let Some(rz) = swarm.behaviour_mut().rendezvous_client.as_mut() else {
                    let _ = resp.send(Err(Error::Disabled {
                        protocol: Protocol::Rendezvous,
                    }));
                    return;
                };

                rz.unregister(namespace.clone(), peer_id);

                let _ = resp.send(Ok(()));
            }
            RendezvousCommand::Discover {
                namespace,
                peer_id,
                cookie,
                ttl,
                resp,
            } => {
                let key = (peer_id, namespace);
                if let Some(queue) = self.pending_rendezvous_discover.get_mut(&key) {
                    queue.push_back((cookie, ttl, resp));
                    return;
                }

                if let Err(error) = self.start_rendezvous_discovery(&key, cookie.clone(), ttl) {
                    let _ = resp.send(Err(error));
                    return;
                }

                self.pending_rendezvous_discover
                    .entry(key)
                    .or_default()
                    .push_back((cookie, ttl, resp));
            }
        }
    }

    fn start_rendezvous_discovery(
        &mut self,
        key: &(libp2p::PeerId, Option<libp2p::rendezvous::Namespace>),
        cookie: Option<libp2p::rendezvous::Cookie>,
        ttl: Option<u64>,
    ) -> ConnexaResult<()> {
        let swarm = self.swarm.as_mut().expect("swarm is active");
        let Some(rendezvous) = swarm.behaviour_mut().rendezvous_client.as_mut() else {
            return Err(Error::Disabled {
                protocol: Protocol::Rendezvous,
            });
        };

        rendezvous.discover(key.1.clone(), cookie, ttl, key.0);
        Ok(())
    }

    fn finish_rendezvous_discovery(
        &mut self,
        key: (libp2p::PeerId, Option<libp2p::rendezvous::Namespace>),
        result: crate::task::RendezvousDiscoverResponse,
    ) {
        let Some(mut queue) = self.pending_rendezvous_discover.shift_remove(&key) else {
            return;
        };

        if let Some((_, _, response)) = queue.pop_front() {
            let _ = response.send(result);
        }

        while let Some((cookie, ttl, _)) = queue.front() {
            match self.start_rendezvous_discovery(&key, cookie.clone(), *ttl) {
                Ok(()) => {
                    self.pending_rendezvous_discover.insert(key, queue);
                    return;
                }
                Err(error) => {
                    if let Some((_, _, response)) = queue.pop_front() {
                        let _ = response.send(Err(error));
                    }
                }
            }
        }
    }

    fn start_rendezvous_registration(
        &mut self,
        key: &(libp2p::PeerId, libp2p::rendezvous::Namespace),
        ttl: Option<u64>,
    ) -> ConnexaResult<()> {
        let swarm = self.swarm.as_mut().expect("swarm is active");
        let Some(rendezvous) = swarm.behaviour_mut().rendezvous_client.as_mut() else {
            return Err(Error::Disabled {
                protocol: Protocol::Rendezvous,
            });
        };

        rendezvous
            .register(key.1.clone(), key.0, ttl)
            .map_err(|error| std::io::Error::other(error).into())
    }

    fn finish_rendezvous_registration(
        &mut self,
        key: (libp2p::PeerId, libp2p::rendezvous::Namespace),
        result: ConnexaResult<()>,
    ) {
        let Some(mut queue) = self.pending_rendezvous_register.shift_remove(&key) else {
            return;
        };

        if let Some((_, response)) = queue.pop_front() {
            let _ = response.send(result);
        }

        while let Some((ttl, _)) = queue.front() {
            match self.start_rendezvous_registration(&key, *ttl) {
                Ok(()) => {
                    self.pending_rendezvous_register.insert(key, queue);
                    return;
                }
                Err(error) => {
                    if let Some((_, response)) = queue.pop_front() {
                        let _ = response.send(Err(error));
                    }
                }
            }
        }
    }

    pub fn process_rendezvous_server_event(&mut self, event: RendezvousServerEvent) {
        match event {
            RendezvousServerEvent::DiscoverServed {
                enquirer,
                registrations,
            } => {
                tracing::debug!(%enquirer, ?registrations, "discovered");
            }
            RendezvousServerEvent::DiscoverNotServed { enquirer, error } => {
                tracing::error!(%enquirer, ?error, "failed to serve a discover request");
            }
            RendezvousServerEvent::PeerRegistered { peer, registration } => {
                let namespace = registration.namespace;
                tracing::debug!(%peer, %namespace, "registered to namespace");
            }
            RendezvousServerEvent::PeerNotRegistered {
                peer,
                namespace,
                error,
            } => {
                tracing::error!(%peer, %namespace, ?error, "not register to namespace");
            }
            RendezvousServerEvent::PeerUnregistered { peer, namespace } => {
                tracing::debug!(%peer, %namespace, "unregistered from namespace");
            }
            RendezvousServerEvent::RegistrationExpired(Registration {
                namespace,
                record,
                ttl,
            }) => {
                let peer_id = record.peer_id();
                tracing::debug!(%namespace, %peer_id, %ttl, "peer registration expired");
            }
        }
    }

    pub fn process_rendezvous_client_event(&mut self, event: RendezvousClientEvent) {
        match event {
            RendezvousClientEvent::Discovered {
                rendezvous_node,
                registrations,
                cookie,
            } => {
                tracing::debug!(%rendezvous_node, ?cookie, ?registrations, "discovered");

                let discovered_peers = registrations
                    .into_iter()
                    .map(|registration| {
                        let peer_id = registration.record.peer_id();
                        let addrs = registration.record.addresses().to_vec();
                        (peer_id, addrs)
                    })
                    .collect::<Vec<_>>();
                let namespace = cookie.namespace().cloned();
                self.finish_rendezvous_discovery(
                    (rendezvous_node, namespace),
                    Ok((cookie, discovered_peers)),
                );
            }
            RendezvousClientEvent::DiscoverFailed {
                rendezvous_node,
                namespace,
                error,
            } => {
                tracing::error!(%rendezvous_node, ?namespace, ?error, "failed to discover ");
                self.finish_rendezvous_discovery(
                    (rendezvous_node, namespace),
                    Err(Error::Rendezvous(crate::error::rendezvous::Error::from(
                        error,
                    ))),
                );
            }
            RendezvousClientEvent::Registered {
                rendezvous_node,
                ttl,
                namespace,
            } => {
                tracing::debug!(%rendezvous_node, %namespace, %ttl, "registered to namespace");
                self.finish_rendezvous_registration((rendezvous_node, namespace), Ok(()));
            }
            RendezvousClientEvent::RegisterFailed {
                rendezvous_node,
                namespace,
                error,
            } => {
                tracing::error!(%rendezvous_node, %namespace, ?error, "failed to register to namespace");
                self.finish_rendezvous_registration(
                    (rendezvous_node, namespace),
                    Err(Error::Rendezvous(crate::error::rendezvous::Error::from(
                        error,
                    ))),
                );
            }
            RendezvousClientEvent::Expired { peer } => {
                tracing::debug!(%peer, "expired");
            }
        }
    }
}
