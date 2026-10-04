use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use futures::Stream;

#[cfg(all(feature = "std", feature = "timeout"))]
use futures_timeout::Timeout;

#[cfg(all(feature = "std", feature = "timeout"))]
use crate::error::TimedError;

#[cfg(feature = "alloc")]
#[pin_project::pin_project]
pub struct InnerMap<K, S> {
    key: K,
    #[pin]
    inner: Option<S>,
    wake_on_success: bool,
}

#[cfg(feature = "alloc")]
impl<K, S> InnerMap<K, S> {
    pub fn new(key: K, inner: S) -> Self {
        Self {
            key,
            inner: Some(inner),
            wake_on_success: false,
        }
    }

    pub fn set_wake_on_success_pin(self: Pin<&mut Self>, wake_on_success: bool) -> bool {
        let this = self.project();
        let previous = *this.wake_on_success;

        *this.wake_on_success = wake_on_success;
        previous != wake_on_success
    }

    pub fn key(&self) -> &K {
        &self.key
    }

    pub fn key_pin(self: Pin<&Self>) -> &K {
        self.project_ref().key
    }

    pub fn inner_pin_ref(self: Pin<&Self>) -> Option<Pin<&S>> {
        self.project_ref().inner.as_pin_ref()
    }

    pub fn key_value_pin_ref(self: Pin<&Self>) -> Option<(&K, Pin<&S>)> {
        let this = self.project_ref();

        this.inner.as_pin_ref().map(|inner| (this.key, inner))
    }

    pub fn key_value_mut(&mut self) -> Option<(&K, &mut S)> {
        let Self { ref key, inner, .. } = self;
        inner.as_mut().map(|s| (key, s))
    }

    pub fn inner_mut(&mut self) -> Option<&mut S> {
        self.inner.as_mut()
    }

    pub fn take_inner(&mut self) -> Option<S> {
        self.inner.take()
    }

    pub fn take_inner_pin(self: Pin<&mut Self>) -> Option<S>
    where
        S: Unpin,
    {
        self.project().inner.get_mut().take()
    }

    pub fn key_value_pin(self: Pin<&mut Self>) -> Option<(&K, Pin<&mut S>)> {
        let this = self.project();
        this.inner.as_pin_mut().map(|inner| (&*this.key, inner))
    }

    pub fn inner_pin(self: Pin<&mut Self>) -> Option<Pin<&mut S>> {
        self.project().inner.as_pin_mut()
    }
}

#[cfg(feature = "alloc")]
impl<K, S> Future for InnerMap<K, S>
where
    K: Clone,
    S: Future,
{
    type Output = (K, Option<S::Output>);

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();

        let Some(st) = this.inner.as_mut().as_pin_mut() else {
            return Poll::Ready((this.key.clone(), None));
        };

        let output = futures::ready!(st.poll(cx));
        this.inner.set(None);
        Poll::Ready((this.key.clone(), Some(output)))
    }
}

#[cfg(feature = "alloc")]
impl<K, S> Stream for InnerMap<K, S>
where
    K: Clone,
    S: Stream,
{
    type Item = (K, Option<S::Item>);

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        let Some(st) = this.inner.as_mut().as_pin_mut() else {
            // Note: While we could panic for any attempts to poll the stream that doesnt exist or have been terminated,
            //       we opt to just return `Poll::Ready(None)` and letting upstream define how to handle a terminated stream
            //       although in the future this could change as we should not be polling any terminated streams or futures.
            return Poll::Ready(None);
        };

        match st.poll_next(cx) {
            Poll::Ready(Some(value)) => {
                if *this.wake_on_success {
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
                this.inner.set(None);
                Poll::Ready(Some((this.key.clone(), None)))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

#[cfg(all(feature = "std", feature = "timeout"))]
#[pin_project::pin_project]
pub struct Timed<F>(#[pin] Timeout<F>);

#[cfg(all(feature = "std", feature = "timeout"))]
impl<F> Timed<F> {
    pub(crate) fn into_inner(self) -> F {
        self.0.into_inner()
    }
}

#[cfg(all(feature = "std", feature = "timeout"))]
impl<F> core::ops::Deref for Timed<F> {
    type Target = F;
    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

#[cfg(all(feature = "std", feature = "timeout"))]
impl<F> core::ops::DerefMut for Timed<F> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.deref_mut()
    }
}

#[cfg(all(feature = "std", feature = "timeout"))]
impl<F> Timed<F> {
    pub(crate) fn new(item: F, timeout: core::time::Duration) -> Self {
        Self(Timeout::new(item, timeout))
    }
}

#[cfg(all(feature = "std", feature = "timeout"))]
impl<F> Future for Timed<F>
where
    F: Future,
{
    type Output = Result<F::Output, TimedError>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.as_mut().project();
        this.0.poll(cx).map_err(|_| TimedError)
    }
}

#[cfg(all(feature = "std", feature = "timeout"))]
impl<F> Stream for Timed<F>
where
    F: Stream,
{
    type Item = Result<F::Item, TimedError>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.as_mut().project();
        this.0.poll_next(cx).map_err(|_| TimedError)
    }
}
