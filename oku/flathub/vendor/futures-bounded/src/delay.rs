use std::future::Future;
use std::pin::Pin;
use std::task::{ready, Context, Poll};
use std::time::Duration;

use futures_util::future::BoxFuture;
use futures_util::FutureExt as _;

pub struct Delay {
    duration: Duration,
    inner: BoxFuture<'static, ()>,
}

impl Delay {
    #[cfg(feature = "tokio")]
    pub fn tokio(duration: Duration) -> Self {
        Self {
            duration,
            inner: tokio::time::sleep(duration).boxed(),
        }
    }

    #[cfg(feature = "futures-timer")]
    pub fn futures_timer(duration: Duration) -> Self {
        Self {
            duration,
            inner: futures_timer::Delay::new(duration).boxed(),
        }
    }
}

impl Future for Delay {
    type Output = Duration;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();

        ready!(this.inner.poll_unpin(cx));

        Poll::Ready(this.duration)
    }
}
