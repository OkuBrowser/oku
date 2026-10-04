//! Async rust task spawning and utilities that work natively (using tokio) and in browsers
//! (using wasm-bindgen-futures).

#[cfg(not(wasm_browser))]
pub use tokio::spawn;
#[cfg(not(wasm_browser))]
pub use tokio::task::{AbortHandle, Id, JoinError, JoinHandle, JoinSet};
#[cfg(not(wasm_browser))]
pub use tokio_util::task::AbortOnDropHandle;
#[cfg(wasm_browser)]
pub use wasm::*;

#[cfg(wasm_browser)]
mod wasm {
    use std::{
        cell::RefCell,
        fmt::{self, Debug},
        future::{Future, IntoFuture},
        pin::Pin,
        rc::Rc,
        sync::Mutex,
        task::{Context, Poll, Waker},
    };

    use futures_lite::{stream::StreamExt, FutureExt};
    use send_wrapper::SendWrapper;

    static TASK_ID_COUNTER: Mutex<u64> = Mutex::new(0);

    fn next_task_id() -> u64 {
        let mut counter = TASK_ID_COUNTER.lock().unwrap();
        *counter += 1;
        *counter
    }

    /// An opaque ID that uniquely identifies a task relative to all other currently running tasks.
    #[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, derive_more::Display)]
    pub struct Id(u64);

    /// Wasm shim for tokio's `JoinSet`.
    ///
    /// Uses a [`futures_buffered::FuturesUnordered`] queue of
    /// [`JoinHandle`]s inside.
    pub struct JoinSet<T> {
        handles: futures_buffered::FuturesUnordered<JoinHandleWithId<T>>,
        // We need to keep a second list of JoinHandles so we can access them for cancellation
        to_cancel: Vec<JoinHandle<T>>,
    }

    impl<T> Debug for JoinSet<T> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("JoinSet").field("len", &self.len()).finish()
        }
    }

    impl<T> Default for JoinSet<T> {
        fn default() -> Self {
            Self::new()
        }
    }

    impl<T> JoinSet<T> {
        /// Creates a new, empty `JoinSet`
        pub fn new() -> Self {
            Self {
                handles: futures_buffered::FuturesUnordered::new(),
                to_cancel: Vec::new(),
            }
        }

        /// Spawns a task into this `JoinSet`.
        pub fn spawn(&mut self, fut: impl IntoFuture<Output = T> + 'static) -> AbortHandle
        where
            T: 'static,
        {
            let handle = JoinHandle::new();
            let state = handle.task.state.clone();
            let handle_for_spawn = JoinHandle {
                task: handle.task.clone(),
            };
            let handle_for_cancel = JoinHandle {
                task: handle.task.clone(),
            };

            wasm_bindgen_futures::spawn_local(SpawnFuture {
                handle: handle_for_spawn,
                fut: fut.into_future(),
            });

            self.handles.push(JoinHandleWithId(handle));
            self.to_cancel.push(handle_for_cancel);
            AbortHandle { state }
        }

        /// Alias to [`Self::spawn`].
        ///
        /// Mirrors [`tokio::JoinSet::spawn_local](https://docs.rs/tokio/latest/tokio/task/struct.JoinSet.html#method.spawn_local).
        /// Because all tasks in WebAssembly are local, this is a simple alias to [`Self::spawn`].
        pub fn spawn_local(&mut self, fut: impl IntoFuture<Output = T> + 'static) -> AbortHandle
        where
            T: 'static,
        {
            self.spawn(fut)
        }

        /// Aborts all tasks inside this `JoinSet`
        pub fn abort_all(&self) {
            self.to_cancel.iter().for_each(JoinHandle::abort);
        }

        /// Awaits the next `JoinSet`'s completion.
        ///
        /// If you `.spawn` a new task onto this `JoinSet` while the future
        /// returned from this is currently pending, then this future will
        /// continue to be pending, even if the newly spawned future is already
        /// finished.
        ///
        /// TODO(matheus23): Fix this limitation.
        ///
        /// Current work around is to recreate the `join_next` future when
        /// you newly spawned a task onto it. This seems to be the usual way
        /// the `JoinSet` is used *most of the time* in the iroh codebase anyways.
        pub async fn join_next(&mut self) -> Option<Result<T, JoinError>> {
            self.join_next_with_id()
                .await
                .map(|ret| ret.map(|(_id, out)| out))
        }

        /// Waits until one of the tasks in the set completes and returns its
        /// output, along with the [task ID] of the completed task.
        ///
        /// Returns `None` if the set is empty.
        ///
        /// When this method returns an error, then the id of the task that failed can be accessed
        /// using the [`JoinError::id`] method.
        ///
        /// [task ID]: crate::task::Id
        /// [`JoinError::id`]: fn@crate::task::JoinError::id
        pub async fn join_next_with_id(&mut self) -> Option<Result<(Id, T), JoinError>> {
            futures_lite::future::poll_fn(|cx| self.poll_join_next_with_id(cx)).await
        }

        /// Polls for one of the tasks in the set to complete.
        ///
        /// If this returns `Poll::Ready(Some(_))`, then the task that completed is removed from the set.
        ///
        /// When the method returns `Poll::Pending`, the `Waker` in the provided `Context` is scheduled
        /// to receive a wakeup when a task in the `JoinSet` completes. Note that on multiple calls to
        /// `poll_join_next`, only the `Waker` from the `Context` passed to the most recent call is
        /// scheduled to receive a wakeup.
        ///
        /// # Returns
        ///
        /// This function returns:
        ///
        ///  * `Poll::Pending` if the `JoinSet` is not empty but there is no task whose output is
        ///    available right now.
        ///  * `Poll::Ready(Some(Ok(value)))` if one of the tasks in this `JoinSet` has completed.
        ///    The `value` is the return value of one of the tasks that completed.
        ///  * `Poll::Ready(Some(Err(err)))` if one of the tasks in this `JoinSet` has panicked or been
        ///    aborted. The `err` is the `JoinError` from the panicked/aborted task.
        ///  * `Poll::Ready(None)` if the `JoinSet` is empty.
        pub fn poll_join_next(
            &mut self,
            cx: &mut Context<'_>,
        ) -> Poll<Option<Result<T, JoinError>>> {
            match self.poll_join_next_with_id(cx) {
                Poll::Ready(Some(Ok((_, ret)))) => Poll::Ready(Some(Ok(ret))),
                Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e))),
                Poll::Ready(None) => Poll::Ready(None),
                Poll::Pending => Poll::Pending,
            }
        }

        /// Polls for one of the tasks in the set to complete.
        ///
        /// If this returns `Poll::Ready(Some(_))`, then the task that completed is removed from the set.
        ///
        /// When the method returns `Poll::Pending`, the `Waker` in the provided `Context` is scheduled
        /// to receive a wakeup when a task in the `JoinSet` completes. Note that on multiple calls to
        /// `poll_join_next`, only the `Waker` from the `Context` passed to the most recent call is
        /// scheduled to receive a wakeup.
        ///
        /// # Returns
        ///
        /// This function returns:
        ///
        ///  * `Poll::Pending` if the `JoinSet` is not empty but there is no task whose output is
        ///    available right now.
        ///  * `Poll::Ready(Some(Ok((id, value))))` if one of the tasks in this `JoinSet` has completed.
        ///    The `value` is the return value of one of the tasks that completed, and
        ///    `id` is the [task ID] of that task.
        ///  * `Poll::Ready(Some(Err(err)))` if one of the tasks in this `JoinSet` has panicked or been
        ///    aborted. The `err` is the `JoinError` from the panicked/aborted task.
        ///  * `Poll::Ready(None)` if the `JoinSet` is empty.
        ///
        /// [task ID]: crate::task::Id
        pub fn poll_join_next_with_id(
            &mut self,
            cx: &mut Context<'_>,
        ) -> Poll<Option<Result<(Id, T), JoinError>>> {
            let ret = self.handles.poll_next(cx);
            // clean up handles that are either cancelled or have finished
            self.to_cancel.retain(JoinHandle::is_running);
            ret
        }

        /// Returns whether there's any tasks that are either still running or
        /// have pending results in this `JoinSet`.
        pub fn is_empty(&self) -> bool {
            self.handles.is_empty()
        }

        /// Returns the amount of tasks that are either still running or have
        /// pending results in this `JoinSet`.
        pub fn len(&self) -> usize {
            self.handles.len()
        }

        /// Waits for all tasks to finish. If any of them returns a JoinError,
        /// this will panic.
        pub async fn join_all(mut self) -> Vec<T> {
            let mut output = Vec::new();
            while let Some(res) = self.join_next().await {
                match res {
                    Ok(t) => output.push(t),
                    Err(err) => panic!("{err}"),
                }
            }
            output
        }

        /// Aborts all tasks and then waits for them to finish, ignoring panics.
        pub async fn shutdown(&mut self) {
            self.abort_all();
            while let Some(_res) = self.join_next().await {}
        }
    }

    impl<T> Drop for JoinSet<T> {
        fn drop(&mut self) {
            self.abort_all()
        }
    }

    /// A handle to a spawned task.
    pub struct JoinHandle<T> {
        task: Task<T>,
    }

    struct Task<T> {
        // Using SendWrapper here is safe as long as you keep all of your
        // work on the main UI worker in the browser.
        // The only exception to that being the case would be if our user
        // would use multiple Wasm instances with a single SharedArrayBuffer,
        // put the instances on different Web Workers and finally shared
        // the JoinHandle across the Web Worker boundary.
        // In that case, using the JoinHandle would panic.
        state: SendWrapper<Rc<RefCell<State>>>,
        result: SendWrapper<Rc<RefCell<Option<T>>>>,
    }

    impl<T> Clone for Task<T> {
        fn clone(&self) -> Self {
            Self {
                state: self.state.clone(),
                result: self.result.clone(),
            }
        }
    }

    #[derive(Debug)]
    struct State {
        id: Id,
        cancelled: bool,
        completed: bool,
        waker_handler: Option<Waker>,
        waker_spawn_fn: Option<Waker>,
    }

    impl State {
        fn cancel(&mut self) {
            if !self.cancelled {
                self.cancelled = true;
                self.wake();
            }
        }

        fn complete(&mut self) {
            self.completed = true;
            self.wake();
        }

        fn is_complete(&self) -> bool {
            self.completed || self.cancelled
        }

        fn wake(&mut self) {
            if let Some(waker) = self.waker_handler.take() {
                waker.wake();
            }
            if let Some(waker) = self.waker_spawn_fn.take() {
                waker.wake();
            }
        }

        fn register_handler(&mut self, cx: &mut Context<'_>) {
            match self.waker_handler {
                // clone_from can be marginally faster in some cases
                Some(ref mut waker) => waker.clone_from(cx.waker()),
                None => self.waker_handler = Some(cx.waker().clone()),
            }
        }

        fn register_spawn_fn(&mut self, cx: &mut Context<'_>) {
            match self.waker_spawn_fn {
                // clone_from can be marginally faster in some cases
                Some(ref mut waker) => waker.clone_from(cx.waker()),
                None => self.waker_spawn_fn = Some(cx.waker().clone()),
            }
        }
    }

    impl<T> Debug for JoinHandle<T> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            if self.task.state.valid() {
                let state = self.task.state.borrow();
                f.debug_struct("JoinHandle")
                    .field("id", &state.id)
                    .field("cancelled", &state.cancelled)
                    .field("completed", &state.completed)
                    .finish()
            } else {
                f.debug_tuple("JoinHandle")
                    .field(&format_args!("<other thread>"))
                    .finish()
            }
        }
    }

    impl<T> JoinHandle<T> {
        fn new() -> Self {
            Self {
                task: Task {
                    state: SendWrapper::new(Rc::new(RefCell::new(State {
                        cancelled: false,
                        completed: false,
                        waker_handler: None,
                        waker_spawn_fn: None,
                        id: Id(next_task_id()),
                    }))),
                    result: SendWrapper::new(Rc::new(RefCell::new(None))),
                },
            }
        }

        /// Aborts this task.
        pub fn abort(&self) {
            self.task.state.borrow_mut().cancel();
        }

        /// Returns a new [`AbortHandle`] that can be used to remotely abort this task.
        ///
        /// Awaiting a task cancelled by the [`AbortHandle`] might complete as usual if the task was
        /// already completed at the time it was cancelled, but most likely it
        /// will fail with a [cancelled] `JoinError`.
        ///
        /// [cancelled]: JoinError::is_cancelled
        pub fn abort_handle(&self) -> AbortHandle {
            AbortHandle {
                state: self.task.state.clone(),
            }
        }

        /// Returns a [task ID] that uniquely identifies this task relative to other
        /// currently spawned tasks.
        ///
        /// [task ID]: crate::task::Id
        pub fn id(&self) -> Id {
            let state = self.task.state.borrow();
            state.id
        }

        /// Checks if the task associated with this `JoinHandle` has finished.
        pub fn is_finished(&self) -> bool {
            let state = self.task.state.borrow();
            state.is_complete()
        }

        fn is_running(&self) -> bool {
            !self.is_finished()
        }
    }

    /// An error that can occur when waiting for the completion of a task.
    #[derive(derive_more::Display, Debug, Clone, Copy)]
    #[display("{cause}")]
    pub struct JoinError {
        cause: JoinErrorCause,
        id: Id,
    }

    #[derive(derive_more::Display, Debug, Clone, Copy)]
    enum JoinErrorCause {
        /// The error that's returned when the task that's being waited on
        /// has been cancelled.
        #[display("task was cancelled")]
        Cancelled,
    }

    impl std::error::Error for JoinError {}

    impl JoinError {
        /// Returns whether this join error is due to cancellation.
        ///
        /// Always true in this Wasm implementation, because we don't
        /// unwind panics in tasks.
        /// All panics just happen on the main thread anyways.
        pub fn is_cancelled(&self) -> bool {
            matches!(self.cause, JoinErrorCause::Cancelled)
        }

        /// Returns whether this is a panic. Always `false` in Wasm,
        /// because when a task panics, it's not unwound, instead it
        /// panics directly to the main thread.
        pub fn is_panic(&self) -> bool {
            false
        }

        /// Returns the panic, if the task has panicked.
        ///
        /// Always returns `Err(self)`, in Wasm, because when a task panics, it's not unwound,
        /// instead it panics directly to the main thread.
        pub fn try_into_panic(self) -> Result<Box<dyn std::any::Any + Send + 'static>, JoinError> {
            Err(self)
        }

        /// Returns a task ID that identifies the task which errored relative to other currently spawned tasks.
        pub fn id(&self) -> Id {
            self.id
        }
    }

    impl<T> Future for JoinHandle<T> {
        type Output = Result<T, JoinError>;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let mut state = self.task.state.borrow_mut();
            if state.cancelled {
                return Poll::Ready(Err(JoinError {
                    cause: JoinErrorCause::Cancelled,
                    id: state.id,
                }));
            }

            let mut result = self.task.result.borrow_mut();
            if let Some(result) = result.take() {
                return Poll::Ready(Ok(result));
            }

            state.register_handler(cx);
            Poll::Pending
        }
    }

    struct JoinHandleWithId<T>(JoinHandle<T>);

    impl<T> Future for JoinHandleWithId<T> {
        type Output = Result<(Id, T), JoinError>;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            match self.0.poll(cx) {
                Poll::Ready(out) => Poll::Ready(out.map(|out| (self.0.id(), out))),
                Poll::Pending => Poll::Pending,
            }
        }
    }

    #[pin_project::pin_project]
    struct SpawnFuture<Fut: Future<Output = T>, T> {
        handle: JoinHandle<T>,
        #[pin]
        fut: Fut,
    }

    impl<Fut: Future<Output = T>, T> Future for SpawnFuture<Fut, T> {
        type Output = ();

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let this = self.project();
            let mut state = this.handle.task.state.borrow_mut();

            if state.cancelled {
                return Poll::Ready(());
            }

            match this.fut.poll(cx) {
                Poll::Ready(value) => {
                    let _ = this.handle.task.result.borrow_mut().insert(value);
                    state.complete();
                    Poll::Ready(())
                }
                Poll::Pending => {
                    state.register_spawn_fn(cx);
                    Poll::Pending
                }
            }
        }
    }

    /// An owned permission to abort a spawned task, without awaiting its completion.
    #[derive(Clone)]
    pub struct AbortHandle {
        state: SendWrapper<Rc<RefCell<State>>>,
    }

    impl Debug for AbortHandle {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            if self.state.valid() {
                let state = self.state.borrow();
                f.debug_struct("AbortHandle")
                    .field("id", &state.id)
                    .field("cancelled", &state.cancelled)
                    .field("completed", &state.completed)
                    .finish()
            } else {
                f.debug_tuple("AbortHandle")
                    .field(&format_args!("<other thread>"))
                    .finish()
            }
        }
    }

    impl AbortHandle {
        /// Abort the task associated with the handle.
        pub fn abort(&self) {
            self.state.borrow_mut().cancel();
        }

        /// Returns a [task ID] that uniquely identifies this task relative to other
        /// currently spawned tasks.
        ///
        /// [task ID]: crate::task::Id
        pub fn id(&self) -> Id {
            self.state.borrow().id
        }

        /// Checks if the task associated with this `AbortHandle` has finished.
        pub fn is_finished(&self) -> bool {
            let state = self.state.borrow();
            state.cancelled && state.completed
        }
    }

    /// Similar to a `JoinHandle`, except it automatically aborts
    /// the task when it's dropped.
    #[pin_project::pin_project(PinnedDrop)]
    #[derive(derive_more::Debug, derive_more::Deref)]
    #[debug("AbortOnDropHandle")]
    #[must_use = "Dropping the handle aborts the task immediately"]
    pub struct AbortOnDropHandle<T>(#[pin] JoinHandle<T>);

    #[pin_project::pinned_drop]
    impl<T> PinnedDrop for AbortOnDropHandle<T> {
        fn drop(self: Pin<&mut Self>) {
            self.0.abort();
        }
    }

    impl<T> Future for AbortOnDropHandle<T> {
        type Output = <JoinHandle<T> as Future>::Output;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            self.project().0.poll(cx)
        }
    }

    impl<T> AbortOnDropHandle<T> {
        /// Converts a `JoinHandle` into one that aborts on drop.
        pub fn new(task: JoinHandle<T>) -> Self {
            Self(task)
        }

        /// Returns a new [`AbortHandle`] that can be used to remotely abort this task,
        /// equivalent to [`JoinHandle::abort_handle`].
        pub fn abort_handle(&self) -> AbortHandle {
            self.0.abort_handle()
        }

        /// Abort the task associated with this handle,
        /// equivalent to [`JoinHandle::abort`].
        pub fn abort(&self) {
            self.0.abort()
        }

        /// Checks if the task associated with this handle is finished,
        /// equivalent to [`JoinHandle::is_finished`].
        pub fn is_finished(&self) -> bool {
            self.0.is_finished()
        }
    }

    /// Spawns a future as a task in the browser runtime.
    ///
    /// This is powered by `wasm_bidngen_futures`.
    pub fn spawn<T: 'static>(fut: impl IntoFuture<Output = T> + 'static) -> JoinHandle<T> {
        let handle = JoinHandle::new();

        wasm_bindgen_futures::spawn_local(SpawnFuture {
            handle: JoinHandle {
                task: handle.task.clone(),
            },
            fut: fut.into_future(),
        });

        handle
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    #[cfg(not(wasm_browser))]
    use tokio::test;
    #[cfg(wasm_browser)]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    use crate::task;

    #[test]
    async fn task_abort() {
        let h1 = task::spawn(async {
            crate::time::sleep(Duration::from_millis(10)).await;
        });
        let h2 = task::spawn(async {
            crate::time::sleep(Duration::from_millis(10)).await;
        });
        assert!(h1.id() != h2.id());

        h1.abort();
        assert!(h1.await.err().unwrap().is_cancelled());
        assert!(h2.await.is_ok());
    }

    #[test]
    async fn join_set_abort() {
        let fut = || async { 22 };
        let mut set = task::JoinSet::new();
        let h1 = set.spawn(fut());
        let h2 = set.spawn(fut());
        assert!(h1.id() != h2.id());
        h2.abort();

        let mut has_err = false;
        let mut has_ok = false;
        while let Some(ret) = set.join_next_with_id().await {
            match ret {
                Err(err) => {
                    if !has_err {
                        assert!(err.is_cancelled());
                        has_err = true;
                    } else {
                        panic!()
                    }
                }
                Ok((id, out)) => {
                    if !has_ok {
                        assert_eq!(id, h1.id());
                        assert_eq!(out, 22);
                        has_ok = true;
                    } else {
                        panic!()
                    }
                }
            }
        }
        assert!(has_err);
        assert!(has_ok);
    }
}
