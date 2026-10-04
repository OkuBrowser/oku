//! Structured concurrency scopes for async tasks.
//!
//! A [`Scope`] lets you spawn futures that borrow from the enclosing stack
//! frame, analogous to [`std::thread::scope`]. Unlike thread scope, the
//! tasks are driven cooperatively by the scope's own future, they make
//! progress whenever the scope is polled so no runtime-level `spawn` is
//! involved and no `unsafe` is needed to extend lifetimes.
//!
//! The [`scope`] function is the entry point. Inside the async closure,
//! call [`Scope::spawn`] to enqueue a task; the returned [`ScopedJoinHandle`]
//! resolves with the task's output. Any tasks still running when the user
//! closure's future completes are drained before `scope` returns, so every
//! borrow is released before the stack frame goes away.

use crate::{
    AbortableJoinHandle, CommunicationTask, CompletionGuard, Executor, ExecutorBlocking,
    ExecutorTimeout, InnerJoinHandle, JoinHandle, TimeoutError, UnboundedCommunicationTask,
    abortable_result, error::JoinError,
};
use core::future::{Future, poll_fn};
use core::marker::PhantomData;
use core::pin::Pin;
use core::task::{Context, Poll};
use futures::channel::mpsc::{Receiver, UnboundedReceiver};
use futures::channel::oneshot;
use futures::future::{AbortHandle, BoxFuture};
use futures::stream::FuturesUnordered;
use futures::task::AtomicWaker;
use futures::{FutureExt, StreamExt, TryFutureExt};
use futures_timeout::Timeout;
use parking_lot::Mutex;
use pollable_map::optional::Optional;
use std::panic::AssertUnwindSafe;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Weak};

struct ScopeState<'scope> {
    inbox: Mutex<Vec<BoxFuture<'scope, ()>>>,
    waker: AtomicWaker,
}

/// A scope within which tasks can be spawned that borrow from the enclosing
/// stack frame.
pub struct Scope<'scope, 'env: 'scope> {
    state: Weak<ScopeState<'scope>>,
    _scope: PhantomData<&'scope mut &'scope ()>,
    _env: PhantomData<&'env mut &'env ()>,
}

impl<'scope, 'env> Scope<'scope, 'env> {
    fn new() -> Self {
        Self {
            state: Weak::new(),
            _scope: PhantomData,
            _env: PhantomData,
        }
    }

    fn push(&self, task: BoxFuture<'scope, ()>) {
        if let Some(state) = self.state.upgrade() {
            state.inbox.lock().push(task);
            state.waker.wake();
        }
    }

    /// Spawn a task into this scope.
    ///
    /// The future may borrow any data that outlives its lifetime `'env`. The task will
    /// be polled cooperatively alongside the scope's user closure and any
    /// other spawned tasks.
    pub fn spawn<Fut>(&'scope self, fut: Fut) -> ScopedJoinHandle<Fut::Output>
    where
        Fut: Future + Send + 'scope,
        Fut::Output: Send + 'scope,
    {
        let (tx, rx) = oneshot::channel();
        let wrapped: BoxFuture<'scope, ()> = async move {
            let output = AssertUnwindSafe(fut)
                .catch_unwind()
                .await
                .map_err(|_| JoinError::Panicked);
            // If the receiver was dropped, the caller doesn't care about
            // the output so we will discard it.
            let _ = tx.send(output);
        }
        .boxed();

        self.push(wrapped);

        ScopedJoinHandle { rx }
    }

    /// Spawn a task into this scope and return an [`AbortableJoinHandle`].
    ///
    /// This mirrors [`Executor::spawn_abortable`] for cooperatively
    /// scheduled tasks. The returned handle aborts the task when all
    /// references to it have been dropped.
    pub fn spawn_abortable<Fut>(&'scope self, fut: Fut) -> AbortableJoinHandle<Fut::Output>
    where
        Fut: Future + Send + 'scope,
        Fut::Output: Send + 'scope,
    {
        let (abort_handle, abort_reg) = AbortHandle::new_pair();
        let abortable = abortable_result(fut, abort_reg);
        let (tx, rx) = oneshot::channel();
        let finished = Arc::new(AtomicBool::new(false));
        let completion = CompletionGuard::new(finished.clone());

        let wrapped: BoxFuture<'scope, ()> = async move {
            let _completion = completion;
            let val = abortable.await;
            let _ = tx.send(val);
        }
        .boxed();
        self.push(wrapped);

        let join = JoinHandle {
            inner: InnerJoinHandle::CustomHandle {
                inner: Optional::new(rx),
                handle: abort_handle,
                finished,
            },
        };
        AbortableJoinHandle::from(join)
    }

    /// Spawn a task into this scope without keeping a handle to it.
    ///
    /// Equivalent to [`Executor::dispatch`] for scoped tasks.
    pub fn dispatch<Fut>(&'scope self, fut: Fut)
    where
        Fut: Future + Send + 'scope,
        Fut::Output: Send + 'scope,
    {
        drop(self.spawn(fut));
    }

    /// Spawn a message-driven coroutine into this scope.
    ///
    /// Equivalent to [`Executor::spawn_coroutine`] for scoped tasks.
    pub fn spawn_coroutine<T, F, Fut>(&'scope self, f: F) -> CommunicationTask<T>
    where
        F: FnMut(T) -> Fut + Send + 'scope,
        Fut: Future<Output = ()> + Send + 'scope,
        T: Send + 'scope,
    {
        self.spawn_coroutine_with_buffer(1, f)
    }

    /// Like [`Scope::spawn_coroutine`] but with a configurable channel
    /// buffer.
    pub fn spawn_coroutine_with_buffer<T, F, Fut>(
        &'scope self,
        buffer: usize,
        mut f: F,
    ) -> CommunicationTask<T>
    where
        F: FnMut(T) -> Fut + Send + 'scope,
        Fut: Future<Output = ()> + Send + 'scope,
        T: Send + 'scope,
    {
        let (tx, mut rx) = futures::channel::mpsc::channel(buffer);
        let task_handle = self.spawn_abortable(async move {
            while let Some(message) = rx.next().await {
                f(message).await;
            }
        });
        CommunicationTask::new(task_handle, tx)
    }

    /// Spawn an unbounded message-driven coroutine into this scope.
    ///
    /// Equivalent to [`Executor::spawn_unbounded_coroutine`] for scoped
    /// tasks.
    pub fn spawn_unbounded_coroutine<T, F, Fut>(
        &'scope self,
        mut f: F,
    ) -> UnboundedCommunicationTask<T>
    where
        F: FnMut(T) -> Fut + Send + 'scope,
        Fut: Future<Output = ()> + Send + 'scope,
        T: Send + 'scope,
    {
        let (tx, mut rx) = futures::channel::mpsc::unbounded();
        let task_handle = self.spawn_abortable(async move {
            while let Some(message) = rx.next().await {
                f(message).await;
            }
        });
        UnboundedCommunicationTask::new(task_handle, tx)
    }

    /// Spawn a message-driven coroutine with caller-provided context.
    ///
    /// If the context must be borrowed across awaits, use
    /// [`Scope::spawn_coroutine_with_receiver_and_context`].
    pub fn spawn_coroutine_with_context<T, C, F, Fut>(
        &'scope self,
        context: C,
        f: F,
    ) -> CommunicationTask<T>
    where
        F: FnMut(&mut C, T) -> Fut + Send + 'scope,
        Fut: Future<Output = ()> + Send + 'scope,
        C: Send + 'scope,
        T: Send + 'scope,
    {
        self.spawn_coroutine_with_buffer_and_context(context, 1, f)
    }

    /// Like [`Scope::spawn_coroutine_with_context`] but with a configurable
    /// channel buffer.
    pub fn spawn_coroutine_with_buffer_and_context<T, C, F, Fut>(
        &'scope self,
        context: C,
        buffer: usize,
        mut f: F,
    ) -> CommunicationTask<T>
    where
        F: FnMut(&mut C, T) -> Fut + Send + 'scope,
        Fut: Future<Output = ()> + Send + 'scope,
        C: Send + 'scope,
        T: Send + 'scope,
    {
        let (tx, mut rx) = futures::channel::mpsc::channel(buffer);
        let task_handle = self.spawn_abortable(async move {
            let mut context = context;
            while let Some(message) = rx.next().await {
                f(&mut context, message).await;
            }
        });
        CommunicationTask::new(task_handle, tx)
    }

    /// Spawn an unbounded message-driven coroutine with caller-provided
    /// context.
    pub fn spawn_unbounded_coroutine_with_context<T, C, F, Fut>(
        &'scope self,
        context: C,
        mut f: F,
    ) -> UnboundedCommunicationTask<T>
    where
        F: FnMut(&mut C, T) -> Fut + Send + 'scope,
        Fut: Future<Output = ()> + Send + 'scope,
        C: Send + 'scope,
        T: Send + 'scope,
    {
        let (tx, mut rx) = futures::channel::mpsc::unbounded();
        let task_handle = self.spawn_abortable(async move {
            let mut context = context;
            while let Some(message) = rx.next().await {
                f(&mut context, message).await;
            }
        });
        UnboundedCommunicationTask::new(task_handle, tx)
    }

    /// Spawn a coroutine that receives the bounded channel directly.
    pub fn spawn_coroutine_with_receiver<T, F, Fut>(&'scope self, f: F) -> CommunicationTask<T>
    where
        F: FnMut(Receiver<T>) -> Fut,
        Fut: Future<Output = ()> + Send + 'scope,
    {
        self.spawn_coroutine_with_receiver_and_buffer(1, f)
    }

    /// Like [`Scope::spawn_coroutine_with_receiver`] but with a configurable
    /// channel buffer.
    pub fn spawn_coroutine_with_receiver_and_buffer<T, F, Fut>(
        &'scope self,
        buffer: usize,
        mut f: F,
    ) -> CommunicationTask<T>
    where
        F: FnMut(Receiver<T>) -> Fut,
        Fut: Future<Output = ()> + Send + 'scope,
    {
        let (tx, rx) = futures::channel::mpsc::channel(buffer);
        let task_handle = self.spawn_abortable(f(rx));
        CommunicationTask::new(task_handle, tx)
    }

    /// Spawn a coroutine that receives caller-provided context and the
    /// bounded channel directly.
    pub fn spawn_coroutine_with_receiver_and_context<T, F, C, Fut>(
        &'scope self,
        context: C,
        f: F,
    ) -> CommunicationTask<T>
    where
        F: FnMut(C, Receiver<T>) -> Fut,
        Fut: Future<Output = ()> + Send + 'scope,
    {
        self.spawn_coroutine_with_receiver_buffer_and_context(context, 1, f)
    }

    /// Like [`Scope::spawn_coroutine_with_receiver_and_context`] but with a
    /// configurable channel buffer.
    pub fn spawn_coroutine_with_receiver_buffer_and_context<T, F, C, Fut>(
        &'scope self,
        context: C,
        buffer: usize,
        mut f: F,
    ) -> CommunicationTask<T>
    where
        F: FnMut(C, Receiver<T>) -> Fut,
        Fut: Future<Output = ()> + Send + 'scope,
    {
        let (tx, rx) = futures::channel::mpsc::channel(buffer);
        let task_handle = self.spawn_abortable(f(context, rx));
        CommunicationTask::new(task_handle, tx)
    }

    /// Spawn a coroutine that receives the unbounded channel directly.
    pub fn spawn_unbounded_coroutine_with_receiver<T, F, Fut>(
        &'scope self,
        mut f: F,
    ) -> UnboundedCommunicationTask<T>
    where
        F: FnMut(UnboundedReceiver<T>) -> Fut,
        Fut: Future<Output = ()> + Send + 'scope,
    {
        let (tx, rx) = futures::channel::mpsc::unbounded();
        let task_handle = self.spawn_abortable(f(rx));
        UnboundedCommunicationTask::new(task_handle, tx)
    }

    /// Spawn a coroutine that receives caller-provided context and the
    /// unbounded channel directly.
    pub fn spawn_unbounded_coroutine_with_receiver_and_context<T, F, C, Fut>(
        &'scope self,
        context: C,
        mut f: F,
    ) -> UnboundedCommunicationTask<T>
    where
        F: FnMut(C, UnboundedReceiver<T>) -> Fut,
        Fut: Future<Output = ()> + Send + 'scope,
    {
        let (tx, rx) = futures::channel::mpsc::unbounded();
        let task_handle = self.spawn_abortable(f(context, rx));
        UnboundedCommunicationTask::new(task_handle, tx)
    }

    /// Spawn a scoped task that must complete within `duration`.
    ///
    /// If the future does not finish before `duration` elapses, it is dropped and the task
    /// completes with [`TimeoutError`]. This is the scoped analogue of
    /// [`ExecutorTimeout::spawn_timeout`].
    pub fn spawn_timeout<F>(
        &'scope self,
        duration: std::time::Duration,
        f: F,
    ) -> ScopedJoinHandle<Result<F::Output, TimeoutError>>
    where
        F: Future + Send + 'scope,
        F::Output: Send + 'scope,
    {
        self.spawn(Timeout::from_future(f, duration).map_err(|_| TimeoutError))
    }

    /// Spawns a task after waiting for a duration before the task is polled.
    pub fn spawn_delay<F>(
        &'scope self,
        duration: std::time::Duration,
        f: F,
    ) -> ScopedJoinHandle<F::Output>
    where
        F: Future + Send + 'scope,
        F::Output: Send + 'scope,
    {
        self.spawn(async move {
            let _ = Timeout::from_future(futures::future::pending::<()>(), duration).await;
            f.await
        })
    }

    /// Spawn a scoped task that must complete within `duration`, returning an
    /// [`AbortableJoinHandle`] that cancels the task once all references to it are dropped.
    ///
    /// If the future does not finish before `duration` elapses, it is dropped and the task
    /// completes with [`TimeoutError`].
    pub fn spawn_abortable_timeout<F>(
        &'scope self,
        duration: std::time::Duration,
        f: F,
    ) -> AbortableJoinHandle<Result<F::Output, TimeoutError>>
    where
        F: Future + Send + 'scope,
        F::Output: Send + 'scope,
    {
        self.spawn_abortable(Timeout::from_future(f, duration).map_err(|_| TimeoutError))
    }

    /// Spawns a task after waiting for a duration before the task is polled.
    pub fn spawn_abortable_delay<F>(
        &'scope self,
        duration: std::time::Duration,
        f: F,
    ) -> AbortableJoinHandle<F::Output>
    where
        F: Future + Send + 'scope,
        F::Output: Send + 'scope,
    {
        self.spawn_abortable(async move {
            let _ = Timeout::from_future(futures::future::pending::<()>(), duration).await;
            f.await
        })
    }
}

/// Drive a scope once: absorb any inbox entries, then poll the active
/// set until no more immediate progress can be made.
fn drive_scope<'scope>(
    active: &mut FuturesUnordered<BoxFuture<'scope, ()>>,
    state: &ScopeState<'scope>,
    cx: &mut Context<'_>,
) -> (bool, bool) {
    let mut made_progress = false;
    loop {
        state.waker.register(cx.waker());

        // Take the queued tasks while holding the lock only long enough to
        // swap the inbox, never while polling user code.
        let incoming = std::mem::take(&mut *state.inbox.lock());
        active.extend(incoming);
        match active.poll_next_unpin(cx) {
            Poll::Ready(Some(())) => made_progress = true,
            Poll::Ready(None) => {
                state.waker.register(cx.waker());
                if state.inbox.lock().is_empty() {
                    return (made_progress, true);
                }
            }
            Poll::Pending => {
                state.waker.register(cx.waker());
                if state.inbox.lock().is_empty() {
                    return (made_progress, false);
                }
            }
        }
    }
}

/// A handle to a task spawned on a [`Scope`].
///
/// Awaiting the handle yields the task's output. A panicking task yields
/// [`JoinError::Panicked`]. If the scope is dropped before the task finishes
/// (for example, because the scope future was cancelled), awaiting yields
/// [`JoinError::Cancelled`].
///
/// The handle does not borrow the spawned future itself. It can therefore
/// outlive the scope when its output type is also `'static`, which borrowed output
/// types retain their normal lifetime restrictions.
pub struct ScopedJoinHandle<T> {
    rx: oneshot::Receiver<Result<T, JoinError>>,
}

impl<T> Future for ScopedJoinHandle<T> {
    type Output = Result<T, JoinError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.rx).poll(cx) {
            Poll::Ready(Ok(result)) => Poll::Ready(result),
            Poll::Ready(Err(_)) => Poll::Ready(Err(JoinError::Cancelled)),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Create a scope for spawning tasks that borrow from the enclosing stack
/// frame.
///
/// The provided async closure receives a reference to the [`Scope`] and
/// returns a future. That future is driven alongside any tasks spawned on
/// the scope; when it completes, any still-running spawned tasks are
/// drained before `scope` returns its value.
///
/// # Example
///
/// ```no_run
/// # async fn run() {
/// use async_rt::scoped::scope;
///
/// let data = vec![1, 2, 3, 4];
/// let data = &data;
/// let sum = scope(async |s| {
///     let a = s.spawn(async { data[0] + data[1] });
///     let b = s.spawn(async { data[2] + data[3] });
///     a.await.unwrap() + b.await.unwrap()
/// })
/// .await;
/// assert_eq!(sum, 10);
/// # }
/// ```
pub async fn scope<'env, F, T>(f: F) -> T
where
    F: for<'scope> AsyncFnOnce(&'scope Scope<'scope, 'env>) -> T,
{
    // Declaration order is part of the safety invariant: `active` is dropped
    // first, then `state` (and its queued tasks), and finally `scope`.
    let mut scope = Scope::new();
    let state = Arc::new(ScopeState {
        inbox: Mutex::new(Vec::new()),
        waker: AtomicWaker::new(),
    });
    scope.state = Arc::downgrade(&state);
    // The active task set lives on the driver, not inside `Scope`. Only
    // `drive_scope` touches it, so no lock guards it, and the inbox mutex
    // is enough for the spawn side.
    let mut active = FuturesUnordered::new();

    let result = {
        let user_fut = f(&scope);
        let mut user_fut = std::pin::pin!(user_fut);

        poll_fn(|cx| {
            loop {
                if let Poll::Ready(r) = user_fut.as_mut().poll(cx) {
                    return Poll::Ready(r);
                }
                let (made_progress, _empty) = drive_scope(&mut active, &state, cx);
                if !made_progress {
                    return Poll::Pending;
                }
            }
        })
        .await
    };

    // Drain any remaining spawned tasks before the scope goes away, so
    // every borrow of the data is released before this frame unwinds.
    poll_fn(|cx| {
        let (_made_progress, empty) = drive_scope(&mut active, &state, cx);
        if empty {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    })
    .await;

    result
}

/// An [`Executor`] wrapper that tracks spawned tasks so they can be
/// canceled when a scope ends.
///
/// `ScopeExecutor` itself implements [`Executor`], so it can be passed to
/// any code that takes `Executor` every spawn performed through
/// that code will be tracked by the scope.
///
/// Because submitted futures must still be `Send + 'static` (enforced by
/// [`Executor::spawn`]), this flavour of scope does *not* allow borrowing
/// from the enclosing stack frame. Use [`scope`] for that.
pub struct ScopeExecutor<'scope, E> {
    inner: &'scope E,
    task_handles: Mutex<Vec<AbortableJoinHandle<()>>>,
    _scope: PhantomData<&'scope mut &'scope ()>,
}

impl<'scope, E> ScopeExecutor<'scope, E> {
    fn new(inner: &'scope E) -> Self {
        Self {
            inner,
            task_handles: Mutex::new(Vec::new()),
            _scope: PhantomData,
        }
    }

    /// Abort every task currently tracked by this scope.
    fn abort_all(&self) {
        for handle in self.task_handles.lock().iter() {
            handle.abort();
        }
    }
}

impl<E> ScopeExecutor<'_, E>
where
    E: Executor,
{
    fn spawn_tracked<F, T>(&self, future: F) -> JoinHandle<T>
    where
        F: Future<Output = Result<T, JoinError>> + Send + 'static,
        T: Send + 'static,
    {
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        let (tx, rx) = oneshot::channel();
        let finished = Arc::new(AtomicBool::new(false));
        let completion = CompletionGuard::new(finished.clone());
        let wrapped = async move {
            let _completion = completion;
            let result = abortable_result(future, abort_registration)
                .await
                .and_then(|result| result);
            let _ = tx.send(result);
        };

        // Track an abort-on-drop handle so cancellation remains effective even
        // after the handles are drained from `ScopeExecutor` for joining.
        let task_handle = self.inner.spawn_abortable(wrapped);
        self.task_handles.lock().push(task_handle);

        JoinHandle {
            inner: InnerJoinHandle::CustomHandle {
                inner: Optional::new(rx),
                handle: abort_handle,
                finished,
            },
        }
    }
}

impl<E> Drop for ScopeExecutor<'_, E> {
    fn drop(&mut self) {
        self.abort_all();
    }
}

impl<'scope, E> Executor for ScopeExecutor<'scope, E>
where
    E: Executor,
{
    fn runtime_type(&self) -> Option<&'static str> {
        self.inner.runtime_type()
    }

    fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.spawn_tracked(async move { Ok(future.await) })
    }
}

impl<E> ExecutorBlocking for ScopeExecutor<'_, E>
where
    E: ExecutorBlocking,
{
    fn spawn_blocking<F, R>(&self, f: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let blocking_handle = self.inner.spawn_blocking_abortable(f);
        // Note that the monitor owns the blocking handle, so aborting the monitor drops its
        // abort-on-drop handle, which prevents queued work from starting when
        // the backend supports that. Work already running may continue.
        self.spawn_tracked(blocking_handle)
    }
}

impl<E> ExecutorTimeout for ScopeExecutor<'_, E> where E: Executor {}

/// Run an async closure with a scoped [`Executor`] wrapper.
///
/// If the future containing `executor_scope` itself is dropped
/// (canceled externally), it will abort every outstanding task.
///
/// Unlike [`scope`], futures spawned here must be `Send + 'static`
/// they're going onto the real executor, so they can't borrow from the
/// enclosing stack frame.
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
///     assert_eq!(total, 10);
/// # }
/// ```
pub async fn executor_scope<'scope, E, F, T>(executor: &'scope E, f: F) -> T
where
    E: Executor,
    F: AsyncFnOnce(&ScopeExecutor<'scope, E>) -> T,
{
    let scope_exec = ScopeExecutor::new(executor);
    let result = f(&scope_exec).await;

    // Wait for every spawned task to complete
    let handles: Vec<_> = scope_exec.task_handles.lock().drain(..).collect();
    for handle in handles {
        let _ = handle.await;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "tokio")]
    use futures_timer::Delay;
    #[cfg(feature = "tokio")]
    use std::time::Duration;

    #[tokio::test]
    async fn borrows_stack_data() {
        let data = vec![1, 2, 3, 4];
        let data = &data;
        let sum = scope(async |s: &Scope<'_, '_>| {
            let a = s.spawn(async move { data[0] + data[1] });
            let b = s.spawn(async move { data[2] + data[3] });
            a.await.unwrap() + b.await.unwrap()
        })
        .await;
        assert_eq!(sum, 10);
    }

    #[tokio::test]
    async fn drains_unawaited_tasks() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let counter = AtomicUsize::new(0);
        let counter_ref = &counter;
        scope(async |s: &Scope<'_, '_>| {
            for _ in 0..8 {
                s.spawn(async move {
                    counter_ref.fetch_add(1, Ordering::SeqCst);
                });
            }
        })
        .await;
        assert_eq!(counter.load(Ordering::SeqCst), 8);
    }

    #[tokio::test]
    async fn returns_closure_value() {
        let v: i32 = scope(async |_s: &Scope<'_, '_>| 42).await;
        assert_eq!(v, 42);
    }

    #[tokio::test]
    async fn join_handle_yields_output() {
        let out = scope(async |s: &Scope<'_, '_>| {
            let h = s.spawn(async { "hello" });
            h.await.unwrap()
        })
        .await;
        assert_eq!(out, "hello");
    }

    #[cfg(panic = "unwind")]
    #[tokio::test]
    async fn join_handle_reports_task_panic() {
        let result = scope(async |s: &Scope<'_, '_>| {
            s.spawn(async { panic!("expected scoped task panic") })
                .await
        })
        .await;

        assert!(matches!(result, Err(JoinError::Panicked)));
    }

    #[cfg(panic = "unwind")]
    #[tokio::test]
    async fn unawaited_task_panic_does_not_stop_other_tasks() {
        let completed = AtomicBool::new(false);

        scope(async |s: &Scope<'_, '_>| {
            s.spawn(async { panic!("expected unawaited scoped task panic") });
            s.spawn(async {
                completed.store(true, std::sync::atomic::Ordering::SeqCst);
            });
        })
        .await;

        assert!(completed.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[tokio::test]
    async fn abortable_handle_reports_completion_without_being_polled() {
        scope(async |s: &Scope<'_, '_>| {
            let handle = s.spawn_abortable(async {});

            // Yield the user future so the cooperative driver can complete
            // the child without polling its join handle.
            crate::task::yield_now().await;

            assert!(handle.is_finished());
        })
        .await;
    }

    #[test]
    fn pushing_task_wakes_scope_driver() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::task::{Wake, Waker};

        struct WakeCounter(AtomicUsize);

        impl Wake for WakeCounter {
            fn wake(self: Arc<Self>) {
                self.0.fetch_add(1, Ordering::SeqCst);
            }

            fn wake_by_ref(self: &Arc<Self>) {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }

        let mut scope = Scope::new();
        let state = Arc::new(ScopeState {
            inbox: Mutex::new(Vec::new()),
            waker: AtomicWaker::new(),
        });
        scope.state = Arc::downgrade(&state);

        let wake_counter = Arc::new(WakeCounter(AtomicUsize::new(0)));
        let waker = Waker::from(wake_counter.clone());
        state.waker.register(&waker);

        let scope_ref = &scope;
        scope.push(
            async move {
                // Consume the driver's registered waker while tasks are
                // being polled, forcing drive_scope to register it again.
                scope_ref.push(async {}.boxed());
            }
            .boxed(),
        );

        assert_eq!(state.inbox.lock().len(), 1);
        assert_eq!(wake_counter.0.load(Ordering::SeqCst), 1);

        let mut active = FuturesUnordered::new();
        let mut context = Context::from_waker(&waker);
        let (_, empty) = drive_scope(&mut active, &state, &mut context);
        assert!(empty);
        assert_eq!(wake_counter.0.load(Ordering::SeqCst), 2);

        // The nested push consumed the previous registration. This push
        // only wakes the driver if drive_scope registered again afterward.
        scope.push(async {}.boxed());
        assert_eq!(wake_counter.0.load(Ordering::SeqCst), 3);

        let (_, empty) = drive_scope(&mut active, &state, &mut context);
        assert!(empty);

        let state_ref = Arc::downgrade(&state);
        scope.push(
            poll_fn(move |_cx| {
                // Model a push that occurs after registration but before
                // the inbox is drained: its wake is consumed, then the
                // newly active task returns Pending.
                state_ref.upgrade().unwrap().waker.wake();
                Poll::<()>::Pending
            })
            .boxed(),
        );
        assert_eq!(wake_counter.0.load(Ordering::SeqCst), 4);

        let (_, empty) = drive_scope(&mut active, &state, &mut context);
        assert!(!empty);
        let wakes_after_pending = wake_counter.0.load(Ordering::SeqCst);
        assert!(wakes_after_pending > 4);

        // The Pending path must have registered once more before returning.
        scope.push(async {}.boxed());
        assert_eq!(
            wake_counter.0.load(Ordering::SeqCst),
            wakes_after_pending + 1
        );
    }

    #[tokio::test]
    async fn many_concurrent_tasks_complete() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let counter = AtomicUsize::new(0);
        let counter_ref = &counter;
        let total: usize = scope(async |s: &Scope<'_, '_>| {
            let handles: Vec<_> = (0..32)
                .map(|i| {
                    s.spawn(async move {
                        counter_ref.fetch_add(1, Ordering::SeqCst);
                        i
                    })
                })
                .collect();
            let mut sum = 0usize;
            for h in handles {
                sum += h.await.unwrap();
            }
            sum
        })
        .await;
        assert_eq!(total, (0..32).sum());
        assert_eq!(counter.load(Ordering::SeqCst), 32);
    }

    #[tokio::test]
    async fn child_task_can_spawn_nested_task() {
        let result = scope(async |s: &Scope<'_, '_>| {
            let outer = s.spawn(async move {
                let inner = s.spawn(async { 41usize });
                inner.await.unwrap() + 1
            });
            outer.await.unwrap()
        })
        .await;

        assert_eq!(result, 42);
    }

    #[tokio::test]
    async fn drains_unawaited_nested_task() {
        use std::sync::atomic::{AtomicBool, Ordering};

        let nested_ran = AtomicBool::new(false);
        let nested_ran_ref = &nested_ran;

        scope(async |s: &Scope<'_, '_>| {
            s.dispatch(async move {
                s.dispatch(async move {
                    nested_ran_ref.store(true, Ordering::SeqCst);
                });
            });
        })
        .await;

        assert!(nested_ran.load(Ordering::SeqCst));
    }

    #[cfg(feature = "tokio")]
    #[tokio::test]
    async fn executor_scope_runs_tasks() {
        use crate::rt::tokio::TokioExecutor;
        let executor = TokioExecutor;
        let total = executor
            .executor_scope(async |s| {
                let a = s.spawn(async { 1 + 2 });
                let b = s.spawn(async { 3 + 4 });
                a.await.unwrap() + b.await.unwrap()
            })
            .await;
        assert_eq!(total, 10);
    }

    #[cfg(feature = "tokio")]
    #[tokio::test]
    async fn executor_scope_supports_timeouts() {
        use crate::rt::tokio::TokioExecutor;
        use futures::future::pending;

        let executor = TokioExecutor;
        executor
            .executor_scope(async |s| {
                let timeout = s.spawn_timeout(Duration::from_millis(10), pending::<()>());
                assert!(matches!(timeout.await, Ok(Err(TimeoutError))));

                let abortable_timeout =
                    s.spawn_abortable_timeout(Duration::from_millis(10), pending::<()>());
                assert!(matches!(abortable_timeout.await, Ok(Err(TimeoutError))));
            })
            .await;
    }

    #[cfg(feature = "tokio")]
    #[tokio::test]
    async fn executor_scope_supports_blocking_tasks() {
        use crate::rt::tokio::TokioExecutor;

        let executor = TokioExecutor;
        executor
            .executor_scope(async |s| {
                assert_eq!(s.spawn_blocking(|| 42).await.unwrap(), 42);

                let panicked = s
                    .spawn_blocking(|| -> () { panic!("deliberate blocking task panic") })
                    .await;
                assert!(matches!(panicked, Err(JoinError::Panicked)));
            })
            .await;
    }

    #[cfg(feature = "tokio")]
    #[tokio::test]
    async fn executor_scope_drains_unawaited_blocking_tasks() {
        use crate::rt::tokio::TokioExecutor;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let executor = TokioExecutor;
        let completed = Arc::new(AtomicBool::new(false));
        let task_completed = completed.clone();

        executor
            .executor_scope(async move |s| {
                let _handle = s.spawn_blocking(move || {
                    std::thread::sleep(Duration::from_millis(25));
                    task_completed.store(true, Ordering::SeqCst);
                });
            })
            .await;

        assert!(completed.load(Ordering::SeqCst));
    }

    #[cfg(feature = "tokio")]
    #[tokio::test(flavor = "current_thread")]
    async fn executor_scope_blocking_abort_is_reported() {
        use crate::rt::tokio::TokioExecutor;
        use futures::future::{Either, select};

        let executor = TokioExecutor;
        executor
            .executor_scope(async |s| {
                let (started_tx, started_rx) = oneshot::channel();
                let (release_tx, release_rx) = std::sync::mpsc::channel();
                let handle = s.spawn_blocking(move || {
                    let _ = started_tx.send(());
                    let _ = release_rx.recv();
                });

                started_rx.await.unwrap();
                handle.abort();

                let handle = Box::pin(handle);
                let result = match select(handle, Delay::new(Duration::from_secs(1))).await {
                    Either::Left((result, _)) => result,
                    Either::Right((_, handle)) => {
                        release_tx.send(()).unwrap();
                        let _ = handle.await;
                        panic!("aborting the blocking handle did not cancel its monitor");
                    }
                };

                release_tx.send(()).unwrap();
                assert!(matches!(result, Err(JoinError::Aborted)));
            })
            .await;
    }

    #[cfg(feature = "tokio")]
    #[tokio::test(flavor = "current_thread")]
    async fn executor_scope_handle_reports_completion_without_being_polled() {
        use crate::rt::tokio::TokioExecutor;

        let executor = TokioExecutor;
        executor
            .executor_scope(async |s| {
                let (completed_tx, completed_rx) = oneshot::channel();
                let handle = s.spawn(async move {
                    let _ = completed_tx.send(());
                });

                completed_rx.await.unwrap();

                // On the current-thread runtime, the spawned wrapper finishes
                // before the task it woke can be polled again.
                assert!(handle.is_finished());
            })
            .await;
    }

    #[tokio::test]
    async fn scope_spawn_coroutine_receives_messages() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let total = AtomicUsize::new(0);
        let total_ref = &total;

        scope(async |s: &Scope<'_, '_>| {
            let mut task = s.spawn_coroutine(|value| async move {
                total_ref.fetch_add(value, Ordering::SeqCst);
            });
            for v in [1usize, 2, 3, 4] {
                task.send(v).await.unwrap();
            }
            drop(task); // closes channel → coroutine exits cleanly
        })
        .await;

        assert_eq!(total.load(Ordering::SeqCst), 10);
    }

    #[tokio::test]
    async fn scope_receiver_coroutine_receives_messages() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let total = AtomicUsize::new(0);
        let total_ref = &total;

        scope(async |s: &Scope<'_, '_>| {
            let mut task = s.spawn_coroutine_with_receiver(|mut rx| async move {
                while let Some(value) = rx.next().await {
                    total_ref.fetch_add(value, Ordering::SeqCst);
                }
            });
            for value in [1usize, 2, 3, 4] {
                task.send(value).await.unwrap();
            }
            drop(task);
        })
        .await;

        assert_eq!(total.load(Ordering::SeqCst), 10);
    }

    #[tokio::test]
    async fn scope_coroutine_api_matches_executor() {
        use futures::future::ready;

        scope(async |s: &Scope<'_, '_>| {
            let task = s.spawn_coroutine_with_buffer(2, |_value: usize| ready(()));
            drop(task);

            let task = s.spawn_unbounded_coroutine(|_value: usize| ready(()));
            drop(task);

            let task =
                s.spawn_coroutine_with_context(0usize, |context: &mut usize, value: usize| {
                    *context += value;
                    ready(())
                });
            drop(task);

            let task = s.spawn_coroutine_with_buffer_and_context(
                0usize,
                2,
                |context: &mut usize, value: usize| {
                    *context += value;
                    ready(())
                },
            );
            drop(task);

            let task = s.spawn_unbounded_coroutine_with_context(
                0usize,
                |context: &mut usize, value: usize| {
                    *context += value;
                    ready(())
                },
            );
            drop(task);

            let task =
                s.spawn_coroutine_with_receiver_and_buffer(2, |_rx: Receiver<usize>| async {});
            drop(task);

            let task = s.spawn_coroutine_with_receiver_and_context(
                0usize,
                |_context, _rx: Receiver<usize>| async {},
            );
            drop(task);

            let task = s.spawn_coroutine_with_receiver_buffer_and_context(
                0usize,
                2,
                |_context, _rx: Receiver<usize>| async {},
            );
            drop(task);

            let task =
                s.spawn_unbounded_coroutine_with_receiver(|_rx: UnboundedReceiver<usize>| async {});
            drop(task);

            let task = s.spawn_unbounded_coroutine_with_receiver_and_context(
                0usize,
                |_context, _rx: UnboundedReceiver<usize>| async {},
            );
            drop(task);
        })
        .await;
    }

    #[tokio::test]
    async fn scope_dispatch_runs_fire_and_forget() {
        use std::sync::atomic::{AtomicBool, Ordering};
        let flag = AtomicBool::new(false);
        let flag_ref = &flag;

        scope(async |s: &Scope<'_, '_>| {
            s.dispatch(async move {
                flag_ref.store(true, Ordering::SeqCst);
            });
        })
        .await;

        assert!(flag.load(Ordering::SeqCst));
    }

    #[cfg(feature = "tokio")]
    #[tokio::test]
    async fn executor_scope_drains_unawaited_tasks() {
        use crate::rt::tokio::TokioExecutor;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let executor = TokioExecutor;
        let flag = Arc::new(AtomicBool::new(false));
        let flag_clone = flag.clone();

        executor
            .executor_scope(async move |s| {
                // Spawn a task but do NOT await its handle.
                // executor_scope should wait for it to complete before
                // returning.
                let _h = s.spawn(async move {
                    Delay::new(Duration::from_millis(50)).await;
                    flag_clone.store(true, Ordering::SeqCst);
                });
            })
            .await;

        assert!(
            flag.load(Ordering::SeqCst),
            "unawaited task should have completed before executor_scope returned"
        );
    }

    #[cfg(feature = "tokio")]
    #[tokio::test]
    async fn executor_scope_swallows_task_panic() {
        use crate::rt::tokio::TokioExecutor;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let executor = TokioExecutor;
        let sibling_done = Arc::new(AtomicUsize::new(0));
        let sibling_done_clone = sibling_done.clone();

        let result = executor
            .executor_scope(async move |s| {
                // Task A: panics. Its JoinHandle is dropped without
                // being awaited. the panic goes to the runtime.
                let _panicker = s.spawn(async {
                    panic!("deliberate test panic");
                });
                // Task B: completes normally. The scope should still
                // drain it before returning.
                let _sibling = s.spawn(async move {
                    sibling_done_clone.fetch_add(1, Ordering::SeqCst);
                });
                42usize
            })
            .await;

        assert_eq!(result, 42);
        assert_eq!(sibling_done.load(Ordering::SeqCst), 1);
    }

    #[cfg(feature = "tokio")]
    #[tokio::test]
    async fn executor_scope_aborts_on_external_cancel() {
        use crate::rt::tokio::TokioExecutor;
        use futures::future::Either;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let executor = TokioExecutor;
        let flag = Arc::new(AtomicBool::new(false));
        let flag_clone = flag.clone();

        {
            let scope_fut = executor.executor_scope(async move |s| {
                let _h = s.spawn(async move {
                    Delay::new(Duration::from_millis(200)).await;
                    flag_clone.store(true, Ordering::SeqCst);
                });
                futures::future::pending::<()>().await;
            });

            let scope_fut = std::pin::pin!(scope_fut);
            let timer = std::pin::pin!(Delay::new(Duration::from_millis(30)));
            let result = futures::future::select(scope_fut, timer).await;
            assert!(
                matches!(result, Either::Right(_)),
                "timer should have won the race"
            );
        }

        // Give the (supposedly aborted) task plenty of time.
        Delay::new(Duration::from_millis(300)).await;
        assert!(
            !flag.load(Ordering::SeqCst),
            "task should have been aborted by scope drop"
        );
    }

    #[cfg(feature = "tokio")]
    #[tokio::test]
    async fn executor_scope_aborts_when_cancelled_during_drain() {
        use crate::rt::tokio::TokioExecutor;
        use futures::future::{Either, pending, select};

        let executor = TokioExecutor;
        let (started_tx, started_rx) = oneshot::channel();
        let (held_tx, held_rx) = oneshot::channel::<()>();

        let scope_fut = Box::pin(executor.executor_scope(async move |s| {
            let _handle = s.spawn(async move {
                let _held_until_task_drop = held_tx;
                let _ = started_tx.send(());
                pending::<()>().await;
            });
        }));

        let scope_fut = match select(scope_fut, started_rx).await {
            Either::Right((Ok(()), scope_fut)) => scope_fut,
            Either::Left(_) => panic!("scope unexpectedly completed"),
            Either::Right((Err(_), _)) => panic!("child task never started"),
        };

        drop(scope_fut);

        match select(held_rx, Delay::new(Duration::from_secs(1))).await {
            Either::Left((Err(_), _)) => {}
            Either::Left((Ok(_), _)) => unreachable!("child never sends a value"),
            Either::Right(_) => panic!("child remained detached after scope cancellation"),
        }
    }
}
