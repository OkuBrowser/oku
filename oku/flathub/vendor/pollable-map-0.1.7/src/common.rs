use futures::{FutureExt, Stream, StreamExt};
use futures_timeout::{Timeout, TimeoutExt};
use std::future::Future;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

pub struct InnerMap<K, S> {
    key: K,
    inner: Option<S>,
    wake_on_success: bool,
}

impl<K, S> InnerMap<K, S> {
    pub fn new(key: K, inner: S) -> Self {
        Self {
            key,
            inner: Some(inner),
            wake_on_success: false,
        }
    }

    pub fn set_wake_on_success(&mut self, wake_on_success: bool) -> bool {
        let prev = self.wake_on_success;
        self.wake_on_success = wake_on_success;
        wake_on_success != prev
    }

    pub fn key(&self) -> &K {
        &self.key
    }

    pub fn key_value(&self) -> Option<(&K, &S)> {
        let Self { key, inner, .. } = self;
        inner.as_ref().map(|st| (key, st))
    }

    pub fn key_value_mut(&mut self) -> Option<(&K, &mut S)> {
        let Self { ref key, inner, .. } = self;
        inner.as_mut().map(|s| (key, s))
    }

    pub fn inner(&self) -> Option<&S> {
        self.inner.as_ref()
    }

    pub fn inner_mut(&mut self) -> Option<&mut S> {
        self.inner.as_mut()
    }

    pub fn take_inner(&mut self) -> Option<S> {
        self.inner.take()
    }
}

impl<K, S> InnerMap<K, S>
where
    K: Unpin + Clone,
    S: Unpin,
{
    pub fn key_value_pin(&mut self) -> Option<(&K, Pin<&mut S>)> {
        let Self { ref key, inner, .. } = self;
        inner.as_mut().map(|s| (key, Pin::new(s)))
    }

    pub fn inner_pin(&mut self) -> Option<Pin<&mut S>> {
        self.inner_mut().map(Pin::new)
    }
}

impl<K, S> Future for InnerMap<K, S>
where
    K: Clone + Unpin,
    S: Future + Unpin,
{
    type Output = (K, Option<S::Output>);

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = &mut *self;
        let Some(st) = this.inner_pin() else {
            return Poll::Ready((this.key.clone(), None));
        };

        let output = futures::ready!(st.poll(cx));
        this.inner.take();
        Poll::Ready((this.key.clone(), Some(output)))
    }
}

impl<K, S> Stream for InnerMap<K, S>
where
    K: Clone + Unpin,
    S: Stream + Unpin,
{
    type Item = (K, Option<S::Item>);

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = &mut *self;
        let Some(st) = this.inner_pin() else {
            // Note: While we could panic for any attempts to poll the stream that doesnt exist or have been terminated,
            //       we opt to just return `Poll::Ready(None)` and letting upstream define how to handle a terminated stream
            //       although in the future this could change as we should not be polling any terminated streams or futures.
            return Poll::Ready(None);
        };

        match st.poll_next(cx) {
            Poll::Ready(Some(value)) => {
                if this.wake_on_success {
                    // Since we made progress, we should attempt to proceed further by waking up the task
                    // TODO: Find a better way to wake task up without needing to call the waker on every successful result
                    //       from stream
                    cx.waker().wake_by_ref();
                }
                Poll::Ready(Some((this.key.clone(), Some(value))))
            }
            Poll::Ready(None) => {
                // Note: Although some streams can return a `Poll::Ready(None)`, we will have to assume that the stream is completely finished
                //       and terminated at this point and that we should not attempt to poll again.
                //       In the future, we could probably provide a flag that would allow us to take the inner stream or keep it and attempt on polling it again
                //       without actually terminating it.
                this.inner.take();
                Poll::Ready(Some((this.key.clone(), None)))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

pub struct Timed<F>(Timeout<F>);

impl<F> Deref for Timed<F> {
    type Target = F;
    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl<F> DerefMut for Timed<F> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.deref_mut()
    }
}

impl<F> Timed<F> {
    pub(crate) fn new(item: F, timeout: Duration) -> Self {
        Self(item.timeout(timeout))
    }
}

impl<F> Future for Timed<F>
where
    F: Future + Unpin,
{
    type Output = std::io::Result<F::Output>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.0.poll_unpin(cx)
    }
}

impl<F> Stream for Timed<F>
where
    F: Stream + Unpin,
{
    type Item = std::io::Result<F::Item>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.0.poll_next_unpin(cx)
    }
}
