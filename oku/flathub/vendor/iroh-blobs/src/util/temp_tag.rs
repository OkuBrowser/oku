#![allow(dead_code)]
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, Weak},
};

use serde::{Deserialize, Serialize};
use tracing::{trace, warn};

use crate::{api::proto::Scope, BlobFormat, Hash, HashAndFormat};

/// An ephemeral, in-memory tag that protects content while the process is running.
///
/// If format is raw, this will protect just the blob
/// If format is collection, this will protect the collection and all blobs in it
#[derive(Debug, Serialize, Deserialize)]
#[must_use = "TempTag is a temporary tag that should be used to protect content while the process is running. \
       If you want to keep the content alive, use TempTag::leak()"]
pub struct TempTag {
    /// The hash and format we are pinning
    inner: HashAndFormat,
    /// optional callback to call on drop
    #[serde(skip)]
    on_drop: Option<Weak<dyn TagDrop>>,
}

impl AsRef<Hash> for TempTag {
    fn as_ref(&self) -> &Hash {
        &self.inner.hash
    }
}

/// A trait for things that can track liveness of blobs and collections.
///
/// This trait works together with [TempTag] to keep track of the liveness of a
/// blob or collection.
///
/// It is important to include the format in the liveness tracking, since
/// protecting a collection means protecting the blob and all its children,
/// whereas protecting a raw blob only protects the blob itself.
pub trait TagCounter: TagDrop + Sized {
    /// Called on creation of a temp tag
    fn on_create(&self, inner: &HashAndFormat);

    /// Get this as a weak reference for use in temp tags
    fn as_weak(self: &Arc<Self>) -> Weak<dyn TagDrop> {
        let on_drop: Arc<dyn TagDrop> = self.clone();
        Arc::downgrade(&on_drop)
    }

    /// Create a new temp tag for the given hash and format
    fn temp_tag(self: &Arc<Self>, inner: HashAndFormat) -> TempTag {
        self.on_create(&inner);
        TempTag::new(inner, Some(self.as_weak()))
    }
}

/// Trait used from temp tags to notify an abstract store that a temp tag is
/// being dropped.
pub trait TagDrop: std::fmt::Debug + Send + Sync + 'static {
    /// Called on drop
    fn on_drop(&self, inner: &HashAndFormat);
}

impl From<&TempTag> for HashAndFormat {
    fn from(val: &TempTag) -> Self {
        val.inner
    }
}

impl From<TempTag> for HashAndFormat {
    fn from(val: TempTag) -> Self {
        val.inner
    }
}

impl TempTag {
    /// Create a new temp tag for the given hash and format
    ///
    /// This should only be used by store implementations.
    ///
    /// The caller is responsible for increasing the refcount on creation and to
    /// make sure that temp tags that are created between a mark phase and a sweep
    /// phase are protected.
    pub fn new(inner: HashAndFormat, on_drop: Option<Weak<dyn TagDrop>>) -> Self {
        Self { inner, on_drop }
    }

    /// The empty temp tag. We don't track the empty blob since we always have it.
    pub fn leaking_empty(format: BlobFormat) -> Self {
        Self {
            inner: HashAndFormat {
                hash: Hash::EMPTY,
                format,
            },
            on_drop: None,
        }
    }

    /// The hash of the pinned item
    pub fn hash(&self) -> Hash {
        self.inner.hash
    }

    /// The format of the pinned item
    pub fn format(&self) -> BlobFormat {
        self.inner.format
    }

    /// The hash and format of the pinned item
    pub fn hash_and_format(&self) -> HashAndFormat {
        self.inner
    }

    /// Keep the item alive until the end of the process
    pub fn leak(&mut self) {
        // set the liveness tracker to None, so that the refcount is not decreased
        // during drop. This means that the refcount will never reach 0 and the
        // item will not be gced until the end of the process.
        self.on_drop = None;
    }
}

impl Drop for TempTag {
    fn drop(&mut self) {
        if let Some(on_drop) = self.on_drop.take() {
            if let Some(on_drop) = on_drop.upgrade() {
                on_drop.on_drop(&self.inner);
            }
        }
    }
}

#[derive(Debug, Default, Clone)]
struct TempCounters {
    /// number of raw temp tags for a hash
    raw: u64,
    /// number of hash seq temp tags for a hash
    hash_seq: u64,
}

impl TempCounters {
    fn counter(&self, format: BlobFormat) -> u64 {
        match format {
            BlobFormat::Raw => self.raw,
            BlobFormat::HashSeq => self.hash_seq,
        }
    }

    fn counter_mut(&mut self, format: BlobFormat) -> &mut u64 {
        match format {
            BlobFormat::Raw => &mut self.raw,
            BlobFormat::HashSeq => &mut self.hash_seq,
        }
    }

    fn inc(&mut self, format: BlobFormat) {
        let counter = self.counter_mut(format);
        *counter = counter.checked_add(1).unwrap();
    }

    fn dec(&mut self, format: BlobFormat) {
        let counter = self.counter_mut(format);
        *counter = counter.saturating_sub(1);
    }

    fn is_empty(&self) -> bool {
        self.raw == 0 && self.hash_seq == 0
    }
}

#[derive(Debug, Default)]
pub(crate) struct TempTags {
    scopes: HashMap<Scope, Arc<TempTagScope>>,
    next_scope: u64,
}

impl TempTags {
    pub fn create_scope(&mut self) -> (Scope, Arc<TempTagScope>) {
        self.next_scope += 1;
        let id = Scope(self.next_scope);
        let scope = self.scopes.entry(id).or_default();
        (id, scope.clone())
    }

    pub fn end_scope(&mut self, scope: Scope) {
        self.scopes.remove(&scope);
    }

    pub fn list(&self) -> Vec<HashAndFormat> {
        self.scopes
            .values()
            .flat_map(|scope| scope.list())
            .collect()
    }

    pub fn create(&mut self, scope: Scope, content: HashAndFormat) -> TempTag {
        let scope = self.scopes.entry(scope).or_default();

        scope.temp_tag(content)
    }

    pub fn contains(&self, hash: Hash) -> bool {
        self.scopes
            .values()
            .any(|scope| scope.0.lock().unwrap().contains(&HashAndFormat::raw(hash)))
    }
}

#[derive(Debug, Default)]
pub(crate) struct TempTagScope(Mutex<TempCounterMap>);

impl TempTagScope {
    pub fn list(&self) -> impl Iterator<Item = HashAndFormat> + 'static {
        let guard = self.0.lock().unwrap();
        let res = guard.keys();
        drop(guard);
        res.into_iter()
    }
}

impl TagDrop for TempTagScope {
    fn on_drop(&self, inner: &HashAndFormat) {
        trace!("Dropping temp tag {:?}", inner);
        self.0.lock().unwrap().dec(inner);
    }
}

impl TagCounter for TempTagScope {
    fn on_create(&self, inner: &HashAndFormat) {
        trace!("Creating temp tag {:?}", inner);
        self.0.lock().unwrap().inc(*inner);
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct TempCounterMap(HashMap<Hash, TempCounters>);

impl TempCounterMap {
    pub fn inc(&mut self, value: HashAndFormat) {
        self.0.entry(value.hash).or_default().inc(value.format)
    }

    pub fn dec(&mut self, value: &HashAndFormat) {
        let HashAndFormat { hash, format } = value;
        let Some(counters) = self.0.get_mut(hash) else {
            warn!("Decrementing non-existent temp tag");
            return;
        };
        counters.dec(*format);
        if counters.is_empty() {
            self.0.remove(hash);
        }
    }

    pub fn contains(&self, haf: &HashAndFormat) -> bool {
        let Some(entry) = self.0.get(&haf.hash) else {
            return false;
        };
        entry.counter(haf.format) > 0
    }

    pub fn keys(&self) -> Vec<HashAndFormat> {
        let mut res = Vec::new();
        for (k, v) in self.0.iter() {
            if v.raw > 0 {
                res.push(HashAndFormat::raw(*k));
            }
            if v.hash_seq > 0 {
                res.push(HashAndFormat::hash_seq(*k));
            }
        }
        res
    }
}
