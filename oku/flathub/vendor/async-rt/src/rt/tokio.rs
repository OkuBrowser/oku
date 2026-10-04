use crate::{
    Executor, ExecutorBlockOn, ExecutorBlocking, ExecutorTimeout, InnerJoinHandle, JoinHandle,
};
use std::future::Future;
use std::sync::Arc;
use tokio::runtime::{Handle, Runtime};

/// Tokio executor
#[derive(Default, Clone, Copy, Debug, PartialOrd, PartialEq, Eq)]
pub struct TokioExecutor;

impl Executor for TokioExecutor {
    fn runtime_type(&self) -> Option<&'static str> {
        Some("tokio")
    }

    fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let handle = tokio::task::spawn(future);
        let inner = InnerJoinHandle::tokio(handle);
        JoinHandle { inner }
    }
}

impl ExecutorBlocking for TokioExecutor {
    fn spawn_blocking<F, R>(&self, f: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let handle = tokio::task::spawn_blocking(f);
        let inner = InnerJoinHandle::tokio(handle);
        JoinHandle { inner }
    }
}

impl ExecutorTimeout for TokioExecutor {}

impl ExecutorBlockOn for TokioExecutor {
    /// Blocks the current thread until the provided future has completed.
    /// Note that this should not run inside a tokio runtime context.
    fn block_on<F: Future>(&self, f: F) -> F::Output {
        let handle = Handle::current();
        handle.block_on(f)
    }
}

/// Tokio executor with an [`Runtime`] or [`Handle`]
#[derive(Clone, Debug)]
pub struct TokioRuntimeExecutor {
    handle: Handle,
    _runtime: Option<Arc<Runtime>>,
}

impl TokioRuntimeExecutor {
    /// Creates a tokio runtime with the current thread scheduler selected.
    pub fn with_single_thread() -> std::io::Result<Self> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        Ok(Self::with_runtime(runtime))
    }

    /// Creates a tokio runtime with multi-thread scheduler selected.
    pub fn with_multi_thread() -> std::io::Result<Self> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;
        Ok(Self::with_runtime(runtime))
    }

    /// Create an executor with the supplied [`Runtime`].
    ///
    /// Note that the runtime remains alive until the final clone of this executor is
    /// dropped.
    pub fn with_runtime(runtime: Runtime) -> Self {
        let runtime = Arc::new(runtime);
        let handle = runtime.handle().clone();
        Self {
            _runtime: Some(runtime),
            handle,
        }
    }

    /// Create an executor with the supplied [`Runtime`].
    ///
    /// Note that this executor does not own or keep the associated runtime alive. The
    /// runtime must remain running for spawned tasks to execute.
    pub fn with_handle(handle: Handle) -> Self {
        Self {
            handle,
            _runtime: None,
        }
    }

    /// Create an executor from the existing Runtime
    ///
    /// Note that this returns an error when called outside a Tokio runtime context. The
    /// resulting executor does not prevent that runtime from shutting down.
    pub fn from_current_handle() -> std::io::Result<Self> {
        let handle = Handle::try_current().map_err(std::io::Error::other)?;
        Ok(Self::with_handle(handle))
    }
}

impl Executor for TokioRuntimeExecutor {
    fn runtime_type(&self) -> Option<&'static str> {
        Some("tokio")
    }

    fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let handle = self.handle.spawn(future);
        let inner = InnerJoinHandle::tokio(handle);
        JoinHandle { inner }
    }
}

impl ExecutorBlocking for TokioRuntimeExecutor {
    fn spawn_blocking<F, R>(&self, f: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let handle = self.handle.spawn_blocking(f);
        let inner = InnerJoinHandle::tokio(handle);
        JoinHandle { inner }
    }
}

impl ExecutorTimeout for TokioRuntimeExecutor {}

impl ExecutorBlockOn for TokioRuntimeExecutor {
    /// Blocks the current thread until the provided future has completed.
    ///
    /// Note that when this executor owns a [`Runtime`], that runtime is driven directly.
    /// When created from a [`Handle`], this delegates to [`Handle::block_on`] and
    /// inherits its runtime-specific limitations. See [`Runtime::block_on`] and
    /// [`Handle::block_on`] for details.
    fn block_on<F: Future>(&self, f: F) -> F::Output {
        match self._runtime.as_ref() {
            None => self.handle.block_on(f),
            Some(runtime) => runtime.block_on(f),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{TokioExecutor, TokioRuntimeExecutor};
    use crate::error::JoinError;
    use crate::{Executor, ExecutorBlockOn, ExecutorBlocking, ExecutorTimeout, TimeoutError};
    use futures::channel::mpsc::{Receiver, UnboundedReceiver};
    use futures_timer::Delay;

    #[tokio::test]
    async fn explicit_abort_is_reported_as_aborted() {
        let handle = TokioExecutor.spawn(futures::future::pending::<()>());

        handle.abort();

        assert!(matches!(handle.await, Err(JoinError::Aborted)));
    }

    #[cfg(panic = "unwind")]
    #[tokio::test]
    async fn task_panic_is_reported_as_panicked() {
        async fn panic_task() -> usize {
            panic!("expected task panic");
        }

        let handle = TokioExecutor.spawn(panic_task());

        assert!(matches!(handle.await, Err(JoinError::Panicked)));
    }

    #[test]
    fn runtime_shutdown_is_reported_as_cancelled() {
        let executor = TokioRuntimeExecutor::with_multi_thread().unwrap();
        let handle = executor.spawn(futures::future::pending::<()>());

        drop(executor);

        assert!(matches!(
            futures::executor::block_on(handle),
            Err(JoinError::Cancelled)
        ));
    }

    #[test]
    fn block_on_drives_owned_current_thread_runtime() {
        let executor = TokioRuntimeExecutor::with_single_thread().unwrap();
        let handle = executor.spawn(async { 42 });

        assert_eq!(executor.block_on(handle).unwrap(), 42);
    }

    #[tokio::test]
    async fn default_abortable_task() {
        let executor = TokioExecutor;

        async fn task(tx: futures::channel::oneshot::Sender<()>) {
            futures_timer::Delay::new(std::time::Duration::from_secs(5)).await;
            let _ = tx.send(());
            unreachable!();
        }

        let (tx, rx) = futures::channel::oneshot::channel::<()>();

        let handle = executor.spawn_abortable(task(tx));

        drop(handle);
        let result = rx.await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn task_coroutine() {
        let executor = TokioExecutor;

        enum Message {
            Send(String, futures::channel::oneshot::Sender<String>),
        }

        let mut task = executor.spawn_coroutine(|msg: Message| async move {
            match msg {
                Message::Send(msg, sender) => {
                    sender.send(msg).unwrap();
                }
            }
        });

        let (tx, rx) = futures::channel::oneshot::channel::<String>();
        let msg = Message::Send("Hello".into(), tx);

        task.send(msg).await.unwrap();
        let resp = rx.await.unwrap();
        assert_eq!(resp, "Hello");
    }

    #[tokio::test]
    async fn task_coroutine_with_context() {
        let executor = TokioExecutor;

        type Resp = futures::channel::oneshot::Sender<usize>;

        let mut task =
            executor.spawn_coroutine_with_context(0usize, |counter: &mut usize, resp: Resp| {
                *counter += 1;
                let n = *counter;
                async move {
                    resp.send(n).unwrap();
                }
            });

        let (tx1, rx1) = futures::channel::oneshot::channel::<usize>();
        let (tx2, rx2) = futures::channel::oneshot::channel::<usize>();
        task.send(tx1).await.unwrap();
        task.send(tx2).await.unwrap();
        assert_eq!(rx1.await.unwrap(), 1);
        assert_eq!(rx2.await.unwrap(), 2);
    }

    #[tokio::test]
    async fn task_coroutine_with_receiver() {
        use futures::stream::StreamExt;
        let executor = TokioExecutor;

        enum Message {
            Send(String, futures::channel::oneshot::Sender<String>),
        }

        let mut task =
            executor.spawn_coroutine_with_receiver(|mut rx: Receiver<Message>| async move {
                while let Some(msg) = rx.next().await {
                    match msg {
                        Message::Send(msg, sender) => {
                            sender.send(msg).unwrap();
                        }
                    }
                }
            });

        let (tx, rx) = futures::channel::oneshot::channel::<String>();
        let msg = Message::Send("Hello".into(), tx);

        task.send(msg).await.unwrap();
        let resp = rx.await.unwrap();
        assert_eq!(resp, "Hello");
    }

    #[tokio::test]
    async fn task_coroutine_with_receiver_and_context() {
        use futures::stream::StreamExt;
        let executor = TokioExecutor;

        #[derive(Default)]
        struct State {
            message: String,
        }

        enum Message {
            Set(String),
            Get(futures::channel::oneshot::Sender<String>),
        }

        let mut task = executor.spawn_coroutine_with_receiver_and_context(
            State::default(),
            |mut state, mut rx: Receiver<Message>| async move {
                while let Some(msg) = rx.next().await {
                    match msg {
                        Message::Set(msg) => {
                            state.message = msg;
                        }
                        Message::Get(resp) => {
                            resp.send(state.message.clone()).unwrap();
                        }
                    }
                }
            },
        );

        let msg = Message::Set("Hello".into());

        task.send(msg).await.unwrap();
        let (tx, rx) = futures::channel::oneshot::channel::<String>();
        let msg = Message::Get(tx);
        task.send(msg).await.unwrap();
        let resp = rx.await.unwrap();
        assert_eq!(resp, "Hello");
    }

    #[tokio::test]
    async fn task_unbounded_coroutine() {
        let executor = TokioExecutor;

        enum Message {
            Send(String, futures::channel::oneshot::Sender<String>),
        }

        let mut task = executor.spawn_unbounded_coroutine(|msg: Message| async move {
            match msg {
                Message::Send(msg, sender) => {
                    sender.send(msg).unwrap();
                }
            }
        });

        let (tx, rx) = futures::channel::oneshot::channel::<String>();
        let msg = Message::Send("Hello".into(), tx);

        task.send(msg).unwrap();
        let resp = rx.await.unwrap();
        assert_eq!(resp, "Hello");
    }

    #[tokio::test]
    async fn task_unbounded_coroutine_with_context() {
        let executor = TokioExecutor;

        type Resp = futures::channel::oneshot::Sender<usize>;

        let mut task = executor.spawn_unbounded_coroutine_with_context(
            0usize,
            |counter: &mut usize, resp: Resp| {
                *counter += 1;
                let n = *counter;
                async move {
                    resp.send(n).unwrap();
                }
            },
        );

        let (tx1, rx1) = futures::channel::oneshot::channel::<usize>();
        let (tx2, rx2) = futures::channel::oneshot::channel::<usize>();
        task.send(tx1).unwrap();
        task.send(tx2).unwrap();
        assert_eq!(rx1.await.unwrap(), 1);
        assert_eq!(rx2.await.unwrap(), 2);
    }

    #[tokio::test]
    async fn task_unbounded_coroutine_with_receiver() {
        use futures::stream::StreamExt;
        let executor = TokioExecutor;

        enum Message {
            Send(String, futures::channel::oneshot::Sender<String>),
        }

        let mut task = executor.spawn_unbounded_coroutine_with_receiver(
            |mut rx: UnboundedReceiver<Message>| async move {
                while let Some(msg) = rx.next().await {
                    match msg {
                        Message::Send(msg, sender) => {
                            sender.send(msg).unwrap();
                        }
                    }
                }
            },
        );

        let (tx, rx) = futures::channel::oneshot::channel::<String>();
        let msg = Message::Send("Hello".into(), tx);

        task.send(msg).unwrap();
        let resp = rx.await.unwrap();
        assert_eq!(resp, "Hello");
    }

    #[tokio::test]
    async fn task_unbounded_coroutine_with_receiver_and_context() {
        use futures::stream::StreamExt;
        let executor = TokioExecutor;

        #[derive(Default)]
        struct State {
            message: String,
        }

        enum Message {
            Set(String),
            Get(futures::channel::oneshot::Sender<String>),
        }

        let mut task = executor.spawn_unbounded_coroutine_with_receiver_and_context(
            State::default(),
            |mut state, mut rx: UnboundedReceiver<Message>| async move {
                while let Some(msg) = rx.next().await {
                    match msg {
                        Message::Set(msg) => {
                            state.message = msg;
                        }
                        Message::Get(resp) => {
                            resp.send(state.message.clone()).unwrap();
                        }
                    }
                }
            },
        );

        let msg = Message::Set("Hello".into());

        task.send(msg).unwrap();
        let (tx, rx) = futures::channel::oneshot::channel::<String>();
        let msg = Message::Get(tx);
        task.send(msg).unwrap();
        let resp = rx.await.unwrap();
        assert_eq!(resp, "Hello");
    }

    #[tokio::test]
    async fn timeout_task() {
        let executor = TokioExecutor;

        let task = executor.spawn_timeout(
            std::time::Duration::from_millis(10),
            futures::future::pending::<()>(),
        );
        let resp = task.await.unwrap();
        assert!(matches!(resp.unwrap_err(), TimeoutError));
    }

    #[tokio::test]
    async fn complete_before_timeout_task() {
        let executor = TokioExecutor;

        let task = executor.spawn_timeout(
            std::time::Duration::from_millis(10),
            futures::future::ready("Hello"),
        );
        let resp = task.await.unwrap();
        assert!(resp.is_ok());
        let result = resp.unwrap();
        assert_eq!(result, "Hello");
    }

    #[tokio::test]
    async fn delay_task() {
        let executor = TokioExecutor;
        let duration = std::time::Duration::from_millis(20);
        let started = std::time::Instant::now();

        let task = executor.spawn_delay(duration, async { "Hello" });
        let result = task.await.unwrap();

        assert!(started.elapsed() >= duration);
        assert_eq!(result, "Hello");
    }

    #[tokio::test]
    async fn abortable_delay_task() {
        let executor = TokioExecutor;
        let (tx, rx) = futures::channel::oneshot::channel();
        let task = executor.spawn_abortable_delay(std::time::Duration::from_secs(5), async move {
            let _ = tx.send(());
        });

        drop(task);

        assert!(rx.await.is_err());
    }

    #[tokio::test]
    async fn race_before_timeout_task() {
        let executor = TokioExecutor;

        let task = executor.spawn_timeout(std::time::Duration::from_millis(500), async {
            Delay::new(std::time::Duration::from_millis(10)).await;
            "Hello"
        });
        let resp = task.await.unwrap();
        assert!(resp.is_ok());
        let result = resp.unwrap();
        assert_eq!(result, "Hello");
    }

    #[tokio::test]
    async fn abortable_timeout_task() {
        let executor = TokioExecutor;

        let task = executor.spawn_abortable_timeout(
            std::time::Duration::from_millis(10),
            futures::future::pending::<()>(),
        );
        let resp = task.await.unwrap();
        assert!(matches!(resp.unwrap_err(), TimeoutError));
    }

    #[tokio::test]
    async fn blocking_task() {
        let executor = TokioExecutor;

        let task = executor.spawn_blocking(|| {
            std::thread::sleep(std::time::Duration::from_millis(100));
            "Hello"
        });
        let resp = task.await.unwrap();
        assert_eq!(resp, "Hello");
    }
}
