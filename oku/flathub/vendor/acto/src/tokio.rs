use crate::{
    runtime::MailboxSize, ActoAborted, ActoHandle, ActoId, ActoRuntime, PanicInfo, PanicOrAbort,
    Receiver, Sender,
};
use parking_lot::RwLock;
use smol_str::SmolStr;
use std::{
    any::{type_name, Any},
    future::Future,
    ops::Deref,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tokio::{
    runtime::{Builder, Handle, Runtime},
    sync::mpsc::{self, error::TrySendError},
    task::JoinError,
};

/// Owned handle to an [`AcTokioRuntime`].
///
/// Dropping this handle will abort all actors spawned by this runtime.
pub struct AcTokio(AcTokioRuntime);

impl AcTokio {
    /// Create a new [`AcTokio`] runtime with the given name and number of threads.
    ///
    /// ```
    /// # use acto::{AcTokio, ActoInput, ActoRuntime, ActoHandle};
    /// let rt = AcTokio::new("test", 1).unwrap();
    ///
    /// let actor = rt.spawn_actor("test", |mut ctx| async move {
    ///     let ActoInput::<String, ()>::Message(name) = ctx.recv().await else { return };
    ///     println!("Hello, {}!", name);
    /// });
    ///
    /// // Send a message to the actor, after which it will terminate.
    /// actor.me.send("world".to_owned());
    /// // Wait for the actor to terminate.
    /// rt.with_rt(|rt| rt.block_on(actor.handle.join())).unwrap();
    /// ```
    ///
    /// In the above example the last step is crucial, without it nothing may be printed.
    /// The following demonstrates that this object owns the tokio runtime:
    ///
    /// ```
    /// # use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
    /// use acto::{AcTokio, ActoRuntime, ActoCell};
    ///
    /// let flag = Arc::new(AtomicBool::new(false));
    ///
    /// // This serves as a drop detector so we know the actor was killed.
    /// struct X(Arc<AtomicBool>);
    /// impl Drop for X {
    ///     fn drop(&mut self) {
    ///         self.0.store(true, Ordering::Relaxed);
    ///     }
    /// }
    /// let x = X(flag.clone());
    ///
    /// let tokio = AcTokio::new("test", 1).unwrap();
    /// tokio.spawn_actor("test", move |mut ctx: ActoCell<(), _>| async move {
    ///     // make sure to move the drop detector inside this scope
    ///     let _y = x;
    ///     // wait forever
    ///     loop { ctx.recv().await; }
    /// });
    ///
    /// // this will synchronously await termination of all actors and of the threads that run them
    /// drop(tokio);
    ///
    /// // verify that indeed the `_y` was dropped inside the actor
    /// assert!(flag.load(Ordering::Relaxed));
    /// ```
    pub fn new(name: impl Into<String>, num_threads: usize) -> std::io::Result<Self> {
        Ok(Self(AcTokioRuntime::new(name, num_threads)?))
    }

    /// Create a new [`AcTokio`] runtime from an existing [`tokio::runtime::Handle`].
    ///
    /// This is useful if you want to use an existing tokio runtime, for example if you want to use [`acto`] in a library.
    /// Dropping this value will not abort the actors spawned by this runtime, but it will prevent new actors from
    /// being spawned, so make sure to keep it around long enough!
    pub fn from_handle(name: impl Into<String>, handle: Handle) -> Self {
        Self(AcTokioRuntime::from_handle(name, handle))
    }
}

impl Deref for AcTokio {
    type Target = AcTokioRuntime;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Drop for AcTokio {
    fn drop(&mut self) {
        self.0.inner.rt.write().take();
    }
}

/// An [`ActoRuntime`] based on [`tokio::runtime`] and [`tokio::sync::mpsc`] queues.
///
/// In order to create it, see [`AcTokio::new()`].
#[derive(Clone)]
pub struct AcTokioRuntime {
    inner: Arc<Inner>,
    mailbox_size: usize,
}

impl AcTokioRuntime {
    fn new(name: impl Into<String>, num_threads: usize) -> std::io::Result<Self> {
        let name = name.into();
        tracing::debug!(%name, ?num_threads, "creating");
        let rt = Builder::new_multi_thread()
            .thread_name(&name)
            .worker_threads(num_threads)
            .enable_all()
            .build()?;
        Ok(Self {
            inner: Arc::new(Inner {
                name,
                rt: RwLock::new(Some(RuntimeOrHandle::Runtime(rt))),
            }),
            mailbox_size: 128,
        })
    }

    fn from_handle(name: impl Into<String>, handle: Handle) -> Self {
        let name = name.into();
        tracing::debug!(%name, "creating");
        Self {
            inner: Arc::new(Inner {
                name,
                rt: RwLock::new(Some(RuntimeOrHandle::Handle(handle))),
            }),
            mailbox_size: 128,
        }
    }

    /// Perform a task using the underlying runtime.
    ///
    /// Beware that while this function is running, dropping the [`AcTokio`] handle will block until the function is finished.
    /// Returns `None` if the runtime has been dropped.
    pub fn with_rt<U>(&self, f: impl FnOnce(&Handle) -> U) -> Option<U> {
        let _span = tracing::debug_span!("with_rt").entered();
        self.inner.rt.read().as_ref().map(|rt| f(&*rt))
    }
}

enum RuntimeOrHandle {
    Runtime(Runtime),
    Handle(Handle),
}

impl Deref for RuntimeOrHandle {
    type Target = Handle;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Runtime(rt) => rt.handle(),
            Self::Handle(h) => h,
        }
    }
}

struct Inner {
    name: String,
    rt: RwLock<Option<RuntimeOrHandle>>,
}

impl ActoRuntime for AcTokioRuntime {
    type ActoHandle<O: Send + 'static> = TokioJoinHandle<O>;
    type Sender<M: Send + 'static> = TokioSender<M>;
    type Receiver<M: Send + 'static> = TokioReceiver<M>;

    fn name(&self) -> &str {
        &self.inner.name
    }

    fn mailbox<M: Send + 'static>(&self) -> (Self::Sender<M>, Self::Receiver<M>) {
        let (tx, rx) = mpsc::channel(self.mailbox_size);
        (TokioSender(tx), TokioReceiver(rx))
    }

    fn spawn_task<T>(&self, id: ActoId, name: SmolStr, task: T) -> Self::ActoHandle<T::Output>
    where
        T: std::future::Future + Send + 'static,
        T::Output: Send + 'static,
    {
        let _span = tracing::debug_span!("spawn_task").entered();
        TokioJoinHandle(
            id,
            name,
            self.inner.rt.read().as_ref().map(|rt| rt.spawn(task)),
        )
    }
}

impl MailboxSize for AcTokioRuntime {
    type Output = Self;

    fn with_mailbox_size(&self, mailbox_size: usize) -> Self::Output {
        Self {
            inner: self.inner.clone(),
            mailbox_size,
        }
    }
}

pub struct TokioSender<M>(mpsc::Sender<M>);
impl<M: Send + 'static> Sender<M> for TokioSender<M> {
    fn send(&self, msg: M) -> bool {
        if let Err(e) = self.0.try_send(msg) {
            match e {
                TrySendError::Full(_) => {
                    tracing::debug!(
                        msg = type_name::<M>(),
                        "dropping message due to full mailbox"
                    );
                }
                TrySendError::Closed(_) => {
                    tracing::debug!(
                        msg = type_name::<M>(),
                        "dropping message due to closed mailbox"
                    );
                }
            }
            return false;
        }
        true
    }
    fn send_wait(&self, msg: M) -> Pin<Box<dyn Future<Output = bool> + Send + 'static>> {
        let tx = self.0.clone();
        Box::pin(async move {
            if let Err(_) = tx.send(msg).await {
                tracing::debug!(
                    msg = type_name::<M>(),
                    "dropping message due to closed mailbox"
                );
                return false;
            }
            true
        })
    }
}

pub struct TokioReceiver<M>(mpsc::Receiver<M>);
impl<M: Send + 'static> Receiver<M> for TokioReceiver<M> {
    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<M> {
        self.0.poll_recv(cx).map(Option::unwrap)
    }
}

pub struct TokioJoinHandle<O>(ActoId, SmolStr, Option<tokio::task::JoinHandle<O>>);

impl<O: Send + 'static> ActoHandle for TokioJoinHandle<O> {
    type Output = O;

    fn id(&self) -> ActoId {
        self.0
    }

    fn name(&self) -> &str {
        &self.1
    }

    fn abort_pinned(self: Pin<&mut Self>) {
        tracing::debug!(name = ?self.0, handle = ?self.2.is_some(), "aborting");
        let handle = self.get_mut().2.take();
        if let Some(handle) = handle {
            handle.abort();
        }
    }

    fn is_finished(&self) -> bool {
        self.2.as_ref().map(|h| h.is_finished()).unwrap_or(true)
    }

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<O, PanicOrAbort>> {
        if let Some(handle) = &mut self.as_mut().get_mut().2 {
            Pin::new(handle).poll(cx).map(|r| {
                r.map_err(|err| {
                    tracing::debug!(?err, "actor aborted");
                    PanicOrAbort::Panic(Box::new(TokioPanic::from(err)))
                })
            })
        } else {
            tracing::debug!("actor aborted");
            Poll::Ready(Err(PanicOrAbort::Abort(ActoAborted::new(
                self.as_ref().1.as_str(),
            ))))
        }
    }
}

#[derive(Debug)]
enum TokioPanic {
    Join(Box<dyn Any + Send + 'static>),
    Cancelled,
}

impl PanicInfo for TokioPanic {
    fn is_cancelled(&self) -> bool {
        matches!(self, Self::Cancelled)
    }

    fn payload(&self) -> Option<&(dyn Any + Send + 'static)> {
        match self {
            TokioPanic::Join(j) => Some(j.as_ref()),
            TokioPanic::Cancelled => None,
        }
    }

    fn cause(&self) -> String {
        match self {
            TokioPanic::Join(j) => j
                .downcast_ref::<&'static str>()
                .map(|s| *s)
                .or_else(|| j.downcast_ref::<String>().map(|s| &**s))
                .unwrap_or("opaque panic")
                .to_owned(),
            TokioPanic::Cancelled => "cancelled via Tokio".to_owned(),
        }
    }
}

impl From<JoinError> for TokioPanic {
    fn from(err: JoinError) -> Self {
        if err.is_cancelled() {
            Self::Cancelled
        } else {
            Self::Join(err.into_panic())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AcTokio;
    use crate::{ActoCell, ActoHandle, ActoId, ActoInput, ActoRuntime, SupervisionRef};
    use std::{
        collections::BTreeMap,
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        },
        time::Duration,
    };
    use tokio::sync::oneshot;

    #[test]
    fn run() {
        let sys = AcTokio::new("test", 1).unwrap();
        let flag = Arc::new(AtomicBool::new(false));
        let flag2 = flag.clone();
        let SupervisionRef { me: r, handle: j } =
            sys.spawn_actor("super", |mut ctx: ActoCell<_, _>| async move {
                flag2.store(true, Ordering::Relaxed);
                ctx.recv().await;
                flag2.store(false, Ordering::Relaxed);
                42
            });
        std::thread::sleep(Duration::from_millis(100));
        assert!(flag.load(Ordering::Relaxed));
        r.send(());
        std::thread::sleep(Duration::from_millis(100));
        assert!(!flag.load(Ordering::Relaxed));
        let ret = sys.with_rt(|rt| rt.block_on(j.join())).unwrap();
        assert_eq!(ret.unwrap(), 42);
    }

    #[test]
    fn child() {
        let sys = AcTokio::new("test", 2).unwrap();
        let SupervisionRef { me: r, handle: j } = sys.spawn_actor(
            "super",
            |mut ctx: ActoCell<_, _, Result<i32, i32>>| async move {
                let mut v: Vec<i32> = vec![];
                let mut running = BTreeMap::<ActoId, (i32, oneshot::Sender<()>)>::new();
                loop {
                    match ctx.recv().await {
                        ActoInput::NoMoreSenders => break,
                        ActoInput::Supervision { id, result, .. } => {
                            let (arg, tx) = running.remove(&id).unwrap();
                            v.push(arg);
                            v.push(result.unwrap().unwrap_or_else(|x| x));
                            tx.send(()).unwrap();
                        }
                        ActoInput::Message((x, tx)) => {
                            v.push(x);
                            let r = ctx.spawn_supervised(
                                "child",
                                |mut ctx: ActoCell<_, _>| async move {
                                    if let ActoInput::Message(x) = ctx.recv().await {
                                        Ok(2 * x)
                                    } else {
                                        Err(5)
                                    }
                                },
                            );
                            r.send(x);
                            running.insert(r.id(), (x, tx));
                        }
                    }
                }
                v
            },
        );
        let (tx, rx) = oneshot::channel();
        r.send((1, tx));
        sys.with_rt(|rt| rt.block_on(rx)).unwrap().unwrap();
        let (tx, rx) = oneshot::channel();
        r.send((2, tx));
        sys.with_rt(|rt| rt.block_on(rx)).unwrap().unwrap();
        drop(r);
        let v = sys.with_rt(|rt| rt.block_on(j.join())).unwrap().unwrap();
        assert_eq!(v, vec![1, 1, 2, 2, 2, 4]);
    }
}
