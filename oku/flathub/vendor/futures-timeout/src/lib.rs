use core::ops::{Deref, DerefMut};
use core::pin::Pin;
use core::task::{Context, Poll};
use core::time::Duration;

use futures_core::future::FusedFuture;
use futures_core::stream::FusedStream;
use futures_core::{Future, Stream};
use futures_timer::Delay;
use pin_project::pin_project;

pub trait TimeoutStreamExt: Stream + Sized {
    /// Requires a [`Stream`] to complete before the specific duration has elapsed.
    ///
    /// **Note: If a [`Stream`] returns an item, the timer will reset until `Poll::Ready(None)` is returned**
    fn timeout(self, duration: Duration) -> Timeout<Self> {
        Timeout {
            inner: self,
            timer: Some(Delay::new(duration)),
            duration,
        }
    }
}

pub trait TimeoutFutureExt: Future + Sized {
    /// Requires a [`Future`] to complete before the specific duration has elapsed.
    fn timeout(self, duration: Duration) -> Timeout<Self> {
        Timeout {
            inner: self,
            timer: Some(Delay::new(duration)),
            duration,
        }
    }
}

impl<T> TimeoutFutureExt for T where T: Future {}
impl<T> TimeoutStreamExt for T where T: Stream {}

#[derive(Debug)]
pub struct TimeoutError;
impl core::fmt::Display for TimeoutError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Operation timed out")
    }
}

impl core::error::Error for TimeoutError {}

#[derive(Debug)]
#[pin_project]
pub struct Timeout<T> {
    #[pin]
    inner: T,
    timer: Option<Delay>,
    duration: Duration,
}

impl<T> Timeout<T> {

    /// Create a timeout for Stream or Future.
    pub fn new(inner: T, duration: Duration) -> Self {
        Timeout {
            inner,
            timer: Some(Delay::new(duration)),
            duration,
        }
    }

    /// Create a timeout from a stream
    pub fn from_stream(inner: T, duration: Duration) -> Self
    where
        T: Stream,
    {
        Timeout {
            inner,
            timer: Some(Delay::new(duration)),
            duration,
        }
    }

    /// Create a timeout from a future
    pub fn from_future(inner: T, duration: Duration) -> Self
    where
        T: Future,
    {
        Timeout {
            inner,
            timer: Some(Delay::new(duration)),
            duration,
        }
    }

    /// Consumes Timeout and returns the inner value
    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T> Deref for Timeout<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for Timeout<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T: Future> Future for Timeout<T> {
    type Output = Result<T::Output, TimeoutError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();

        let Some(timer) = this.timer.as_mut() else {
            return Poll::Ready(Err(TimeoutError));
        };

        match this.inner.poll(cx) {
            Poll::Ready(value) => {
                this.timer.take();
                return Poll::Ready(Ok(value));
            }
            Poll::Pending => {}
        }

        core::task::ready!(Pin::new(timer).poll(cx));
        this.timer.take();
        Poll::Ready(Err(TimeoutError))
    }
}

impl<T: Future> FusedFuture for Timeout<T> {
    fn is_terminated(&self) -> bool {
        self.timer.is_none()
    }
}

impl<T: Stream> Stream for Timeout<T> {
    type Item = Result<T::Item, TimeoutError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();

        let Some(timer) = this.timer.as_mut() else {
            return Poll::Ready(None);
        };

        match this.inner.poll_next(cx) {
            Poll::Ready(Some(value)) => {
                timer.reset(*this.duration);
                return Poll::Ready(Some(Ok(value)));
            }
            Poll::Ready(None) => {
                this.timer.take();
                return Poll::Ready(None);
            }
            Poll::Pending => {}
        }

        core::task::ready!(Pin::new(timer).poll(cx));
        this.timer.take();
        Poll::Ready(Some(Err(TimeoutError)))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<T: Stream> FusedStream for Timeout<T> {
    fn is_terminated(&self) -> bool {
        self.timer.is_none()
    }
}

#[cfg(test)]
mod test {
    use core::time::Duration;

    use futures::{StreamExt, TryStreamExt};

    #[test]
    fn fut_timeout() {
        use crate::TimeoutFutureExt;
        futures::executor::block_on(
            futures_timer::Delay::new(Duration::from_secs(10)).timeout(Duration::from_secs(5)),
        )
        .expect_err("timeout after timer elapsed");
    }

    #[test]
    fn stream_timeout() {
        use crate::TimeoutStreamExt;
        futures::executor::block_on(async move {
            let mut st = futures::stream::once(async move {
                futures_timer::Delay::new(Duration::from_secs(10)).await;
                0
            })
            .timeout(Duration::from_secs(5))
            .boxed();

            st.try_next()
                .await
                .expect_err("timeout after timer elapsed");
        });
    }
}
