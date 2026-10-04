#![cfg(all(
    feature = "macros",
    feature = "tokio",
    feature = "smol",
    not(target_arch = "wasm32")
))]

#[async_rt::test(executor = "smol")]
async fn explicit_executor_updates_task_executor() {
    assert_eq!(async_rt::task::runtime_type(), Some("smol"));
    assert_eq!(async_rt::task::spawn(async { 42 }).await.unwrap(), 42);
}
