use futures::future::FusedFuture;
use futures::stream::FusedStream;
use futures::Stream;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

/// A reusable future or stream based on `Option`.
///
/// By default, `Optional` will be empty, similar to `Option::None`, which would return [`Poll::Pending`] when polled,
/// but if a [`Future`] or [`Stream`] is supplied either upon construction via [`Optional::new`] or
/// is set via [`Optional::replace`], it would then be polled once [`Optional`]
/// is polled. Once the future is polled to completion, the results will be returned, with
/// [`Optional`] being empty.
pub struct Optional<T> {
    task: Option<T>,
    waker: Option<Waker>,
}

impl<T: Unpin> Unpin for Optional<T> {}

impl<T> Default for Optional<T> {
    fn default() -> Self {
        Self {
            task: None,
            waker: None,
        }
    }
}

impl<T> From<Option<T>> for Optional<T> {
    fn from(task: Option<T>) -> Self {
        Self { task, waker: None }
    }
}

impl<T> From<T> for Optional<T> {
    fn from(fut: T) -> Self {
        Self {
            task: Some(fut),
            waker: None,
        }
    }
}

impl<T> Optional<T> {
    /// Construct a new [`Optional`] with an existing [`Future`] or [`Stream`].
    pub fn new(task: T) -> Self {
        Self {
            task: Some(task),
            waker: None,
        }
    }

    /// Construct a new [`Optional`] with an existing [`Future`].
    pub fn with_future(future: T) -> Self
    where
        T: Future,
    {
        Self::new(future)
    }

    /// Construct a new [`Optional`] with an existing [`Stream`].
    pub fn with_stream(stream: T) -> Self
    where
        T: Stream,
    {
        Self::new(stream)
    }

    /// Takes the future or stream out, leaving the [`Optional`] empty.
    pub fn take(&mut self) -> Option<T> {
        let fut = self.task.take();
        if let Some(waker) = self.waker.take() {
            waker.wake();
        }
        fut
    }

    /// Returns true if the future or stream still exist.
    pub fn is_some(&self) -> bool {
        self.task.is_some()
    }

    /// Returns false if the future or stream doesn't exist or has been completed.
    pub fn is_none(&self) -> bool {
        self.task.is_none()
    }

    /// Returns reference of the future or stream.
    pub fn as_ref(&self) -> Option<&T> {
        self.task.as_ref()
    }

    /// Returns mutable reference of the future or stream.
    pub fn as_mut(&mut self) -> Option<&mut T> {
        self.task.as_mut()
    }

    /// Replaces the current the future or stream with a new one, returning the previous value if present.
    pub fn replace(&mut self, task: T) -> Option<T> {
        let fut = self.task.replace(task);
        if let Some(waker) = self.waker.take() {
            waker.wake();
        }
        fut
    }

    /// Returns a constructed `Option<Pin<&mut T>>`.
    pub fn as_pin_mut(&mut self) -> Option<Pin<&mut T>>
    where
        T: Unpin,
    {
        self.task.as_mut().map(Pin::new)
    }

    /// Returns a constructed `Option<Pin<&T>>`.
    pub fn as_pin_ref(&self) -> Option<Pin<&T>>
    where
        T: Unpin,
    {
        self.task.as_ref().map(Pin::new)
    }
}

impl<F> Future for Optional<F>
where
    F: Future + Unpin,
{
    type Output = F::Output;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let Some(future) = self.as_pin_mut() else {
            self.waker.replace(cx.waker().clone());
            return Poll::Pending;
        };

        match future.poll(cx) {
            Poll::Ready(output) => {
                self.task.take();
                Poll::Ready(output)
            }
            Poll::Pending => {
                self.waker.replace(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

impl<F: Future> FusedFuture for Optional<F>
where
    F: Future + Unpin,
{
    fn is_terminated(&self) -> bool {
        self.task.is_none()
    }
}

impl<S> Stream for Optional<S>
where
    S: Stream + Unpin,
{
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let Some(stream) = self.as_pin_mut() else {
            self.waker.replace(cx.waker().clone());
            return Poll::Pending;
        };

        match stream.poll_next(cx) {
            Poll::Ready(Some(output)) => Poll::Ready(Some(output)),
            Poll::Ready(None) => {
                self.task.take();
                Poll::Ready(None)
            }
            Poll::Pending => {
                self.waker.replace(cx.waker().clone());
                Poll::Pending
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self.task.as_ref() {
            Some(st) => st.size_hint(),
            None => (0, Some(0)),
        }
    }
}

impl<S> FusedStream for Optional<S>
where
    S: Stream + Unpin,
{
    fn is_terminated(&self) -> bool {
        self.task.is_none()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use futures::StreamExt;

    #[test]
    fn test_optional_future() {
        let mut future = Optional::new(futures::future::ready(0));
        assert!(future.is_some());
        let waker = futures::task::noop_waker_ref();

        let val = Pin::new(&mut future).poll(&mut Context::from_waker(waker));
        assert_eq!(val, Poll::Ready(0));
        assert!(future.is_none());
    }

    #[test]
    fn reusable_optional_future() {
        let mut future = Optional::new(futures::future::ready(0));
        assert!(future.is_some());
        let waker = futures::task::noop_waker_ref();

        let val = Pin::new(&mut future).poll(&mut Context::from_waker(waker));
        assert_eq!(val, Poll::Ready(0));
        assert!(future.is_none());

        future.replace(futures::future::ready(1));
        assert!(future.is_some());

        let val = Pin::new(&mut future).poll(&mut Context::from_waker(waker));
        assert_eq!(val, Poll::Ready(1));
        assert!(future.is_none());
    }

    #[test]
    fn convert_future_to_optional_future() {
        let fut = futures::future::ready(0);

        let mut future = Optional::from(fut);
        assert!(future.is_some());
        let waker = futures::task::noop_waker_ref();

        let val = Pin::new(&mut future).poll(&mut Context::from_waker(waker));
        assert_eq!(val, Poll::Ready(0));
        assert!(future.is_none());
    }

    #[test]
    fn test_optional_stream() {
        let mut stream = Optional::new(futures::stream::once(async { 0 }).boxed());
        assert!(stream.is_some());
        let waker = futures::task::noop_waker_ref();

        let val = Pin::new(&mut stream).poll_next(&mut Context::from_waker(waker));
        assert_eq!(val, Poll::Ready(Some(0)));
        assert!(stream.is_some());

        let val = Pin::new(&mut stream).poll_next(&mut Context::from_waker(waker));
        assert_eq!(val, Poll::Ready(None));
        assert!(stream.is_none());
    }

    #[test]
    fn reusable_optional_stream() {
        let mut stream = Optional::new(futures::stream::once(async { 0 }).boxed());
        assert!(stream.is_some());
        let waker = futures::task::noop_waker_ref();

        let val = Pin::new(&mut stream).poll_next(&mut Context::from_waker(waker));
        assert_eq!(val, Poll::Ready(Some(0)));
        assert!(stream.is_some());

        let val = Pin::new(&mut stream).poll_next(&mut Context::from_waker(waker));
        assert_eq!(val, Poll::Ready(None));
        assert!(stream.is_none());

        stream.replace(futures::stream::once(async { 1 }).boxed());
        assert!(stream.is_some());

        let val = Pin::new(&mut stream).poll_next(&mut Context::from_waker(waker));
        assert_eq!(val, Poll::Ready(Some(1)));
        assert!(stream.is_some());

        let val = Pin::new(&mut stream).poll_next(&mut Context::from_waker(waker));
        assert_eq!(val, Poll::Ready(None));
        assert!(stream.is_none());
    }

    #[test]
    fn convert_stream_to_optional_stream() {
        let st = futures::stream::once(async { 0 }).boxed();

        let mut stream = Optional::from(st);

        assert!(stream.is_some());
        let waker = futures::task::noop_waker_ref();

        let val = Pin::new(&mut stream).poll_next(&mut Context::from_waker(waker));
        assert_eq!(val, Poll::Ready(Some(0)));
        assert!(stream.is_some());

        let val = Pin::new(&mut stream).poll_next(&mut Context::from_waker(waker));
        assert_eq!(val, Poll::Ready(None));
        assert!(stream.is_none());
    }
}
