use crate::{Executor, ExecutorBlockOn, ExecutorBlocking, ExecutorTimeout, JoinHandle};
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::task::{Context, Poll};

/// Track running tasks.
///
/// Note that there is no guarantee that the runtime would drop the future after it is done.
/// Therefore, this should only be used for purely approx statistics and not actual numbers.
/// Additionally, it does not track any tasks spawned directly by the runtime but only by
/// [`Executor::spawn`] through this implementation against [`Executor`].
pub struct TrackerExecutor<E> {
    executor: E,
    counter: Arc<AtomicUsize>,
}

impl<E> Debug for TrackerExecutor<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrackerExecutor").finish()
    }
}

impl<E: Executor> TrackerExecutor<E> {
    pub fn new(executor: E) -> Self {
        Self {
            executor,
            counter: Arc::default(),
        }
    }

    /// Number of active tasks.
    pub fn count(&self) -> usize {
        self.counter.load(std::sync::atomic::Ordering::Relaxed)
    }
}

struct FutureCounter<F> {
    future: F,
    counter: Arc<AtomicUsize>,
}

impl<F> FutureCounter<F> {
    pub fn new(future: F, counter: Arc<AtomicUsize>) -> Self {
        counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Self { future, counter }
    }
}

impl<F> Future for FutureCounter<F>
where
    F: Future + 'static + Unpin,
{
    type Output = F::Output;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.future).poll(cx)
    }
}

impl<F> Drop for FutureCounter<F> {
    fn drop(&mut self) {
        self.counter
            .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
    }
}

impl<E: Executor> Executor for TrackerExecutor<E> {
    fn runtime_type(&self) -> Option<&'static str> {
        self.executor.runtime_type()
    }

    fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let counter = self.counter.clone();
        let future = Box::pin(future);
        let future = FutureCounter::new(future, counter);
        self.executor.spawn(future)
    }
}

impl<E: ExecutorBlocking> ExecutorBlocking for TrackerExecutor<E> {
    fn spawn_blocking<F, R>(&self, f: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        struct AtomicCounterDrop(Arc<AtomicUsize>);

        impl AtomicCounterDrop {
            pub fn new(counter: Arc<AtomicUsize>) -> Self {
                counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                Self(counter)
            }
        }

        impl Drop for AtomicCounterDrop {
            fn drop(&mut self) {
                self.0.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
            }
        }

        let counter = AtomicCounterDrop::new(self.counter.clone());

        self.executor.spawn_blocking(move || {
            let _counter = counter;
            f()
        })
    }
}

impl<E: ExecutorTimeout> ExecutorTimeout for TrackerExecutor<E> {}

impl<E: ExecutorBlockOn> ExecutorBlockOn for TrackerExecutor<E> {
    fn block_on<F: Future>(&self, future: F) -> F::Output {
        self.executor.block_on(future)
    }
}

#[cfg(test)]
mod tests {

    #[cfg(feature = "threadpool")]
    #[test]
    fn test_tracker_threadpool_executor() {
        use super::TrackerExecutor;
        use crate::Executor;
        use crate::rt::threadpool::ThreadPoolExecutor;
        futures::executor::block_on(async {
            let executor = TrackerExecutor::new(ThreadPoolExecutor);
            let handle = executor.spawn(futures::future::pending::<()>());
            assert_eq!(executor.count(), 1);
            handle.abort();
            let _ = handle.await;
            assert_eq!(executor.count(), 0);
        });
    }

    #[cfg(feature = "tokio")]
    #[tokio::test]
    async fn test_tracker_tokio_executor() {
        use super::TrackerExecutor;
        use crate::Executor;
        use crate::rt::tokio::TokioRuntimeExecutor;

        let executor = TrackerExecutor::new(TokioRuntimeExecutor::from_current_handle().unwrap());
        let handle = executor.spawn(futures::future::pending::<()>());
        assert_eq!(executor.count(), 1);
        handle.abort();
        let _ = handle.await;
        assert_eq!(executor.count(), 0);
    }
}
