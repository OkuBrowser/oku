use crate::{ActoHandle, ActoInput, ActoRef, ActoRuntime, Receiver, SupervisionRef};
use std::{
    future::{poll_fn, Future},
    pin::Pin,
    task::Poll,
};

/// The confines of an actor, and the engine that makes it work.
///
/// Every actor is provided with an `ActoCell` when it is started, which is its
/// means of interacting with other actors.
///
/// The type parameter `R` is present so that the Actor can formulate further
/// requirements in its type signature (e.g. `R: MailboxSize`).
pub struct ActoCell<M: Send + 'static, R: ActoRuntime, S: 'static = ()> {
    me: ActoRef<M>,
    runtime: R,
    recv: R::Receiver<M>,
    supervised: Vec<Pin<Box<dyn ActoHandle<Output = S>>>>,
    no_senders_signaled: bool,
}

impl<M: Send + 'static, R: ActoRuntime, S: 'static> Drop for ActoCell<M, R, S> {
    fn drop(&mut self) {
        for mut h in self.supervised.drain(..) {
            h.as_mut().abort_pinned();
        }
        self.me.dead();
    }
}

impl<M: Send + 'static, R: ActoRuntime, S: Send + 'static> ActoCell<M, R, S> {
    pub(crate) fn new(me: ActoRef<M>, runtime: R, recv: R::Receiver<M>) -> Self {
        Self {
            me,
            runtime,
            recv,
            supervised: vec![],
            no_senders_signaled: false,
        }
    }

    /// Get access to the [`ActoRuntime`] driving this actor, e.g. to customize mailbox size for spawned actors.
    ///
    /// See [`MailboxSize`].
    pub fn rt(&self) -> &R {
        &self.runtime
    }

    /// The actor’s own [`ActoRef`] handle, which it may send elsewhere to receive messages.
    pub fn me(&mut self) -> ActoRef<M> {
        self.no_senders_signaled = false;
        self.me.clone()
    }

    /// Asynchronously `.await` the reception of inputs.
    ///
    /// These may either be a message (sent via an [`ActoRef`]), the notification that all
    /// external `ActoRef`s have been dropped, or the termination notice of a supervised
    /// actor.
    pub fn recv(&mut self) -> impl Future<Output = ActoInput<M, S>> + '_ {
        poll_fn(|cx| {
            for idx in 0..self.supervised.len() {
                let p = self.supervised[idx].as_mut().poll(cx);
                if let Poll::Ready(result) = p {
                    let handle = self.supervised.remove(idx);
                    tracing::trace!(src = ?handle.name(), "supervision");
                    return Poll::Ready(ActoInput::Supervision {
                        id: handle.id(),
                        name: handle.name().to_owned(),
                        result,
                    });
                }
            }
            if let Poll::Ready(msg) = self.recv.poll(cx) {
                tracing::trace!("got message");
                return Poll::Ready(ActoInput::Message(msg));
            }
            if self.me.get_count() == 0 {
                tracing::trace!("no more senders");
                if !self.no_senders_signaled {
                    self.no_senders_signaled = true;
                    return Poll::Ready(ActoInput::NoMoreSenders);
                }
            } else if !self.no_senders_signaled {
                // only install waker if we’re interested in emitting NoMoreSenders
                self.me.waker(cx.waker().clone());
                // re-check in case last ref was dropped between check and lock
                if self.me.get_count() == 0 {
                    tracing::trace!(me = ?self.me.name(), "no sender");
                    self.no_senders_signaled = true;
                    return Poll::Ready(ActoInput::NoMoreSenders);
                }
            }
            tracing::trace!("Poll::Pending");
            Poll::Pending
        })
    }

    /// Create a new actor on the same [`ActoRuntime`] as the current one.
    ///
    /// ```rust
    /// use acto::{ActoCell, ActoInput, ActoRuntime};
    ///
    /// async fn actor<M: Send + 'static, R: ActoRuntime>(cell: ActoCell<M, R>) {
    ///     // spawn and forget
    ///     cell.spawn("name", |cell: ActoCell<i32, _>| async move { todo!() });
    ///     // spawn, retrieve handle, do not supervise
    ///     let a_ref = cell.spawn("super", |mut cell: ActoCell<_, _, ()>| async move {
    ///         if let ActoInput::Message(msg) = cell.recv().await {
    ///             cell.supervise(msg);
    ///         }
    ///     }).me;
    ///     // spawn and let some other actor supervise
    ///     let s_ref = cell.spawn("other", |cell: ActoCell<i32, _>| async move { todo!() });
    ///     a_ref.send(s_ref);
    /// }
    /// ```
    pub fn spawn<M2, F, Fut, S2>(
        &self,
        name: &str,
        actor: F,
    ) -> SupervisionRef<M2, R::ActoHandle<Fut::Output>>
    where
        F: FnOnce(ActoCell<M2, R, S2>) -> Fut,
        Fut: Future + Send + 'static,
        Fut::Output: Send + 'static,
        S2: Send + 'static,
        M2: Send + 'static,
    {
        self.runtime.spawn_actor(name, actor)
    }

    /// Create a new actor on the same [`ActoRuntime`] as the current one and [`ActoCell::supervise`] it.
    pub fn spawn_supervised<M2, F, Fut, S2, O>(&mut self, name: &str, actor: F) -> ActoRef<M2>
    where
        F: FnOnce(ActoCell<M2, R, S2>) -> Fut,
        Fut: Future<Output = O> + Send + 'static,
        O: Into<S> + Send + 'static,
        S2: Send + 'static,
        M2: Send + 'static,
    {
        self.supervise(self.spawn(name, actor))
    }

    /// Supervise another actor.
    ///
    /// When that actor terminates, this actor will receive [`ActoInput::Supervision`] for it.
    /// When this actor terminates, all supervised actors will be aborted.
    pub fn supervise<T, H, O>(&mut self, actor: SupervisionRef<T, H>) -> ActoRef<T>
    where
        T: Send + 'static,
        H: ActoHandle<Output = O> + Send + 'static,
        O: Into<S> + Send + 'static,
    {
        tracing::trace!(target = ?actor.me.name(), "supervise");
        self.supervised
            .push(Box::pin(actor.handle.map(|x: O| x.into())));
        actor.me
    }
}
