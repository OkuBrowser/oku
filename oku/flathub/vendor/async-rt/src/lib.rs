pub mod arc;
pub mod error;
#[cfg(feature = "fs")]
pub mod fs;
pub mod global;
#[cfg(all(
    feature = "net",
    not(target_arch = "wasm32"),
    any(feature = "tokio", feature = "smol", feature = "compio")
))]
pub mod net;
pub mod rt;
pub mod task;
pub mod tracker;

#[cfg(feature = "either")]
pub mod either;
pub mod rc;
pub mod scoped;

use std::fmt::{Debug, Formatter};
use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicBool, Ordering};

pub use crate::error::JoinError;
pub use crate::error::TimeoutError;
pub use crate::scoped::{Scope, ScopeExecutor, ScopedJoinHandle};
#[cfg(all(
    feature = "macros",
    feature = "compio",
    not(any(feature = "tokio", feature = "smol", target_arch = "wasm32"))
))]
#[doc(inline)]
pub use async_rt_macros::{main_compio as main, test_compio as test};
#[cfg(all(
    feature = "macros",
    not(any(
        feature = "tokio",
        feature = "smol",
        feature = "compio",
        feature = "threadpool",
        feature = "lite"
    )),
    not(target_arch = "wasm32")
))]
#[doc(inline)]
pub use async_rt_macros::{main_fail as main, test_fail as test};
#[cfg(all(
    feature = "macros",
    feature = "lite",
    not(any(
        feature = "tokio",
        feature = "smol",
        feature = "compio",
        feature = "threadpool",
        target_arch = "wasm32"
    ))
))]
#[doc(inline)]
pub use async_rt_macros::{main_lite as main, test_lite as test};
#[cfg(all(
    feature = "macros",
    feature = "smol",
    not(any(feature = "tokio", target_arch = "wasm32"))
))]
#[doc(inline)]
pub use async_rt_macros::{main_smol as main, test_smol as test};
#[cfg(all(
    feature = "macros",
    feature = "threadpool",
    not(any(
        feature = "tokio",
        feature = "smol",
        feature = "compio",
        target_arch = "wasm32"
    ))
))]
#[doc(inline)]
pub use async_rt_macros::{main_threadpool as main, test_threadpool as test};
#[cfg(all(feature = "macros", feature = "tokio", not(target_arch = "wasm32")))]
#[doc(inline)]
pub use async_rt_macros::{main_tokio as main, test_tokio as test};
#[cfg(all(feature = "macros", target_arch = "wasm32"))]
#[doc(inline)]
pub use async_rt_macros::{main_wasm_fail as main, test_wasm_fail as test};
use futures::channel::mpsc::{Receiver, UnboundedReceiver};
use futures::future::{AbortHandle, AbortRegistration, Abortable};
use futures::task::AtomicWaker;
use futures::{FutureExt, SinkExt, StreamExt, TryFutureExt};
use futures_timeout::Timeout;
#[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
use parking_lot::Mutex;
use pollable_map::optional::Optional;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

#[cfg(feature = "macros")]
extern crate self as async_rt;

#[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
type BoxCancelFuture<T> = Pin<Box<dyn Future<Output = Option<T>> + Send + 'static>>;

#[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
type StartCompioCancel<T> = fn(&Mutex<InnerCompioHandle<T>>, &AtomicBool);

#[cfg_attr(feature = "tokio", allow(dead_code))]
pub(crate) struct CompletionGuard {
    finished: Arc<AtomicBool>,
}

impl CompletionGuard {
    #[cfg_attr(feature = "tokio", allow(dead_code))]
    pub(crate) fn new(finished: Arc<AtomicBool>) -> Self {
        Self { finished }
    }
}

impl Drop for CompletionGuard {
    fn drop(&mut self) {
        self.finished.store(true, Ordering::Release);
    }
}

pub(crate) async fn abortable_result<F>(
    future: F,
    abort_registration: AbortRegistration,
) -> Result<F::Output, JoinError>
where
    F: Future,
{
    let future = AssertUnwindSafe(future).catch_unwind();

    match Abortable::new(future, abort_registration).await {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(_)) => Err(JoinError::Panicked),
        Err(_) => Err(JoinError::Aborted),
    }
}

/// An owned permission to join on a task (await its termination).
///
/// This can be seen as an equivalent to [`std::thread::JoinHandle`] but for [`Future`] tasks rather than a thread.
/// Note that the task associated with this `JoinHandle` will start running when
/// [`Executor::spawn`] is called according to the selected runtime, even if the
/// `JoinHandle` has not been awaited.
///
/// Dropping `JoinHandle` will not abort or cancel the task. In other words, the task will continue to run in the background
/// and any return value will be lost.
///
/// This `struct` is created by the [`Executor::spawn`].
pub struct JoinHandle<T> {
    inner: InnerJoinHandle<T>,
}

impl<T> Debug for JoinHandle<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JoinHandle").finish()
    }
}

#[derive(Default)]
enum InnerJoinHandle<T> {
    #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
    TokioHandle {
        handle: Optional<::tokio::task::JoinHandle<T>>,
        abort_requested: AtomicBool,
    },
    #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
    CompioHandle {
        handle: Mutex<InnerCompioHandle<T>>,
        abort_requested: AtomicBool,
        start_cancel: StartCompioCancel<T>,
    },
    #[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
    SmolHandle { handle: SmolTask<T> },
    #[allow(dead_code)]
    CustomHandle {
        inner: Optional<futures::channel::oneshot::Receiver<Result<T, JoinError>>>,
        handle: AbortHandle,
        finished: Arc<AtomicBool>,
    },
    #[default]
    Empty,
}

#[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
struct SmolTask<T> {
    inner: Optional<::async_task::FallibleTask<Result<T, JoinError>>>,
    abort_handle: AbortHandle,
}

#[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
impl<T> SmolTask<T> {
    fn new(task: ::smol::Task<Result<T, JoinError>>, abort_handle: AbortHandle) -> Self {
        Self {
            inner: Optional::new(task.fallible()),
            abort_handle,
        }
    }

    fn abort(&self) {
        self.abort_handle.abort();
    }

    fn is_finished(&self) -> bool {
        self.inner
            .as_ref()
            .map(|task| task.is_finished())
            .unwrap_or(true)
    }
}

#[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
impl<T> Drop for SmolTask<T> {
    fn drop(&mut self) {
        if let Some(task) = self.inner.take() {
            task.detach();
        }
    }
}

#[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
impl<T> Future for SmolTask<T> {
    type Output = Result<T, JoinError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match futures::ready!(Pin::new(&mut self.inner).poll(cx)) {
            Some(result) => Poll::Ready(result),
            None => Poll::Ready(Err(JoinError::Cancelled)),
        }
    }
}

#[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
enum InnerCompioHandle<T> {
    Running(Option<::compio::runtime::JoinHandle<T>>),
    Cancelling(CompioCancel<T>),
    Ready(Option<Option<T>>),
}

#[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
impl<T> Unpin for InnerCompioHandle<T> {}

#[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
struct CompioCancelWake {
    outer: AtomicWaker,
}

#[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
impl futures::task::ArcWake for CompioCancelWake {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        arc_self.outer.wake();
    }
}

#[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
struct CompioCancel<T> {
    future: BoxCancelFuture<T>,
    wake: Arc<CompioCancelWake>,
}

#[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
impl<T> CompioCancel<T> {
    fn new(future: BoxCancelFuture<T>) -> Self {
        Self {
            future,
            wake: Arc::new(CompioCancelWake {
                outer: AtomicWaker::new(),
            }),
        }
    }

    fn poll_inner(&mut self) -> Poll<Option<T>> {
        let waker = futures::task::waker(self.wake.clone());
        let mut cx = Context::from_waker(&waker);
        self.future.as_mut().poll(&mut cx)
    }
}

#[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
impl<T> Future for CompioCancel<T> {
    type Output = Option<T>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.wake.outer.register(cx.waker());
        self.poll_inner()
    }
}

// Note: We implement a custom drop due to compio `JoinHandle` cancellation of the task upon drop
//       therefore we need to detach the handle upon drop so it will continue running in the
//       background to follow the same flow as the other executors implementations.
#[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
impl<T> Drop for InnerCompioHandle<T> {
    fn drop(&mut self) {
        if let InnerCompioHandle::Running(handle) = self {
            let Some(handle) = handle.take() else {
                return;
            };
            handle.detach();
        }
    }
}

#[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
impl<T> Future for InnerCompioHandle<T> {
    type Output = Result<Option<T>, JoinError>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = &mut *self;

        match this {
            InnerCompioHandle::Running(handle) => {
                let Some(mut handle) = handle.take() else {
                    return Poll::Ready(Ok(None));
                };
                match Pin::new(&mut handle).poll(cx) {
                    Poll::Ready(result) => Poll::Ready(result.map(Some).map_err(JoinError::from)),
                    Poll::Pending => {
                        *this = InnerCompioHandle::Running(Some(handle));
                        Poll::Pending
                    }
                }
            }
            InnerCompioHandle::Cancelling(future) => match Pin::new(future).poll(cx) {
                Poll::Ready(result) => {
                    *this = InnerCompioHandle::Ready(None);
                    Poll::Ready(Ok(result))
                }
                Poll::Pending => Poll::Pending,
            },
            InnerCompioHandle::Ready(result) => match result.take() {
                Some(result) => Poll::Ready(Ok(result)),
                None => Poll::Ready(Err(JoinError::Empty)),
            },
        }
    }
}

#[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
fn start_compio_cancel<T: Send + 'static>(
    state: &Mutex<InnerCompioHandle<T>>,
    requested: &AtomicBool,
) {
    let mut state = state.lock();

    let InnerCompioHandle::Running(slot) = &mut *state else {
        return;
    };
    let Some(handle) = slot.take() else {
        return;
    };

    requested.store(true, Ordering::Release);

    let future: BoxCancelFuture<T> = Box::pin(handle.cancel());
    let mut future = CompioCancel::new(future);
    *state = match future.poll_inner() {
        Poll::Ready(result) => InnerCompioHandle::Ready(Some(result)),
        Poll::Pending => InnerCompioHandle::Cancelling(future),
    };
}

#[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
impl<T> InnerCompioHandle<T> {
    fn is_finished(&mut self) -> bool {
        match self {
            InnerCompioHandle::Running(handle) => {
                handle.as_ref().map(|h| h.is_finished()).unwrap_or(true)
            }
            InnerCompioHandle::Cancelling(future) => match future.poll_inner() {
                Poll::Ready(result) => {
                    *self = InnerCompioHandle::Ready(Some(result));
                    true
                }
                Poll::Pending => false,
            },
            InnerCompioHandle::Ready(_) => true,
        }
    }
}

impl<T> InnerJoinHandle<T> {
    #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
    fn tokio(handle: ::tokio::task::JoinHandle<T>) -> Self {
        Self::TokioHandle {
            handle: Optional::new(handle),
            abort_requested: AtomicBool::new(false),
        }
    }

    #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
    fn compio(handle: ::compio::runtime::JoinHandle<T>) -> Self
    where
        T: Send + 'static,
    {
        Self::CompioHandle {
            handle: Mutex::new(InnerCompioHandle::Running(Some(handle))),
            abort_requested: AtomicBool::new(false),
            start_cancel: start_compio_cancel::<T>,
        }
    }

    #[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
    fn smol(handle: ::smol::Task<Result<T, JoinError>>, abort_handle: AbortHandle) -> Self {
        Self::SmolHandle {
            handle: SmolTask::new(handle, abort_handle),
        }
    }
}

impl<T> JoinHandle<T> {
    /// Provide an empty [`JoinHandle`] with no associated task.
    pub fn empty() -> Self {
        JoinHandle {
            inner: InnerJoinHandle::Empty,
        }
    }
}

impl<T> JoinHandle<T> {
    /// Abort the task associated with the handle.
    ///
    /// If cancellation is observed after this method is called, awaiting the
    /// handle returns [`JoinError::Aborted`]. Cancellation without an abort
    /// request through this handle returns [`JoinError::Cancelled`].
    pub fn abort(&self) {
        match &self.inner {
            #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
            InnerJoinHandle::TokioHandle {
                handle,
                abort_requested,
            } => {
                if let Some(handle) = handle.as_ref() {
                    abort_requested.store(true, Ordering::Release);
                    handle.abort();
                }
            }
            #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
            InnerJoinHandle::CompioHandle {
                handle,
                abort_requested,
                start_cancel,
            } => {
                start_cancel(handle, abort_requested);
            }
            #[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
            InnerJoinHandle::SmolHandle { handle } => handle.abort(),
            InnerJoinHandle::CustomHandle { handle, .. } => handle.abort(),
            InnerJoinHandle::Empty => {}
        }
    }

    /// Check if the task associated with this `JoinHandle` has finished.
    ///
    /// Note that this method can return false even if [`JoinHandle::abort`] has been called on the
    /// task due to the time it may take for the task to cancel.
    pub fn is_finished(&self) -> bool {
        match &self.inner {
            #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
            InnerJoinHandle::TokioHandle { handle, .. } => {
                handle.as_ref().map(|h| h.is_finished()).unwrap_or(true)
            }
            #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
            InnerJoinHandle::CompioHandle { handle, .. } => handle.lock().is_finished(),
            #[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
            InnerJoinHandle::SmolHandle { handle } => handle.is_finished(),
            InnerJoinHandle::CustomHandle {
                inner, finished, ..
            } => finished.load(Ordering::Acquire) || inner.is_none(),
            InnerJoinHandle::Empty => true,
        }
    }

    /// Replace the current handle with the provided [`JoinHandle`], while dropping the source.
    ///
    /// # Warning
    ///
    /// Note that if this is called with a non-empty handle, the existing task
    /// will not be terminated when it is replaced and could run indefinitely.
    /// Best to use when `JoinHandle` is `JoinHandle::empty` where empty is a used
    /// placeholder.
    pub fn replace(&mut self, mut handle: JoinHandle<T>) {
        self.inner = std::mem::take(&mut handle.inner);
    }

    /// Replace the current handle with the provided [`JoinHandle`], making the source become
    /// an equivalent to [`JoinHandle::empty`].
    ///
    /// # Warning
    ///
    /// Note that if this is called with a non-empty handle, the existing task
    /// will not be terminated when it is replaced and could run indefinitely.
    /// Best to use when `JoinHandle` is `JoinHandle::empty` where empty is a used
    /// placeholder, and you want to update the source later with another task handle.
    pub fn replace_in_place(&mut self, handle: &mut JoinHandle<T>) {
        self.inner = std::mem::take(&mut handle.inner);
    }
}

impl<T> Future for JoinHandle<T> {
    type Output = Result<T, JoinError>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let inner = &mut self.inner;
        match inner {
            #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
            InnerJoinHandle::TokioHandle {
                handle,
                abort_requested,
            } => {
                let fut = futures::ready!(Pin::new(handle).poll(cx));

                match fut {
                    Ok(val) => Poll::Ready(Ok(val)),
                    Err(e) if e.is_cancelled() && abort_requested.load(Ordering::Acquire) => {
                        Poll::Ready(Err(JoinError::Aborted))
                    }
                    Err(e) => Poll::Ready(Err(e.into())),
                }
            }
            #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
            InnerJoinHandle::CompioHandle {
                handle,
                abort_requested,
                ..
            } => {
                let handle = &mut *handle.lock();
                let fut = futures::ready!(Pin::new(handle).poll(cx));

                match fut {
                    Ok(Some(val)) => Poll::Ready(Ok(val)),
                    Ok(None) if abort_requested.load(Ordering::Acquire) => {
                        Poll::Ready(Err(JoinError::Aborted))
                    }
                    Ok(None) => Poll::Ready(Err(JoinError::Empty)),
                    Err(_) if abort_requested.load(Ordering::Acquire) => {
                        Poll::Ready(Err(JoinError::Aborted))
                    }
                    Err(e) => Poll::Ready(Err(e)),
                }
            }
            #[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
            InnerJoinHandle::SmolHandle { handle } => Pin::new(handle).poll(cx),
            InnerJoinHandle::CustomHandle { inner, .. } => {
                let fut = futures::ready!(Pin::new(inner).poll(cx));
                match fut {
                    Ok(result) => Poll::Ready(result),
                    Err(_) => Poll::Ready(Err(JoinError::Cancelled)),
                }
            }
            InnerJoinHandle::Empty => Poll::Ready(Err(JoinError::Empty)),
        }
    }
}

/// The same as [`JoinHandle`] but designed to abort the task when all associated references
/// to the returned `AbortableJoinHandle` have been dropped.
#[derive(Clone)]
pub struct AbortableJoinHandle<T> {
    handle: Arc<InnerHandle<T>>,
}

impl<T> Debug for AbortableJoinHandle<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AbortableJoinHandle").finish()
    }
}

impl<T> From<JoinHandle<T>> for AbortableJoinHandle<T> {
    fn from(handle: JoinHandle<T>) -> Self {
        AbortableJoinHandle {
            handle: Arc::new(InnerHandle {
                inner: parking_lot::Mutex::new(handle),
                waker: AtomicWaker::new(),
            }),
        }
    }
}

impl<T> AbortableJoinHandle<T> {
    /// Provide an empty [`AbortableJoinHandle`] with no associated task.
    pub fn empty() -> Self {
        Self {
            handle: Arc::new(InnerHandle {
                inner: parking_lot::Mutex::new(JoinHandle::empty()),
                waker: AtomicWaker::new(),
            }),
        }
    }
}

impl<T> AbortableJoinHandle<T> {
    /// See [`JoinHandle::abort`]
    pub fn abort(&self) {
        self.handle.inner.lock().abort();
        self.handle.wake();
    }

    /// See [`JoinHandle::is_finished`]
    pub fn is_finished(&self) -> bool {
        self.handle.inner.lock().is_finished()
    }

    /// Replace the current handle with an existing one.
    ///
    /// # Warning
    ///
    /// Note that if this is called with a non-empty handle, the existing task
    /// will not be terminated when it is replaced.
    pub fn replace(&self, other: AbortableJoinHandle<T>) {
        if Arc::ptr_eq(&self.handle, &other.handle) {
            return;
        }

        let replacement = {
            let mut source = other.handle.inner.lock();
            std::mem::replace(&mut *source, JoinHandle::empty())
        };

        other.handle.wake();

        let previous = {
            let mut destination = self.handle.inner.lock();
            std::mem::replace(&mut *destination, replacement)
        };

        drop(previous);

        self.handle.wake();
    }
}

struct InnerHandle<T> {
    pub inner: parking_lot::Mutex<JoinHandle<T>>,
    pub waker: AtomicWaker,
}

impl<T> Drop for InnerHandle<T> {
    fn drop(&mut self) {
        self.inner.lock().abort();
    }
}

impl<T> InnerHandle<T> {
    pub fn wake(&self) {
        self.waker.wake();
    }
}

impl<T> Future for AbortableJoinHandle<T> {
    type Output = Result<T, JoinError>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.handle.waker.register(cx.waker());
        let inner = &mut *self.handle.inner.lock();
        Pin::new(inner).poll(cx)
    }
}

/// A task that accepts messages
pub struct CommunicationTask<T> {
    _task_handle: AbortableJoinHandle<()>,
    _channel_tx: futures::channel::mpsc::Sender<T>,
}

impl<T> Clone for CommunicationTask<T> {
    fn clone(&self) -> Self {
        CommunicationTask {
            _task_handle: self._task_handle.clone(),
            _channel_tx: self._channel_tx.clone(),
        }
    }
}

impl<T> Debug for CommunicationTask<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommunicationTask").finish()
    }
}

impl<T> CommunicationTask<T> {
    pub(crate) fn new(
        task_handle: AbortableJoinHandle<()>,
        channel_tx: futures::channel::mpsc::Sender<T>,
    ) -> Self {
        Self {
            _task_handle: task_handle,
            _channel_tx: channel_tx,
        }
    }

    /// Send a message to the task
    pub async fn send(&mut self, data: T) -> std::io::Result<()> {
        self._channel_tx
            .send(data)
            .await
            .map_err(std::io::Error::other)
    }

    /// Attempts to send a message to the task, returning an error if the channel is full or closed due to the task being aborted.
    pub fn try_send(&mut self, data: T) -> std::io::Result<()> {
        self._channel_tx
            .try_send(data)
            .map_err(|e| std::io::Error::other(e.to_string()))
    }

    /// Abort the task
    pub fn abort(mut self) {
        self._channel_tx.close_channel();
        self._task_handle.abort();
    }

    /// Check to determine if the task is active.
    pub fn is_active(&self) -> bool {
        !self._task_handle.is_finished() && !self._channel_tx.is_closed()
    }
}

/// A task that accepts messages
pub struct UnboundedCommunicationTask<T> {
    _task_handle: AbortableJoinHandle<()>,
    _channel_tx: futures::channel::mpsc::UnboundedSender<T>,
}

impl<T> Clone for UnboundedCommunicationTask<T> {
    fn clone(&self) -> Self {
        UnboundedCommunicationTask {
            _task_handle: self._task_handle.clone(),
            _channel_tx: self._channel_tx.clone(),
        }
    }
}

impl<T> Debug for UnboundedCommunicationTask<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UnboundedCommunicationTask").finish()
    }
}

impl<T> UnboundedCommunicationTask<T> {
    pub(crate) fn new(
        task_handle: AbortableJoinHandle<()>,
        channel_tx: futures::channel::mpsc::UnboundedSender<T>,
    ) -> Self {
        Self {
            _task_handle: task_handle,
            _channel_tx: channel_tx,
        }
    }

    /// Send a message to task
    pub fn send(&mut self, data: T) -> std::io::Result<()> {
        self._channel_tx
            .unbounded_send(data)
            .map_err(|e| std::io::Error::other(e.to_string()))
    }

    /// Abort the task
    pub fn abort(self) {
        self._channel_tx.close_channel();
        self._task_handle.abort();
    }

    /// Check to determine if the task is active.
    pub fn is_active(&self) -> bool {
        !self._task_handle.is_finished() && !self._channel_tx.is_closed()
    }
}

pub trait Executor {
    /// Returns an optional runtime name of the executor.
    fn runtime_type(&self) -> Option<&'static str> {
        None
    }

    /// Spawns a new asynchronous task in the background, returning a Future [`JoinHandle`] for it.
    fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static;

    /// Spawns a new asynchronous task in the background, returning an abortable handle that will cancel the task
    /// once the handle is dropped.
    ///
    /// Note: This function is used if the task is expected to run until the handle is dropped. It is recommended to use
    /// [`Executor::spawn`] or [`Executor::dispatch`] otherwise.
    fn spawn_abortable<F>(&self, future: F) -> AbortableJoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let handle = self.spawn(future);
        handle.into()
    }

    /// Spawns a new asynchronous task in the background without an handle.
    /// Basically the same as [`Executor::spawn`].
    fn dispatch<F>(&self, future: F)
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.spawn(future);
    }

    /// Spawns a new asynchronous task that accepts messages to the task.
    /// This function returns a handle that allows sending a message, or if there is no reference to the handle at all
    /// (in other words, all handles are dropped), the task would be aborted.
    fn spawn_coroutine<T, F, Fut>(&self, f: F) -> CommunicationTask<T>
    where
        F: FnMut(T) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
        T: Send + 'static,
    {
        Self::spawn_coroutine_with_buffer(self, 1, f)
    }

    /// Spawns a new asynchronous task that accepts messages to the task with a set buffer.
    /// This function returns a handle that allows sending a message, or if there is no reference to the handle at all
    /// (in other words, all handles are dropped), the task would be aborted.
    fn spawn_coroutine_with_buffer<T, F, Fut>(
        &self,
        buffer: usize,
        mut f: F,
    ) -> CommunicationTask<T>
    where
        F: FnMut(T) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
        T: Send + 'static,
    {
        let (tx, mut rx) = futures::channel::mpsc::channel(buffer);
        let _task_handle = self.spawn_abortable(async move {
            while let Some(msg) = rx.next().await {
                f(msg).await;
            }
        });
        CommunicationTask {
            _task_handle,
            _channel_tx: tx,
        }
    }

    /// Spawns a new asynchronous task that accepts unbounded messages to the task.
    /// This function returns a handle that allows sending a message, or if there is no reference to the handle at all
    /// (in other words, all handles are dropped), the task would be aborted.
    fn spawn_unbounded_coroutine<T, F, Fut>(&self, mut f: F) -> UnboundedCommunicationTask<T>
    where
        F: FnMut(T) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
        T: Send + 'static,
    {
        let (tx, mut rx) = futures::channel::mpsc::unbounded();
        let _task_handle = self.spawn_abortable(async move {
            while let Some(msg) = rx.next().await {
                f(msg).await;
            }
        });
        UnboundedCommunicationTask {
            _task_handle,
            _channel_tx: tx,
        }
    }

    /// Spawns a new asynchronous task with provided context that accepts messages to the task.
    /// This function returns a handle that allows sending a message, or if there is no reference to the handle at all
    /// (in other words, all handles are dropped), the task would be aborted.
    ///
    /// # Note
    /// If state must be borrowed across awaits,
    /// use [`Executor::spawn_coroutine_with_receiver_and_context`].
    fn spawn_coroutine_with_context<T, C, F, Fut>(&self, context: C, f: F) -> CommunicationTask<T>
    where
        F: FnMut(&mut C, T) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
        C: Send + 'static,
        T: Send + 'static,
    {
        Self::spawn_coroutine_with_buffer_and_context(self, context, 1, f)
    }

    /// Spawns a new asynchronous task with provided context that accepts messages to the task with a set buffer.
    /// This function returns a handle that allows sending a message, or if there is no reference to the handle at all
    /// (in other words, all handles are dropped), the task would be aborted.
    fn spawn_coroutine_with_buffer_and_context<T, C, F, Fut>(
        &self,
        context: C,
        buffer: usize,
        mut f: F,
    ) -> CommunicationTask<T>
    where
        F: FnMut(&mut C, T) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
        C: Send + 'static,
        T: Send + 'static,
    {
        let (tx, mut rx) = futures::channel::mpsc::channel(buffer);
        let _task_handle = self.spawn_abortable(async move {
            let mut context = context;
            while let Some(msg) = rx.next().await {
                f(&mut context, msg).await;
            }
        });
        CommunicationTask {
            _task_handle,
            _channel_tx: tx,
        }
    }

    /// Spawns a new asynchronous task with provided context that accepts unbounded messages to the task.
    /// This function returns a handle that allows sending a message, or if there is no reference to the handle at all
    /// (in other words, all handles are dropped), the task would be aborted.
    fn spawn_unbounded_coroutine_with_context<T, C, F, Fut>(
        &self,
        context: C,
        mut f: F,
    ) -> UnboundedCommunicationTask<T>
    where
        F: FnMut(&mut C, T) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
        C: Send + 'static,
        T: Send + 'static,
    {
        let (tx, mut rx) = futures::channel::mpsc::unbounded();
        let _task_handle = self.spawn_abortable(async move {
            let mut context = context;
            while let Some(msg) = rx.next().await {
                f(&mut context, msg).await;
            }
        });
        UnboundedCommunicationTask {
            _task_handle,
            _channel_tx: tx,
        }
    }

    /// Spawns a new asynchronous task that accepts messages to the task using [`channels`](futures::channel::mpsc).
    /// This function returns a handle that allows sending a message, or if there is no reference to the handle at all
    /// (in other words, all handles are dropped), the task would be aborted.
    fn spawn_coroutine_with_receiver<T, F, Fut>(&self, f: F) -> CommunicationTask<T>
    where
        F: FnMut(Receiver<T>) -> Fut,
        Fut: Future<Output = ()> + Send + 'static,
    {
        Self::spawn_coroutine_with_receiver_and_buffer(self, 1, f)
    }

    /// Spawns a new asynchronous task with a set channel buffer that accepts messages to the task using [`channels`](futures::channel::mpsc).
    /// This function returns a handle that allows sending a message, or if there is no reference to the handle at all
    /// (in other words, all handles are dropped), the task would be aborted.
    fn spawn_coroutine_with_receiver_and_buffer<T, F, Fut>(
        &self,
        buffer: usize,
        mut f: F,
    ) -> CommunicationTask<T>
    where
        F: FnMut(Receiver<T>) -> Fut,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let (tx, rx) = futures::channel::mpsc::channel(buffer);
        let fut = f(rx);
        let _task_handle = self.spawn_abortable(fut);
        CommunicationTask {
            _task_handle,
            _channel_tx: tx,
        }
    }

    /// Spawns a new asynchronous task with provided context that accepts messages to the task using [`channels`](futures::channel::mpsc).
    /// This function returns a handle that allows sending a message, or if there is no reference to the handle at all
    /// (in other words, all handles are dropped), the task would be aborted.
    fn spawn_coroutine_with_receiver_and_context<T, F, C, Fut>(
        &self,
        context: C,
        f: F,
    ) -> CommunicationTask<T>
    where
        F: FnMut(C, Receiver<T>) -> Fut,
        Fut: Future<Output = ()> + Send + 'static,
    {
        Self::spawn_coroutine_with_receiver_buffer_and_context(self, context, 1, f)
    }

    /// Spawns a new asynchronous task with a set channel buffer and provided context that accepts messages to the task using [`channels`](futures::channel::mpsc).
    /// This function returns a handle that allows sending a message, or if there is no reference to the handle at all
    /// (in other words, all handles are dropped), the task would be aborted.
    fn spawn_coroutine_with_receiver_buffer_and_context<T, F, C, Fut>(
        &self,
        context: C,
        buffer: usize,
        mut f: F,
    ) -> CommunicationTask<T>
    where
        F: FnMut(C, Receiver<T>) -> Fut,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let (tx, rx) = futures::channel::mpsc::channel(buffer);
        let fut = f(context, rx);
        let _task_handle = self.spawn_abortable(fut);
        CommunicationTask {
            _task_handle,
            _channel_tx: tx,
        }
    }

    /// Spawns a new asynchronous task that accepts messages to the task using [`channels`](futures::channel::mpsc).
    /// This function returns a handle that allows sending a message, or if there is no reference to the handle at all
    /// (in other words, all handles are dropped), the task would be aborted.
    fn spawn_unbounded_coroutine_with_receiver<T, F, Fut>(
        &self,
        mut f: F,
    ) -> UnboundedCommunicationTask<T>
    where
        F: FnMut(UnboundedReceiver<T>) -> Fut,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let (tx, rx) = futures::channel::mpsc::unbounded();
        let fut = f(rx);
        let _task_handle = self.spawn_abortable(fut);
        UnboundedCommunicationTask {
            _task_handle,
            _channel_tx: tx,
        }
    }

    /// Spawns a new asynchronous task with provided context that accepts messages to the task using [`channels`](futures::channel::mpsc).
    /// This function returns a handle that allows sending a message, or if there is no reference to the handle at all
    /// (in other words, all handles are dropped), the task would be aborted.
    fn spawn_unbounded_coroutine_with_receiver_and_context<T, F, C, Fut>(
        &self,
        context: C,
        mut f: F,
    ) -> UnboundedCommunicationTask<T>
    where
        F: FnMut(C, UnboundedReceiver<T>) -> Fut,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let (tx, rx) = futures::channel::mpsc::unbounded();
        let fut = f(context, rx);
        let _task_handle = self.spawn_abortable(fut);
        UnboundedCommunicationTask {
            _task_handle,
            _channel_tx: tx,
        }
    }

    /// Create a structured-concurrency scope in which tasks may be spawned
    /// that borrow from the enclosing stack frame.
    ///
    /// Unlike [`Executor::spawn`], tasks spawned on the [`Scope`] are driven
    /// cooperatively by the returned future so they may borrow any data that outlives
    /// the `'env` lifetime. Every task is either completed or canceled before
    /// `scope` returns, so borrows never outlive the stack frame.
    ///
    /// This is the async analogue of [`std::thread::scope`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// # async fn run() {
    /// use async_rt::Executor;
    /// use async_rt::global::ConfiguredExecutor;
    ///
    /// let executor: ConfiguredExecutor = ConfiguredExecutor::default();
    /// let data = vec![1, 2, 3, 4];
    /// let sum = executor
    ///     .scope(async |s| {
    ///         let a = s.spawn(async { data[0] + data[1] });
    ///         let b = s.spawn(async { data[2] + data[3] });
    ///         a.await.unwrap() + b.await.unwrap()
    ///     })
    ///     .await;
    /// assert_eq!(sum, 10);
    /// # }
    /// ```
    fn scope<'env, F, T>(&self, f: F) -> impl Future<Output = T>
    where
        F: for<'scope> AsyncFnOnce(&'scope Scope<'scope, 'env>) -> T,
    {
        scoped::scope(f)
    }

    /// Run an async closure with a scoped [`Executor`] wrapper that
    /// forwards spawns to this executor, waits for all spawned tasks
    /// to finish when the closure returns, and aborts any outstanding
    /// tasks if the scope future itself is cancelled.
    ///
    /// Unlike [`Executor::scope`], tasks run on the real executor (so
    /// they get real parallelism) but must be `Send + 'static`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # async fn run() {
    /// use async_rt::Executor;
    /// use async_rt::global::ConfiguredExecutor;
    ///
    /// let executor: ConfiguredExecutor = ConfiguredExecutor::default();
    /// let total = executor
    ///     .executor_scope(async |s| {
    ///         let a = s.spawn(async { 1 + 2 });
    ///         let b = s.spawn(async { 3 + 4 });
    ///         a.await.unwrap() + b.await.unwrap()
    ///     })
    ///     .await;
    /// assert_eq!(total, 10);
    /// # }
    /// ```
    fn executor_scope<'scope, F, T>(&'scope self, f: F) -> impl Future<Output = T>
    where
        Self: Sized,
        F: AsyncFnOnce(&ScopeExecutor<'scope, Self>) -> T,
    {
        scoped::executor_scope(self, f)
    }
}

pub trait ExecutorBlocking: Executor {
    /// Spawn a thread in the background with the ability to concurrently await on the results without
    /// blocking the executor.
    ///
    /// Note that there is no real way to abort the thread, and calling [`JoinHandle::abort`]
    /// would only abort the underlining tasked that is being polled but not the thread itself.
    fn spawn_blocking<F, R>(&self, f: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static;

    /// Spawns a thread in the background, returning an abortable handle that will cancel it
    /// once the handle is dropped.
    ///
    /// Note: This function is used if the task is expected to run until the handle is dropped.
    /// It is recommended to use [`ExecutorBlocking::spawn_blocking`]. Additionally, there is no guarantee
    /// that the thread will cancel or abort.
    fn spawn_blocking_abortable<F, R>(&self, f: F) -> AbortableJoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let handle = self.spawn_blocking(f);
        handle.into()
    }
}

pub trait ExecutorTimeout: Executor {
    /// Spawns a new asynchronous task in the background that must complete within `duration`,
    /// returning a [`JoinHandle`].
    ///
    /// If the future does not finish before `duration` elapses, it is dropped and the task
    /// completes with [`TimeoutError`].
    fn spawn_timeout<F>(
        &self,
        duration: std::time::Duration,
        f: F,
    ) -> JoinHandle<Result<F::Output, TimeoutError>>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.spawn(Timeout::from_future(f, duration).map_err(|_| TimeoutError))
    }

    /// Spawns a task after waiting for a duration before the task is polled.
    fn spawn_delay<F>(&self, duration: std::time::Duration, f: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.spawn(async move {
            let _ = Timeout::from_future(futures::future::pending::<()>(), duration).await;
            f.await
        })
    }

    /// Spawns a new asynchronous task in the background that must complete within `duration`,
    /// returning an abortable handle that will cancel the task once the handle is dropped.
    ///
    /// If the future does not finish before `duration` elapses, it is dropped and the task
    /// completes with [`TimeoutError`].
    fn spawn_abortable_timeout<F>(
        &self,
        duration: std::time::Duration,
        f: F,
    ) -> AbortableJoinHandle<Result<F::Output, TimeoutError>>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.spawn_abortable(Timeout::from_future(f, duration).map_err(|_| TimeoutError))
    }

    /// Spawns an abortable task after waiting for `duration` before the task is polled.
    fn spawn_abortable_delay<F>(
        &self,
        duration: std::time::Duration,
        f: F,
    ) -> AbortableJoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.spawn_abortable(async move {
            let _ = Timeout::from_future(futures::future::pending::<()>(), duration).await;
            f.await
        })
    }
}

pub trait ExecutorBlockOn: Executor {
    /// Blocks the current thread until the provided future has completed.
    ///
    /// Note that calling this function within an executor context may cause a deadlock.
    fn block_on<F: Future>(&self, future: F) -> F::Output;
}

#[cfg(test)]
mod tests {
    use crate::CompletionGuard;
    use crate::error::JoinError;
    use crate::{Executor, ExecutorBlocking, InnerJoinHandle, JoinHandle};
    use futures::future::AbortHandle;
    use pollable_map::optional::Optional;
    use std::future::Future;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;

    async fn task(tx: futures::channel::oneshot::Sender<()>) {
        futures_timer::Delay::new(std::time::Duration::from_secs(5)).await;
        let _ = tx.send(());
        unreachable!();
    }

    #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
    #[tokio::test]
    async fn replacing_handle_wakes_pending_clone() {
        use futures::task::{ArcWake, waker_ref};
        use std::pin::Pin;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::task::Context;

        struct WakeCounter(AtomicUsize);

        impl ArcWake for WakeCounter {
            fn wake_by_ref(arc_self: &Arc<Self>) {
                arc_self.0.fetch_add(1, Ordering::SeqCst);
            }
        }

        let executor = crate::rt::tokio::TokioExecutor;
        let handle = executor.spawn_abortable(futures::future::pending::<usize>());
        let mut pending_clone = handle.clone();

        let wake_counter = Arc::new(WakeCounter(AtomicUsize::new(0)));
        let waker = waker_ref(&wake_counter);
        let mut context = Context::from_waker(&waker);

        assert!(Pin::new(&mut pending_clone).poll(&mut context).is_pending());
        assert_eq!(wake_counter.0.load(Ordering::SeqCst), 0);

        let replacement = executor.spawn_abortable(async { 42 });
        handle.replace(replacement);

        assert!(wake_counter.0.load(Ordering::SeqCst) > 0);
        assert_eq!(pending_clone.await.unwrap(), 42);
    }

    #[test]
    fn custom_abortable_task() {
        struct FuturesExecutor {
            pool: futures::executor::ThreadPool,
        }

        impl Default for FuturesExecutor {
            fn default() -> Self {
                Self {
                    pool: futures::executor::ThreadPool::new().unwrap(),
                }
            }
        }

        impl Executor for FuturesExecutor {
            fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
            where
                F: Future + Send + 'static,
                F::Output: Send + 'static,
            {
                let (abort_handle, abort_registration) = AbortHandle::new_pair();
                let future = crate::abortable_result(future, abort_registration);
                let (tx, rx) = futures::channel::oneshot::channel();

                let fin = Arc::new(AtomicBool::new(false));
                let finished = fin.clone();
                let completion = CompletionGuard::new(fin);
                let fut = async move {
                    let _completion = completion;
                    let val = future.await;
                    let _ = tx.send(val);
                };

                self.pool.spawn_ok(fut);
                let inner = InnerJoinHandle::CustomHandle {
                    inner: Optional::new(rx),
                    handle: abort_handle,
                    finished,
                };

                JoinHandle { inner }
            }
        }

        impl ExecutorBlocking for FuturesExecutor {
            fn spawn_blocking<F, R>(&self, _: F) -> JoinHandle<R>
            where
                F: FnOnce() -> R + Send + 'static,
                R: Send + 'static,
            {
                unimplemented!()
            }
        }

        futures::executor::block_on(async move {
            let executor = FuturesExecutor::default();

            let (tx, rx) = futures::channel::oneshot::channel::<()>();
            let handle = executor.spawn_abortable(task(tx));
            drop(handle);
            let result = rx.await;
            assert!(result.is_err());
        });
    }

    #[test]
    fn empty_handle_reports_empty() {
        let handle = JoinHandle::<()>::empty();

        assert!(matches!(
            futures::executor::block_on(handle),
            Err(JoinError::Empty)
        ));
    }

    #[test]
    fn custom_handle_reports_cancelled_when_sender_dropped() {
        let (tx, rx) = futures::channel::oneshot::channel::<Result<(), JoinError>>();
        let (abort_handle, _abort_registration) = AbortHandle::new_pair();
        let handle = JoinHandle {
            inner: InnerJoinHandle::CustomHandle {
                inner: Optional::new(rx),
                handle: abort_handle,
                finished: Arc::new(AtomicBool::new(false)),
            },
        };

        // Dropping the sender without producing a value mimics a task the
        // executor discarded (e.g. runtime shutdown) rather than an explicit
        // abort, which should surface as `Cancelled` rather than `Aborted`.
        drop(tx);

        assert!(matches!(
            futures::executor::block_on(handle),
            Err(JoinError::Cancelled)
        ));
    }
}
