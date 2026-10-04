//! A simple iroh connection pool
//!
//! Entry point is [`ConnectionPool`]. You create a connection pool for a specific
//! ALPN and [`Options`]. Then the pool will manage connections for you.
//!
//! Access to connections is via the [`ConnectionPool::get_or_connect`] method, which
//! gives you access to a connection via a [`ConnectionRef`] if possible.
//!
//! It is important that you keep the [`ConnectionRef`] alive while you are using
//! the connection.
use std::{
    collections::{HashMap, VecDeque},
    io,
    ops::Deref,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use iroh::{
    Endpoint, EndpointId,
    endpoint::{ConnectError, Connection},
};
use n0_error::{e, stack_error};
use n0_future::{
    FuturesUnordered, MaybeFuture, Stream, StreamExt,
    future::{self},
    time::Duration,
};
use tokio::sync::{
    Notify,
    mpsc::{self, error::SendError as TokioSendError},
    oneshot,
};
use tracing::{debug, error, info, trace};

pub type OnConnected =
    Arc<dyn Fn(&Endpoint, &Connection) -> n0_future::future::Boxed<io::Result<()>> + Send + Sync>;

/// Configuration options for the connection pool
#[derive(derive_more::Debug, Clone)]
pub struct Options {
    /// How long to keep idle connections around.
    pub idle_timeout: Duration,
    /// Timeout for connect. This includes the time spent in on_connect, if set.
    pub connect_timeout: Duration,
    /// Maximum number of connections to hand out.
    pub max_connections: usize,
    /// An optional callback that can be used to wait for the connection to enter some state.
    /// An example usage could be to wait for the connection to become direct before handing
    /// it out to the user.
    #[debug(skip)]
    pub on_connected: Option<OnConnected>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            idle_timeout: Duration::from_secs(5),
            connect_timeout: Duration::from_secs(1),
            max_connections: 1024,
            on_connected: None,
        }
    }
}

impl Options {
    /// Set the on_connected callback
    pub fn with_on_connected<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn(Endpoint, Connection) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = io::Result<()>> + Send + 'static,
    {
        self.on_connected = Some(Arc::new(move |ep, conn| {
            let ep = ep.clone();
            let conn = conn.clone();
            Box::pin(f(ep, conn))
        }));
        self
    }
}

/// A reference to a connection that is owned by a connection pool.
#[derive(Debug)]
pub struct ConnectionRef {
    connection: iroh::endpoint::Connection,
    _permit: OneConnection,
}

impl Deref for ConnectionRef {
    type Target = iroh::endpoint::Connection;

    fn deref(&self) -> &Self::Target {
        &self.connection
    }
}

impl ConnectionRef {
    fn new(connection: iroh::endpoint::Connection, counter: OneConnection) -> Self {
        Self {
            connection,
            _permit: counter,
        }
    }
}

/// Error when a connection can not be acquired
///
/// This includes the normal iroh connection errors as well as pool specific
/// errors such as timeouts and connection limits.
#[stack_error(derive, add_meta)]
#[derive(Clone)]
pub enum PoolConnectError {
    /// Connection pool is shut down
    #[error("Connection pool is shut down")]
    Shutdown {},
    /// Timeout during connect
    #[error("Timeout during connect")]
    Timeout {},
    /// Too many connections
    #[error("Too many connections")]
    TooManyConnections {},
    /// Error during connect
    #[error(transparent)]
    ConnectError { source: Arc<ConnectError> },
    /// Error during on_connect callback
    #[error(transparent)]
    OnConnectError {
        #[error(std_err)]
        source: Arc<io::Error>,
    },
}

impl From<ConnectError> for PoolConnectError {
    fn from(e: ConnectError) -> Self {
        e!(PoolConnectError::ConnectError, Arc::new(e))
    }
}

impl From<io::Error> for PoolConnectError {
    fn from(e: io::Error) -> Self {
        e!(PoolConnectError::OnConnectError, Arc::new(e))
    }
}

/// Error when calling a fn on the [`ConnectionPool`].
///
/// The only thing that can go wrong is that the connection pool is shut down.
#[stack_error(derive, add_meta)]
pub enum ConnectionPoolError {
    /// The connection pool has been shut down
    #[error("The connection pool has been shut down")]
    Shutdown {},
}

enum ActorMessage {
    RequestRef(RequestRef),
    ConnectionIdle { id: EndpointId },
    ConnectionShutdown { id: EndpointId },
}

struct RequestRef {
    id: EndpointId,
    tx: oneshot::Sender<Result<ConnectionRef, PoolConnectError>>,
}

struct Context {
    options: Options,
    endpoint: Endpoint,
    owner: ConnectionPool,
    alpn: Vec<u8>,
}

impl Context {
    async fn run_connection_actor(
        self: Arc<Self>,
        node_id: EndpointId,
        mut rx: mpsc::Receiver<RequestRef>,
    ) {
        let context = self;

        let conn_fut = {
            let context = context.clone();
            async move {
                let conn = context
                    .endpoint
                    .connect(node_id, &context.alpn)
                    .await
                    .map_err(PoolConnectError::from)?;
                if let Some(on_connect) = &context.options.on_connected {
                    on_connect(&context.endpoint, &conn)
                        .await
                        .map_err(PoolConnectError::from)?;
                }
                Result::<Connection, PoolConnectError>::Ok(conn)
            }
        };

        // Connect to the node
        let state = n0_future::time::timeout(context.options.connect_timeout, conn_fut)
            .await
            .map_err(|_| e!(PoolConnectError::Timeout))
            .and_then(|r| r);
        let conn_close = match &state {
            Ok(conn) => {
                let conn = conn.clone();
                MaybeFuture::Some(async move { conn.closed().await })
            }
            Err(e) => {
                debug!(%node_id, "Failed to connect {e:?}, requesting shutdown");
                if context.owner.close(node_id).await.is_err() {
                    return;
                }
                MaybeFuture::None
            }
        };

        let counter = ConnectionCounter::new();
        let idle_timer = MaybeFuture::default();
        let idle_stream = counter.clone().idle_stream();

        tokio::pin!(idle_timer, idle_stream, conn_close);

        loop {
            tokio::select! {
                biased;

                // Handle new work
                handler = rx.recv() => {
                    match handler {
                        Some(RequestRef { id, tx }) => {
                            assert!(id == node_id, "Not for me!");
                            match &state {
                                Ok(state) => {
                                    let res = ConnectionRef::new(state.clone(), counter.get_one());
                                    info!(%node_id, "Handing out ConnectionRef {}", counter.current());

                                    // clear the idle timer
                                    idle_timer.as_mut().set_none();
                                    tx.send(Ok(res)).ok();
                                }
                                Err(cause) => {
                                    tx.send(Err(cause.clone())).ok();
                                }
                            }
                        }
                        None => {
                            // Channel closed - exit
                            break;
                        }
                    }
                }

                _ = &mut conn_close => {
                    // connection was closed by somebody, notify owner that we should be removed
                    context.owner.close(node_id).await.ok();
                }

                _ = idle_stream.next() => {
                    if !counter.is_idle() {
                        continue;
                    };
                    // notify the pool that we are idle.
                    trace!(%node_id, "Idle");
                    if context.owner.idle(node_id).await.is_err() {
                        // If we can't notify the pool, we are shutting down
                        break;
                    }
                    // set the idle timer
                    idle_timer.as_mut().set_future(n0_future::time::sleep(context.options.idle_timeout));
                }

                // Idle timeout - request shutdown
                _ = &mut idle_timer => {
                    trace!(%node_id, "Idle timer expired, requesting shutdown");
                    context.owner.close(node_id).await.ok();
                    // Don't break here - wait for main actor to close our channel
                }
            }
        }

        if let Ok(connection) = state {
            let reason = if counter.is_idle() { b"idle" } else { b"drop" };
            connection.close(0u32.into(), reason);
        }

        trace!(%node_id, "Connection actor shutting down");
    }
}

struct Actor {
    rx: mpsc::Receiver<ActorMessage>,
    connections: HashMap<EndpointId, mpsc::Sender<RequestRef>>,
    context: Arc<Context>,
    // idle set (most recent last)
    // todo: use a better data structure if this becomes a performance issue
    idle: VecDeque<EndpointId>,
    // per connection tasks
    tasks: FuturesUnordered<future::Boxed<()>>,
}

impl Actor {
    pub fn new(
        endpoint: Endpoint,
        alpn: &[u8],
        options: Options,
    ) -> (Self, mpsc::Sender<ActorMessage>) {
        let (tx, rx) = mpsc::channel(100);
        (
            Self {
                rx,
                connections: HashMap::new(),
                idle: VecDeque::new(),
                context: Arc::new(Context {
                    options,
                    alpn: alpn.to_vec(),
                    endpoint,
                    owner: ConnectionPool { tx: tx.clone() },
                }),
                tasks: FuturesUnordered::new(),
            },
            tx,
        )
    }

    fn add_idle(&mut self, id: EndpointId) {
        self.remove_idle(id);
        self.idle.push_back(id);
    }

    fn remove_idle(&mut self, id: EndpointId) {
        self.idle.retain(|&x| x != id);
    }

    fn pop_oldest_idle(&mut self) -> Option<EndpointId> {
        self.idle.pop_front()
    }

    fn remove_connection(&mut self, id: EndpointId) {
        self.connections.remove(&id);
        self.remove_idle(id);
    }

    async fn handle_msg(&mut self, msg: ActorMessage) {
        match msg {
            ActorMessage::RequestRef(mut msg) => {
                let id = msg.id;
                self.remove_idle(id);
                // Try to send to existing connection actor
                if let Some(conn_tx) = self.connections.get(&id) {
                    if let Err(TokioSendError(e)) = conn_tx.send(msg).await {
                        msg = e;
                    } else {
                        return;
                    }
                    // Connection actor died, remove it
                    self.remove_connection(id);
                }

                // No connection actor or it died - check limits
                if self.connections.len() >= self.context.options.max_connections {
                    if let Some(idle) = self.pop_oldest_idle() {
                        // remove the oldest idle connection to make room for one more
                        trace!("removing oldest idle connection {}", idle);
                        self.connections.remove(&idle);
                    } else {
                        msg.tx
                            .send(Err(e!(PoolConnectError::TooManyConnections)))
                            .ok();
                        return;
                    }
                }
                let (conn_tx, conn_rx) = mpsc::channel(100);
                self.connections.insert(id, conn_tx.clone());

                let context = self.context.clone();

                self.tasks
                    .push(Box::pin(context.run_connection_actor(id, conn_rx)));

                // Send the handler to the new actor
                if conn_tx.send(msg).await.is_err() {
                    error!(%id, "Failed to send handler to new connection actor");
                    self.connections.remove(&id);
                }
            }
            ActorMessage::ConnectionIdle { id } => {
                self.add_idle(id);
                trace!(%id, "connection idle");
            }
            ActorMessage::ConnectionShutdown { id } => {
                // Remove the connection from our map - this closes the channel
                self.remove_connection(id);
                trace!(%id, "removed connection");
            }
        }
    }

    pub async fn run(mut self) {
        loop {
            tokio::select! {
                biased;

                msg = self.rx.recv() => {
                    if let Some(msg) = msg {
                        self.handle_msg(msg).await;
                    } else {
                        break;
                    }
                }

                _ = self.tasks.next(), if !self.tasks.is_empty() => {}
            }
        }
    }
}

/// A connection pool
#[derive(Debug, Clone)]
pub struct ConnectionPool {
    tx: mpsc::Sender<ActorMessage>,
}

impl ConnectionPool {
    pub fn new(endpoint: Endpoint, alpn: &[u8], options: Options) -> Self {
        let (actor, tx) = Actor::new(endpoint, alpn, options);

        // Spawn the main actor
        n0_future::task::spawn(actor.run());

        Self { tx }
    }

    /// Returns either a fresh connection or a reference to an existing one.
    ///
    /// This is guaranteed to return after approximately [Options::connect_timeout]
    /// with either an error or a connection.
    pub async fn get_or_connect(
        &self,
        id: EndpointId,
    ) -> std::result::Result<ConnectionRef, PoolConnectError> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(ActorMessage::RequestRef(RequestRef { id, tx }))
            .await
            .map_err(|_| e!(PoolConnectError::Shutdown))?;
        rx.await.map_err(|_| e!(PoolConnectError::Shutdown))?
    }

    /// Close an existing connection, if it exists
    ///
    /// This will finish pending tasks and close the connection. New tasks will
    /// get a new connection if they are submitted after this call
    pub async fn close(&self, id: EndpointId) -> std::result::Result<(), ConnectionPoolError> {
        self.tx
            .send(ActorMessage::ConnectionShutdown { id })
            .await
            .map_err(|_| e!(ConnectionPoolError::Shutdown))?;
        Ok(())
    }

    /// Notify the connection pool that a connection is idle.
    ///
    /// Should only be called from connection handlers.
    pub(crate) async fn idle(
        &self,
        id: EndpointId,
    ) -> std::result::Result<(), ConnectionPoolError> {
        self.tx
            .send(ActorMessage::ConnectionIdle { id })
            .await
            .map_err(|_| e!(ConnectionPoolError::Shutdown))?;
        Ok(())
    }
}

#[derive(Debug)]
struct ConnectionCounterInner {
    count: AtomicUsize,
    notify: Notify,
}

#[derive(Debug, Clone)]
struct ConnectionCounter {
    inner: Arc<ConnectionCounterInner>,
}

impl ConnectionCounter {
    fn new() -> Self {
        Self {
            inner: Arc::new(ConnectionCounterInner {
                count: Default::default(),
                notify: Notify::new(),
            }),
        }
    }

    fn current(&self) -> usize {
        self.inner.count.load(Ordering::SeqCst)
    }

    /// Increase the connection count and return a guard for the new connection
    fn get_one(&self) -> OneConnection {
        self.inner.count.fetch_add(1, Ordering::SeqCst);
        OneConnection {
            inner: self.inner.clone(),
        }
    }

    fn is_idle(&self) -> bool {
        self.inner.count.load(Ordering::SeqCst) == 0
    }

    /// Infinite stream that yields when the connection is briefly idle.
    ///
    /// Note that you still have to check if the connection is still idle when
    /// you get the notification.
    ///
    /// Also note that this stream is triggered on [OneConnection::drop], so it
    /// won't trigger initially even though a [ConnectionCounter] starts up as
    /// idle.
    fn idle_stream(self) -> impl Stream<Item = ()> {
        n0_future::stream::unfold(self, |c| async move {
            c.inner.notify.notified().await;
            Some(((), c))
        })
    }
}

/// Guard for one connection
#[derive(Debug)]
struct OneConnection {
    inner: Arc<ConnectionCounterInner>,
}

impl Drop for OneConnection {
    fn drop(&mut self) {
        if self.inner.count.fetch_sub(1, Ordering::SeqCst) == 1 {
            self.inner.notify.notify_waiters();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, sync::Arc, time::Duration};

    use iroh::{
        EndpointAddr, EndpointId, RelayMode, SecretKey, TransportAddr,
        address_lookup::MemoryLookup,
        endpoint::{Connection, presets},
        protocol::{AcceptError, ProtocolHandler, Router},
    };
    use n0_error::{AnyError, Result, StdResultExt};
    use n0_future::{BufferedStreamExt, StreamExt, io, stream};
    use testresult::TestResult;
    use tracing::trace;

    use super::{ConnectionPool, OnConnected, Options, PoolConnectError};

    const ECHO_ALPN: &[u8] = b"echo";

    #[derive(Debug, Clone)]
    struct Echo;

    impl ProtocolHandler for Echo {
        async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
            let conn_id = connection.stable_id();
            let id = connection.remote_id();
            trace!(%id, %conn_id, "Accepting echo connection");
            loop {
                match connection.accept_bi().await {
                    Ok((mut send, mut recv)) => {
                        trace!(%id, %conn_id, "Accepted echo request");
                        tokio::io::copy(&mut recv, &mut send).await?;
                        send.finish().map_err(AcceptError::from_err)?;
                    }
                    Err(e) => {
                        trace!(%id, %conn_id, "Failed to accept echo request {e}");
                        break;
                    }
                }
            }
            Ok(())
        }
    }

    async fn echo_client(conn: &Connection, text: &[u8]) -> Result<Vec<u8>> {
        let conn_id = conn.stable_id();
        let id = conn.remote_id();
        trace!(%id, %conn_id, "Sending echo request");
        let (mut send, mut recv) = conn.open_bi().await.anyerr()?;
        send.write_all(text).await.anyerr()?;
        send.finish().anyerr()?;
        let response = recv.read_to_end(1000).await.anyerr()?;
        trace!(%id, %conn_id, "Received echo response");
        Ok(response)
    }

    async fn echo_server() -> TestResult<(EndpointAddr, Router)> {
        let endpoint = iroh::Endpoint::builder(presets::N0)
            .alpns(vec![ECHO_ALPN.to_vec()])
            .bind()
            .await?;
        endpoint.online().await;
        let addr = endpoint.addr();
        let router = iroh::protocol::Router::builder(endpoint)
            .accept(ECHO_ALPN, Echo)
            .spawn();

        Ok((addr, router))
    }

    async fn echo_servers(n: usize) -> TestResult<(Vec<EndpointId>, Vec<Router>, MemoryLookup)> {
        let res = stream::iter(0..n)
            .map(|_| echo_server())
            .buffered_unordered(16)
            .collect::<Vec<_>>()
            .await;
        let res: Vec<(EndpointAddr, Router)> = res.into_iter().collect::<TestResult<Vec<_>>>()?;
        let (addrs, routers): (Vec<_>, Vec<_>) = res.into_iter().unzip();
        let ids = addrs.iter().map(|a| a.id).collect::<Vec<_>>();
        let address_lookup = MemoryLookup::from_endpoint_info(addrs);
        Ok((ids, routers, address_lookup))
    }

    async fn shutdown_routers(routers: Vec<Router>) {
        stream::iter(routers)
            .for_each_concurrent(16, |router| async move {
                let _ = router.shutdown().await;
            })
            .await;
    }

    fn test_options() -> Options {
        Options {
            idle_timeout: Duration::from_millis(100),
            connect_timeout: Duration::from_secs(5),
            max_connections: 32,
            on_connected: None,
        }
    }

    struct EchoClient {
        pool: ConnectionPool,
    }

    impl EchoClient {
        async fn echo(
            &self,
            id: EndpointId,
            text: Vec<u8>,
        ) -> Result<Result<(usize, Vec<u8>), AnyError>, PoolConnectError> {
            let conn = self.pool.get_or_connect(id).await?;
            let id = conn.stable_id();
            match echo_client(&conn, &text).await {
                Ok(res) => Ok(Ok((id, res))),
                Err(e) => Ok(Err(e)),
            }
        }
    }

    #[tokio::test]
    // #[traced_test]
    async fn connection_pool_errors() -> TestResult<()> {
        // set up static address lookup for all addrs
        let address_lookup = MemoryLookup::new();
        let endpoint = iroh::Endpoint::builder(presets::Minimal)
            .relay_mode(RelayMode::Default)
            .address_lookup(address_lookup.clone())
            .bind()
            .await?;
        let pool = ConnectionPool::new(endpoint.clone(), ECHO_ALPN, test_options());
        let client = EchoClient { pool };
        {
            let non_existing = SecretKey::from_bytes(&[0; 32]).public();
            let res = client.echo(non_existing, b"Hello, world!".to_vec()).await;
            // trying to connect to a non-existing id will fail with ConnectError
            // because we don't have any information about the endpoint.
            assert!(matches!(res, Err(PoolConnectError::ConnectError { .. })));
        }
        {
            let non_listening = SecretKey::from_bytes(&[0; 32]).public();
            // make up fake node info
            address_lookup.add_endpoint_info(EndpointAddr {
                id: non_listening,
                addrs: vec![TransportAddr::Ip("127.0.0.1:12121".parse().unwrap())]
                    .into_iter()
                    .collect(),
            });
            // trying to connect to an id for which we have info, but the other
            // end is not listening, will lead to a timeout.
            let res = client.echo(non_listening, b"Hello, world!".to_vec()).await;
            assert!(matches!(res, Err(PoolConnectError::Timeout { .. })));
        }
        endpoint.close().await;
        Ok(())
    }

    #[tokio::test]
    // #[traced_test]
    async fn connection_pool_smoke() -> TestResult<()> {
        let n = 32;
        let (ids, routers, address_lookup) = echo_servers(n).await?;
        // build a client endpoint that can resolve all the endpoint ids
        let endpoint = iroh::Endpoint::builder(presets::Minimal)
            .relay_mode(RelayMode::Default)
            .address_lookup(address_lookup.clone())
            .bind()
            .await?;
        let pool = ConnectionPool::new(endpoint.clone(), ECHO_ALPN, test_options());
        let client = EchoClient { pool };
        let mut connection_ids = BTreeMap::new();
        let msg = b"Hello, pool!".to_vec();
        for id in &ids {
            let (cid1, res) = client.echo(*id, msg.clone()).await??;
            assert_eq!(res, msg);
            let (cid2, res) = client.echo(*id, msg.clone()).await??;
            assert_eq!(res, msg);
            assert_eq!(cid1, cid2);
            connection_ids.insert(id, cid1);
        }
        n0_future::time::sleep(Duration::from_millis(1000)).await;
        for id in &ids {
            let cid1 = *connection_ids.get(id).expect("Connection ID not found");
            let (cid2, res) = client.echo(*id, msg.clone()).await??;
            assert_eq!(res, msg);
            assert_ne!(cid1, cid2);
        }
        shutdown_routers(routers).await;
        endpoint.close().await;
        Ok(())
    }

    /// Tests that idle connections are being reclaimed to make room if we hit the
    /// maximum connection limit.
    #[tokio::test]
    // #[traced_test]
    async fn connection_pool_idle() -> TestResult<()> {
        let n = 32;
        let (ids, routers, address_lookup) = echo_servers(n).await?;
        // build a client endpoint that can resolve all the endpoint ids
        let endpoint = iroh::Endpoint::builder(presets::Minimal)
            .relay_mode(RelayMode::Default)
            .address_lookup(address_lookup.clone())
            .bind()
            .await?;
        let pool = ConnectionPool::new(
            endpoint.clone(),
            ECHO_ALPN,
            Options {
                idle_timeout: Duration::from_secs(100),
                max_connections: 8,
                ..test_options()
            },
        );
        let client = EchoClient { pool };
        let msg = b"Hello, pool!".to_vec();
        for id in &ids {
            let (_, res) = client.echo(*id, msg.clone()).await??;
            assert_eq!(res, msg);
        }
        shutdown_routers(routers).await;
        endpoint.close().await;
        Ok(())
    }

    /// Uses an on_connected callback that just errors out every time.
    ///
    /// This is a basic smoke test that on_connected gets called at all.
    #[tokio::test]
    // #[traced_test]
    async fn on_connected_error() -> TestResult<()> {
        let n = 1;
        let (ids, routers, address_lookup) = echo_servers(n).await?;
        let endpoint = iroh::Endpoint::builder(presets::Minimal)
            .relay_mode(RelayMode::Default)
            .address_lookup(address_lookup)
            .bind()
            .await?;
        let on_connected: OnConnected =
            Arc::new(|_, _| Box::pin(async { Err(io::Error::other("on_connect failed")) }));
        let pool = ConnectionPool::new(
            endpoint.clone(),
            ECHO_ALPN,
            Options {
                on_connected: Some(on_connected),
                ..test_options()
            },
        );
        let client = EchoClient { pool };
        let msg = b"Hello, pool!".to_vec();
        for id in &ids {
            let res = client.echo(*id, msg.clone()).await;
            assert!(matches!(res, Err(PoolConnectError::OnConnectError { .. })));
        }
        shutdown_routers(routers).await;
        endpoint.close().await;
        Ok(())
    }

    /// Uses an on_connected callback to ensure that the connection is direct.
    #[tokio::test]
    // #[traced_test]
    async fn on_connected_direct() -> TestResult<()> {
        let n = 1;
        let (ids, routers, address_lookup) = echo_servers(n).await?;
        let endpoint = iroh::Endpoint::builder(presets::Minimal)
            .relay_mode(RelayMode::Default)
            .address_lookup(address_lookup)
            .bind()
            .await?;
        let on_connected = |_, conn: Connection| async move {
            let mut stream = conn.paths_stream();
            while let Some(paths) = stream.next().await {
                if paths.iter().any(|path| path.is_ip()) {
                    return Ok(());
                }
            }
            Err(io::Error::other("connection closed before becoming direct"))
        };
        let pool = ConnectionPool::new(
            endpoint.clone(),
            ECHO_ALPN,
            test_options().with_on_connected(on_connected),
        );
        let client = EchoClient { pool };
        let msg = b"Hello, pool!".to_vec();
        for id in &ids {
            let res = client.echo(*id, msg.clone()).await;
            assert!(res.is_ok());
        }
        shutdown_routers(routers).await;
        endpoint.close().await;
        Ok(())
    }

    /// Check that when a connection is closed, the pool will give you a new
    /// connection next time you want one.
    ///
    /// This test fails if the connection watch is disabled.
    #[tokio::test]
    // #[traced_test]
    async fn watch_close() -> TestResult<()> {
        let n = 1;
        let (ids, routers, address_lookup) = echo_servers(n).await?;
        let endpoint = iroh::Endpoint::builder(presets::Minimal)
            .relay_mode(RelayMode::Default)
            .address_lookup(address_lookup)
            .bind()
            .await?;

        let pool = ConnectionPool::new(endpoint.clone(), ECHO_ALPN, test_options());
        let conn = pool.get_or_connect(ids[0]).await?;
        let cid1 = conn.stable_id();
        conn.close(0u32.into(), b"test");
        n0_future::time::sleep(Duration::from_millis(500)).await;
        let conn = pool.get_or_connect(ids[0]).await?;
        let cid2 = conn.stable_id();
        assert_ne!(cid1, cid2);
        shutdown_routers(routers).await;
        endpoint.close().await;
        Ok(())
    }
}
