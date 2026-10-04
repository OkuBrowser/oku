use crate::acto_ref::ActoRefInner;
use std::{
    future::Future,
    marker::PhantomData,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use sync_wrapper::SyncWrapper;

/// A named closure for sending messages to a given actor.
///
/// This type is used between a runtime implementation and `acto`.
pub trait Sender<M>: Send + Sync + 'static {
    fn send(&self, msg: M) -> bool;
    fn send_wait(&self, msg: M) -> Pin<Box<dyn Future<Output = bool> + Send + 'static>>;
}

pub(crate) struct BlackholeSender<M>(PhantomData<SyncWrapper<M>>);

impl<M> BlackholeSender<M> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<M: Send + 'static> Sender<M> for BlackholeSender<M> {
    fn send(&self, _msg: M) -> bool {
        false
    }
    fn send_wait(&self, _msg: M) -> Pin<Box<dyn Future<Output = bool> + Send + 'static>> {
        Box::pin(async { false })
    }
}

pub(crate) struct MappedSender<M, M2, F> {
    f: F,
    inner: Arc<ActoRefInner<dyn Sender<M>>>,
    _ph: PhantomData<SyncWrapper<M2>>,
}

impl<M, M2, F> MappedSender<M, M2, F> {
    pub fn new(f: F, s: Arc<ActoRefInner<dyn Sender<M>>>) -> Self {
        Self {
            f,
            inner: s,
            _ph: PhantomData,
        }
    }
}

impl<M: Send + 'static, M2: Send + 'static, F> Sender<M2> for MappedSender<M, M2, F>
where
    F: Fn(M2) -> M + Send + Sync + 'static,
{
    fn send(&self, msg: M2) -> bool {
        self.inner.sender.send((self.f)(msg))
    }
    fn send_wait(&self, msg: M2) -> Pin<Box<dyn Future<Output = bool> + Send + 'static>> {
        self.inner.sender.send_wait((self.f)(msg))
    }
}

/// A named closure for receiving messages at a given actor.
///
/// This type is used between a runtime implementation and `acto`.
pub trait Receiver<M>: Send + 'static {
    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<M>;
}
