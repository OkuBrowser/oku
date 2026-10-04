//! Implements the [`MaybeFuture`] utility.

use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use pin_project::pin_project;

/// A future which may not be present.
///
/// This is a single type which may optionally contain a future.  If there is no inner
/// future polling will always return [`Poll::Pending`].
///
/// When the inner future is set, then [`MaybeFuture`] is polled and the inner future
/// completes, then the poll returns the value of the inner future and [`MaybeFuture`]'s
/// state is set to None.
///
/// The [`Default`] impl will create a [`MaybeFuture`] without an inner.
///
/// # Example
///
/// One major use case for this is ergonomically disabling branches in a `tokio::select!`.
///
/// ```ignore-wasm32-unknown-unknown
/// use std::time::Duration;
///
/// use n0_future::{task, time, MaybeFuture};
///
/// # #[tokio::main(flavor = "current_thread", start_paused = true)]
/// # async fn main() {
/// let start = time::Instant::now();
///
/// let (send, mut recv) = tokio::sync::mpsc::channel(10);
/// task::spawn(async move {
///     // Send for the first time after 2s
///     time::sleep(Duration::from_millis(2000)).await;
///     let _ = send.send(()).await;
///     println!("{:?}: Sent", start.elapsed());
///     // Send after only 100ms
///     time::sleep(Duration::from_millis(100)).await;
///     let _ = send.send(()).await;
///     println!("{:?}: Sent", start.elapsed());
///     // Send again after only 100ms
///     time::sleep(Duration::from_millis(100)).await;
///     let _ = send.send(()).await;
///     println!("{:?}: Sent", start.elapsed());
///     // Finally send "too late" after 1100ms:
///     time::sleep(Duration::from_millis(1100)).await;
///     let _ = send.send(()).await;
///     println!("{:?}: Sent", start.elapsed());
/// });
///
/// let mut timeout_fut = std::pin::pin!(MaybeFuture::default());
/// loop {
///     tokio::select! {
///         // If a timeout hasn't been set yet (a first msg hasn't been received)
///         // then this won't trigger.
///         _ = &mut timeout_fut => {
///             println!("{:?}: Timed out!", start.elapsed());
///             return;
///         }
///         _ = recv.recv() => {
///             // Set (or reset) the timeout
///             timeout_fut.as_mut().set_future(time::sleep(Duration::from_millis(1000)));
///             println!("{:?}: Received!", start.elapsed());
///         }
///     }
/// }
/// # }
/// ```
///
/// This example prints:
/// ```plain
/// 2s: Sent
/// 2s: Received!
/// 2.1s: Sent
/// 2.1s: Received!
/// 2.2s: Sent
/// 2.2s: Received!
/// 3.2s: Timed out!
/// ```
///
/// The last send times out, but it doesn't time out before the first send.
#[derive(Default, Debug)]
#[pin_project(project = MaybeFutureProj, project_replace = MaybeFutureProjReplace)]
pub enum MaybeFuture<T> {
    /// The state in which it wraps a future to be polled.
    Some(#[pin] T),
    /// The state in which there's no future set, and polling will always return [`Poll::Pending`]
    #[default]
    None,
}

impl<T> MaybeFuture<T> {
    /// Sets the future to None again.
    pub fn set_none(mut self: Pin<&mut Self>) {
        self.as_mut().project_replace(Self::None);
    }

    /// Sets a new future.
    pub fn set_future(mut self: Pin<&mut Self>, fut: T) {
        self.as_mut().project_replace(Self::Some(fut));
    }

    /// Returns `true` if the inner is empty.
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    /// Returns `true` if the inner contains a future.
    pub fn is_some(&self) -> bool {
        matches!(self, Self::Some(_))
    }
}

impl<T: Future> Future for MaybeFuture<T> {
    type Output = T::Output;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.as_mut().project();
        let poll_res = match this {
            MaybeFutureProj::Some(ref mut t) => t.as_mut().poll(cx),
            MaybeFutureProj::None => Poll::Pending,
        };
        match poll_res {
            Poll::Ready(val) => {
                self.as_mut().project_replace(Self::None);
                Poll::Ready(val)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

#[cfg(all(not(wasm_browser), test))]
mod tests {
    use std::pin::pin;

    use super::*;
    use crate::time::Duration;

    #[tokio::test(start_paused = true)]
    async fn test_maybefuture_poll_after_use() {
        let fut = async move { "hello" };
        let mut maybe_fut = pin!(MaybeFuture::Some(fut));
        let res = (&mut maybe_fut).await;

        assert_eq!(res, "hello");

        // Now poll again
        let res = tokio::time::timeout(Duration::from_millis(10), maybe_fut).await;
        assert!(res.is_err());
    }

    #[tokio::test(start_paused = true)]
    async fn test_maybefuture_mut_ref() {
        let mut fut = Box::pin(async move { "hello" });
        let mut maybe_fut = pin!(MaybeFuture::Some(&mut fut));
        let res = (&mut maybe_fut).await;

        assert_eq!(res, "hello");

        // Now poll again
        let res = tokio::time::timeout(Duration::from_millis(10), maybe_fut).await;
        assert!(res.is_err());
    }

    #[tokio::test(start_paused = true)]
    async fn example() {
        use std::time::Duration;

        use crate::{task, time};

        let start = time::Instant::now();

        let (send, mut recv) = tokio::sync::mpsc::channel(10);
        task::spawn(async move {
            // Send for the first time after 2s
            time::sleep(Duration::from_millis(2000)).await;
            let _ = send.send(()).await;
            println!("{:?}: Sent", start.elapsed());
            // Send after only 100ms
            time::sleep(Duration::from_millis(100)).await;
            let _ = send.send(()).await;
            println!("{:?}: Sent", start.elapsed());
            // Send again after only 100ms
            time::sleep(Duration::from_millis(100)).await;
            let _ = send.send(()).await;
            println!("{:?}: Sent", start.elapsed());
            // Finally send "too late" after 1100ms:
            time::sleep(Duration::from_millis(1100)).await;
            let _ = send.send(()).await;
            println!("{:?}: Sent", start.elapsed());
        });

        let mut timeout_fut = std::pin::pin!(MaybeFuture::default());
        loop {
            tokio::select! {
                // If a timeout hasn't been set yet (a first msg hasn't been received)
                // then this won't trigger.
                _ = &mut timeout_fut => {
                    println!("{:?}: Timed out!", start.elapsed());
                    return;
                }
                _ = recv.recv() => {
                    // Set (or reset) the timeout
                    timeout_fut.as_mut().set_future(time::sleep(Duration::from_millis(1000)));
                    println!("{:?}: Received!", start.elapsed());
                }
            }
        }
    }
}
