use crate::error::TimeoutError;
use crate::{
    AbortableJoinHandle, Executor, ExecutorBlockOn, ExecutorBlocking, ExecutorTimeout, JoinHandle,
};
use either::Either;
use std::future::Future;
use std::time::Duration;

impl<L, R> Executor for Either<L, R>
where
    L: Executor,
    R: Executor,
{
    fn runtime_type(&self) -> Option<&'static str> {
        match self {
            Either::Left(l) => l.runtime_type(),
            Either::Right(r) => r.runtime_type(),
        }
    }

    fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        match self {
            Either::Left(l) => l.spawn(future),
            Either::Right(r) => r.spawn(future),
        }
    }
}

impl<L, R> ExecutorBlocking for Either<L, R>
where
    L: ExecutorBlocking,
    R: ExecutorBlocking,
{
    fn spawn_blocking<F, T>(&self, f: F) -> JoinHandle<T>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        match self {
            Either::Left(l) => l.spawn_blocking(f),
            Either::Right(r) => r.spawn_blocking(f),
        }
    }
}

impl<L, R> ExecutorTimeout for Either<L, R>
where
    L: ExecutorTimeout,
    R: ExecutorTimeout,
{
    fn spawn_timeout<F>(
        &self,
        duration: Duration,
        f: F,
    ) -> JoinHandle<Result<F::Output, TimeoutError>>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        match self {
            Either::Left(l) => l.spawn_timeout(duration, f),
            Either::Right(r) => r.spawn_timeout(duration, f),
        }
    }

    fn spawn_delay<F>(&self, duration: Duration, f: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        match self {
            Either::Left(l) => l.spawn_delay(duration, f),
            Either::Right(r) => r.spawn_delay(duration, f),
        }
    }

    fn spawn_abortable_timeout<F>(
        &self,
        duration: Duration,
        f: F,
    ) -> AbortableJoinHandle<Result<F::Output, TimeoutError>>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        match self {
            Either::Left(l) => l.spawn_abortable_timeout(duration, f),
            Either::Right(r) => r.spawn_abortable_timeout(duration, f),
        }
    }

    fn spawn_abortable_delay<F>(&self, duration: Duration, f: F) -> AbortableJoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        match self {
            Either::Left(l) => l.spawn_abortable_delay(duration, f),
            Either::Right(r) => r.spawn_abortable_delay(duration, f),
        }
    }
}

impl<L, R> ExecutorBlockOn for Either<L, R>
where
    L: ExecutorBlockOn,
    R: ExecutorBlockOn,
{
    fn block_on<F: Future>(&self, f: F) -> F::Output {
        match self {
            Either::Left(l) => l.block_on(f),
            Either::Right(r) => r.block_on(f),
        }
    }
}
