use crate::{
    Executor, ExecutorBlockOn, ExecutorBlocking, ExecutorTimeout, InnerJoinHandle, JoinHandle,
    abortable_result,
};
use futures::future::AbortHandle;
use std::future::Future;
use std::sync::Arc;

/// Runs tasks using Smol's shared executor.
#[derive(Default, Clone, Copy, Debug, PartialOrd, PartialEq, Eq)]
pub struct SmolExecutor;

impl Executor for SmolExecutor {
    fn runtime_type(&self) -> Option<&'static str> {
        Some("smol")
    }

    fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        let task = smol::spawn(abortable_result(future, abort_registration));
        let inner = InnerJoinHandle::smol(task, abort_handle);
        JoinHandle { inner }
    }
}

impl ExecutorBlocking for SmolExecutor {
    fn spawn_blocking<F, R>(&self, f: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        self.spawn(smol::unblock(f))
    }
}

impl ExecutorTimeout for SmolExecutor {}

impl ExecutorBlockOn for SmolExecutor {
    fn block_on<F: Future>(&self, f: F) -> F::Output {
        smol::block_on(f)
    }
}

/// Runs tasks using a Smol executor directly
#[derive(Clone, Debug)]
pub struct SmolRuntimeExecutor<'a> {
    executor: Arc<smol::Executor<'a>>,
}

impl<'a> SmolRuntimeExecutor<'a> {
    /// Creates a new Smol executor.
    pub fn new() -> Self {
        Self {
            executor: Arc::new(smol::Executor::new()),
        }
    }

    /// Creates an executor using the provided Smol executor.
    pub fn with_executor(executor: smol::Executor<'a>) -> Self {
        Self {
            executor: Arc::new(executor),
        }
    }
}

impl Default for SmolRuntimeExecutor<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Executor for SmolRuntimeExecutor<'a> {
    fn runtime_type(&self) -> Option<&'static str> {
        Some("smol")
    }

    fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        let task = self
            .executor
            .spawn(abortable_result(future, abort_registration));
        let inner = InnerJoinHandle::smol(task, abort_handle);
        JoinHandle { inner }
    }
}

impl<'a> ExecutorBlocking for SmolRuntimeExecutor<'a> {
    fn spawn_blocking<F, R>(&self, f: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        self.spawn(smol::unblock(f))
    }
}

impl<'a> ExecutorTimeout for SmolRuntimeExecutor<'a> {}

impl<'a> ExecutorBlockOn for SmolRuntimeExecutor<'a> {
    fn block_on<F: Future>(&self, f: F) -> F::Output {
        smol::block_on(self.executor.run(f))
    }
}

#[cfg(test)]
mod tests {
    use crate::error::JoinError;
    use crate::rt::smol::{SmolExecutor, SmolRuntimeExecutor};
    use crate::{Executor, ExecutorBlockOn, ExecutorBlocking, ExecutorTimeout, TimeoutError};
    use futures::channel::mpsc::{Receiver, UnboundedReceiver};
    use futures_timer::Delay;

    #[test]
    fn explicit_abort_is_reported_as_aborted() {
        SmolExecutor.block_on(async move {
            let handle = SmolExecutor.spawn(futures::future::pending::<()>());

            handle.abort();

            assert!(matches!(handle.await, Err(JoinError::Aborted)));
        })
    }

    #[test]
    fn joinhandle_doesnt_abort_on_drop() {
        SmolExecutor.block_on(async move {
            let (started_tx, started_rx) = futures::channel::oneshot::channel();
            let (release_tx, release_rx) = futures::channel::oneshot::channel();
            let (completed_tx, completed_rx) = futures::channel::oneshot::channel();
            let handle = SmolExecutor.spawn(async move {
                let _ = started_tx.send(());
                let _ = release_rx.await;
                let _ = completed_tx.send(());
            });

            started_rx.await.unwrap();
            drop(handle);
            release_tx.send(()).unwrap();

            completed_rx.await.unwrap();
        })
    }

    #[cfg(panic = "unwind")]
    #[test]
    fn task_panic_is_reported_as_panicked() {
        SmolExecutor.block_on(async move {
            async fn panic_task() -> usize {
                panic!("expected task panic");
            }

            let handle = SmolExecutor.spawn(panic_task());

            assert!(matches!(handle.await, Err(JoinError::Panicked)));
        })
    }

    #[test]
    fn runtime_shutdown_is_reported_as_cancelled() {
        let executor = SmolRuntimeExecutor::new();
        let handle = executor.spawn(futures::future::pending::<()>());

        drop(executor);

        assert!(matches!(
            futures::executor::block_on(handle),
            Err(JoinError::Cancelled)
        ));
    }

    #[test]
    fn block_on_drives_owned_current_thread_runtime() {
        let executor = SmolRuntimeExecutor::new();
        let handle = executor.spawn(async { 42 });

        assert_eq!(executor.block_on(handle).unwrap(), 42);
    }

    #[test]
    fn default_abortable_task() {
        SmolExecutor.block_on(async move {
            let executor = SmolExecutor;

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
        })
    }

    #[test]
    fn task_coroutine() {
        SmolExecutor.block_on(async move {
            let executor = SmolExecutor;

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
        })
    }

    #[test]
    fn task_coroutine_with_context() {
        SmolExecutor.block_on(async move {
            let executor = SmolExecutor;

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
        })
    }

    #[test]
    fn task_coroutine_with_receiver() {
        use futures::stream::StreamExt;

        SmolExecutor.block_on(async move {
            let executor = SmolExecutor;

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
        })
    }

    #[test]
    fn task_coroutine_with_receiver_and_context() {
        use futures::stream::StreamExt;

        SmolExecutor.block_on(async move {
            let executor = SmolExecutor;

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
        })
    }

    #[test]
    fn task_unbounded_coroutine() {
        SmolExecutor.block_on(async move {
            let executor = SmolExecutor;

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
        })
    }

    #[test]
    fn task_unbounded_coroutine_with_context() {
        SmolExecutor.block_on(async move {
            let executor = SmolExecutor;

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
        })
    }

    #[test]
    fn task_unbounded_coroutine_with_receiver() {
        use futures::stream::StreamExt;

        SmolExecutor.block_on(async move {
            let executor = SmolExecutor;

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
        })
    }

    #[test]
    fn task_unbounded_coroutine_with_receiver_and_context() {
        use futures::stream::StreamExt;

        SmolExecutor.block_on(async move {
            let executor = SmolExecutor;

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
        })
    }

    #[test]
    fn timeout_task() {
        SmolExecutor.block_on(async move {
            let executor = SmolExecutor;

            let task = executor.spawn_timeout(
                std::time::Duration::from_millis(10),
                futures::future::pending::<()>(),
            );
            let resp = task.await.unwrap();
            assert!(matches!(resp.unwrap_err(), TimeoutError));
        })
    }

    #[test]
    fn complete_before_timeout_task() {
        SmolExecutor.block_on(async move {
            let executor = SmolExecutor;

            let task = executor.spawn_timeout(
                std::time::Duration::from_millis(10),
                futures::future::ready("Hello"),
            );
            let resp = task.await.unwrap();
            assert!(resp.is_ok());
            let result = resp.unwrap();
            assert_eq!(result, "Hello");
        })
    }

    #[test]
    fn delay_task() {
        SmolExecutor.block_on(async move {
            let executor = SmolExecutor;
            let duration = std::time::Duration::from_millis(20);
            let started = std::time::Instant::now();

            let task = executor.spawn_delay(duration, async { "Hello" });
            let result = task.await.unwrap();

            assert!(started.elapsed() >= duration);
            assert_eq!(result, "Hello");
        })
    }

    #[test]
    fn abortable_delay_task() {
        SmolExecutor.block_on(async move {
            let executor = SmolExecutor;
            let (tx, rx) = futures::channel::oneshot::channel();
            let task =
                executor.spawn_abortable_delay(std::time::Duration::from_secs(5), async move {
                    let _ = tx.send(());
                });

            drop(task);

            assert!(rx.await.is_err());
        })
    }

    #[test]
    fn race_before_timeout_task() {
        SmolExecutor.block_on(async move {
            let executor = SmolExecutor;

            let task = executor.spawn_timeout(std::time::Duration::from_millis(500), async {
                Delay::new(std::time::Duration::from_millis(10)).await;
                "Hello"
            });
            let resp = task.await.unwrap();
            assert!(resp.is_ok());
            let result = resp.unwrap();
            assert_eq!(result, "Hello");
        })
    }

    #[test]
    fn abortable_timeout_task() {
        SmolExecutor.block_on(async move {
            let executor = SmolExecutor;

            let task = executor.spawn_abortable_timeout(
                std::time::Duration::from_millis(10),
                futures::future::pending::<()>(),
            );
            let resp = task.await.unwrap();
            assert!(matches!(resp.unwrap_err(), TimeoutError));
        })
    }

    #[test]
    fn blocking_task() {
        SmolExecutor.block_on(async move {
            let executor = SmolExecutor;

            let task = executor.spawn_blocking(|| {
                std::thread::sleep(std::time::Duration::from_millis(100));
                "Hello"
            });
            let resp = task.await.unwrap();
            assert_eq!(resp, "Hello");
        })
    }
}
