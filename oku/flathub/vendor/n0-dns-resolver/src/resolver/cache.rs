//! TTL-based DNS cache with least-recently-used eviction.

use std::{
    borrow::Cow,
    hash::{Hash, Hasher},
    sync::{Arc, Mutex},
    time::Duration,
};

use lru::LruCache;
use n0_future::time::Instant;

use crate::{Record, RecordKind};

/// Maximum number of entries in the DNS cache.
const MAX_CACHE_ENTRIES: usize = 4096;

/// Maximum TTL for cache entries (1 day).
///
/// Prevents malicious or misconfigured servers from making entries
/// effectively permanent by returning very large TTL values.
const MAX_TTL_SECS: u32 = 86400;

/// Fallback negative-caching TTL when a negative response carries no SOA.
///
/// Negative results (NODATA, NXDOMAIN) are normally cached for the RFC 2308
/// SOA-derived TTL (see [`super::query::negative_ttl`]), capped by
/// [`NEGATIVE_TTL_MAX_SECS`]. When the authority section has no SOA to derive
/// from, this short fixed value is used instead: long enough to collapse a
/// thundering herd of concurrent lookups for the same absent name onto one
/// query, short enough that a name which becomes resolvable is picked up
/// promptly.
pub(super) const NEGATIVE_TTL_SECS: u32 = 30;

/// Upper bound on the SOA-derived negative-caching TTL (1 hour).
///
/// Caps how long an absence is trusted so a name that starts resolving is not
/// pinned as absent for the zone's full (possibly long) SOA lifetime. Matches
/// the negative-TTL cap used by unbound.
pub(super) const NEGATIVE_TTL_MAX_SECS: u32 = 3600;

/// A cached lookup outcome: records, or an authenticated absence.
#[derive(Debug, Clone)]
pub(super) enum CachedResult {
    /// Records of the queried kind were found.
    Positive(Vec<Record>),
    /// The name exists but has no records of the queried kind (NODATA).
    NoData,
    /// The name does not exist (NXDOMAIN).
    NxDomain,
}

/// Normalizes a host to its cache-key form.
///
/// The key is the host lowercased, with any single trailing dot removed.
///
/// DNS names are case-insensitive, and a fully-qualified name with a trailing dot
/// denotes the same node as the bare name. Normalizing here keeps
/// `Example.COM.` and `example.com` in one entry, matching the hosts-file
/// normalization in `Hosts::normalize`.
/// A name already in that form is borrowed rather than copied. Every cache
/// probe passes through here, ahead of the lock, and query names are
/// overwhelmingly lowercase and undotted already.
fn normalize(host: &str) -> Cow<'_, str> {
    let host = host.strip_suffix('.').unwrap_or(host);
    match host.bytes().any(|byte| byte.is_ascii_uppercase()) {
        true => Cow::Owned(host.to_ascii_lowercase()),
        false => Cow::Borrowed(host),
    }
}

/// Hashes `(host, kind)` into a u64 key for allocation-free cache lookups.
///
/// A hash collision between different (host, kind) pairs could return records
/// for the wrong entry. The kind is part of the key, so a collision can only be
/// between two distinct host+kind pairs; [`DnsCache::get`] rechecks both against
/// the stored entry and treats a mismatch as a miss. With 64-bit hashes and a
/// few-thousand-entry cache, the birthday-bound probability is ~1e-13 per
/// lookup, negligible in practice.
///
/// `host` is expected to already be [`normalize`]d so that names differing only
/// in case or a trailing dot hash to the same key.
///
/// A pre-hashed `u64` key is used rather than a `Hash`-deriving key struct on
/// purpose: the latter would allocate a `String` for the key on every `get`,
/// whereas hashing the borrowed `&str` here keeps lookups allocation-free.
fn cache_key(host: &str, kind: RecordKind) -> u64 {
    let mut hasher = std::hash::DefaultHasher::new();
    host.hash(&mut hasher);
    kind.hash(&mut hasher);
    hasher.finish()
}

/// A cache entry with TTL expiry tracking.
#[derive(Debug, Clone)]
struct CacheEntry {
    /// The host this entry is for, in [`normalize`] form.
    ///
    /// Verified against the query on lookup, together with `kind`, so that a
    /// `u64` key collision returns a miss rather than another entry's records.
    host: String,
    /// The record kind this entry is for, verified on lookup alongside `host`.
    kind: RecordKind,
    /// The answer, which may be positive, NODATA or NXDOMAIN.
    result: CachedResult,
    /// When the entry was stored, which `ttl` counts from.
    inserted_at: Instant,
    /// How long the entry stays fresh, clamped to [`MAX_TTL_SECS`] on insert.
    ttl: Duration,
}

impl CacheEntry {
    /// Returns whether the entry has outlived its TTL.
    ///
    /// An expired entry is not evicted, so [`DnsCache::get_stale`] can still
    /// serve it under serve-stale.
    fn is_expired(&self) -> bool {
        self.inserted_at.elapsed() > self.ttl
    }
}

/// Thread-safe DNS cache with LRU eviction and TTL-based expiry.
///
/// Uses pre-hashed `u64` keys, so a lookup does not allocate a key. The one
/// remaining per-hit allocation is cloning the result, which has to outlive the
/// lock guard.
///
/// Cloning shares the same underlying cache, so a resolver rebuilt on a network
/// change (see [`DnsResolver::reset`]) can carry its cache across rather
/// than starting cold while DNS is still in flux.
///
/// [`DnsResolver::reset`]: super::DnsResolver::reset
#[derive(Debug, Clone)]
pub(super) struct DnsCache {
    /// Shared by every clone, which is what lets a reset carry the cache over.
    inner: Arc<Mutex<LruCache<u64, CacheEntry>>>,
}

impl DnsCache {
    /// Creates an empty cache holding up to [`MAX_CACHE_ENTRIES`] entries.
    pub(super) fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(LruCache::new(
                std::num::NonZeroUsize::new(MAX_CACHE_ENTRIES).expect("non-zero"),
            ))),
        }
    }

    /// Looks up a cached result, returning `None` if it is absent or expired.
    ///
    /// An expired entry is left in place rather than evicted, so [`Self::get_stale`]
    /// can still serve it as a last resort; it is overwritten on the next
    /// successful insert or evicted by LRU pressure.
    pub(super) fn get(&self, host: &str, kind: RecordKind) -> Option<CachedResult> {
        let host = normalize(host);
        let key = cache_key(&host, kind);
        let mut inner = self.inner.lock().expect("poisoned");
        let entry = inner.get(&key)?;
        // Reject `u64` key collisions: only serve an exact host+kind match.
        if entry.host != *host || entry.kind != kind {
            return None;
        }
        if entry.is_expired() {
            return None;
        }
        Some(entry.result.clone())
    }

    /// Looks up an expired positive entry for serve-stale (RFC 8767).
    ///
    /// Returns the records of an entry that has expired but is still within
    /// `max_stale` of its expiry, so the resolver can answer from stale data
    /// when live resolution fails. Only positive results are served stale;
    /// a stale absence is not useful and is never returned.
    pub(super) fn get_stale(
        &self,
        host: &str,
        kind: RecordKind,
        max_stale: Duration,
    ) -> Option<Vec<Record>> {
        let host = normalize(host);
        let key = cache_key(&host, kind);
        let mut inner = self.inner.lock().expect("poisoned");
        let entry = inner.get(&key)?;
        if entry.host != host || entry.kind != kind {
            return None;
        }
        // Only expired entries within the stale window, and only positive ones.
        let age = entry.inserted_at.elapsed();
        if age <= entry.ttl || age > entry.ttl + max_stale {
            return None;
        }
        match &entry.result {
            CachedResult::Positive(records) => Some(records.clone()),
            CachedResult::NoData | CachedResult::NxDomain => None,
        }
    }

    /// Inserts a result into the cache under the given TTL.
    ///
    /// A TTL of 0 means do not cache. Positive results carry the response's TTL;
    /// negative results (NODATA, NXDOMAIN) are cached under [`NEGATIVE_TTL_SECS`]
    /// so a burst of concurrent lookups for the same absent name collapses to
    /// one network query.
    pub(super) fn insert(&self, host: &str, kind: RecordKind, result: CachedResult, ttl: u32) {
        if ttl == 0 {
            return;
        }
        let host = normalize(host);
        let entry = CacheEntry {
            host: host.to_string(),
            kind,
            result,
            inserted_at: Instant::now(),
            ttl: Duration::from_secs(ttl.min(MAX_TTL_SECS) as u64),
        };
        self.inner
            .lock()
            .expect("poisoned")
            .put(cache_key(&host, kind), entry);
    }

    /// Clears all cache entries.
    pub(super) fn clear(&self) {
        self.inner.lock().expect("poisoned").clear();
    }

    /// Inserts an entry that expired `expired_ago` ago, for serve-stale tests.
    ///
    /// Stored with `ttl`, as though inserted `ttl + expired_ago` in the past.
    #[cfg(test)]
    pub(super) fn insert_expired(
        &self,
        host: &str,
        kind: RecordKind,
        result: CachedResult,
        ttl: Duration,
        expired_ago: Duration,
    ) {
        let host = normalize(host);
        let entry = CacheEntry {
            host: host.to_string(),
            kind,
            result,
            inserted_at: Instant::now() - (ttl + expired_ago),
            ttl,
        };
        self.inner
            .lock()
            .expect("poisoned")
            .put(cache_key(&host, kind), entry);
    }

    /// Returns the stored TTL for `host`/`kind`, for tests that assert clamping.
    #[cfg(test)]
    fn stored_ttl(&self, host: &str, kind: RecordKind) -> Option<Duration> {
        let host = normalize(host);
        let mut inner = self.inner.lock().expect("poisoned");
        inner.get(&cache_key(&host, kind)).map(|entry| entry.ttl)
    }
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use super::*;

    const ADDR: Ipv4Addr = Ipv4Addr::new(10, 0, 0, 1);

    fn positive() -> CachedResult {
        CachedResult::Positive(vec![Record::A(ADDR)])
    }

    /// Asserts that `result` holds exactly one A record for `expected`.
    fn assert_single_a(result: Option<CachedResult>, expected: Ipv4Addr) {
        match result {
            Some(CachedResult::Positive(records)) => {
                assert_eq!(records, [Record::A(expected)])
            }
            other => panic!("expected a positive result for {expected}, got {other:?}"),
        }
    }

    #[test]
    fn ttl_zero_is_not_cached() {
        let cache = DnsCache::new();
        cache.insert("example.com", RecordKind::A, positive(), 0);
        assert!(cache.get("example.com", RecordKind::A).is_none());
    }

    #[test]
    fn hit_within_ttl() {
        let cache = DnsCache::new();
        cache.insert("example.com", RecordKind::A, positive(), 300);
        assert_single_a(cache.get("example.com", RecordKind::A), ADDR);
    }

    #[test]
    fn ttl_over_max_is_clamped() {
        let cache = DnsCache::new();
        cache.insert(
            "example.com",
            RecordKind::A,
            positive(),
            MAX_TTL_SECS + 1000,
        );
        assert_eq!(
            cache.stored_ttl("example.com", RecordKind::A),
            Some(Duration::from_secs(u64::from(MAX_TTL_SECS)))
        );
    }

    #[test]
    fn lookup_is_case_and_trailing_dot_insensitive() {
        let cache = DnsCache::new();
        cache.insert("example.com", RecordKind::A, positive(), 300);
        assert_single_a(cache.get("Example.COM.", RecordKind::A), ADDR);
    }

    #[test]
    fn get_stale_serves_expired_positive_within_window() {
        let cache = DnsCache::new();
        // Expired 5s ago.
        cache.insert_expired(
            "stale.example",
            RecordKind::A,
            positive(),
            Duration::from_secs(10),
            Duration::from_secs(5),
        );
        // A fresh get misses, since the entry is expired.
        assert!(cache.get("stale.example", RecordKind::A).is_none());
        // A stale get within a 60s window serves it.
        match cache.get_stale("stale.example", RecordKind::A, Duration::from_secs(60)) {
            Some(records) => match records.as_slice() {
                [Record::A(ip)] => assert_eq!(*ip, ADDR),
                other => panic!("expected one A record, got {other:?}"),
            },
            None => panic!("expected a stale positive result"),
        }
        // Outside the stale window (only 2s allowed, expired 5s ago): none.
        assert!(
            cache
                .get_stale("stale.example", RecordKind::A, Duration::from_secs(2))
                .is_none()
        );
    }

    #[test]
    fn get_stale_ignores_fresh_and_negative_entries() {
        let cache = DnsCache::new();
        // A fresh (non-expired) entry is not served stale.
        cache.insert("fresh.example", RecordKind::A, positive(), 300);
        assert!(
            cache
                .get_stale("fresh.example", RecordKind::A, Duration::from_secs(60))
                .is_none()
        );
        // A stale negative entry is never served.
        cache.insert_expired(
            "nx.example",
            RecordKind::A,
            CachedResult::NxDomain,
            Duration::from_secs(10),
            Duration::from_secs(5),
        );
        assert!(
            cache
                .get_stale("nx.example", RecordKind::A, Duration::from_secs(60))
                .is_none()
        );
    }

    #[test]
    fn negative_results_are_cached() {
        let cache = DnsCache::new();
        cache.insert(
            "nx.example",
            RecordKind::A,
            CachedResult::NxDomain,
            NEGATIVE_TTL_SECS,
        );
        assert!(matches!(
            cache.get("nx.example", RecordKind::A),
            Some(CachedResult::NxDomain)
        ));
        cache.insert(
            "nodata.example",
            RecordKind::Aaaa,
            CachedResult::NoData,
            NEGATIVE_TTL_SECS,
        );
        assert!(matches!(
            cache.get("nodata.example", RecordKind::Aaaa),
            Some(CachedResult::NoData)
        ));
    }
}
