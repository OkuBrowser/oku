use crate::{
    CompletionGuard, Executor, ExecutorBlockOn, ExecutorBlocking, ExecutorTimeout, InnerJoinHandle,
    JoinHandle, abortable_result,
};
use futures::future::{AbortHandle, BoxFuture};
use futures::task::AtomicWaker;
use parking_lot::Mutex;
use pollable_map::optional::Optional;
use std::fmt::{Debug, Formatter};
use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock};
use std::task::{Context, Poll, Wake, Waker};

thread_local! {
    static WAKER_LOCAL_THREAD: Waker = Waker::from(Arc::new(LocalWaker(std::thread::current())));
}

static LITE_EXECUTOR: LazyLock<LiteRuntimeExecutor> = LazyLock::new(LiteRuntimeExecutor::default);

struct LocalWaker(std::thread::Thread);

impl Wake for LocalWaker {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.0.unpark();
    }
}

struct DriveGuard<'a>(&'a AtomicBool);

impl Drop for DriveGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

struct ActiveTasks<'a> {
    queue: &'a Mutex<Vec<BoxFuture<'static, ()>>>,
    tasks: Vec<BoxFuture<'static, ()>>,
}

impl Drop for ActiveTasks<'_> {
    fn drop(&mut self) {
        self.queue.lock().append(&mut self.tasks);
    }
}

/// A light single-threaded executor backed by a shared runtime.
///
/// Tasks only make progress while [`LiteExecutor::block_on`] is running. Only one
/// thread may drive the shared runtime at a time.
#[derive(Default, Clone, Copy, Debug, PartialOrd, PartialEq, Eq)]
pub struct LiteExecutor;

impl Executor for LiteExecutor {
    fn runtime_type(&self) -> Option<&'static str> {
        Some("lite")
    }

    fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        LITE_EXECUTOR.spawn(future)
    }
}

impl ExecutorBlocking for LiteExecutor {
    fn spawn_blocking<F, R>(&self, f: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        LITE_EXECUTOR.spawn_blocking(f)
    }
}

impl ExecutorTimeout for LiteExecutor {}

impl ExecutorBlockOn for LiteExecutor {
    fn block_on<F: Future>(&self, future: F) -> F::Output {
        LITE_EXECUTOR.block_on(future)
    }
}

/// A light single-threaded executor with minimal dependencies.
///
/// Tasks only make progress while [`LiteRuntimeExecutor::block_on`] is running. Only one
/// thread may drive an executor and its clones at a time.
#[derive(Default, Clone)]
pub struct LiteRuntimeExecutor {
    queued_tasks: Arc<Mutex<Vec<BoxFuture<'static, ()>>>>,
    waker: Arc<AtomicWaker>,
    driving: Arc<AtomicBool>,
}

impl Debug for LiteRuntimeExecutor {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LiteRuntimeExecutor").finish()
    }
}

impl PartialEq for LiteRuntimeExecutor {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.queued_tasks, &other.queued_tasks)
    }
}

impl Eq for LiteRuntimeExecutor {}

impl LiteRuntimeExecutor {
    /// Creates a new lightweight runtime.
    pub fn new() -> Self {
        Self::default()
    }

    fn take_queued_tasks(&self, tasks: &mut Vec<BoxFuture<'static, ()>>) -> bool {
        let mut queued = self.queued_tasks.lock();
        if queued.is_empty() {
            return false;
        }

        tasks.append(&mut queued);
        true
    }
}

impl Executor for LiteRuntimeExecutor {
    fn runtime_type(&self) -> Option<&'static str> {
        Some("lite")
    }

    fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        let future = abortable_result(future, abort_registration);
        let (tx, rx) = futures::channel::oneshot::channel();
        let finished = Arc::new(AtomicBool::new(false));
        let completion = CompletionGuard::new(finished.clone());
        let task = async move {
            let _completion = completion;
            let result = future.await;
            let _ = tx.send(result);
        };

        self.queued_tasks.lock().push(Box::pin(task));
        self.waker.wake();

        let inner = InnerJoinHandle::CustomHandle {
            inner: Optional::new(rx),
            handle: abort_handle,
            finished,
        };

        JoinHandle { inner }
    }
}

impl ExecutorBlocking for LiteRuntimeExecutor {
    fn spawn_blocking<F, R>(&self, f: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        self.spawn(async move {
            let (tx, rx) = futures::channel::oneshot::channel();
            let _handle = std::thread::spawn(move || {
                let result = f();
                let _ = tx.send(result);
            });
            rx.await.expect("blocking task should not be dropped")
        })
    }
}

impl ExecutorTimeout for LiteRuntimeExecutor {}

impl ExecutorBlockOn for LiteRuntimeExecutor {
    fn block_on<F: Future>(&self, future: F) -> F::Output {
        if self
            .driving
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            panic!("LiteRuntimeExecutor is already being driven");
        }
        let _drive_guard = DriveGuard(&self.driving);
        let mut future = core::pin::pin!(future);
        let mut active = ActiveTasks {
            queue: &self.queued_tasks,
            tasks: Vec::new(),
        };

        WAKER_LOCAL_THREAD.with(|waker| {
            let mut context = Context::from_waker(waker);

            loop {
                self.waker.register(waker);
                self.take_queued_tasks(&mut active.tasks);

                if let Poll::Ready(output) = future.as_mut().poll(&mut context) {
                    return output;
                }

                let mut made_progress = false;
                let mut index = 0;
                while index < active.tasks.len() {
                    if active.tasks[index].as_mut().poll(&mut context).is_ready() {
                        drop(active.tasks.swap_remove(index));
                        made_progress = true;
                    } else {
                        index += 1;
                    }
                }

                if self.take_queued_tasks(&mut active.tasks) || made_progress {
                    continue;
                }

                self.waker.register(waker);
                if self.take_queued_tasks(&mut active.tasks) {
                    continue;
                }

                std::thread::park();
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{LiteExecutor, LiteRuntimeExecutor};
    use crate::error::JoinError;
    use crate::{Executor, ExecutorBlockOn, ExecutorBlocking, ExecutorTimeout};
    use std::panic::AssertUnwindSafe;
    use std::time::Duration;

    #[test]
    fn ready_future_completes() {
        assert_eq!(LiteRuntimeExecutor::new().block_on(async { 42 }), 42);
    }

    #[test]
    fn shared_executor_drives_spawned_tasks() {
        let handle = LiteExecutor.spawn(async { 42 });

        assert_eq!(LiteExecutor.block_on(handle).unwrap(), 42);
    }

    #[test]
    fn spawned_task_completes() {
        let executor = LiteRuntimeExecutor::new();
        let handle = executor.spawn(async { 42 });

        assert_eq!(executor.block_on(handle).unwrap(), 42);
    }

    #[test]
    fn nested_spawn_completes() {
        let executor = LiteRuntimeExecutor::new();
        let nested_executor = executor.clone();
        let handle =
            executor.spawn(async move { nested_executor.spawn(async { 41 }).await.unwrap() + 1 });

        assert_eq!(executor.block_on(handle).unwrap(), 42);
    }

    #[test]
    fn task_spawned_from_another_thread_wakes_driver() {
        let executor = LiteRuntimeExecutor::new();
        let other_executor = executor.clone();
        let (start_tx, start_rx) = std::sync::mpsc::channel();
        let (value_tx, value_rx) = futures::channel::oneshot::channel();

        let worker = std::thread::spawn(move || {
            start_rx.recv().unwrap();
            std::thread::sleep(Duration::from_millis(10));
            other_executor.dispatch(async move {
                value_tx.send(42).unwrap();
            });
        });

        let value = executor.block_on(async move {
            start_tx.send(()).unwrap();
            value_rx.await.unwrap()
        });

        worker.join().unwrap();
        assert_eq!(value, 42);
    }

    #[test]
    fn unfinished_tasks_are_preserved() {
        let executor = LiteRuntimeExecutor::new();
        let handle = executor.spawn(async { 42 });

        executor.block_on(async {});

        assert!(!handle.is_finished());
        assert_eq!(executor.block_on(handle).unwrap(), 42);
    }

    #[cfg(panic = "unwind")]
    #[test]
    fn unfinished_tasks_are_preserved_after_block_on_panics() {
        let executor = LiteRuntimeExecutor::new();
        let handle = executor.spawn(async { 42 });

        let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
            executor.block_on(async { panic!("expected driver panic") });
        }));

        assert!(result.is_err());
        assert!(!handle.is_finished());
        assert_eq!(executor.block_on(handle).unwrap(), 42);
    }

    #[test]
    fn spawn_blocking_completes() {
        let executor = LiteRuntimeExecutor::new();
        let handle = executor.spawn_blocking(|| 42);

        assert_eq!(executor.block_on(handle).unwrap(), 42);
    }

    #[test]
    fn timeout_completes() {
        let executor = LiteRuntimeExecutor::new();
        let handle = executor.spawn_timeout(Duration::from_millis(10), async {
            futures::future::pending::<()>().await
        });

        assert!(executor.block_on(handle).unwrap().is_err());
    }

    #[cfg(panic = "unwind")]
    #[test]
    fn task_panic_is_classified() {
        let executor = LiteRuntimeExecutor::new();
        let handle = executor.spawn(async { panic!("expected task panic") });

        assert!(matches!(
            executor.block_on(handle),
            Err(JoinError::Panicked)
        ));
    }

    #[test]
    fn explicit_abort_is_reported_as_aborted() {
        let executor = LiteRuntimeExecutor::new();
        let handle = executor.spawn(futures::future::pending::<()>());
        handle.abort();

        assert!(matches!(executor.block_on(handle), Err(JoinError::Aborted)));
    }

    #[cfg(panic = "unwind")]
    #[test]
    fn concurrent_block_on_panics() {
        let executor = LiteRuntimeExecutor::new();
        let other_executor = executor.clone();
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = futures::channel::oneshot::channel();

        let driver = std::thread::spawn(move || {
            other_executor.block_on(async move {
                started_tx.send(()).unwrap();
                release_rx.await.unwrap();
            });
        });
        started_rx.recv().unwrap();

        let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
            executor.block_on(async {});
        }));

        release_tx.send(()).unwrap();
        driver.join().unwrap();
        assert!(result.is_err());
    }
}
