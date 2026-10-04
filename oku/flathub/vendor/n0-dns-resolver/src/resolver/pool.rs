//! A small connection pool for TCP and DNS-over-TLS queries.
//!
//! UDP is connectionless and DNS-over-HTTPS pools inside `reqwest`, so only
//! plain-TCP and DoT connections are pooled here. Reusing a connection
//! amortizes the TCP (and, for DoT, TLS) handshake across repeated queries to
//! the same nameserver.
//!
//! A connection is checked out for exclusive use (one in-flight query at a
//! time, no pipelining) and returned on success. Two things keep idle
//! connections from accumulating:
//!
//! - On checkout, connections older than [`IDLE_TIMEOUT`] are discarded rather
//!   than handed out, and at most [`MAX_IDLE_PER_KEY`] are kept per nameserver.
//! - A background task ([`prune_loop`]) sweeps every [`PRUNE_INTERVAL`] and drops
//!   idle connections that no checkout has touched, so a nameserver queried
//!   once and never again does not pin a socket open forever.
//!
//! The task is spawned lazily on the first check-in (always inside an async
//! query, hence inside a runtime). It holds only a [`Weak`] reference, so it
//! self-terminates once the pool is dropped (and dropping the pool closes its
//! idle sockets right away rather than waiting on the task).

use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex, OnceLock, Weak},
};

use n0_future::{
    task::spawn,
    time::{self, Duration, Instant},
};
use tokio::net::TcpStream;

/// Idle connections older than this are discarded instead of reused.
const IDLE_TIMEOUT: Duration = Duration::from_secs(10);
/// Maximum idle connections kept per pool key.
const MAX_IDLE_PER_KEY: usize = 2;
/// How often the background task sweeps idle connections.
const PRUNE_INTERVAL: Duration = IDLE_TIMEOUT;

/// An established DNS-over-TLS stream.
#[cfg(transport_tls)]
pub(super) type TlsStream = tokio_rustls::client::TlsStream<TcpStream>;

/// Pool key for a DoT connection: address plus TLS server name.
///
/// Keying on both means a connection is never reused for a server name it was
/// not validated against.
#[cfg(transport_tls)]
pub(super) type TlsKey = (SocketAddr, Option<String>);

/// A pooled idle connection and when it was last returned to the pool.
struct Idle<S> {
    stream: S,
    last_used: Instant,
}

impl<S> Idle<S> {
    /// Wraps a stream that has just been returned to the pool.
    fn new(stream: S) -> Self {
        Self {
            stream,
            last_used: Instant::now(),
        }
    }

    /// Returns whether the connection has sat idle past [`IDLE_TIMEOUT`].
    fn is_stale(&self) -> bool {
        self.last_used.elapsed() >= IDLE_TIMEOUT
    }
}

/// The pooled connections, shared between the [`ConnPool`] and its pruning task.
#[derive(Default)]
struct Inner {
    /// Idle plain-TCP connections, keyed by nameserver address.
    tcp: Mutex<HashMap<SocketAddr, Vec<Idle<TcpStream>>>>,
    /// Idle DoT connections, keyed by address and TLS server name.
    ///
    /// Keyed separately from `tcp` so a connection is never reused for a
    /// different server name than it was validated against.
    #[cfg(transport_tls)]
    tls: Mutex<HashMap<TlsKey, Vec<Idle<TlsStream>>>>,
}

impl Inner {
    /// Drops every idle connection past [`IDLE_TIMEOUT`] and any key left empty.
    fn prune(&self) {
        prune_map(&self.tcp);
        #[cfg(transport_tls)]
        prune_map(&self.tls);
    }
}

/// A pool of idle TCP and DoT connections, keyed by nameserver.
#[derive(Clone)]
pub(super) struct ConnPool {
    /// Shared with every clone of the pool and with the pruning task.
    inner: Arc<Inner>,
    /// Set once the idle-connection pruning task has been spawned.
    pruner_spawned: OnceLock<()>,
}

impl std::fmt::Debug for ConnPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConnPool").finish_non_exhaustive()
    }
}

impl ConnPool {
    /// Creates an empty pool. The pruning task starts on the first insert.
    pub(super) fn new() -> Self {
        Self {
            inner: Arc::new(Inner::default()),
            pruner_spawned: OnceLock::new(),
        }
    }

    /// Takes an idle TCP connection to `addr`, if a fresh one is pooled.
    pub(super) fn checkout_tcp(&self, addr: SocketAddr) -> Option<TcpStream> {
        take_fresh(self.inner.tcp.lock().expect("poisoned").get_mut(&addr)?)
    }

    /// Returns a healthy TCP connection to the pool for reuse.
    pub(super) fn checkin_tcp(&self, addr: SocketAddr, stream: TcpStream) {
        push_capped(
            self.inner
                .tcp
                .lock()
                .expect("poisoned")
                .entry(addr)
                .or_default(),
            stream,
        );
        self.ensure_pruner();
    }

    /// Takes an idle DoT connection for `key`, if a fresh one is pooled.
    #[cfg(transport_tls)]
    pub(super) fn checkout_tls(&self, key: &TlsKey) -> Option<TlsStream> {
        take_fresh(self.inner.tls.lock().expect("poisoned").get_mut(key)?)
    }

    /// Returns a healthy DoT connection to the pool for reuse.
    #[cfg(transport_tls)]
    pub(super) fn checkin_tls(&self, key: TlsKey, stream: TlsStream) {
        push_capped(
            self.inner
                .tls
                .lock()
                .expect("poisoned")
                .entry(key)
                .or_default(),
            stream,
        );
        self.ensure_pruner();
    }

    /// Spawns the pruning task on first use.
    ///
    /// Check-in always runs inside an async query, so a runtime is guaranteed
    /// to be present.
    fn ensure_pruner(&self) {
        self.pruner_spawned.get_or_init(|| {
            spawn(prune_loop(Arc::downgrade(&self.inner)));
        });
    }
}

/// Periodically drops idle connections, exiting once the pool is gone.
async fn prune_loop(inner: Weak<Inner>) {
    loop {
        time::sleep(PRUNE_INTERVAL).await;
        let Some(inner) = inner.upgrade() else {
            return;
        };
        inner.prune();
    }
}

/// Pops the most-recently-used connection still within [`IDLE_TIMEOUT`].
///
/// Any staler ones encountered on the way are discarded.
fn take_fresh<S>(idle: &mut Vec<Idle<S>>) -> Option<S> {
    while let Some(conn) = idle.pop() {
        if !conn.is_stale() {
            return Some(conn.stream);
        }
    }
    None
}

/// Returns a connection to the pool, dropping the oldest if the key is at cap.
fn push_capped<S>(idle: &mut Vec<Idle<S>>, stream: S) {
    idle.push(Idle::new(stream));
    if idle.len() > MAX_IDLE_PER_KEY {
        idle.remove(0);
    }
}

/// Drops every stale connection in `map`, then removes any key left empty.
fn prune_map<K, S>(map: &Mutex<HashMap<K, Vec<Idle<S>>>>) {
    map.lock().expect("poisoned").retain(|_, idle| {
        idle.retain(|conn| !conn.is_stale());
        !idle.is_empty()
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A TCP connection paired with its accepted server end.
    ///
    /// Holding the server end keeps the client side from being reset.
    async fn loopback_pair() -> (TcpStream, SocketAddr, TcpStream) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (client, accepted) = tokio::join!(TcpStream::connect(addr), listener.accept());
        (client.unwrap(), addr, accepted.unwrap().0)
    }

    /// Returns how many idle TCP connections `pool` is holding.
    fn tcp_idle_count(pool: &ConnPool) -> usize {
        pool.inner.tcp.lock().unwrap().values().map(Vec::len).sum()
    }

    #[tokio::test]
    async fn take_fresh_returns_mru_and_drops_stale() {
        let mut idle = vec![
            Idle {
                stream: 1u32,
                last_used: Instant::now() - IDLE_TIMEOUT * 2,
            },
            Idle::new(2u32),
        ];
        // The most-recently-used (back) fresh connection comes out first.
        assert_eq!(take_fresh(&mut idle), Some(2));
        // Only the stale one is left, and it is discarded rather than returned.
        assert_eq!(take_fresh(&mut idle), None);
        assert!(idle.is_empty());
    }

    #[tokio::test]
    async fn push_capped_drops_oldest_over_cap() {
        let mut idle = Vec::new();
        for i in 0..(MAX_IDLE_PER_KEY as u32 + 1) {
            push_capped(&mut idle, i);
        }
        assert_eq!(idle.len(), MAX_IDLE_PER_KEY);
        // The very first (oldest) connection was evicted.
        assert_eq!(idle.first().map(|c| c.stream), Some(1));
    }

    #[tokio::test]
    async fn prune_map_drops_stale_then_empty_keys() {
        let map: Mutex<HashMap<u8, Vec<Idle<u32>>>> = Mutex::new(HashMap::new());
        {
            let mut m = map.lock().unwrap();
            m.insert(
                1,
                vec![Idle {
                    stream: 10,
                    last_used: Instant::now() - IDLE_TIMEOUT * 2,
                }],
            );
            m.insert(2, vec![Idle::new(20)]);
        }
        prune_map(&map);
        let m = map.lock().unwrap();
        // Key 1 had only a stale connection, so it is gone entirely.
        assert!(!m.contains_key(&1));
        // Key 2's fresh connection survives.
        assert_eq!(m.get(&2).map(Vec::len), Some(1));
    }

    #[tokio::test]
    async fn tcp_checkout_checkin_roundtrip() {
        let (client, addr, _server) = loopback_pair().await;

        let pool = ConnPool::new();
        assert!(pool.checkout_tcp(addr).is_none());
        pool.checkin_tcp(addr, client);
        assert!(pool.checkout_tcp(addr).is_some());
        assert!(pool.checkout_tcp(addr).is_none());
    }

    #[tokio::test(start_paused = true)]
    async fn pruner_evicts_untouched_idle_connections() {
        let (client, addr, _server) = loopback_pair().await;

        let pool = ConnPool::new();
        pool.checkin_tcp(addr, client);
        assert_eq!(tcp_idle_count(&pool), 1);

        // Advance past the idle timeout so the pruning task sweeps the connection even
        // though nothing checks it out. (`start_paused` auto-advances the clock.)
        time::sleep(IDLE_TIMEOUT + PRUNE_INTERVAL).await;
        tokio::task::yield_now().await;

        assert_eq!(tcp_idle_count(&pool), 0);
    }
}
