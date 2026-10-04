use std::{fmt::Debug, io, ops::Deref};

use iroh::endpoint::VarInt;
use irpc::{
    channel::{mpsc, none::NoSender, oneshot},
    rpc_requests, Channels, WithChannels,
};
use n0_error::{e, stack_error};
use serde::{Deserialize, Serialize};

use crate::{
    protocol::{
        GetManyRequest, GetRequest, ObserveRequest, PushRequest, ERR_INTERNAL, ERR_LIMIT,
        ERR_PERMISSION,
    },
    provider::{events::irpc_ext::IrpcClientExt, TransferStats},
    Hash,
};

/// Mode for connect events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum ConnectMode {
    /// We don't get notification of connect events at all.
    #[default]
    None,
    /// We get a notification for connect events.
    Notify,
    /// We get a request for connect events and can reject incoming connections.
    Intercept,
}

/// Request mode for observe requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum ObserveMode {
    /// We don't get notification of connect events at all.
    #[default]
    None,
    /// We get a notification for connect events.
    Notify,
    /// We get a request for connect events and can reject incoming connections.
    Intercept,
}

/// Request mode for all data related requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum RequestMode {
    /// We don't get request events at all.
    #[default]
    None,
    /// We get a notification for each request, but no transfer events.
    Notify,
    /// We get a request for each request, and can reject incoming requests, but no transfer events.
    Intercept,
    /// We get a notification for each request as well as detailed transfer events.
    NotifyLog,
    /// We get a request for each request, and can reject incoming requests.
    /// We also get detailed transfer events.
    InterceptLog,
    /// This request type is completely disabled. All requests will be rejected.
    ///
    /// This means that requests of this kind will always be rejected, whereas
    /// None means that we don't get any events, but requests will be processed normally.
    Disabled,
}

/// Throttling mode for requests that support throttling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum ThrottleMode {
    /// We don't get these kinds of events at all
    #[default]
    None,
    /// We call throttle to give the event handler a way to throttle requests
    Intercept,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AbortReason {
    /// The request was aborted because a limit was exceeded. It is OK to try again later.
    RateLimited,
    /// The request was aborted because the client does not have permission to perform the operation.
    Permission,
}

/// Errors that can occur when sending progress updates.
#[stack_error(derive, add_meta, from_sources)]
pub enum ProgressError {
    #[error("limit")]
    Limit {},
    #[error("permission")]
    Permission {},
    #[error(transparent)]
    Internal { source: irpc::Error },
}

impl From<ProgressError> for io::Error {
    fn from(value: ProgressError) -> Self {
        match value {
            ProgressError::Limit { .. } => io::ErrorKind::QuotaExceeded.into(),
            ProgressError::Permission { .. } => io::ErrorKind::PermissionDenied.into(),
            ProgressError::Internal { source, .. } => source.into(),
        }
    }
}

pub trait HasErrorCode {
    fn code(&self) -> VarInt;
}

impl HasErrorCode for ProgressError {
    fn code(&self) -> VarInt {
        match self {
            ProgressError::Limit { .. } => ERR_LIMIT,
            ProgressError::Permission { .. } => ERR_PERMISSION,
            ProgressError::Internal { .. } => ERR_INTERNAL,
        }
    }
}

impl ProgressError {
    pub fn reason(&self) -> &'static [u8] {
        match self {
            ProgressError::Limit { .. } => b"limit",
            ProgressError::Permission { .. } => b"permission",
            ProgressError::Internal { .. } => b"internal",
        }
    }
}

impl From<AbortReason> for ProgressError {
    fn from(value: AbortReason) -> Self {
        match value {
            AbortReason::RateLimited => n0_error::e!(ProgressError::Limit),
            AbortReason::Permission => n0_error::e!(ProgressError::Permission),
        }
    }
}

impl From<irpc::channel::mpsc::RecvError> for ProgressError {
    fn from(value: irpc::channel::mpsc::RecvError) -> Self {
        n0_error::e!(ProgressError::Internal, value.into())
    }
}

impl From<irpc::channel::oneshot::RecvError> for ProgressError {
    fn from(value: irpc::channel::oneshot::RecvError) -> Self {
        n0_error::e!(ProgressError::Internal, value.into())
    }
}

impl From<irpc::channel::SendError> for ProgressError {
    fn from(value: irpc::channel::SendError) -> Self {
        n0_error::e!(ProgressError::Internal, value.into())
    }
}

pub type EventResult = Result<(), AbortReason>;
pub type ClientResult = Result<(), ProgressError>;

/// Event mask to configure which events are sent to the event handler.
///
/// This can also be used to completely disable certain request types. E.g.
/// push requests are disabled by default, as they can write to the local store.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventMask {
    /// Connection event mask
    pub connected: ConnectMode,
    /// Get request event mask
    pub get: RequestMode,
    /// Get many request event mask
    pub get_many: RequestMode,
    /// Push request event mask
    pub push: RequestMode,
    /// Observe request event mask
    pub observe: ObserveMode,
    /// throttling is somewhat costly, so you can disable it completely
    pub throttle: ThrottleMode,
}

impl Default for EventMask {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl EventMask {
    /// All event notifications are fully disabled. Push requests are disabled by default.
    pub const DEFAULT: Self = Self {
        connected: ConnectMode::None,
        get: RequestMode::None,
        get_many: RequestMode::None,
        push: RequestMode::Disabled,
        throttle: ThrottleMode::None,
        observe: ObserveMode::None,
    };

    /// All event notifications for read-only requests are fully enabled.
    ///
    /// If you want to enable push requests, which can write to the local store, you
    /// need to do it manually. Providing constants that have push enabled would
    /// risk misuse.
    pub const ALL_READONLY: Self = Self {
        connected: ConnectMode::Intercept,
        get: RequestMode::InterceptLog,
        get_many: RequestMode::InterceptLog,
        push: RequestMode::Disabled,
        throttle: ThrottleMode::Intercept,
        observe: ObserveMode::Intercept,
    };
}

/// Newtype wrapper that wraps an event so that it is a distinct type for the notify variant.
#[derive(Debug, Serialize, Deserialize)]
pub struct Notify<T>(T);

impl<T> Deref for Notify<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Default, Clone)]
pub struct EventSender {
    mask: EventMask,
    inner: Option<irpc::Client<ProviderProto>>,
}

#[derive(Debug, Default)]
enum RequestUpdates {
    /// Request tracking was not configured, all ops are no-ops
    #[default]
    None,
    /// Active request tracking, all ops actually send
    Active(mpsc::Sender<RequestUpdate>),
    /// Disabled request tracking, we just hold on to the sender so it drops
    /// once the request is completed or aborted.
    Disabled(#[allow(dead_code)] mpsc::Sender<RequestUpdate>),
}

#[derive(Debug)]
pub struct RequestTracker {
    updates: RequestUpdates,
    throttle: Option<(irpc::Client<ProviderProto>, u64, u64)>,
}

impl RequestTracker {
    fn new(
        updates: RequestUpdates,
        throttle: Option<(irpc::Client<ProviderProto>, u64, u64)>,
    ) -> Self {
        Self { updates, throttle }
    }

    /// A request tracker that doesn't track anything.
    pub const NONE: Self = Self {
        updates: RequestUpdates::None,
        throttle: None,
    };

    /// Transfer for index `index` started, size `size` in bytes.
    pub async fn transfer_started(&self, index: u64, hash: &Hash, size: u64) -> irpc::Result<()> {
        if let RequestUpdates::Active(tx) = &self.updates {
            tx.send(
                TransferStarted {
                    index,
                    hash: *hash,
                    size,
                }
                .into(),
            )
            .await?;
        }
        Ok(())
    }

    /// Transfer progress for the previously reported blob, end_offset is the new end offset in bytes.
    pub async fn transfer_progress(&mut self, len: u64, end_offset: u64) -> ClientResult {
        if let RequestUpdates::Active(tx) = &mut self.updates {
            tx.try_send(TransferProgress { end_offset }.into()).await?;
        }
        if let Some((throttle, connection_id, request_id)) = &self.throttle {
            throttle
                .rpc(Throttle {
                    connection_id: *connection_id,
                    request_id: *request_id,
                    size: len,
                })
                .await??;
        }
        Ok(())
    }

    /// Transfer completed for the previously reported blob.
    pub async fn transfer_completed(&self, f: impl Fn() -> Box<TransferStats>) -> irpc::Result<()> {
        if let RequestUpdates::Active(tx) = &self.updates {
            tx.send(TransferCompleted { stats: f() }.into()).await?;
        }
        Ok(())
    }

    /// Transfer aborted for the previously reported blob.
    pub async fn transfer_aborted(&self, f: impl Fn() -> Box<TransferStats>) -> irpc::Result<()> {
        if let RequestUpdates::Active(tx) = &self.updates {
            tx.send(TransferAborted { stats: f() }.into()).await?;
        }
        Ok(())
    }
}

/// Client for progress notifications.
///
/// For most event types, the client can be configured to either send notifications or requests that
/// can have a response.
impl EventSender {
    /// A client that does not send anything.
    pub const DEFAULT: Self = Self {
        mask: EventMask::DEFAULT,
        inner: None,
    };

    pub fn new(client: tokio::sync::mpsc::Sender<ProviderMessage>, mask: EventMask) -> Self {
        Self {
            mask,
            inner: Some(irpc::Client::from(client)),
        }
    }

    pub fn channel(
        capacity: usize,
        mask: EventMask,
    ) -> (Self, tokio::sync::mpsc::Receiver<ProviderMessage>) {
        let (tx, rx) = tokio::sync::mpsc::channel(capacity);
        (Self::new(tx, mask), rx)
    }

    /// Log request events at trace level.
    pub fn tracing(&self, mask: EventMask) -> Self {
        use tracing::trace;
        let (tx, mut rx) = tokio::sync::mpsc::channel(32);
        n0_future::task::spawn(async move {
            fn log_request_events(
                mut rx: irpc::channel::mpsc::Receiver<RequestUpdate>,
                connection_id: u64,
                request_id: u64,
            ) {
                n0_future::task::spawn(async move {
                    while let Ok(Some(update)) = rx.recv().await {
                        trace!(%connection_id, %request_id, "{update:?}");
                    }
                });
            }
            while let Some(msg) = rx.recv().await {
                match msg {
                    ProviderMessage::ClientConnected(msg) => {
                        trace!("{:?}", msg.inner);
                        msg.tx.send(Ok(())).await.ok();
                    }
                    ProviderMessage::ClientConnectedNotify(msg) => {
                        trace!("{:?}", msg.inner);
                    }
                    ProviderMessage::ConnectionClosed(msg) => {
                        trace!("{:?}", msg.inner);
                    }
                    ProviderMessage::GetRequestReceived(msg) => {
                        trace!("{:?}", msg.inner);
                        msg.tx.send(Ok(())).await.ok();
                        log_request_events(msg.rx, msg.inner.connection_id, msg.inner.request_id);
                    }
                    ProviderMessage::GetRequestReceivedNotify(msg) => {
                        trace!("{:?}", msg.inner);
                        log_request_events(msg.rx, msg.inner.connection_id, msg.inner.request_id);
                    }
                    ProviderMessage::GetManyRequestReceived(msg) => {
                        trace!("{:?}", msg.inner);
                        msg.tx.send(Ok(())).await.ok();
                        log_request_events(msg.rx, msg.inner.connection_id, msg.inner.request_id);
                    }
                    ProviderMessage::GetManyRequestReceivedNotify(msg) => {
                        trace!("{:?}", msg.inner);
                        log_request_events(msg.rx, msg.inner.connection_id, msg.inner.request_id);
                    }
                    ProviderMessage::PushRequestReceived(msg) => {
                        trace!("{:?}", msg.inner);
                        msg.tx.send(Ok(())).await.ok();
                        log_request_events(msg.rx, msg.inner.connection_id, msg.inner.request_id);
                    }
                    ProviderMessage::PushRequestReceivedNotify(msg) => {
                        trace!("{:?}", msg.inner);
                        log_request_events(msg.rx, msg.inner.connection_id, msg.inner.request_id);
                    }
                    ProviderMessage::ObserveRequestReceived(msg) => {
                        trace!("{:?}", msg.inner);
                        msg.tx.send(Ok(())).await.ok();
                        log_request_events(msg.rx, msg.inner.connection_id, msg.inner.request_id);
                    }
                    ProviderMessage::ObserveRequestReceivedNotify(msg) => {
                        trace!("{:?}", msg.inner);
                        log_request_events(msg.rx, msg.inner.connection_id, msg.inner.request_id);
                    }
                    ProviderMessage::Throttle(msg) => {
                        trace!("{:?}", msg.inner);
                        msg.tx.send(Ok(())).await.ok();
                    }
                }
            }
        });
        Self {
            mask,
            inner: Some(irpc::Client::from(tx)),
        }
    }

    /// A new client has been connected.
    pub async fn client_connected(&self, f: impl Fn() -> ClientConnected) -> ClientResult {
        if let Some(client) = &self.inner {
            match self.mask.connected {
                ConnectMode::None => {}
                ConnectMode::Notify => client.notify(Notify(f())).await?,
                ConnectMode::Intercept => client.rpc(f()).await??,
            }
        };
        Ok(())
    }

    /// A connection has been closed.
    pub async fn connection_closed(&self, f: impl Fn() -> ConnectionClosed) -> ClientResult {
        if let Some(client) = &self.inner {
            client.notify(f()).await?;
        };
        Ok(())
    }

    /// Abstract request, to DRY the 3 to 4 request types.
    ///
    /// DRYing stuff with lots of bounds is no fun at all...
    pub(crate) async fn request<Req>(
        &self,
        f: impl FnOnce() -> Req,
        connection_id: u64,
        request_id: u64,
    ) -> Result<RequestTracker, ProgressError>
    where
        ProviderProto: From<RequestReceived<Req>>,
        ProviderMessage: From<WithChannels<RequestReceived<Req>, ProviderProto>>,
        RequestReceived<Req>: Channels<
            ProviderProto,
            Tx = oneshot::Sender<EventResult>,
            Rx = mpsc::Receiver<RequestUpdate>,
        >,
        ProviderProto: From<Notify<RequestReceived<Req>>>,
        ProviderMessage: From<WithChannels<Notify<RequestReceived<Req>>, ProviderProto>>,
        Notify<RequestReceived<Req>>:
            Channels<ProviderProto, Tx = NoSender, Rx = mpsc::Receiver<RequestUpdate>>,
    {
        let client = self.inner.as_ref();
        Ok(self.create_tracker((
            match self.mask.get {
                RequestMode::None => RequestUpdates::None,
                RequestMode::Notify if client.is_some() => {
                    let msg = RequestReceived {
                        request: f(),
                        connection_id,
                        request_id,
                    };
                    RequestUpdates::Disabled(
                        client.unwrap().notify_streaming(Notify(msg), 32).await?,
                    )
                }
                RequestMode::Intercept if client.is_some() => {
                    let msg = RequestReceived {
                        request: f(),
                        connection_id,
                        request_id,
                    };
                    let (tx, rx) = client.unwrap().client_streaming(msg, 32).await?;
                    // bail out if the request is not allowed
                    rx.await??;
                    RequestUpdates::Disabled(tx)
                }
                RequestMode::NotifyLog if client.is_some() => {
                    let msg = RequestReceived {
                        request: f(),
                        connection_id,
                        request_id,
                    };
                    RequestUpdates::Active(client.unwrap().notify_streaming(Notify(msg), 32).await?)
                }
                RequestMode::InterceptLog if client.is_some() => {
                    let msg = RequestReceived {
                        request: f(),
                        connection_id,
                        request_id,
                    };
                    let (tx, rx) = client.unwrap().client_streaming(msg, 32).await?;
                    // bail out if the request is not allowed
                    rx.await??;
                    RequestUpdates::Active(tx)
                }
                RequestMode::Disabled => {
                    return Err(e!(ProgressError::Permission));
                }
                _ => RequestUpdates::None,
            },
            connection_id,
            request_id,
        )))
    }

    fn create_tracker(
        &self,
        (updates, connection_id, request_id): (RequestUpdates, u64, u64),
    ) -> RequestTracker {
        let throttle = match self.mask.throttle {
            ThrottleMode::None => None,
            ThrottleMode::Intercept => self
                .inner
                .clone()
                .map(|client| (client, connection_id, request_id)),
        };
        RequestTracker::new(updates, throttle)
    }
}

#[rpc_requests(message = ProviderMessage, rpc_feature = "rpc")]
#[derive(Debug, Serialize, Deserialize)]
pub enum ProviderProto {
    /// A new client connected to the provider.
    #[rpc(tx = oneshot::Sender<EventResult>)]
    ClientConnected(ClientConnected),

    /// A new client connected to the provider. Notify variant.
    #[rpc(tx = NoSender)]
    ClientConnectedNotify(Notify<ClientConnected>),

    /// A client disconnected from the provider.
    #[rpc(tx = NoSender)]
    ConnectionClosed(ConnectionClosed),

    /// A new get request was received from the provider.
    #[rpc(rx = mpsc::Receiver<RequestUpdate>, tx = oneshot::Sender<EventResult>)]
    GetRequestReceived(RequestReceived<GetRequest>),

    /// A new get request was received from the provider (notify variant).
    #[rpc(rx = mpsc::Receiver<RequestUpdate>, tx = NoSender)]
    GetRequestReceivedNotify(Notify<RequestReceived<GetRequest>>),

    /// A new get many request was received from the provider.
    #[rpc(rx = mpsc::Receiver<RequestUpdate>, tx = oneshot::Sender<EventResult>)]
    GetManyRequestReceived(RequestReceived<GetManyRequest>),

    /// A new get many request was received from the provider (notify variant).
    #[rpc(rx = mpsc::Receiver<RequestUpdate>, tx = NoSender)]
    GetManyRequestReceivedNotify(Notify<RequestReceived<GetManyRequest>>),

    /// A new push request was received from the provider.
    #[rpc(rx = mpsc::Receiver<RequestUpdate>, tx = oneshot::Sender<EventResult>)]
    PushRequestReceived(RequestReceived<PushRequest>),

    /// A new push request was received from the provider (notify variant).
    #[rpc(rx = mpsc::Receiver<RequestUpdate>, tx = NoSender)]
    PushRequestReceivedNotify(Notify<RequestReceived<PushRequest>>),

    /// A new observe request was received from the provider.
    #[rpc(rx = mpsc::Receiver<RequestUpdate>, tx = oneshot::Sender<EventResult>)]
    ObserveRequestReceived(RequestReceived<ObserveRequest>),

    /// A new observe request was received from the provider (notify variant).
    #[rpc(rx = mpsc::Receiver<RequestUpdate>, tx = NoSender)]
    ObserveRequestReceivedNotify(Notify<RequestReceived<ObserveRequest>>),

    /// Request to throttle sending for a specific data request.
    #[rpc(tx = oneshot::Sender<EventResult>)]
    Throttle(Throttle),
}

mod proto {
    use iroh::EndpointId;
    use serde::{Deserialize, Serialize};

    use crate::{provider::TransferStats, Hash};

    #[derive(Debug, Serialize, Deserialize)]
    pub struct ClientConnected {
        pub connection_id: u64,
        pub endpoint_id: Option<EndpointId>,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct ConnectionClosed {
        pub connection_id: u64,
    }

    /// A new get request was received from the provider.
    #[derive(Debug, Serialize, Deserialize)]
    pub struct RequestReceived<R> {
        /// The connection id. Multiple requests can be sent over the same connection.
        pub connection_id: u64,
        /// The request id. There is a new id for each request.
        pub request_id: u64,
        /// The request
        pub request: R,
    }

    /// Request to throttle sending for a specific request.
    #[derive(Debug, Serialize, Deserialize)]
    pub struct Throttle {
        /// The connection id. Multiple requests can be sent over the same connection.
        pub connection_id: u64,
        /// The request id. There is a new id for each request.
        pub request_id: u64,
        /// Size of the chunk to be throttled. This will usually be 16 KiB.
        pub size: u64,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct TransferProgress {
        /// The end offset of the chunk that was sent.
        pub end_offset: u64,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct TransferStarted {
        pub index: u64,
        pub hash: Hash,
        pub size: u64,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct TransferCompleted {
        pub stats: Box<TransferStats>,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct TransferAborted {
        pub stats: Box<TransferStats>,
    }

    /// Stream of updates for a single request
    #[derive(Debug, Serialize, Deserialize, derive_more::From)]
    pub enum RequestUpdate {
        /// Start of transfer for a blob, mandatory event
        Started(TransferStarted),
        /// Progress for a blob - optional event
        Progress(TransferProgress),
        /// Successful end of transfer
        Completed(TransferCompleted),
        /// Aborted end of transfer
        Aborted(TransferAborted),
    }
}
pub use proto::*;

mod irpc_ext {
    use std::future::Future;

    use irpc::{
        channel::{mpsc, none::NoSender},
        Channels, RpcMessage, Service, WithChannels,
    };

    pub trait IrpcClientExt<S: Service> {
        fn notify_streaming<Req, Update>(
            &self,
            msg: Req,
            local_update_cap: usize,
        ) -> impl Future<Output = irpc::Result<mpsc::Sender<Update>>>
        where
            S: From<Req>,
            S::Message: From<WithChannels<Req, S>>,
            Req: Channels<S, Tx = NoSender, Rx = mpsc::Receiver<Update>>,
            Update: RpcMessage;
    }

    impl<S: Service> IrpcClientExt<S> for irpc::Client<S> {
        fn notify_streaming<Req, Update>(
            &self,
            msg: Req,
            local_update_cap: usize,
        ) -> impl Future<Output = irpc::Result<mpsc::Sender<Update>>>
        where
            S: From<Req>,
            S::Message: From<WithChannels<Req, S>>,
            Req: Channels<S, Tx = NoSender, Rx = mpsc::Receiver<Update>>,
            Update: RpcMessage,
        {
            let client = self.clone();
            async move {
                let request = client.request().await?;
                match request {
                    irpc::Request::Local(local) => {
                        let (req_tx, req_rx) = mpsc::channel(local_update_cap);
                        local
                            .send((msg, NoSender, req_rx))
                            .await
                            .map_err(irpc::Error::from)?;
                        Ok(req_tx)
                    }
                    #[cfg(feature = "rpc")]
                    irpc::Request::Remote(remote) => {
                        let (s, _) = remote.write(msg).await?;
                        Ok(s.into())
                    }
                    #[cfg(not(feature = "rpc"))]
                    irpc::Request::Remote(_) => {
                        unreachable!()
                    }
                }
            }
        }
    }
}
