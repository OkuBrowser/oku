use crate::{
    Executor, ExecutorBlockOn, ExecutorBlocking, ExecutorTimeout, InnerJoinHandle, JoinHandle,
};
use compio::runtime::Runtime;

/// Compio executor
#[derive(Default, Clone, Copy, Debug, PartialOrd, PartialEq, Eq)]
pub struct CompioExecutor;

impl Executor for CompioExecutor {
    fn runtime_type(&self) -> Option<&'static str> {
        Some("compio")
    }

    fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let handle = compio::runtime::spawn(future);
        let inner = InnerJoinHandle::compio(handle);
        JoinHandle { inner }
    }
}

impl ExecutorBlocking for CompioExecutor {
    fn spawn_blocking<F, R>(&self, f: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let handle = compio::runtime::spawn_blocking(f);
        let inner = InnerJoinHandle::compio(handle);
        JoinHandle { inner }
    }
}

impl ExecutorTimeout for CompioExecutor {}

impl ExecutorBlockOn for CompioExecutor {
    fn block_on<F: Future>(&self, f: F) -> F::Output {
        let handle = Runtime::current();
        handle.block_on(f)
    }
}

/// Compio executor with an [`Runtime`]
///
/// # Note
///
/// Creating or supplying this runtime by itself will not drive task to completion when spawned.
/// This will be useful if you are already calling [`CompioRuntimeExecutor::from_current_runtime`] or
/// [`CompioRuntimeExecutor::with_runtime`] with an existing runtime.
#[derive(Clone, Debug)]
pub struct CompioRuntimeExecutor {
    runtime: Runtime,
}

impl CompioRuntimeExecutor {
    /// Creates a compio runtime.
    pub fn new() -> std::io::Result<Self> {
        let runtime = Runtime::builder().build()?;
        Ok(Self::with_runtime(runtime))
    }

    /// Create an executor with the supplied [`Runtime`].
    ///
    /// Note that this executor schedules tasks but does not drive the runtime.
    /// The supplied runtime must be driven externally.
    pub fn with_runtime(runtime: Runtime) -> Self {
        Self { runtime }
    }

    /// Create an executor from the existing Runtime
    ///
    /// Note that this returns an error when called outside a Compio runtime context. The
    /// resulting executor does not prevent that runtime from shutting down.
    pub fn from_current_runtime() -> std::io::Result<Self> {
        let handle =
            Runtime::try_current().ok_or_else(|| std::io::Error::other("no runtime running"))?;
        Ok(Self::with_runtime(handle))
    }
}

impl Executor for CompioRuntimeExecutor {
    fn runtime_type(&self) -> Option<&'static str> {
        Some("compio")
    }

    fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let handle = self.runtime.spawn(future);
        let inner = InnerJoinHandle::compio(handle);
        JoinHandle { inner }
    }
}

impl ExecutorBlocking for CompioRuntimeExecutor {
    fn spawn_blocking<F, R>(&self, f: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let handle = self.runtime.spawn_blocking(f);
        let inner = InnerJoinHandle::compio(handle);
        JoinHandle { inner }
    }
}

impl ExecutorTimeout for CompioRuntimeExecutor {}

impl ExecutorBlockOn for CompioRuntimeExecutor {
    fn block_on<F: Future>(&self, f: F) -> F::Output {
        self.runtime.block_on(f)
    }
}

#[cfg(test)]
mod tests {
    use super::{CompioExecutor, CompioRuntimeExecutor};
    use crate::error::JoinError;
    use crate::{Executor, ExecutorBlockOn, ExecutorBlocking, ExecutorTimeout, TimeoutError};
    use futures::channel::mpsc::{Receiver, UnboundedReceiver};
    use futures::{SinkExt, StreamExt};
    use futures_timer::Delay;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[compio::test]
    async fn explicit_abort_is_reported_as_aborted() {
        let handle = CompioExecutor.spawn(futures::future::pending::<()>());

        handle.abort();

        assert!(handle.is_finished());
        assert!(matches!(handle.await, Err(JoinError::Aborted)));
    }

    #[compio::test]
    async fn joinhandle_doesnt_abort_on_drop() {
        let (mut tx, mut rx) = futures::channel::mpsc::channel(1);
        let handle = CompioExecutor.spawn(async move {
            let _ = tx.send(()).await;
        });
        drop(handle);

        let val = rx.next().await;
        assert!(val.is_some());
    }

    #[test]
    fn drive_runtime_to_completion() {
        let runtime = CompioRuntimeExecutor::new().unwrap();
        runtime.block_on(async {
            let (mut tx, mut rx) = futures::channel::mpsc::channel(1);

            runtime.spawn(async move {
                let _ = tx.send(()).await;
            });

            let val = rx.next().await;
            assert!(val.is_some());
        });
    }

    #[compio::test]
    async fn abort_is_requested_before_handle_is_awaited() {
        struct DropGuard(Arc<AtomicBool>);

        impl Drop for DropGuard {
            fn drop(&mut self) {
                self.0.store(true, Ordering::Release);
            }
        }

        let dropped = Arc::new(AtomicBool::new(false));
        let task_dropped = dropped.clone();
        let (started_tx, started_rx) = futures::channel::oneshot::channel();
        let handle = CompioExecutor.spawn(async move {
            let _guard = DropGuard(task_dropped);
            let _ = started_tx.send(());
            futures::future::pending::<()>().await;
        });

        started_rx.await.unwrap();
        handle.abort();

        crate::task::yield_now().await;

        assert!(dropped.load(Ordering::Acquire));
        assert!(handle.is_finished());
        assert!(matches!(handle.await, Err(JoinError::Aborted)));
    }

    #[cfg(panic = "unwind")]
    #[compio::test]
    async fn task_panic_is_reported_as_panicked() {
        async fn panic_task() -> usize {
            panic!("expected task panic");
        }

        let handle = CompioExecutor.spawn(panic_task());

        assert!(matches!(handle.await, Err(JoinError::Panicked)));
    }

    #[test]
    fn runtime_shutdown_is_reported_as_cancelled() {
        let executor = CompioRuntimeExecutor::new().unwrap();
        let handle = executor.spawn(futures::future::pending::<()>());

        drop(executor);

        assert!(matches!(
            futures::executor::block_on(handle),
            Err(JoinError::Cancelled)
        ));
    }

    #[compio::test]
    async fn default_abortable_task() {
        let executor = CompioExecutor;

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

    #[compio::test]
    async fn task_coroutine() {
        let executor = CompioExecutor;

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

    #[compio::test]
    async fn task_coroutine_with_context() {
        let executor = CompioExecutor;

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

    #[compio::test]
    async fn task_coroutine_with_receiver() {
        use futures::stream::StreamExt;
        let executor = CompioExecutor;

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

    #[compio::test]
    async fn task_coroutine_with_receiver_and_context() {
        use futures::stream::StreamExt;
        let executor = CompioExecutor;

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

    #[compio::test]
    async fn task_unbounded_coroutine() {
        let executor = CompioExecutor;

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

    #[compio::test]
    async fn task_unbounded_coroutine_with_context() {
        let executor = CompioExecutor;

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

    #[compio::test]
    async fn task_unbounded_coroutine_with_receiver() {
        use futures::stream::StreamExt;
        let executor = CompioExecutor;

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

    #[compio::test]
    async fn task_unbounded_coroutine_with_receiver_and_context() {
        use futures::stream::StreamExt;
        let executor = CompioExecutor;

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

    #[compio::test]
    async fn timeout_task() {
        let executor = CompioExecutor;

        let task = executor.spawn_timeout(
            std::time::Duration::from_millis(10),
            futures::future::pending::<()>(),
        );
        let resp = task.await.unwrap();
        assert!(matches!(resp.unwrap_err(), TimeoutError));
    }

    #[compio::test]
    async fn complete_before_timeout_task() {
        let executor = CompioExecutor;

        let task = executor.spawn_timeout(
            std::time::Duration::from_millis(10),
            futures::future::ready("Hello"),
        );
        let resp = task.await.unwrap();
        assert!(resp.is_ok());
        let result = resp.unwrap();
        assert_eq!(result, "Hello");
    }

    #[compio::test]
    async fn delay_task() {
        let executor = CompioExecutor;
        let duration = std::time::Duration::from_millis(20);
        let started = std::time::Instant::now();

        let task = executor.spawn_delay(duration, async { "Hello" });
        let result = task.await.unwrap();

        assert!(started.elapsed() >= duration);
        assert_eq!(result, "Hello");
    }

    #[compio::test]
    async fn abortable_delay_task() {
        let executor = CompioExecutor;
        let (tx, rx) = futures::channel::oneshot::channel();
        let task = executor.spawn_abortable_delay(std::time::Duration::from_secs(5), async move {
            let _ = tx.send(());
        });

        drop(task);

        assert!(rx.await.is_err());
    }

    #[compio::test]
    async fn race_before_timeout_task() {
        let executor = CompioExecutor;

        let task = executor.spawn_timeout(std::time::Duration::from_millis(500), async {
            Delay::new(std::time::Duration::from_millis(10)).await;
            "Hello"
        });
        let resp = task.await.unwrap();
        assert!(resp.is_ok());
        let result = resp.unwrap();
        assert_eq!(result, "Hello");
    }

    #[compio::test]
    async fn abortable_timeout_task() {
        let executor = CompioExecutor;

        let task = executor.spawn_abortable_timeout(
            std::time::Duration::from_millis(10),
            futures::future::pending::<()>(),
        );
        let resp = task.await.unwrap();
        assert!(matches!(resp.unwrap_err(), TimeoutError));
    }

    #[compio::test]
    async fn blocking_task() {
        let executor = CompioExecutor;

        let task = executor.spawn_blocking(|| {
            std::thread::sleep(std::time::Duration::from_millis(100));
            "Hello"
        });
        let resp = task.await.unwrap();
        assert_eq!(resp, "Hello");
    }
}
