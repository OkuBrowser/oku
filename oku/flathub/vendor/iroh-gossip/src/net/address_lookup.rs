//! An address lookup service to gather addressing info collected from gossip Join and ForwardJoin messages.

use std::{
    collections::{btree_map::Entry, BTreeMap},
    sync::{Arc, RwLock},
    time::Duration,
};

use iroh::address_lookup::{self, AddressLookup, EndpointData, EndpointInfo};
use iroh_base::EndpointId;
use n0_future::{
    boxed::BoxStream,
    stream::{self, StreamExt},
    task::AbortOnDropHandle,
    time::SystemTime,
};

pub(crate) struct RetentionOpts {
    /// How long to keep received endpoint info records alive before pruning them
    retention: Duration,
    /// How often to check for expired entries
    evict_interval: Duration,
}

impl Default for RetentionOpts {
    fn default() -> Self {
        Self {
            retention: Duration::from_secs(60 * 5),
            evict_interval: Duration::from_secs(30),
        }
    }
}

/// An address lookup service that expires endpoints after some time.
///
/// It is added to the endpoint when constructing a gossip instance, and the gossip actor
/// then adds endpoint addresses as received with Join or ForwardJoin messages.
#[derive(Debug, Clone)]
pub(crate) struct GossipAddressLookup {
    endpoints: NodeMap,
    _task_handle: Arc<AbortOnDropHandle<()>>,
}

type NodeMap = Arc<RwLock<BTreeMap<EndpointId, StoredEndpointInfo>>>;

#[derive(Debug)]
struct StoredEndpointInfo {
    data: EndpointData,
    last_updated: SystemTime,
}

impl Default for GossipAddressLookup {
    fn default() -> Self {
        Self::new()
    }
}

impl GossipAddressLookup {
    const PROVENANCE: &'static str = "gossip";

    /// Creates a new gossip address lookup instance.
    pub(crate) fn new() -> Self {
        Self::with_opts(Default::default())
    }

    pub(crate) fn with_opts(opts: RetentionOpts) -> Self {
        let endpoints: NodeMap = Default::default();
        let task = {
            let endpoints = Arc::downgrade(&endpoints);
            n0_future::task::spawn(async move {
                let mut interval = n0_future::time::interval(opts.evict_interval);
                loop {
                    interval.tick().await;
                    let Some(endpoints) = endpoints.upgrade() else {
                        break;
                    };
                    let now = SystemTime::now();
                    endpoints.write().expect("poisoned").retain(|_k, v| {
                        let age = now.duration_since(v.last_updated).unwrap_or(Duration::MAX);
                        age <= opts.retention
                    });
                }
            })
        };
        Self {
            endpoints,
            _task_handle: Arc::new(AbortOnDropHandle::new(task)),
        }
    }

    /// Augments endpoint addressing information for the given endpoint ID.
    ///
    /// The provided addressing information is combined with the existing info in the in-memory
    /// lookup.  Any new direct addresses are added to those already present while the
    /// relay URL is overwritten.
    pub(crate) fn add(&self, endpoint_info: impl Into<EndpointInfo>) {
        let last_updated = SystemTime::now();
        let EndpointInfo { endpoint_id, data } = endpoint_info.into();
        let mut guard = self.endpoints.write().expect("poisoned");
        match guard.entry(endpoint_id) {
            Entry::Occupied(mut entry) => {
                let existing = entry.get_mut();
                existing.data.add_addrs(data.addrs().cloned());
                existing.data.set_user_data(data.user_data().cloned());
                existing.last_updated = last_updated;
            }
            Entry::Vacant(entry) => {
                entry.insert(StoredEndpointInfo { data, last_updated });
            }
        }
    }
}

impl AddressLookup for GossipAddressLookup {
    fn resolve(
        &self,
        endpoint_id: EndpointId,
    ) -> Option<BoxStream<Result<address_lookup::Item, address_lookup::Error>>> {
        let guard = self.endpoints.read().expect("poisoned");
        let info = guard.get(&endpoint_id)?;
        let last_updated = info
            .last_updated
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("time drift")
            .as_micros() as u64;
        let item = address_lookup::Item::new(
            EndpointInfo::from_parts(endpoint_id, info.data.clone()),
            Self::PROVENANCE,
            Some(last_updated),
        );
        Some(stream::iter(Some(Ok(item))).boxed())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use iroh::{address_lookup::AddressLookup, EndpointAddr, SecretKey};
    use n0_future::StreamExt;
    use rand::{RngExt, SeedableRng};

    use super::{GossipAddressLookup, RetentionOpts};

    #[tokio::test]
    async fn test_retention() {
        let opts = RetentionOpts {
            evict_interval: Duration::from_millis(100),
            retention: Duration::from_millis(500),
        };
        let disco = GossipAddressLookup::with_opts(opts);

        let rng = &mut rand::rngs::ChaCha12Rng::seed_from_u64(1);
        let k1 = SecretKey::from_bytes(&rng.random());
        let a1 = EndpointAddr::new(k1.public());

        disco.add(a1);

        assert!(matches!(
            disco.resolve(k1.public()).unwrap().next().await,
            Some(Ok(_))
        ));

        tokio::time::sleep(Duration::from_millis(200)).await;

        assert!(matches!(
            disco.resolve(k1.public()).unwrap().next().await,
            Some(Ok(_))
        ));

        tokio::time::sleep(Duration::from_millis(700)).await;

        assert!(disco.resolve(k1.public()).is_none());
    }
}
