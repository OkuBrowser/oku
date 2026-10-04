//! Shared local wantlist storage and its driver for the peer-keyed rewrite.

use std::collections::{HashMap, VecDeque};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use futures::{FutureExt, Stream};
use futures_timer::Delay;
use ipld_core::cid::Cid;
use parking_lot::RwLock;

use super::message::RequestType;

const REBROADCAST_INTERVAL: Duration = Duration::from_secs(30);
const DEFAULT_DISCOVERY_INTERVAL: Duration = Duration::from_secs(30);
const TICK: Duration = Duration::from_millis(100);

#[derive(Debug, Clone)]
pub struct WantEntry {
    pub want_type: RequestType,
    pub priority: i32,
    pub has_provider: bool,
    deadline: Option<Instant>,
    discovery_interval: Duration,
    last_discovery: Option<Instant>,
}

#[derive(Default)]
struct WantlistInner {
    wants: HashMap<Cid, WantEntry>,
}

#[derive(Clone, Default)]
pub struct Wantlist {
    inner: Arc<RwLock<WantlistInner>>,
}

impl Wantlist {
    pub fn want(&self, cid: Cid, want_type: RequestType, priority: i32, timeout: Option<Duration>) {
        let now = Instant::now();
        let deadline = timeout.map(|duration| now + duration);
        self.inner
            .write()
            .wants
            .entry(cid)
            .and_modify(|entry| {
                entry.deadline = match (entry.deadline, deadline) {
                    (Some(existing), Some(incoming)) => Some(existing.max(incoming)),
                    _ => None,
                };
            })
            .or_insert(WantEntry {
                want_type,
                priority,
                deadline,
                discovery_interval: timeout.map(|d| d / 2).unwrap_or(DEFAULT_DISCOVERY_INTERVAL),
                last_discovery: None,
                has_provider: false,
            });
    }

    pub fn cancel(&self, cid: &Cid) -> bool {
        self.inner.write().wants.remove(cid).is_some()
    }

    pub fn contains(&self, cid: &Cid) -> bool {
        self.inner.read().wants.contains_key(cid)
    }

    pub fn is_empty(&self) -> bool {
        self.inner.read().wants.is_empty()
    }

    pub fn cids(&self) -> Vec<Cid> {
        self.inner.read().wants.keys().copied().collect()
    }

    pub fn entries(&self) -> Vec<(Cid, WantEntry)> {
        self.inner
            .read()
            .wants
            .iter()
            .map(|(cid, entry)| (*cid, entry.clone()))
            .collect()
    }

    /// Record that some peer has the cid, suppressing further provider discovery for it.
    pub fn note_have(&self, cid: &Cid) {
        if let Some(entry) = self.inner.write().wants.get_mut(cid) {
            entry.has_provider = true;
        }
    }

    pub fn rearm_discovery(&self, cid: &Cid) {
        if let Some(entry) = self.inner.write().wants.get_mut(cid) {
            entry.has_provider = false;
            entry.last_discovery = None;
        }
    }

    /// Returns `(cids needing provider discovery, expired cids)`, removing the expired wants and
    /// stamping the discovery time of the returned cids.
    fn poll_due(&self) -> (Vec<Cid>, Vec<Cid>) {
        let now = Instant::now();
        let mut inner = self.inner.write();
        let mut need = Vec::new();
        let mut expired = Vec::new();
        inner.wants.retain(|cid, entry| {
            if entry.deadline.is_some_and(|deadline| now >= deadline) {
                expired.push(*cid);
                return false;
            }
            let due = entry
                .last_discovery
                .is_none_or(|last| now.duration_since(last) >= entry.discovery_interval);
            if !entry.has_provider && due {
                entry.last_discovery = Some(now);
                need.push(*cid);
            }
            true
        });
        (need, expired)
    }
}

#[derive(Debug)]
pub enum WantlistEvent {
    NeedProviders(Cid),
    Expired(Cid),
    Rebroadcast,
}

pub struct WantlistDriver {
    wantlist: Wantlist,
    tick: Delay,
    rebroadcast: Delay,
    queue: VecDeque<WantlistEvent>,
}

impl WantlistDriver {
    pub fn new(wantlist: Wantlist) -> Self {
        Self {
            wantlist,
            tick: Delay::new(TICK),
            rebroadcast: Delay::new(REBROADCAST_INTERVAL),
            queue: VecDeque::new(),
        }
    }
}

impl Stream for WantlistDriver {
    type Item = WantlistEvent;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        if let Some(event) = this.queue.pop_front() {
            return Poll::Ready(Some(event));
        }

        while let Poll::Ready(()) = this.tick.poll_unpin(cx) {
            this.tick.reset(TICK);
            let (need, expired) = this.wantlist.poll_due();
            this.queue
                .extend(expired.into_iter().map(WantlistEvent::Expired));
            this.queue
                .extend(need.into_iter().map(WantlistEvent::NeedProviders));
            if let Some(event) = this.queue.pop_front() {
                return Poll::Ready(Some(event));
            }
        }

        while let Poll::Ready(()) = this.rebroadcast.poll_unpin(cx) {
            this.rebroadcast.reset(REBROADCAST_INTERVAL);
            if !this.wantlist.is_empty() {
                return Poll::Ready(Some(WantlistEvent::Rebroadcast));
            }
        }

        Poll::Pending
    }
}
