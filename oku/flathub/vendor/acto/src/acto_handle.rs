use crate::{ActoId, PanicOrAbort};
use std::{
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

/// A handle for aborting or joining a running actor.
pub trait ActoHandle: Send + 'static {
    type Output;

    /// The ID of the underlying actor.
    fn id(&self) -> ActoId;

    /// The name of the underlying actor.
    fn name(&self) -> &str;

    /// Abort the actor’s task.
    ///
    /// Use this method if you don’t need the handle afterwards; otherwise use [`abort_pinned`].
    /// Behavior is undefined if the actor is not [cancellation safe].
    ///
    /// [cancellation safe]: https://docs.rs/tokio/latest/tokio/macro.select.html#cancellation-safety
    fn abort(mut self)
    where
        Self: Sized,
    {
        // safety:
        // - we drop `self` at the end of the scope without moving it
        // - we don’t use Deref or DerefMut; if the implementor does, it’s their responsibility
        let this = unsafe { Pin::new_unchecked(&mut self) };
        this.abort_pinned();
    }

    /// Abort the actor’s task.
    ///
    /// Use this method if you want to [`join`] the actor’s task later, otherwise
    /// prefer the [`abort`] method that can be called without pinning first.
    /// Behavior is undefined if the actor is not [cancellation safe].
    ///
    /// [cancellation safe]: https://docs.rs/tokio/latest/tokio/macro.select.html#cancellation-safety
    fn abort_pinned(self: Pin<&mut Self>);

    /// Check whether the actor’s task is no longer running
    ///
    /// This may be the case even after [`ActoHandle::abort`] has returned, since task
    /// termination may be asynchronous.
    ///
    /// Note that this is not the same as [`ActoRef::is_gone`], which checks whether
    /// the actor is still capable of receiving messages. An actor could drop its
    /// [`ActoCell`] (yielding `true` there) or it could move it to another async task
    /// (yielding `true` here).
    fn is_finished(&self) -> bool;

    /// Poll this handle for whether the actor is now terminated.
    ///
    /// This method has [`Future`] semantics.
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>)
        -> Poll<Result<Self::Output, PanicOrAbort>>;

    /// Transform the output value of this handle, e.g. before calling [`ActoCell::supervise`].
    fn map<O, F>(self, transform: F) -> MappedActoHandle<Self, F, O>
    where
        Self: Sized,
    {
        MappedActoHandle::new(self, transform)
    }

    /// A future for awaiting the termination of the actor underlying this handle.
    fn join(self) -> ActoHandleFuture<Self>
    where
        Self: Sized,
    {
        ActoHandleFuture { handle: self }
    }
}

pin_project_lite::pin_project! {
    /// The future returned by [`ActoHandle::join`].
    pub struct ActoHandleFuture<J> {
        #[pin] handle: J
    }
}

impl<J: ActoHandle> Future for ActoHandleFuture<J> {
    type Output = Result<J::Output, PanicOrAbort>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let _span = tracing::debug_span!("poll", join = ?self.as_ref().handle.name());
        self.project().handle.poll(cx)
    }
}

pin_project_lite::pin_project! {
    /// An [`ActoHandle`] that results from [`ActoHandle::map`].
    pub struct MappedActoHandle<H, F, O> {
        #[pin] inner: H,
        transform: Option<F>,
        _ph: PhantomData<O>,
    }
}

impl<H, F, O> MappedActoHandle<H, F, O> {
    pub(crate) fn new(inner: H, transform: F) -> Self {
        Self {
            inner,
            transform: Some(transform),
            _ph: PhantomData,
        }
    }
}

impl<H, F, O> ActoHandle for MappedActoHandle<H, F, O>
where
    H: ActoHandle,
    F: FnOnce(H::Output) -> O + Send + Sync + Unpin + 'static,
    O: Send + 'static,
{
    type Output = O;

    fn id(&self) -> ActoId {
        self.inner.id()
    }

    fn name(&self) -> &str {
        self.inner.name()
    }

    fn abort_pinned(self: Pin<&mut Self>) {
        self.project().inner.abort_pinned()
    }

    fn is_finished(&self) -> bool {
        self.inner.is_finished()
    }

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<O, PanicOrAbort>> {
        let this = self.project();
        this.inner.poll(cx).map(|r| {
            let transform = this.transform.take().expect("polled after finish");
            r.map(transform)
        })
    }
}
