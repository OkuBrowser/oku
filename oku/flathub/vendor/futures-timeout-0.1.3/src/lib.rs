use std::io;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use futures::future::FusedFuture;
use futures::stream::FusedStream;
use futures::{Future, FutureExt, Stream};
use futures_timer::Delay;
use pin_project::pin_project;

pub trait TimeoutExt: Sized {
    /// Requires a [`Future`] or [`Stream`] to complete before the specific duration has elapsed.
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

impl<T: Sized> TimeoutExt for T {}

#[derive(Debug)]
#[pin_project]
pub struct Timeout<T> {
    #[pin]
    inner: T,
    timer: Option<Delay>,
    duration: Duration,
}

impl<T> Timeout<T> {
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
    type Output = io::Result<T::Output>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();

        let Some(timer) = this.timer.as_mut() else {
            return Poll::Ready(Err(io::ErrorKind::TimedOut.into()));
        };

        match this.inner.poll(cx) {
            Poll::Ready(value) => return Poll::Ready(Ok(value)),
            Poll::Pending => {}
        }

        futures::ready!(timer.poll_unpin(cx));
        this.timer.take();
        Poll::Ready(Err(io::ErrorKind::TimedOut.into()))
    }
}

impl<T: Future> FusedFuture for Timeout<T> {
    fn is_terminated(&self) -> bool {
        self.timer.is_none()
    }
}

impl<T: Stream> Stream for Timeout<T> {
    type Item = io::Result<T::Item>;

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

        futures::ready!(timer.poll_unpin(cx));
        this.timer.take();
        Poll::Ready(Some(Err(io::ErrorKind::TimedOut.into())))
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
    use std::time::Duration;

    use futures::{StreamExt, TryStreamExt};

    use crate::TimeoutExt;

    #[test]
    fn fut_timeout() {
        futures::executor::block_on(
            futures_timer::Delay::new(Duration::from_secs(10)).timeout(Duration::from_secs(5)),
        )
        .expect_err("timeout after timer elapsed");
    }

    #[test]
    fn stream_timeout() {
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
