use crate::{acto_cell::ActoCell, ActoHandle, ActoId, ActoRef, Receiver, Sender, SupervisionRef};
use smol_str::SmolStr;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

/// For implementors: the interface of a runtime for operating actors.
///
/// Cloning a runtime should be cheap, it SHOULD be using the `Arc<Inner>` pattern.
pub trait ActoRuntime: Clone + Send + Sync + 'static {
    /// The type of handle used for joining the actor’s task.
    type ActoHandle<O: Send + 'static>: ActoHandle<Output = O>;
    /// The type of sender for emitting messages towards the actor.
    type Sender<M: Send + 'static>: Sender<M>;
    /// The type of receiver for obtaining messages sent towards the actor.
    type Receiver<M: Send + 'static>: Receiver<M>;

    /// A name for this runtime, used mainly in logging.
    fn name(&self) -> &str;

    /// Create a new pair of sender and receiver for a fresh actor.
    fn mailbox<M: Send + 'static>(&self) -> (Self::Sender<M>, Self::Receiver<M>);

    /// Spawn an actor’s task to be driven independently and return an [`ActoHandle`]
    /// to abort or join it.
    fn spawn_task<T>(&self, id: ActoId, name: SmolStr, task: T) -> Self::ActoHandle<T::Output>
    where
        T: Future + Send + 'static,
        T::Output: Send + 'static;

    /// Provided function for spawning actors.
    ///
    /// Uses the above utilities and cannot be implemented by downstream crates.
    fn spawn_actor<M, F, Fut, S>(
        &self,
        name: &str,
        actor: F,
    ) -> SupervisionRef<M, Self::ActoHandle<Fut::Output>>
    where
        M: Send + 'static,
        F: FnOnce(ActoCell<M, Self, S>) -> Fut,
        Fut: Future + Send + 'static,
        Fut::Output: Send + 'static,
        S: Send + 'static,
    {
        let (sender, recv) = self.mailbox();
        let id = ActoId::next();
        let mut id_str = [0u8; 16];
        let id_str = write_id(&mut id_str, id);
        let name = [name, "(", self.name(), "/", id_str, ")"]
            .into_iter()
            .collect::<SmolStr>();
        let me = ActoRef::new(id, name.clone(), sender);
        let ctx = ActoCell::new(me.clone(), self.clone(), recv);
        let _span = tracing::debug_span!("creating", actor = %name).entered();
        tracing::trace!("create");
        let task = LoggingTask::new(name.clone(), (actor)(ctx));
        let join = self.spawn_task(id, name, task);
        SupervisionRef { me, handle: join }
    }
}

pin_project_lite::pin_project! {
    struct LoggingTask<F> {
        name: SmolStr,
        // Span is constructed once at task creation and kept alive
        // for the task's entire lifetime. Re-creating a fresh span
        // on every `poll()` (the previous behaviour) makes the span
        // refcount drop to zero between polls, so any inner future
        // that captured a reference to "the current span" — for
        // example via `Span::current()` — could later trigger
        // `tracing-subscriber`'s sharded registry assertion
        // `assert_ne!(refs, 0, "tried to clone a span that already
        // closed")` from `clone_span`. Reproduced under heavy load
        // on the `swarm-discovery` + `iroh-mdns` stack inside the
        // `wwkg-custodian` daemon.
        span: tracing::Span,
        #[pin]
        future: F,
    }
}

impl<F> LoggingTask<F> {
    pub fn new(name: SmolStr, future: F) -> Self {
        let span = tracing::debug_span!("poll", actor = %name);
        Self { name, span, future }
    }
}

impl<F> Future for LoggingTask<F>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let _entered = this.span.enter();
        this.future.poll(cx)
    }
}

fn write_id(buf: &mut [u8; 16], id: ActoId) -> &str {
    let id = id.0;
    if id == 0 {
        return "0";
    }
    let mut written = 0;
    let mut shift = (id.ilog2() & !3) + 4;
    while shift != 0 {
        shift -= 4;
        const HEX: [u8; 16] = *b"0123456789abcdef";
        buf[written] = HEX[(id >> shift) & 15];
        written += 1;
    }
    unsafe { std::str::from_utf8_unchecked(&buf[..written]) }
}

#[test]
fn test_write_id() {
    let mut buf = [0u8; 16];
    for i in 0..1000000 {
        assert_eq!(write_id(&mut buf, ActoId(i)), &format!("{:x}", i));
    }
    #[cfg(target_pointer_width = "64")]
    assert_eq!(write_id(&mut buf, ActoId(usize::MAX)), "ffffffffffffffff");
    #[cfg(target_pointer_width = "32")]
    assert_eq!(write_id(&mut buf, ActoId(usize::MAX)), "ffffffff");
}

/// This trait is implemented by [`ActoRuntime`]s that allow customization of the mailbox size.
///
/// ```rust
/// # use acto::{ActoCell, ActoRef, ActoRuntime, MailboxSize};
/// async fn actor(cell: ActoCell<String, impl MailboxSize>) {
///     let child: ActoRef<u8> = cell.rt()
///         .with_mailbox_size(10)
///         .spawn_actor("child", |_: ActoCell<_, _>| async {})
///         .me;
/// }
/// ```
pub trait MailboxSize: ActoRuntime {
    type Output: ActoRuntime;

    fn with_mailbox_size(&self, mailbox_size: usize) -> Self::Output;
}
