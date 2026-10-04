use crate::error::TimeoutError;
use crate::{
    AbortableJoinHandle, Executor, ExecutorBlockOn, ExecutorBlocking, ExecutorTimeout, JoinHandle,
};
use std::future::Future;
use std::rc::Rc;
use std::time::Duration;

impl<E> Executor for Rc<E>
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

impl<E> ExecutorBlocking for Rc<E>
where
    E: ExecutorBlocking,
{
    fn spawn_blocking<F, R>(&self, f: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        (**self).spawn_blocking(f)
    }
}

impl<E> ExecutorTimeout for Rc<E>
where
    E: ExecutorTimeout,
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
        duration: Duration,
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

impl<E> ExecutorBlockOn for Rc<E>
where
    E: ExecutorBlockOn,
{
    fn block_on<F: Future>(&self, f: F) -> F::Output {
        (**self).block_on(f)
    }
}
