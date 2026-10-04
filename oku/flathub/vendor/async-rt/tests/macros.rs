#![cfg(all(feature = "macros", not(target_arch = "wasm32")))]

#[cfg(any(
    feature = "tokio",
    feature = "smol",
    feature = "compio",
    feature = "threadpool",
    feature = "lite"
))]
use async_rt::JoinError;
use async_rt::{Executor, ExecutorBlockOn, JoinHandle};
use std::future::Future;

#[derive(Clone, Copy)]
struct CustomDriver;

impl Executor for CustomDriver {
    fn runtime_type(&self) -> Option<&'static str> {
        Some("custom")
    }

    fn spawn<F>(&self, _: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        JoinHandle::empty()
    }
}

impl ExecutorBlockOn for CustomDriver {
    fn block_on<F: Future>(&self, future: F) -> F::Output {
        futures::executor::block_on(future)
    }
}

fn custom_driver() -> CustomDriver {
    CustomDriver
}

#[async_rt::main(driver = custom_driver())]
async fn custom_driver_main() -> usize {
    42
}

#[test]
fn main_accepts_custom_driver() {
    assert_eq!(custom_driver_main(), 42);
}

#[async_rt::test(driver = CustomDriver)]
async fn test_accepts_custom_driver() {
    assert_eq!(CustomDriver.runtime_type(), Some("custom"));
}

#[cfg(feature = "lite")]
#[async_rt::main(executor = "lite")]
async fn explicit_lite_main() -> usize {
    assert_eq!(async_rt::task::runtime_type(), Some("lite"));
    async_rt::task::spawn(async { 42 }).await.unwrap()
}

#[cfg(feature = "lite")]
#[test]
fn lite_main_updates_task_executor() {
    assert_eq!(explicit_lite_main(), 42);
}

#[cfg(feature = "lite")]
#[async_rt::test(executor = "lite")]
async fn explicit_lite_test() {
    assert_eq!(async_rt::task::runtime_type(), Some("lite"));
    assert_eq!(async_rt::task::spawn(async { 42 }).await.unwrap(), 42);
}

#[cfg(feature = "lite")]
#[async_rt::main(executor = "lite")]
async fn hold_lite_executor(
    started: std::sync::mpsc::Sender<()>,
    release: futures::channel::oneshot::Receiver<()>,
) {
    started.send(()).unwrap();
    release.await.unwrap();
}

#[cfg(feature = "lite")]
#[test]
fn lite_macro_entries_run_one_at_a_time() {
    let (started_tx, started_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = futures::channel::oneshot::channel();
    let first = std::thread::spawn(move || hold_lite_executor(started_tx, release_rx));
    started_rx.recv().unwrap();

    let (finished_tx, finished_rx) = std::sync::mpsc::channel();
    let second = std::thread::spawn(move || {
        finished_tx.send(explicit_lite_main()).unwrap();
    });

    assert!(
        finished_rx
            .recv_timeout(std::time::Duration::from_millis(20))
            .is_err()
    );
    release_tx.send(()).unwrap();

    first.join().unwrap();
    second.join().unwrap();
    assert_eq!(finished_rx.recv().unwrap(), 42);
}

#[cfg(any(
    feature = "tokio",
    feature = "smol",
    feature = "compio",
    feature = "threadpool",
    feature = "lite"
))]
#[async_rt::main]
async fn default_main() -> Result<usize, JoinError> {
    async_rt::task::spawn(async { 42 }).await
}

#[cfg(any(
    feature = "tokio",
    feature = "smol",
    feature = "compio",
    feature = "threadpool",
    feature = "lite"
))]
#[test]
fn main_uses_the_global_executor() {
    assert_eq!(default_main().unwrap(), 42);
}

#[cfg(any(
    feature = "tokio",
    feature = "smol",
    feature = "compio",
    feature = "threadpool",
    feature = "lite"
))]
#[async_rt::test]
async fn test_uses_the_global_executor() {
    assert_eq!(async_rt::task::spawn(async { 42 }).await.unwrap(), 42);
}

#[cfg(any(
    feature = "tokio",
    feature = "smol",
    feature = "compio",
    feature = "threadpool",
    feature = "lite"
))]
#[async_rt::test]
async fn test_preserves_result_return_types() -> Result<(), JoinError> {
    assert_eq!(async_rt::task::spawn(async { 42 }).await?, 42);
    Ok(())
}

#[cfg(any(
    feature = "tokio",
    feature = "smol",
    feature = "compio",
    feature = "threadpool",
    feature = "lite"
))]
#[async_rt::test]
#[should_panic(expected = "expected panic")]
async fn test_preserves_harness_attributes() {
    panic!("expected panic");
}

#[cfg(feature = "tokio")]
#[async_rt::test(executor = tokio)]
async fn explicit_tokio_executor() {
    let handle = async_rt::rt::tokio::TokioExecutor.spawn(async { 42 });
    assert_eq!(handle.await.unwrap(), 42);
}

#[cfg(feature = "smol")]
#[async_rt::test(executor = "smol")]
async fn explicit_smol_executor() {
    let handle = async_rt::rt::smol::SmolExecutor.spawn(async { 42 });
    assert_eq!(handle.await.unwrap(), 42);
}

#[cfg(feature = "compio")]
#[async_rt::test(executor = "compio")]
async fn explicit_compio_executor() {
    let handle = async_rt::rt::compio::CompioExecutor.spawn(async { 42 });
    assert_eq!(handle.await.unwrap(), 42);
}

#[cfg(feature = "threadpool")]
#[async_rt::test(executor = "threadpool")]
async fn explicit_threadpool_executor() {
    let handle = async_rt::rt::threadpool::ThreadPoolExecutor.spawn(async { 42 });
    assert_eq!(handle.await.unwrap(), 42);
}
