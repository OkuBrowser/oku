use crate::{Executor, ExecutorBlockOn, ExecutorBlocking, ExecutorTimeout, JoinHandle};

/// Placeholder executor that does not provide task execution.
///
/// All execution methods panic when called.
#[derive(Default, Clone, Copy, Debug, PartialOrd, PartialEq, Eq)]
pub struct DummyExecutor;

impl Executor for DummyExecutor {
    fn runtime_type(&self) -> Option<&'static str> {
        Some("dummy")
    }

    fn spawn<F>(&self, _: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        missing_runtime()
    }
}

impl ExecutorBlocking for DummyExecutor {
    fn spawn_blocking<F, R>(&self, _: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        missing_runtime()
    }
}

impl ExecutorTimeout for DummyExecutor {}

impl ExecutorBlockOn for DummyExecutor {
    fn block_on<F: Future>(&self, _: F) -> F::Output {
        missing_runtime()
    }
}

#[cold]
#[track_caller]
fn missing_runtime() -> ! {
    panic!("DummyExecutor does not provide a runtime backend")
}
