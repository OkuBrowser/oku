# futures-timeout

A simple utility that provides timeouts for futures and streams, which utilizes `futures-timer`. This library is compatible with `wasm32-unknown-unknown` target.

```rust
fn main() {
    futures::executor::block_on(async move {
        use std::time::Duration;
        use futures_timeout::TimeoutExt;
        
        let fut = async {
            futures_timer::Delay::new(Duration::from_secs(30)).await;
        };

        fut.timeout(Duration::from_secs(5))
            .await
            .expect_err("should fail");
    });
}
```
