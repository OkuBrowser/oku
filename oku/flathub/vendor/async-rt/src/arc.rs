use crate::error::TimeoutError;
use crate::{
    AbortableJoinHandle, Executor, ExecutorBlockOn, ExecutorBlocking, ExecutorTimeout, JoinHandle,
};
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

impl<E> Executor for Arc<E>
where
    E: Executor,
{
    fn runtime_type(&self) -> Option<&'static str> {
        (**self).runtime_type()
    }

    fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        (**self).spawn(future)
    }
}

impl<E> ExecutorBlocking for Arc<E>
where
    E: ExecutorBlocking,
{
    fn spawn_blocking<F, T>(&self, future: F) -> JoinHandle<T>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        (**self).spawn_blocking(future)
    }

    fn spawn_blocking_abortable<F, R>(&self, f: F) -> AbortableJoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        (**self).spawn_blocking_abortable(f)
    }
}

impl<E> ExecutorTimeout for Arc<E>
where
    E: ExecutorTimeout,
{
    fn spawn_timeout<F>(
        &self,
        duration: std::time::Duration,
        f: F,
    ) -> JoinHandle<Result<F::Output, TimeoutError>>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        (**self).spawn_timeout(duration, f)
    }

    fn spawn_delay<F>(&self, duration: Duration, f: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        (**self).spawn_delay(duration, f)
    }

    fn spawn_abortable_timeout<F>(
        &self,
        duration: std::time::Duration,
        f: F,
    ) -> AbortableJoinHandle<Result<F::Output, TimeoutError>>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        (**self).spawn_abortable_timeout(duration, f)
    }

    fn spawn_abortable_delay<F>(&self, duration: Duration, f: F) -> AbortableJoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        (**self).spawn_abortable_delay(duration, f)
    }
}

impl<E> ExecutorBlockOn for Arc<E>
where
    E: ExecutorBlockOn,
{
    fn block_on<F: Future>(&self, f: F) -> F::Output {
        (**self).block_on(f)
    }
}
