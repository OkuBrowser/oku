use crate::{
    CompletionGuard, Executor, ExecutorBlockOn, ExecutorBlocking, ExecutorTimeout, InnerJoinHandle,
    JoinHandle, abortable_result,
};
use futures::executor::ThreadPool;
use futures::future::AbortHandle;
use pollable_map::optional::Optional;
use std::fmt::Debug;
use std::future::Future;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, LazyLock};

static THREADPOOL_EXECUTOR: LazyLock<ThreadPool> = LazyLock::new(|| ThreadPool::new().unwrap());

/// Executor backed by [`futures::executor::ThreadPool`].
///
/// Note that this executor will use a global threadpool rather than a per-instance threadpool.
/// In other words, creating a new instance of `ThreadPoolExecutor` would continue to reuse the existing thread pool.
#[derive(Default, Clone, Copy, Debug, PartialOrd, PartialEq, Eq)]
pub struct ThreadPoolExecutor;

impl Executor for ThreadPoolExecutor {
    fn runtime_type(&self) -> Option<&'static str> {
        Some("threadpool")
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
            let _ = tx.send(val);
        };

        THREADPOOL_EXECUTOR.spawn_ok(fut);
        let inner = InnerJoinHandle::CustomHandle {
            inner: Optional::new(rx),
            handle: abort_handle,
            finished,
        };

        JoinHandle { inner }
    }
}

impl ExecutorBlocking for ThreadPoolExecutor {
    fn spawn_blocking<F, R>(&self, f: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let fut = async move {
            let (tx, rx) = futures::channel::oneshot::channel();
            let _handle = std::thread::spawn(move || {
                let val = f();
                let _ = tx.send(val);
            });
            rx.await.expect("shouldnt drop")
        };
        self.spawn(fut)
    }
}

impl ExecutorTimeout for ThreadPoolExecutor {}

impl ExecutorBlockOn for ThreadPoolExecutor {
    fn block_on<F: Future>(&self, f: F) -> F::Output {
        futures::executor::block_on(f)
    }
}

#[cfg(test)]
mod tests {
    use super::ThreadPoolExecutor;
    use crate::error::JoinError;
    use crate::{Executor, ExecutorBlocking, ExecutorTimeout, TimeoutError};
    use futures::channel::mpsc::{Receiver, UnboundedReceiver};

    async fn task(tx: futures::channel::oneshot::Sender<()>) {
        futures_timer::Delay::new(std::time::Duration::from_secs(5)).await;
        let _ = tx.send(());
        unreachable!();
    }

    #[cfg(panic = "unwind")]
    #[test]
    fn task_panic_is_classified() {
        async fn panic_task() -> usize {
            panic!("expected task panic");
        }

        let result = futures::executor::block_on(ThreadPoolExecutor.spawn(panic_task()));

        assert!(matches!(result, Err(JoinError::Panicked)));
    }

    #[test]
    fn explicit_abort_is_reported_as_aborted() {
        let handle = ThreadPoolExecutor.spawn(futures::future::pending::<()>());

        handle.abort();

        assert!(matches!(
            futures::executor::block_on(handle),
            Err(JoinError::Aborted)
        ));
    }

    #[test]
    fn default_abortable_task() {
        let executor = ThreadPoolExecutor::default();

        let (tx, rx) = futures::channel::oneshot::channel::<()>();
        let handle = executor.spawn_abortable(task(tx));
        drop(handle);

        futures::executor::block_on(async move {
            let result = rx.await;
            assert!(result.is_err());
        });
    }

    #[test]
    fn task_coroutine() {
        let executor = ThreadPoolExecutor::default();

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
        futures::executor::block_on(async move {
            task.send(msg).await.unwrap();
            let resp = rx.await.unwrap();
            assert_eq!(resp, "Hello");
        });
    }

    #[test]
    fn task_coroutine_with_context() {
        let executor = ThreadPoolExecutor::default();

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
        futures::executor::block_on(async move {
            task.send(tx1).await.unwrap();
            task.send(tx2).await.unwrap();
            assert_eq!(rx1.await.unwrap(), 1);
            assert_eq!(rx2.await.unwrap(), 2);
        });
    }

    #[test]
    fn task_coroutine_with_receiver() {
        use futures::stream::StreamExt;
        let executor = ThreadPoolExecutor::default();

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
        futures::executor::block_on(async move {
            task.send(msg).await.unwrap();
            let resp = rx.await.unwrap();
            assert_eq!(resp, "Hello");
        });
    }

    #[test]
    fn task_coroutine_with_receiver_and_context() {
        use futures::stream::StreamExt;
        let executor = ThreadPoolExecutor::default();

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
                            _ = resp.send(state.message.clone()).unwrap();
                        }
                    }
                }
            },
        );

        futures::executor::block_on(async move {
            let msg = Message::Set("Hello".into());

            task.send(msg).await.unwrap();
            let (tx, rx) = futures::channel::oneshot::channel::<String>();
            let msg = Message::Get(tx);
            task.send(msg).await.unwrap();
            let resp = rx.await.unwrap();
            assert_eq!(resp, "Hello");
        });
    }

    #[test]
    fn task_unbounded_coroutine() {
        let executor = ThreadPoolExecutor::default();

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
        futures::executor::block_on(async move {
            task.send(msg).unwrap();
            let resp = rx.await.unwrap();
            assert_eq!(resp, "Hello");
        });
    }

    #[test]
    fn task_unbounded_coroutine_with_context() {
        let executor = ThreadPoolExecutor::default();

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
        futures::executor::block_on(async move {
            task.send(tx1).unwrap();
            task.send(tx2).unwrap();
            assert_eq!(rx1.await.unwrap(), 1);
            assert_eq!(rx2.await.unwrap(), 2);
        });
    }

    #[test]
    fn task_unbounded_coroutine_with_receiver() {
        use futures::stream::StreamExt;
        let executor = ThreadPoolExecutor::default();

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
        futures::executor::block_on(async move {
            task.send(msg).unwrap();
            let resp = rx.await.unwrap();
            assert_eq!(resp, "Hello");
        });
    }

    #[test]
    fn task_unbounded_coroutine_with_receiver_and_context() {
        use futures::stream::StreamExt;
        let executor = ThreadPoolExecutor::default();

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
                            _ = resp.send(state.message.clone()).unwrap();
                        }
                    }
                }
            },
        );

        futures::executor::block_on(async move {
            let msg = Message::Set("Hello".into());

            task.send(msg).unwrap();
            let (tx, rx) = futures::channel::oneshot::channel::<String>();
            let msg = Message::Get(tx);
            task.send(msg).unwrap();
            let resp = rx.await.unwrap();
            assert_eq!(resp, "Hello");
        });
    }

    #[test]
    fn timeout_task() {
        futures::executor::block_on(async {
            let executor = ThreadPoolExecutor::default();

            let task = executor.spawn_timeout(
                std::time::Duration::from_millis(10),
                futures::future::pending::<()>(),
            );
            let resp = task.await.unwrap();
            assert!(matches!(resp.unwrap_err(), TimeoutError));
        })
    }

    #[test]
    fn abortable_timeout_task() {
        futures::executor::block_on(async {
            let executor = ThreadPoolExecutor::default();

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
        let executor = ThreadPoolExecutor::default();

        futures::executor::block_on(async move {
            let result = executor
                .spawn_blocking(|| {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    "Hello"
                })
                .await;
            assert_eq!(result.unwrap(), "Hello");
        })
    }
}
