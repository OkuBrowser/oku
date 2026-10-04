use crate::behaviour::peer_store::store::Store;
use crate::error::{Error, Protocol};
use crate::task::ConnexaTask;
use crate::types::AutonatCommand;
use libp2p::autonat::v1::Event as AutonatV1Event;
use libp2p::autonat::v2::client::Event as AutonatV2ClientEvent;
use libp2p::autonat::v2::server::Event as AutonatV2ServerEvent;
use libp2p::swarm::NetworkBehaviour;
use std::fmt::Debug;

impl<X, C: NetworkBehaviour, S, T, K> ConnexaTask<X, C, S, T, K>
where
    X: Default + Send + 'static,
    C: Send,
    C::ToSwarm: Debug,
    S: Store,
{
    pub fn process_autonat_v1_command(&mut self, command: AutonatCommand) {
        let Some(swarm) = self.swarm.as_mut() else {
            return;
        };
        match command {
            AutonatCommand::PublicAddress { resp } => {
                let Some(autonat) = swarm.behaviour_mut().autonat_v1.as_mut() else {
                    let _ = resp.send(Err(Error::Disabled {
                        protocol: Protocol::Autonat,
                    }));
                    return;
                };

                let addr = autonat.public_address().cloned();

                let _ = resp.send(Ok(addr));
            }
            AutonatCommand::NatStatus { resp } => {
                let Some(autonat) = swarm.behaviour_mut().autonat_v1.as_mut() else {
                    let _ = resp.send(Err(Error::Disabled {
                        protocol: Protocol::Autonat,
                    }));
                    return;
                };

                let status = autonat.nat_status();

                let _ = resp.send(Ok(status));
            }
            AutonatCommand::AddServer {
                peer,
                address,
                resp,
            } => {
                let Some(autonat) = swarm.behaviour_mut().autonat_v1.as_mut() else {
                    let _ = resp.send(Err(Error::Disabled {
                        protocol: Protocol::Autonat,
                    }));
                    return;
                };

                autonat.add_server(peer, address);
                let _ = resp.send(Ok(()));
            }
            AutonatCommand::RemoveServer { peer, resp } => {
                let Some(autonat) = swarm.behaviour_mut().autonat_v1.as_mut() else {
                    let _ = resp.send(Err(Error::Disabled {
                        protocol: Protocol::Autonat,
                    }));
                    return;
                };

                autonat.remove_server(&peer);

                let _ = resp.send(Ok(()));
            }
            AutonatCommand::Probe { address, resp } => {
                let Some(autonat) = swarm.behaviour_mut().autonat_v1.as_mut() else {
                    let _ = resp.send(Err(Error::Disabled {
                        protocol: Protocol::Autonat,
                    }));
                    return;
                };

                autonat.probe_address(address);

                let _ = resp.send(Ok(()));
            }
        }
    }
    pub fn process_autonat_v1_event(&mut self, event: AutonatV1Event) {
        match event {
            AutonatV1Event::InboundProbe(_) => {}
            AutonatV1Event::OutboundProbe(_) => {}
            AutonatV1Event::StatusChanged { old, new } => {
                tracing::info!(old = ?old, new = ?new, "nat status changed");
            }
        }
    }

    pub fn process_autonat_v2_client_event(&mut self, event: AutonatV2ClientEvent) {
        let AutonatV2ClientEvent {
            tested_addr,
            bytes_sent,
            server,
            result,
        } = event;
        match result {
            Ok(_) => {
                tracing::info!(%tested_addr, %bytes_sent, %server, "autonat v2 response succeed");
            }
            Err(e) => {
                tracing::info!(%tested_addr, %bytes_sent, %server, error=%e, "autonat v2 response error");
            }
        }
    }

    pub fn process_autonat_v2_server_event(&mut self, event: AutonatV2ServerEvent) {
        let AutonatV2ServerEvent {
            all_addrs,
            tested_addr,
            client,
            data_amount,
            result,
        } = event;
        match result {
            Ok(_) => {
                tracing::info!(%tested_addr, ?all_addrs, %client, %data_amount, "autonat v2 response succeed");
            }
            Err(e) => {
                tracing::info!(%tested_addr, ?all_addrs, %client, %data_amount, error=%e, "autonat v2 response error");
            }
        }
    }
}
