use anyhow::anyhow;
use futures::{
    FutureExt, SinkExt,
    channel::{
        mpsc::{Receiver, Sender},
        oneshot,
    },
};
use indexmap::IndexSet;
use pollable_map::optional::Optional;

use crate::{p2p, p2p::MultiaddrExt};

use crate::repo::{Repo, RepoEvent};
use crate::{IpfsEvent, config::BOOTSTRAP_NODES};

use ipld_core::cid::Cid;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::task::{Context as TaskContext, Poll};

use crate::repo::DefaultStorage;

use connexa::behaviour::Behaviour as ConnexaBehaviour;
use connexa::behaviour::peer_store::store::memory::MemoryStore;
use connexa::prelude::identify::Info;
use connexa::prelude::swarm::{NetworkBehaviour, Swarm};
use connexa::prelude::{Multiaddr, PeerId};

#[allow(clippy::type_complexity)]
#[allow(dead_code)]
pub struct IpfsContext {
    pub repo_events: Optional<Receiver<RepoEvent>>,
    pub repo: Repo<DefaultStorage>,
    pub bootstraps: HashSet<Multiaddr>,
    pub find_peer_identify: HashMap<PeerId, Vec<oneshot::Sender<anyhow::Result<Info>>>>,
    pub discovery_tx: Option<Sender<Cid>>,
    pub gateway_tx: Option<Sender<Cid>>,
    pub router_tx: Option<Sender<Cid>>,
    pending_discovery: IndexSet<Cid>,
    pending_gateway: IndexSet<Cid>,
    pending_router: IndexSet<Cid>,
}

impl Default for IpfsContext {
    fn default() -> Self {
        Self {
            repo_events: Default::default(),
            repo: Repo::new_memory(),
            bootstraps: Default::default(),
            find_peer_identify: Default::default(),
            discovery_tx: None,
            gateway_tx: None,
            router_tx: None,
            pending_discovery: Default::default(),
            pending_gateway: Default::default(),
            pending_router: Default::default(),
        }
    }
}

impl IpfsContext {
    pub fn new(repo: &Repo<DefaultStorage>) -> Self {
        Self {
            repo_events: Default::default(),
            repo: repo.clone(),
            bootstraps: Default::default(),
            find_peer_identify: Default::default(),
            discovery_tx: None,
            gateway_tx: None,
            router_tx: None,
            pending_discovery: Default::default(),
            pending_gateway: Default::default(),
            pending_router: Default::default(),
        }
    }

    pub(crate) fn enqueue_retrieval(&mut self, cid: Cid) {
        if self.discovery_tx.is_some() {
            self.pending_discovery.insert(cid);
        }
        if self.gateway_tx.is_some() {
            self.pending_gateway.insert(cid);
        }
        if self.router_tx.is_some() {
            self.pending_router.insert(cid);
        }
    }

    fn enqueue_gateway_retrieval(&mut self, cid: Cid) {
        if self.gateway_tx.is_some() {
            self.pending_gateway.insert(cid);
        }
    }

    fn cancel_pending_retrieval(&mut self, cid: &Cid) {
        self.pending_discovery.shift_remove(cid);
        self.pending_gateway.shift_remove(cid);
        self.pending_router.shift_remove(cid);
    }

    pub(crate) fn flush_retrieval_queues(&mut self, cx: &mut TaskContext<'_>) {
        Self::flush_queue(cx, &mut self.discovery_tx, &mut self.pending_discovery);
        Self::flush_queue(cx, &mut self.gateway_tx, &mut self.pending_gateway);
        Self::flush_queue(cx, &mut self.router_tx, &mut self.pending_router);
    }

    fn flush_queue(
        cx: &mut TaskContext<'_>,
        sender: &mut Option<Sender<Cid>>,
        pending: &mut IndexSet<Cid>,
    ) {
        let Some(sender) = sender.as_mut() else {
            pending.clear();
            return;
        };

        while !pending.is_empty() {
            match sender.poll_ready_unpin(cx) {
                Poll::Ready(Ok(())) => {
                    let cid = *pending.get_index(0).expect("pending queue is not empty");
                    if sender.start_send_unpin(cid).is_ok() {
                        pending.shift_remove_index(0);
                    } else {
                        return;
                    }
                }
                Poll::Ready(Err(_)) => return,
                Poll::Pending => return,
            }
        }
    }
}

impl IpfsContext {
    fn custom_behaviour<'a, N: NetworkBehaviour>(
        &self,
        swarm: &'a mut Swarm<ConnexaBehaviour<p2p::Behaviour<N>, MemoryStore>>,
    ) -> &'a mut p2p::Behaviour<N>
    where
        N::ToSwarm: Debug,
    {
        swarm
            .behaviour_mut()
            .custom
            .as_mut()
            .expect("behaviour enabled")
    }

    pub(crate) fn handle_event<N: NetworkBehaviour>(
        &mut self,
        swarm: &mut Swarm<ConnexaBehaviour<p2p::Behaviour<N>, MemoryStore>>,
        event: IpfsEvent,
    ) where
        N::ToSwarm: Debug,
    {
        match event {
            IpfsEvent::AddPeer(opt, ret) => {
                if let Some(kad) = swarm.behaviour_mut().kademlia.as_mut() {
                    let peer_id = opt.peer_id();
                    let addrs = opt.addresses().to_vec();
                    for addr in addrs {
                        kad.add_address(peer_id, addr);
                    }
                }
                let result = match self.custom_behaviour(swarm).add_peer(opt) {
                    true => Ok(()),
                    false => Err(anyhow::anyhow!("unable to add peer")),
                };

                let _ = ret.send(result);
            }
            IpfsEvent::Addresses(ret) => {
                let custom_behaviour = self.custom_behaviour(swarm);
                let addrs = custom_behaviour
                    .peerbook
                    .connected_peers_addrs()
                    .collect::<Vec<_>>();
                let _ = ret.send(Ok(addrs));
            }
            IpfsEvent::RemovePeer(peer_id, addr, ret) => {
                let custom_behaviour = self.custom_behaviour(swarm);
                let result = match addr {
                    Some(addr) => Ok(custom_behaviour.addressbook.remove_address(&peer_id, &addr)),
                    None => Ok(custom_behaviour.addressbook.remove_peer(&peer_id)),
                };

                let _ = ret.send(result);
            }
            IpfsEvent::Protocol(ret) => {
                let info = self.custom_behaviour(swarm).supported_protocols();
                let _ = ret.send(info);
            }
            // IpfsEvent::WantList(peer, ret) => {
            //     let list = if let Some(peer) = peer {
            //         self.swarm
            //             .behaviour_mut()
            //             .bitswap()
            //             .peer_wantlist(&peer)
            //             .unwrap_or_default()
            //     } else {
            //         self.swarm.behaviour_mut().bitswap().local_wantlist()
            //     };
            //     let _ = ret.send(list);
            // }
            // IpfsEvent::BitswapStats(ret) => {
            //     let stats = self.swarm.behaviour_mut().bitswap().stats();
            //     let peers = self.swarm.behaviour_mut().bitswap().peers();
            //     let wantlist = self.swarm.behaviour_mut().bitswap().local_wantlist();
            //     let _ = ret.send((stats, peers, wantlist).into());
            // }
            IpfsEvent::WantList(peer, ret) => {
                let Some(bitswap) = self.custom_behaviour(swarm).bitswap.as_ref() else {
                    let _ = ret.send(Ok(futures::future::ready(vec![]).boxed()));
                    return;
                };
                let list = match peer {
                    Some(peer_id) => bitswap.peer_wantlist(peer_id),
                    None => bitswap.local_wantlist(),
                };
                let _ = ret.send(Ok(futures::future::ready(list).boxed()));
            }
            IpfsEvent::GetBitswapPeers(ret) => {
                let peers = self
                    .custom_behaviour(swarm)
                    .bitswap
                    .as_ref()
                    .map(|bitswap| bitswap.peers())
                    .unwrap_or_default();
                let _ = ret.send(Ok(futures::future::ready(peers).boxed()));
            }
            IpfsEvent::BitswapStats(ret) => {
                let stats = self
                    .custom_behaviour(swarm)
                    .bitswap
                    .as_ref()
                    .map(|bitswap| bitswap.stats())
                    .unwrap_or_default();
                let _ = ret.send(Ok(futures::future::ready(stats).boxed()));
            }
            IpfsEvent::FindPeerIdentity(peer_id, ret) => {
                let locally_known = self.custom_behaviour(swarm).peerbook.get_peer_info(peer_id);

                let (tx, rx) = oneshot::channel();

                match locally_known {
                    Some(info) => {
                        let _ = tx.send(Ok(info.clone()));
                    }
                    None => {
                        let Some(kad) = swarm.behaviour_mut().kademlia.as_mut() else {
                            let _ = ret.send(Err(anyhow!("kad protocol is disabled")));
                            return;
                        };

                        kad.get_closest_peers(peer_id);
                        self.find_peer_identify.entry(peer_id).or_default().push(tx);
                    }
                }

                let _ = ret.send(Ok(rx));
            }
            IpfsEvent::GetBootstrappers(ret) => {
                let list = Vec::from_iter(self.bootstraps.iter().cloned());
                let _ = ret.send(list);
            }
            IpfsEvent::AddBootstrapper(mut addr, ret) => {
                let Some(kad) = swarm.behaviour_mut().kademlia.as_mut() else {
                    let _ = ret.send(Err(anyhow!("kad protocol is disabled")));
                    return;
                };

                let ret_addr = addr.clone();

                if self.bootstraps.insert(addr.clone())
                    && let Some(peer_id) = addr.extract_peer_id()
                {
                    kad.add_address(&peer_id, addr.clone());
                    self.custom_behaviour(swarm).add_peer((peer_id, addr));
                    // the return value of add_address doesn't implement Debug
                    trace!(peer_id=%peer_id, "tried to add a bootstrapper");
                }
                let _ = ret.send(Ok(ret_addr));
            }
            IpfsEvent::RemoveBootstrapper(mut addr, ret) => {
                let Some(kad) = swarm.behaviour_mut().kademlia.as_mut() else {
                    let _ = ret.send(Err(anyhow!("kad protocol is disabled")));
                    return;
                };

                let result = addr.clone();

                if self.bootstraps.remove(&addr) {
                    if let Some(peer_id) = addr.extract_peer_id() {
                        let prefix: Multiaddr = addr;

                        if let Some(e) = kad.remove_address(&peer_id, &prefix) {
                            info!(peer_id=%peer_id, status=?e.status, "removed bootstrapper");
                        } else {
                            warn!(peer_id=%peer_id, "attempted to remove an unknown bootstrapper");
                        }
                    }

                    let _ = ret.send(Ok(result));
                }
            }
            IpfsEvent::ClearBootstrappers(ret) => {
                let Some(kad) = swarm.behaviour_mut().kademlia.as_mut() else {
                    let _ = ret.send(Err(anyhow!("kad protocol is disabled")));
                    return;
                };

                let removed = self.bootstraps.drain().collect::<Vec<_>>();
                let mut list = Vec::with_capacity(removed.len());

                for mut addr_with_peer_id in removed {
                    let priginal = addr_with_peer_id.clone();
                    let Some(peer_id) = addr_with_peer_id.extract_peer_id() else {
                        continue;
                    };
                    let prefix: Multiaddr = addr_with_peer_id;

                    if let Some(e) = kad.remove_address(&peer_id, &prefix) {
                        info!(peer_id=%peer_id, status=?e.status, "cleared bootstrapper");
                        list.push(priginal);
                    } else {
                        error!(peer_id=%peer_id, "attempted to clear an unknown bootstrapper");
                    }
                }

                let _ = ret.send(Ok(list));
            }
            IpfsEvent::DefaultBootstrap(ret) => {
                if !swarm.behaviour().kademlia.is_enabled() {
                    let _ = ret.send(Err(anyhow!("kad protocol is disabled")));
                    return;
                };

                let mut rets = Vec::new();
                for addr in BOOTSTRAP_NODES {
                    let mut addr = addr
                        .parse::<Multiaddr>()
                        .expect("see test bootstrap_nodes_are_multiaddr_with_peerid");
                    let original: Multiaddr = addr.clone();
                    if self.bootstraps.insert(addr.clone()) {
                        let Some(peer_id) = addr.extract_peer_id() else {
                            continue;
                        };

                        if self
                            .custom_behaviour(swarm)
                            .add_peer((peer_id, addr.clone()))
                        {
                            trace!(peer_id=%peer_id, "tried to restore a bootstrapper");
                            // report with the peerid
                            rets.push(original);
                        }

                        let kad = swarm
                            .behaviour_mut()
                            .kademlia
                            .as_mut()
                            .expect("kad enabled");

                        kad.add_address(&peer_id, addr.clone());
                    }
                }

                let _ = ret.send(Ok(rets));
            }
        }
    }

    pub(crate) fn handle_repo_event<N: NetworkBehaviour>(
        &mut self,
        custom: &mut p2p::Behaviour<N>,
        event: RepoEvent,
    ) where
        N::ToSwarm: Debug,
    {
        match event {
            RepoEvent::WantBlock(cids, peers, timeout) => {
                let Some(bs) = custom.bitswap.as_mut() else {
                    for cid in cids {
                        self.enqueue_gateway_retrieval(cid);
                    }
                    return;
                };
                bs.gets(cids, &peers, timeout);
            }
            RepoEvent::UnwantBlock(cid) => {
                // The repo subscriptions map is the source of truth: the departing waiter removed
                // itself under lock before emitting this event, so a still-present (non-empty) entry
                // means another waiter (or a fresh one) holds the cid. Cancel only when none remain.
                let still_wanted = self
                    .repo
                    .inner
                    .subscriptions
                    .lock()
                    .get(&cid)
                    .is_some_and(|waiters| !waiters.is_empty());
                if !still_wanted {
                    self.cancel_pending_retrieval(&cid);
                    if let Some(bs) = custom.bitswap.as_mut() {
                        bs.cancel(cid);
                    }
                }
            }
            RepoEvent::NewBlock(block) => {
                self.cancel_pending_retrieval(block.cid());
                let Some(bs) = custom.bitswap.as_mut() else {
                    return;
                };
                bs.notify_new_blocks([*block.cid()]);
            }
            RepoEvent::RemovedBlock(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::{FutureExt, StreamExt, task::noop_waker};
    use multihash_codetable::{Code, MultihashDigest};

    #[test]
    fn retrieval_queue_preserves_work_across_channel_backpressure() {
        let repo = Repo::new_memory();
        let mut context = IpfsContext::new(&repo);
        let (sender, mut receiver) = futures::channel::mpsc::channel(1);
        context.discovery_tx = Some(sender);

        let cids = (0_u16..300)
            .map(|value| Cid::new_v1(0x55, Code::Sha2_256.digest(&value.to_le_bytes())))
            .collect::<Vec<_>>();
        for cid in &cids {
            context.enqueue_retrieval(*cid);
            context.enqueue_retrieval(*cid);
        }

        let cancelled = cids[150];
        context.cancel_pending_retrieval(&cancelled);

        let waker = noop_waker();
        let mut task_context = TaskContext::from_waker(&waker);
        let mut received = IndexSet::new();

        for _ in 0..cids.len() {
            context.flush_retrieval_queues(&mut task_context);
            while let Some(Some(cid)) = receiver.next().now_or_never() {
                assert!(
                    received.insert(cid),
                    "queued cid was delivered more than once"
                );
            }
            if context.pending_discovery.is_empty() {
                break;
            }
        }

        context.flush_retrieval_queues(&mut task_context);
        while let Some(Some(cid)) = receiver.next().now_or_never() {
            assert!(
                received.insert(cid),
                "queued cid was delivered more than once"
            );
        }

        let expected = cids
            .into_iter()
            .filter(|cid| *cid != cancelled)
            .collect::<IndexSet<_>>();
        assert_eq!(received, expected);
        assert!(context.pending_discovery.is_empty());
    }
}
