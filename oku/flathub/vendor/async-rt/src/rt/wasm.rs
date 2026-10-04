use crate::{
    CompletionGuard, Executor, ExecutorBlockOn, ExecutorBlocking, ExecutorTimeout, InnerJoinHandle,
    JoinHandle, abortable_result,
};
use futures::future::AbortHandle;
use pollable_map::optional::Optional;
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

/// Wasm executor
#[derive(Default, Clone, Copy, Debug, PartialOrd, PartialEq, Eq)]
pub struct WasmExecutor;

impl Executor for WasmExecutor {
    fn runtime_type(&self) -> Option<&'static str> {
        Some("wasm")
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
        let fut = async move {
            let _completion = completion;
            let val = future.await;
            _ = tx.send(val);
        };

        wasm_bindgen_futures::spawn_local(fut);
        let inner = InnerJoinHandle::CustomHandle {
            inner: Optional::new(rx),
            handle: abort_handle,
            finished,
        };
        JoinHandle { inner }
    }
}

impl ExecutorBlocking for WasmExecutor {
    fn spawn_blocking<F, R>(&self, f: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        // Since wasm does not have any blocking operations, we would just
        // execute the function in the given task and return its underlining value, if any
        // TODO: Maybe check to determine if this is a browser and if so use webworker for offloading
        let fut = async {
            let val = f();
            // we yield back to the executor, so it can make some progress before we return the value here and allow other tasks to continue.
            // not exactly needed
            crate::task::yield_now().await;
            val
        };
        self.spawn(fut)
    }
}

impl ExecutorTimeout for WasmExecutor {}

impl ExecutorBlockOn for WasmExecutor {
    fn block_on<F: Future>(&self, _: F) -> F::Output {
        unimplemented!("wasm32 executor does not implement block_on")
    }
}
