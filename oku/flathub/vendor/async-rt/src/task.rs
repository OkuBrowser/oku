use crate::error::TimeoutError;
use crate::global::BuiltinExecutor;
use crate::{
    AbortableJoinHandle, CommunicationTask, Executor, ExecutorBlockOn, ExecutorBlocking,
    ExecutorTimeout, JoinHandle, Scope, ScopeExecutor, UnboundedCommunicationTask,
};
use futures::channel::mpsc::{Receiver, UnboundedReceiver};
use parking_lot::{Condvar, Mutex};
use std::sync::LazyLock;

struct ExecutorState {
    executor: BuiltinExecutor,
    active: usize,
}

struct ExecutorLock {
    state: Mutex<ExecutorState>,
    available: Condvar,
}

static EXECUTOR: LazyLock<ExecutorLock> = LazyLock::new(|| ExecutorLock {
    state: Mutex::new(ExecutorState {
        executor: BuiltinExecutor::default(),
        active: 0,
    }),
    available: Condvar::new(),
});

pub(crate) fn executor() -> BuiltinExecutor {
    EXECUTOR.state.lock().executor
}

pub(crate) struct ExecutorGuard {
    _private: (),
}

impl Drop for ExecutorGuard {
    fn drop(&mut self) {
        let mut state = EXECUTOR.state.lock();
        state.active -= 1;
        if state.active == 0 {
            EXECUTOR.available.notify_all();
        }
    }
}

pub(crate) fn set_executor(executor: BuiltinExecutor) -> ExecutorGuard {
    let mut state = EXECUTOR.state.lock();
    while state.active != 0 && (state.executor != executor || executor.is_exclusive()) {
        EXECUTOR.available.wait(&mut state);
    }
    if state.active == 0 {
        state.executor = executor;
    }
    state.active += 1;
    ExecutorGuard { _private: () }
}

/// Returns an optional runtime name of the executor.
pub fn runtime_type() -> Option<&'static str> {
    executor().runtime_type()
}

/// Spawns a new asynchronous task in the background, returning a Future [`JoinHandle`] for it.
pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    executor().spawn(future)
}

pub fn spawn_blocking<F, T>(future: F) -> JoinHandle<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    executor().spawn_blocking(future)
}

/// Spawns a new asynchronous task in the background, returning an abortable handle that will cancel the task
/// once the handle is dropped.
///
/// Note: This function is used if the task is expected to run until the handle is dropped. It is recommended to use
/// [`spawn`] or [`dispatch`] otherwise.
pub fn spawn_abortable<F>(future: F) -> AbortableJoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    executor().spawn_abortable(future)
}

/// Spawns a new asynchronous task that must complete within `duration`.
///
/// If it does not, the future is dropped and the task completes with [`TimeoutError`].
pub fn spawn_timeout<F>(
    duration: std::time::Duration,
    future: F,
) -> JoinHandle<Result<F::Output, TimeoutError>>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    executor().spawn_timeout(duration, future)
}

/// Spawns a task after waiting for a duration before the task is polled.
pub fn spawn_delay<F>(duration: std::time::Duration, future: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    executor().spawn_delay(duration, future)
}

/// Spawns a new asynchronous task, returning an abortable handle, that must complete within
/// `duration`.
///
/// If it does not, the future is dropped and the task completes with [`TimeoutError`].
pub fn spawn_abortable_timeout<F>(
    duration: std::time::Duration,
    future: F,
) -> AbortableJoinHandle<Result<F::Output, TimeoutError>>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    executor().spawn_abortable_timeout(duration, future)
}

/// Spawns a task after waiting for a duration before the task is polled.
pub fn spawn_abortable_delay<F>(
    duration: std::time::Duration,
    future: F,
) -> AbortableJoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    executor().spawn_abortable_delay(duration, future)
}

/// Spawns a new asynchronous task in the background without a handle.
/// Basically the same as [`spawn`].
pub fn dispatch<F>(future: F)
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    executor().dispatch(future);
}

/// Spawns a new asynchronous task that accepts messages to the task.
/// This function returns a handle that allows sending a message, or if there is no reference to the handle at all
/// (in other words, all handles are dropped), the task would be aborted.
pub fn spawn_coroutine<T, F, Fut>(f: F) -> CommunicationTask<T>
where
    F: FnMut(T) -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
    T: Send + 'static,
{
    executor().spawn_coroutine(f)
}

/// Spawns a new asynchronous task that accepts messages to the task with a set buffer.
/// This function returns a handle that allows sending a message, or if there is no reference to the handle at all
/// (in other words, all handles are dropped), the task would be aborted.
pub fn spawn_coroutine_with_buffer<T, F, Fut>(buffer: usize, f: F) -> CommunicationTask<T>
where
    F: FnMut(T) -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
    T: Send + 'static,
{
    executor().spawn_coroutine_with_buffer(buffer, f)
}

/// Spawns a new asynchronous task that accepts unbounded messages to the task.
/// This function returns a handle that allows sending a message, or if there is no reference to the handle at all
/// (in other words, all handles are dropped), the task would be aborted.
pub fn spawn_unbounded_coroutine<T, F, Fut>(f: F) -> UnboundedCommunicationTask<T>
where
    F: FnMut(T) -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
    T: Send + 'static,
{
    executor().spawn_unbounded_coroutine(f)
}

/// Spawns a new asynchronous task with provided context that accepts messages to the task.
/// This function returns a handle that allows sending a message, or if there is no reference to the handle at all
/// (in other words, all handles are dropped), the task would be aborted.
///
/// # Note
/// If state must be borrowed across awaits,
/// use [`spawn_coroutine_with_receiver_and_context`].
pub fn spawn_coroutine_with_context<T, C, F, Fut>(context: C, f: F) -> CommunicationTask<T>
where
    F: FnMut(&mut C, T) -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
    C: Send + 'static,
    T: Send + 'static,
{
    executor().spawn_coroutine_with_context(context, f)
}

/// Spawns a new asynchronous task with provided context that accepts messages to the task with a set buffer.
/// This function returns a handle that allows sending a message, or if there is no reference to the handle at all
/// (in other words, all handles are dropped), the task would be aborted.
pub fn spawn_coroutine_with_buffer_and_context<T, C, F, Fut>(
    context: C,
    buffer: usize,
    f: F,
) -> CommunicationTask<T>
where
    F: FnMut(&mut C, T) -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
    C: Send + 'static,
    T: Send + 'static,
{
    executor().spawn_coroutine_with_buffer_and_context(context, buffer, f)
}

/// Spawns a new asynchronous task with provided context that accepts unbounded messages to the task.
/// This function returns a handle that allows sending a message, or if there is no reference to the handle at all
/// (in other words, all handles are dropped), the task would be aborted.
pub fn spawn_unbounded_coroutine_with_context<T, C, F, Fut>(
    context: C,
    f: F,
) -> UnboundedCommunicationTask<T>
where
    F: FnMut(&mut C, T) -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
    C: Send + 'static,
    T: Send + 'static,
{
    executor().spawn_unbounded_coroutine_with_context(context, f)
}

/// Spawns a new asynchronous task that accepts messages to the task using [`channels`](futures::channel::mpsc).
/// This function returns a handle that allows sending a message, or if there is no reference to the handle at all
/// (in other words, all handles are dropped), the task would be aborted.
pub fn spawn_coroutine_with_receiver<T, F, Fut>(f: F) -> CommunicationTask<T>
where
    F: FnMut(Receiver<T>) -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
{
    executor().spawn_coroutine_with_receiver(f)
}

/// Spawns a new asynchronous task with a set channel buffer that accepts messages to the task using [`channels`](futures::channel::mpsc).
/// This function returns a handle that allows sending a message, or if there is no reference to the handle at all
/// (in other words, all handles are dropped), the task would be aborted.
pub fn spawn_coroutine_with_receiver_and_buffer<T, F, Fut>(
    buffer: usize,
    f: F,
) -> CommunicationTask<T>
where
    F: FnMut(Receiver<T>) -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
{
    executor().spawn_coroutine_with_receiver_and_buffer(buffer, f)
}

/// Spawns a new asynchronous task with provided context that accepts messages to the task using [`channels`](futures::channel::mpsc).
/// This function returns a handle that allows sending a message, or if there is no reference to the handle at all
/// (in other words, all handles are dropped), the task would be aborted.
pub fn spawn_coroutine_with_receiver_and_context<T, F, C, Fut>(
    context: C,
    f: F,
) -> CommunicationTask<T>
where
    F: FnMut(C, Receiver<T>) -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
{
    executor().spawn_coroutine_with_receiver_and_context(context, f)
}

/// Spawns a new asynchronous task with a set channel buffer and provided context that accepts messages to the task using [`channels`](futures::channel::mpsc).
/// This function returns a handle that allows sending a message, or if there is no reference to the handle at all
/// (in other words, all handles are dropped), the task would be aborted.
pub fn spawn_coroutine_with_receiver_buffer_and_context<T, F, C, Fut>(
    context: C,
    buffer: usize,
    f: F,
) -> CommunicationTask<T>
where
    F: FnMut(C, Receiver<T>) -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
{
    executor().spawn_coroutine_with_receiver_buffer_and_context(context, buffer, f)
}

/// Spawns a new asynchronous task that accepts messages to the task using [`channels`](futures::channel::mpsc).
/// This function returns a handle that allows sending a message, or if there is no reference to the handle at all
/// (in other words, all handles are dropped), the task would be aborted.
pub fn spawn_unbounded_coroutine_with_receiver<T, F, Fut>(f: F) -> UnboundedCommunicationTask<T>
where
    F: FnMut(UnboundedReceiver<T>) -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
{
    executor().spawn_unbounded_coroutine_with_receiver(f)
}

/// Spawns a new asynchronous task with provided context that accepts messages to the task using [`channels`](futures::channel::mpsc).
/// This function returns a handle that allows sending a message, or if there is no reference to the handle at all
/// (in other words, all handles are dropped), the task would be aborted.
pub fn spawn_unbounded_coroutine_with_receiver_and_context<T, F, C, Fut>(
    context: C,
    f: F,
) -> UnboundedCommunicationTask<T>
where
    F: FnMut(C, UnboundedReceiver<T>) -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
{
    executor().spawn_unbounded_coroutine_with_receiver_and_context(context, f)
}

/// Create a structured-concurrency scope in which tasks may be spawned
/// that borrow from the enclosing stack frame.
///
/// This is the async analogue of [`std::thread::scope`].
pub fn scope<'env, F, T>(f: F) -> impl Future<Output = T>
where
    F: for<'scope> AsyncFnOnce(&'scope Scope<'scope, 'env>) -> T,
{
    crate::scoped::scope(f)
}

/// Run an async closure with a scoped [`Executor`] wrapper that
/// forwards spawns to this executor, waits for all spawned tasks
/// to finish when the closure returns, and aborts any outstanding
/// tasks if the scope future itself is cancelled.
pub fn executor_scope<F, T>(f: F) -> impl Future<Output = T>
where
    F: for<'scope> AsyncFnOnce(&ScopeExecutor<'scope, BuiltinExecutor>) -> T,
{
    async move {
        let executor = executor();
        executor.executor_scope(f).await
    }
}

/// Blocks the current thread until the provided future has completed.
///
/// Note that calling this function within an executor context may cause a deadlock.
pub fn block_on<F: Future>(f: F) -> F::Output {
    executor().block_on(f)
}

#[cfg(not(all(feature = "tokio", not(target_arch = "wasm32"))))]
#[derive(Default)]
struct Yield {
    yielded: bool,
}

#[cfg(not(all(feature = "tokio", not(target_arch = "wasm32"))))]
impl core::future::Future for Yield {
    type Output = ();

    fn poll(
        mut self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<()> {
        if self.yielded {
            return core::task::Poll::Ready(());
        }
        self.yielded = true;
        cx.waker().wake_by_ref();
        core::task::Poll::Pending
    }
}

/// Yields execution back to the runtime
pub fn yield_now() -> impl Future<Output = ()> {
    #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
    {
        tokio::task::yield_now()
    }
    #[cfg(not(all(feature = "tokio", not(target_arch = "wasm32"))))]
    {
        Yield::default()
    }
}

/// Yields execution back to the runtime `amount` times.
pub async fn yield_for(amount: usize) {
    for _ in 0..amount {
        yield_now().await;
    }
}
