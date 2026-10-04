//! Watchable values.
//!
//! A [`Watchable`] exists to keep track of a value which may change over time.  It allows
//! observers to be notified of changes to the value.  The aim is to always be aware of the
//! **last** value, not to observe *every* value change.
//!
//! The reason for this is ergonomics and predictable resource usage: Requiring every
//! intermediate value to be observable would mean that either the side that sets new values
//! using [`Watchable::set`] would need to wait for all "receivers" of these intermediate
//! values to catch up and thus be an async operation, or it would require the receivers
//! to buffer intermediate values until they've been "received" on the [`Watcher`]s with
//! an unlimited buffer size and thus potentially unlimited memory growth.
//!
//! # Example
//!
//! ```
//! use n0_future::StreamExt;
//! use n0_watcher::{Watchable, Watcher as _};
//!
//! #[tokio::main(flavor = "current_thread", start_paused = true)]
//! async fn main() {
//!     let watchable = Watchable::new(None);
//!
//!     // A task that waits for the watcher to be initialized to Some(value) before printing it
//!     let mut watcher = watchable.watch();
//!     tokio::spawn(async move {
//!         let initialized_value = watcher.initialized().await;
//!         println!("initialized: {initialized_value}");
//!     });
//!
//!     // A task that prints every update to the watcher since the initial one:
//!     let mut updates = watchable.watch().stream_updates_only();
//!     tokio::spawn(async move {
//!         while let Some(update) = updates.next().await {
//!             println!("update: {update:?}");
//!         }
//!     });
//!
//!     // A task that prints the current value and then every update it can catch,
//!     // but it also does something else which makes it very slow to pick up new
//!     // values, so it'll skip some:
//!     let mut current_and_updates = watchable.watch().stream();
//!     tokio::spawn(async move {
//!         while let Some(update) = current_and_updates.next().await {
//!             println!("update2: {update:?}");
//!             tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
//!         }
//!     });
//!
//!     for i in 0..20 {
//!         println!("Setting watchable to {i}");
//!         watchable.set(Some(i)).ok();
//!         tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;
//!     }
//! }
//! ```
//!
//! # Similar but different
//!
//! - `async_channel`: This is a multi-producer, multi-consumer channel implementation.
//!   Only at most one consumer will receive each "produced" value.
//!   What we want is to have every "produced" value to be "broadcast" to every receiver.
//! - `tokio::broadcast`: Also a multi-producer, multi-consumer channel implementation.
//!   This is very similar to this crate (`tokio::broadcast::Sender` is like [`Watchable`]
//!   and `tokio::broadcast::Receiver` is like [`Watcher`]), but you can't get the latest
//!   value without `.await`ing on the receiver, and it'll internally store a queue of
//!   intermediate values.
//! - `tokio::watch`: Also a MPSC channel, and unlike `tokio::broadcast` only retains the
//!   latest value. That module has pretty much the same purpose as this crate, but doesn't
//!   implement a poll-based method of getting updates and doesn't implement combinators.
//! - [`std::sync::RwLock`]: (wrapped in an [`std::sync::Arc`]) This allows you access
//!   to the latest values, but might block while it's being set (but that could be short
//!   enough not to matter for async rust purposes).
//!   This doesn't allow you to be notified whenever a new value is written.
//! - The `watchable` crate: We used to use this crate at n0, but we wanted to experiment
//!   with different APIs and needed Wasm support.
#[cfg(not(watcher_loom))]
use std::sync;
use std::{
    collections::VecDeque,
    future::Future,
    pin::Pin,
    sync::{Arc, RwLockReadGuard, Weak},
    task::{self, ready, Poll, Waker},
};

#[cfg(watcher_loom)]
use loom::sync;
use n0_error::StackError;
use sync::{Mutex, RwLock};

/// A wrapper around a value that notifies [`Watcher`]s when the value is modified.
///
/// Only the most recent value is available to any observer, but the observer is guaranteed
/// to be notified of the most recent value.
#[derive(Debug, Default)]
pub struct Watchable<T> {
    shared: Arc<Shared<T>>,
}

impl<T> Clone for Watchable<T> {
    fn clone(&self) -> Self {
        Self {
            shared: self.shared.clone(),
        }
    }
}

/// Abstracts over `Option<T>` and `Vec<T>`
pub trait Nullable<T> {
    /// Converts this value into an `Option`.
    fn into_option(self) -> Option<T>;
}

impl<T> Nullable<T> for Option<T> {
    fn into_option(self) -> Option<T> {
        self
    }
}

impl<T> Nullable<T> for Vec<T> {
    fn into_option(mut self) -> Option<T> {
        self.pop()
    }
}

impl<T: Clone + Eq> Watchable<T> {
    /// Creates a [`Watchable`] initialized to given value.
    pub fn new(value: T) -> Self {
        Self {
            shared: Arc::new(Shared {
                state: RwLock::new(State {
                    value,
                    epoch: INITIAL_EPOCH,
                }),
                wakers: Default::default(),
            }),
        }
    }

    /// Sets a new value.
    ///
    /// Returns `Ok(previous_value)` if the value was different from the one set, or
    /// returns the provided value back as `Err(value)` if the value didn't change.
    ///
    /// Watchers are only notified if the value changed.
    pub fn set(&self, value: T) -> Result<T, T> {
        // We don't actually write when the value didn't change, but there's unfortunately
        // no way to upgrade a read guard to a write guard, and locking as read first, then
        // dropping and locking as write introduces a possible race condition.
        let mut state = self.shared.state.write().expect("poisoned");

        // Find out if the value changed
        let changed = state.value != value;

        let ret = if changed {
            let old = std::mem::replace(&mut state.value, value);
            state.epoch += 1;
            Ok(old)
        } else {
            Err(value)
        };
        drop(state); // No need to write anymore

        // Notify watchers
        if changed {
            for watcher in self.shared.wakers.lock().expect("poisoned").drain(..) {
                watcher.wake();
            }
        }
        ret
    }

    /// Creates a [`Direct`] [`Watcher`], allowing the value to be observed, but not modified.
    pub fn watch(&self) -> Direct<T> {
        Direct {
            state: self.shared.state().clone(),
            shared: Some(Arc::downgrade(&self.shared)),
        }
    }

    /// Returns the currently stored value.
    pub fn get(&self) -> T {
        self.shared.get()
    }

    /// Returns true when there are any watchers actively listening on changes,
    /// or false when all watchers have been dropped or none have been created yet.
    pub fn has_watchers(&self) -> bool {
        // `Watchable`s will increase the strong count
        // `Direct`s watchers (which all watchers descend from) will increase the weak count
        Arc::weak_count(&self.shared) != 0
    }
}

impl<T> Drop for Shared<T> {
    fn drop(&mut self) {
        let Ok(mut watchers) = self.wakers.lock() else {
            return; // Poisoned waking?
        };
        // Wake all watchers once we drop Shared (this happens when
        // the last `Watchable` is dropped).
        // This allows us to notify `NextFut::poll`s and have that
        // return `Disconnected`.
        for watcher in watchers.drain(..) {
            watcher.wake();
        }
    }
}

/// A handle to a value that's represented by one or more underlying [`Watchable`]s.
///
/// A [`Watcher`] can get the current value, and will be notified when the value changes.
/// Only the most recent value is accessible, and if the threads with the underlying [`Watchable`]s
/// change the value faster than the threads with the [`Watcher`] can keep up with, then
/// it'll miss in-between values.
/// When the thread changing the [`Watchable`] pauses updating, the [`Watcher`] will always
/// end up reporting the most recent state eventually.
///
/// Watchers can be modified via [`Watcher::map`] to observe a value derived from the original
/// value via a function.
///
/// Watchers can be combined via [`Watcher::or`] to allow observing multiple values at once and
/// getting an update in case any of the values updates.
///
/// One of the underlying [`Watchable`]s might already be dropped. In that case,
/// the watcher will be "disconnected" and return [`Err(Disconnected)`](Disconnected)
/// on some function calls or, when turned into a stream, that stream will end.
/// This property can also be checked with [`Watcher::is_connected`].
pub trait Watcher: Clone {
    /// The type of value that can change.
    ///
    /// We require `Clone`, because we need to be able to make
    /// the values have a lifetime that's detached from the original [`Watchable`]'s
    /// lifetime.
    ///
    /// We require `Eq`, to be able to check whether the value actually changed or
    /// not, so we can notify or not notify accordingly.
    type Value: Clone + Eq;

    /// Updates the watcher to the latest value and returns that value.
    ///
    /// If any of the underlying [`Watchable`] values have been dropped, then this
    /// might return an outdated value for that watchable, specifically, the latest
    /// value that was fetched for that watchable, as opposed to the latest value
    /// that was set on the watchable before it was dropped.
    ///
    /// The default implementation for this is simply
    /// ```ignore
    /// fn get(&mut self) -> Self::Value {
    ///     self.update();
    ///     self.peek().clone()
    /// }
    /// ```
    fn get(&mut self) -> Self::Value {
        self.update();
        self.peek().clone()
    }

    /// Updates the watcher to the latest value and returns whether it changed.
    ///
    /// Watchers keep track of the "latest known" value they fetched.
    /// This function updates that internal value by looking up the latest value
    /// at the [`Watchable`]\(s\) that this watcher is linked to.
    fn update(&mut self) -> bool;

    /// Returns a reference to the value currently stored in the watcher.
    ///
    /// Watchers keep track of the "latest known" value they fetched.
    /// Calling this won't update the latest value, unlike [`Watcher::get`] or
    /// [`Watcher::update`].
    ///
    /// This can be useful if you want to avoid copying out the internal value
    /// frequently like what [`Watcher::get`] will end up doing.
    fn peek(&self) -> &Self::Value;

    /// Whether this watcher is still connected to all of its underlying [`Watchable`]s.
    ///
    /// Returns false when any of the underlying watchables has been dropped.
    fn is_connected(&self) -> bool;

    /// Polls for the next value, or returns [`Disconnected`] if one of the underlying
    /// [`Watchable`]s has been dropped.
    fn poll_updated(&mut self, cx: &mut task::Context<'_>) -> Poll<Result<(), Disconnected>>;

    /// Returns a future completing with `Ok(value)` once a new value is set, or with
    /// [`Err(Disconnected)`](Disconnected) if the connected [`Watchable`] was dropped.
    ///
    /// # Cancel Safety
    ///
    /// The returned future is cancel-safe.
    fn updated(&mut self) -> NextFut<'_, Self> {
        NextFut { watcher: self }
    }

    /// Returns a future completing once the value is set to [`Some`] value.
    ///
    /// If the current value is [`Some`] value, this future will resolve immediately.
    ///
    /// This is a utility for the common case of storing an [`Option`] inside a
    /// [`Watchable`].
    ///
    /// # Cancel Safety
    ///
    /// The returned future is cancel-safe.
    fn initialized<T, W>(&mut self) -> InitializedFut<'_, T, W, Self>
    where
        W: Nullable<T> + Clone,
        Self: Watcher<Value = W>,
    {
        InitializedFut {
            initial: self.get().into_option(),
            watcher: self,
        }
    }

    /// Returns a stream which will yield the most recent values as items.
    ///
    /// The first item of the stream is the current value, so that this stream can be easily
    /// used to operate on the most recent value.
    ///
    /// Note however, that only the last item is stored.  If the stream is not polled when an
    /// item is available it can be replaced with another item by the time it is polled.
    ///
    /// This stream ends once the original [`Watchable`] has been dropped.
    ///
    /// # Cancel Safety
    ///
    /// The returned stream is cancel-safe.
    fn stream(mut self) -> Stream<Self>
    where
        Self: Unpin,
    {
        Stream {
            initial: Some(self.get()),
            watcher: self,
        }
    }

    /// Returns a stream which will yield the most recent values as items, starting from
    /// the next unobserved future value.
    ///
    /// This means this stream will only yield values when the watched value changes,
    /// the value stored at the time the stream is created is not yielded.
    ///
    /// Note however, that only the last item is stored.  If the stream is not polled when an
    /// item is available it can be replaced with another item by the time it is polled.
    ///
    /// This stream ends once the original [`Watchable`] has been dropped.
    ///
    /// # Cancel Safety
    ///
    /// The returned stream is cancel-safe.
    fn stream_updates_only(self) -> Stream<Self>
    where
        Self: Unpin,
    {
        Stream {
            initial: None,
            watcher: self,
        }
    }

    /// Maps this watcher with a function that transforms the observed values.
    ///
    /// The returned watcher will only register updates, when the *mapped* value
    /// observably changes.
    fn map<T: Clone + Eq>(
        mut self,
        map: impl Fn(Self::Value) -> T + Send + Sync + 'static,
    ) -> Map<Self, T> {
        Map {
            current: (map)(self.get()),
            map: Arc::new(map),
            watcher: self,
        }
    }

    /// Returns a watcher that updates every time this or the other watcher
    /// updates, and yields both watcher's items together when that happens.
    fn or<W: Watcher>(self, other: W) -> Tuple<Self, W> {
        Tuple::new(self, other)
    }
}

/// The immediate, direct observer of a [`Watchable`] value.
///
/// This type is mainly used via the [`Watcher`] interface.
#[derive(Debug, Clone)]
pub struct Direct<T> {
    state: State<T>,
    // We wrap the Weak with an Option, so that we can set it to `None` once we
    // notice that Weak is not upgradable anymore for the first time.
    // This allows the weak pointer's allocation to be freed in case this makes
    // the weak count go to zero (even if Direct is still kept around).
    shared: Option<Weak<Shared<T>>>,
}

impl<T: Clone + Eq> Watcher for Direct<T> {
    type Value = T;

    fn update(&mut self) -> bool {
        let Some(shared) = self.shared.as_ref().and_then(|weak| weak.upgrade()) else {
            self.shared = None; // Weak won't be upgradable in the future, this way we can allow the allocation to be freed
            return false;
        };
        let state = shared.state();
        if state.epoch > self.state.epoch {
            self.state = state.clone();
            true
        } else {
            false
        }
    }

    fn peek(&self) -> &Self::Value {
        &self.state.value
    }

    fn is_connected(&self) -> bool {
        self.shared
            .as_ref()
            .and_then(|weak| weak.upgrade())
            .is_some()
    }

    fn poll_updated(&mut self, cx: &mut task::Context<'_>) -> Poll<Result<(), Disconnected>> {
        let Some(shared) = self.shared.as_ref().and_then(|weak| weak.upgrade()) else {
            self.shared = None; // Weak won't be upgradable in the future, this way we can allow the allocation to be freed
            return Poll::Ready(Err(Disconnected));
        };
        self.state = ready!(shared.poll_updated(cx, self.state.epoch));
        Poll::Ready(Ok(()))
    }
}

#[derive(Debug, Clone)]
pub struct Tuple<S: Watcher, T: Watcher> {
    inner: (S, T),
    current: (S::Value, T::Value),
}

impl<S: Watcher, T: Watcher> Tuple<S, T> {
    pub fn new(mut s: S, mut t: T) -> Self {
        let current = (s.get(), t.get());
        Self {
            inner: (s, t),
            current,
        }
    }
}

impl<S: Watcher, T: Watcher> Watcher for Tuple<S, T> {
    type Value = (S::Value, T::Value);

    fn update(&mut self) -> bool {
        // We need to update all watchers! So don't early-return
        let s_updated = self.inner.0.update();
        let t_updated = self.inner.1.update();
        let updated = s_updated || t_updated;
        if updated {
            self.current = (self.inner.0.peek().clone(), self.inner.1.peek().clone());
        }
        updated
    }

    fn peek(&self) -> &Self::Value {
        &self.current
    }

    fn is_connected(&self) -> bool {
        self.inner.0.is_connected() && self.inner.1.is_connected()
    }

    fn poll_updated(&mut self, cx: &mut task::Context<'_>) -> Poll<Result<(), Disconnected>> {
        let poll_0 = self.inner.0.poll_updated(cx)?;
        let poll_1 = self.inner.1.poll_updated(cx)?;
        if poll_0.is_pending() && poll_1.is_pending() {
            return Poll::Pending;
        }
        if poll_0.is_ready() {
            self.current.0 = self.inner.0.peek().clone();
        }
        if poll_1.is_ready() {
            self.current.1 = self.inner.1.peek().clone();
        }
        Poll::Ready(Ok(()))
    }
}

#[derive(Debug, Clone)]
pub struct Triple<S: Watcher, T: Watcher, U: Watcher> {
    inner: (S, T, U),
    current: (S::Value, T::Value, U::Value),
}

impl<S: Watcher, T: Watcher, U: Watcher> Triple<S, T, U> {
    pub fn new(mut s: S, mut t: T, mut u: U) -> Self {
        let current = (s.get(), t.get(), u.get());
        Self {
            inner: (s, t, u),
            current,
        }
    }
}

impl<S: Watcher, T: Watcher, U: Watcher> Watcher for Triple<S, T, U> {
    type Value = (S::Value, T::Value, U::Value);

    fn update(&mut self) -> bool {
        // We need to update all watchers! So don't early-return
        let s_updated = self.inner.0.update();
        let t_updated = self.inner.1.update();
        let u_updated = self.inner.2.update();
        let updated = s_updated || t_updated || u_updated;
        if updated {
            self.current = (
                self.inner.0.peek().clone(),
                self.inner.1.peek().clone(),
                self.inner.2.peek().clone(),
            );
        }
        updated
    }

    fn peek(&self) -> &Self::Value {
        &self.current
    }

    fn is_connected(&self) -> bool {
        self.inner.0.is_connected() && self.inner.1.is_connected() && self.inner.2.is_connected()
    }

    fn poll_updated(&mut self, cx: &mut task::Context<'_>) -> Poll<Result<(), Disconnected>> {
        let poll_0 = self.inner.0.poll_updated(cx)?;
        let poll_1 = self.inner.1.poll_updated(cx)?;
        let poll_2 = self.inner.2.poll_updated(cx)?;

        if poll_0.is_pending() && poll_1.is_pending() && poll_2.is_pending() {
            return Poll::Pending;
        }
        if poll_0.is_ready() {
            self.current.0 = self.inner.0.peek().clone();
        }
        if poll_1.is_ready() {
            self.current.1 = self.inner.1.peek().clone();
        }
        if poll_2.is_ready() {
            self.current.2 = self.inner.2.peek().clone();
        }
        Poll::Ready(Ok(()))
    }
}

/// Combinator to join two watchers
#[derive(Debug, Clone)]
pub struct Join<T: Clone + Eq, W: Watcher<Value = T>> {
    // invariant: watchers.len() == current.len()
    watchers: Vec<W>,
    current: Vec<T>,
}

impl<T: Clone + Eq, W: Watcher<Value = T>> Join<T, W> {
    /// Joins a set of watchers into a single watcher
    pub fn new(watchers: impl Iterator<Item = W>) -> Self {
        let mut watchers: Vec<W> = watchers.into_iter().collect();

        let mut current = Vec::with_capacity(watchers.len());
        for watcher in &mut watchers {
            current.push(watcher.get());
        }
        Self { watchers, current }
    }
}

impl<T: Clone + Eq, W: Watcher<Value = T>> Watcher for Join<T, W> {
    type Value = Vec<T>;

    fn update(&mut self) -> bool {
        let mut any_updated = false;
        for (value, watcher) in self.current.iter_mut().zip(self.watchers.iter_mut()) {
            if watcher.update() {
                any_updated = true;
                *value = watcher.peek().clone();
            }
        }
        any_updated
    }

    fn peek(&self) -> &Self::Value {
        &self.current
    }

    fn is_connected(&self) -> bool {
        self.watchers.iter().all(|w| w.is_connected())
    }

    fn poll_updated(&mut self, cx: &mut task::Context<'_>) -> Poll<Result<(), Disconnected>> {
        let mut any_updated = false;
        for (value, watcher) in self.current.iter_mut().zip(self.watchers.iter_mut()) {
            if watcher.poll_updated(cx)?.is_ready() {
                any_updated = true;
                *value = watcher.peek().clone();
            }
        }

        if any_updated {
            Poll::Ready(Ok(()))
        } else {
            Poll::Pending
        }
    }
}

/// Wraps a [`Watcher`] to allow observing a derived value.
///
/// See [`Watcher::map`].
#[derive(derive_more::Debug, Clone)]
pub struct Map<W: Watcher, T: Clone + Eq> {
    #[debug("Arc<dyn Fn(W::Value) -> T>")]
    map: Arc<dyn Fn(W::Value) -> T + Send + Sync + 'static>,
    watcher: W,
    current: T,
}

impl<W: Watcher, T: Clone + Eq> Watcher for Map<W, T> {
    type Value = T;

    fn update(&mut self) -> bool {
        if self.watcher.update() {
            let new = (self.map)(self.watcher.peek().clone());
            if new != self.current {
                self.current = new;
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    fn peek(&self) -> &Self::Value {
        &self.current
    }

    fn is_connected(&self) -> bool {
        self.watcher.is_connected()
    }

    fn poll_updated(&mut self, cx: &mut task::Context<'_>) -> Poll<Result<(), Disconnected>> {
        loop {
            ready!(self.watcher.poll_updated(cx)?);
            let new = (self.map)(self.watcher.peek().clone());
            if new != self.current {
                self.current = new;
                return Poll::Ready(Ok(()));
            }
        }
    }
}

/// Future returning the next item after the current one in a [`Watcher`].
///
/// See [`Watcher::updated`].
///
/// # Cancel Safety
///
/// This future is cancel-safe.
#[derive(Debug)]
pub struct NextFut<'a, W: Watcher> {
    watcher: &'a mut W,
}

impl<W: Watcher> Future for NextFut<'_, W> {
    type Output = Result<W::Value, Disconnected>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Self::Output> {
        ready!(self.watcher.poll_updated(cx))?;
        Poll::Ready(Ok(self.watcher.peek().clone()))
    }
}

/// Future returning the current or next value that's [`Some`] value.
/// in a [`Watcher`].
///
/// See [`Watcher::initialized`].
///
/// # Cancel Safety
///
/// This Future is cancel-safe.
#[derive(Debug)]
pub struct InitializedFut<'a, T, V: Nullable<T> + Clone, W: Watcher<Value = V>> {
    initial: Option<T>,
    watcher: &'a mut W,
}

impl<T: Clone + Eq + Unpin, V: Nullable<T> + Clone, W: Watcher<Value = V> + Unpin> Future
    for InitializedFut<'_, T, V, W>
{
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Self::Output> {
        let mut this = self.as_mut();
        if let Some(value) = this.initial.take() {
            return Poll::Ready(value);
        }
        loop {
            if ready!(this.watcher.poll_updated(cx)).is_err() {
                // The value will never be initialized
                return Poll::Pending;
            };
            let value = this.watcher.peek();
            if let Some(value) = value.clone().into_option() {
                return Poll::Ready(value);
            }
        }
    }
}

/// A stream for a [`Watcher`]'s next values.
///
/// See [`Watcher::stream`] and [`Watcher::stream_updates_only`].
///
/// # Cancel Safety
///
/// This stream is cancel-safe.
#[derive(Debug, Clone)]
pub struct Stream<W: Watcher + Unpin> {
    initial: Option<W::Value>,
    watcher: W,
}

impl<W: Watcher + Unpin> n0_future::Stream for Stream<W>
where
    W::Value: Unpin,
{
    type Item = W::Value;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Option<Self::Item>> {
        if let Some(value) = self.as_mut().initial.take() {
            return Poll::Ready(Some(value));
        }
        match self.as_mut().watcher.poll_updated(cx) {
            Poll::Ready(Ok(())) => Poll::Ready(Some(self.as_ref().watcher.peek().clone())),
            Poll::Ready(Err(Disconnected)) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// The error for when a [`Watcher`] is disconnected from its underlying
/// [`Watchable`] value, because of that watchable having been dropped.
#[derive(StackError)]
#[error("Watcher lost connection to underlying Watchable, it was dropped")]
pub struct Disconnected;

// Private:

const INITIAL_EPOCH: u64 = 1;

/// The shared state for a [`Watchable`].
#[derive(Debug, Default)]
struct Shared<T> {
    /// The value to be watched and its current epoch.
    state: RwLock<State<T>>,
    wakers: Mutex<VecDeque<Waker>>,
}

#[derive(Debug, Clone)]
struct State<T> {
    value: T,
    epoch: u64,
}

impl<T: Default> Default for State<T> {
    fn default() -> Self {
        Self {
            value: Default::default(),
            epoch: INITIAL_EPOCH,
        }
    }
}

impl<T: Clone> Shared<T> {
    fn get(&self) -> T {
        self.state.read().expect("poisoned").value.clone()
    }

    fn state(&self) -> RwLockReadGuard<'_, State<T>> {
        self.state.read().expect("poisoned")
    }

    fn poll_updated(&self, cx: &mut task::Context<'_>, last_epoch: u64) -> Poll<State<T>> {
        {
            let state = self.state();

            // We might get spurious wakeups due to e.g. a second-to-last Watchable being dropped.
            // This makes sure we don't accidentally return an update that's not actually an update.
            if last_epoch < state.epoch {
                return Poll::Ready(state.clone());
            }
        }

        self.add_waker(cx);

        #[cfg(watcher_loom)]
        loom::thread::yield_now();

        // We check for an update again to prevent races between putting in wakers and looking for updates.
        {
            let state = self.state();

            if last_epoch < state.epoch {
                return Poll::Ready(state.clone());
            }
        }

        Poll::Pending
    }

    fn add_waker(&self, cx: &mut task::Context<'_>) {
        let mut wakers = self.wakers.lock().expect("poisoned");
        for waker in wakers.iter() {
            if waker.will_wake(cx.waker()) {
                return;
            }
        }
        wakers.push_back(cx.waker().clone());
    }
}

#[cfg(test)]
mod tests {

    use n0_future::{future::poll_once, StreamExt};
    use rand::{rng, RngExt};
    use tokio::{
        task::JoinSet,
        time::{Duration, Instant},
    };
    use tokio_util::sync::CancellationToken;

    use super::*;

    #[tokio::test]
    async fn test_watcher() {
        let cancel = CancellationToken::new();
        let watchable = Watchable::new(17);

        assert_eq!(watchable.watch().stream().next().await.unwrap(), 17);

        let start = Instant::now();
        // spawn watchers
        let mut tasks = JoinSet::new();
        for i in 0..3 {
            let mut watch = watchable.watch().stream();
            let cancel = cancel.clone();
            tasks.spawn(async move {
                println!("[{i}] spawn");
                let mut expected_value = 17;
                loop {
                    tokio::select! {
                        biased;
                        Some(value) = &mut watch.next() => {
                            println!("{:?} [{i}] update: {value}", start.elapsed());
                            assert_eq!(value, expected_value);
                            if expected_value == 17 {
                                expected_value = 0;
                            } else {
                                expected_value += 1;
                            }
                        },
                        _ = cancel.cancelled() => {
                            println!("{:?} [{i}] cancel", start.elapsed());
                            assert_eq!(expected_value, 10);
                            break;
                        }
                    }
                }
            });
        }
        for i in 0..3 {
            let mut watch = watchable.watch().stream_updates_only();
            let cancel = cancel.clone();
            tasks.spawn(async move {
                println!("[{i}] spawn");
                let mut expected_value = 0;
                loop {
                    tokio::select! {
                        biased;
                        Some(value) = watch.next() => {
                            println!("{:?} [{i}] stream update: {value}", start.elapsed());
                            assert_eq!(value, expected_value);
                            expected_value += 1;
                        },
                        _ = cancel.cancelled() => {
                            println!("{:?} [{i}] cancel", start.elapsed());
                            assert_eq!(expected_value, 10);
                            break;
                        }
                        else => {
                            panic!("stream died");
                        }
                    }
                }
            });
        }

        // set value
        for next_value in 0..10 {
            let sleep = Duration::from_nanos(rng().random_range(0..100_000_000));
            println!("{:?} sleep {sleep:?}", start.elapsed());
            tokio::time::sleep(sleep).await;

            let changed = watchable.set(next_value);
            println!("{:?} set {next_value} changed={changed:?}", start.elapsed());
        }

        println!("cancel");
        cancel.cancel();
        while let Some(res) = tasks.join_next().await {
            res.expect("task failed");
        }
    }

    #[test]
    fn test_get() {
        let watchable = Watchable::new(None);
        assert!(watchable.get().is_none());

        watchable.set(Some(1u8)).ok();
        assert_eq!(watchable.get(), Some(1u8));
    }

    #[tokio::test]
    async fn test_initialize() {
        let watchable = Watchable::new(None);

        let mut watcher = watchable.watch();
        let mut initialized = watcher.initialized();

        let poll = poll_once(&mut initialized).await;
        assert!(poll.is_none());

        watchable.set(Some(1u8)).ok();

        let poll = poll_once(&mut initialized).await;
        assert_eq!(poll.unwrap(), 1u8);
    }

    #[tokio::test]
    async fn test_initialize_already_init() {
        let watchable = Watchable::new(Some(1u8));

        let mut watcher = watchable.watch();
        let mut initialized = watcher.initialized();

        let poll = poll_once(&mut initialized).await;
        assert_eq!(poll.unwrap(), 1u8);
    }

    #[test]
    fn test_initialized_always_resolves() {
        #[cfg(not(watcher_loom))]
        use std::thread;

        #[cfg(watcher_loom)]
        use loom::thread;

        let test_case = || {
            let watchable = Watchable::<Option<u8>>::new(None);

            let mut watch = watchable.watch();
            let thread = thread::spawn(move || n0_future::future::block_on(watch.initialized()));

            watchable.set(Some(42)).ok();

            thread::yield_now();

            let value: u8 = thread.join().unwrap();

            assert_eq!(value, 42);
        };

        #[cfg(watcher_loom)]
        loom::model(test_case);
        #[cfg(not(watcher_loom))]
        test_case();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_update_cancel_safety() {
        let watchable = Watchable::new(0);
        let mut watch = watchable.watch();
        const MAX: usize = 100_000;

        let handle = tokio::spawn(async move {
            let mut last_observed = 0;

            while last_observed != MAX {
                tokio::select! {
                    val = watch.updated() => {
                        let Ok(val) = val else {
                            return;
                        };

                        assert_ne!(val, last_observed, "never observe the same value twice, even with cancellation");
                        last_observed = val;
                    }
                    _ = tokio::time::sleep(Duration::from_micros(rng().random_range(0..10_000))) => {
                        // We cancel the other future and start over again
                        continue;
                    }
                }
            }
        });

        for i in 1..=MAX {
            watchable.set(i).ok();
            if rng().random_bool(0.2) {
                tokio::task::yield_now().await;
            }
        }

        tokio::time::timeout(Duration::from_secs(10), handle)
            .await
            .unwrap()
            .unwrap()
    }

    #[tokio::test]
    async fn test_join_simple() {
        let a = Watchable::new(1u8);
        let b = Watchable::new(1u8);

        let mut ab = Join::new([a.watch(), b.watch()].into_iter());

        let stream = ab.clone().stream();
        let handle = tokio::task::spawn(async move { stream.take(5).collect::<Vec<_>>().await });

        // get
        assert_eq!(ab.get(), vec![1, 1]);
        // set a
        a.set(2u8).unwrap();
        tokio::task::yield_now().await;
        assert_eq!(ab.get(), vec![2, 1]);
        // set b
        b.set(3u8).unwrap();
        tokio::task::yield_now().await;
        assert_eq!(ab.get(), vec![2, 3]);

        a.set(3u8).unwrap();
        tokio::task::yield_now().await;
        b.set(4u8).unwrap();
        tokio::task::yield_now().await;

        let values = tokio::time::timeout(Duration::from_secs(5), handle)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            values,
            vec![vec![1, 1], vec![2, 1], vec![2, 3], vec![3, 3], vec![3, 4]]
        );
    }

    #[tokio::test]
    async fn test_updated_then_disconnect_then_get() {
        let watchable = Watchable::new(10);
        let mut watcher = watchable.watch();
        assert_eq!(watchable.get(), 10);
        watchable.set(42).ok();
        assert_eq!(watcher.updated().await.unwrap(), 42);
        drop(watchable);
        assert_eq!(watcher.get(), 42);
    }

    #[tokio::test(start_paused = true)]
    async fn test_update_wakeup_on_watchable_drop() {
        let watchable = Watchable::new(10);
        let mut watcher = watchable.watch();

        let start = Instant::now();
        let (_, result) = tokio::time::timeout(Duration::from_secs(2), async move {
            tokio::join!(
                async move {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    drop(watchable);
                },
                async move { watcher.updated().await }
            )
        })
        .await
        .expect("watcher never updated");
        // We should've updated 1s after start, since that's when the watchable was dropped.
        // If this is 2s, then the watchable dropping didn't wake up the `Watcher::updated` future.
        assert_eq!(start.elapsed(), Duration::from_secs(1));
        assert!(result.is_err());
    }

    #[tokio::test(start_paused = true)]
    async fn test_update_wakeup_always_a_change() {
        let watchable = Watchable::new(10);
        let mut watcher = watchable.watch();

        let task = tokio::spawn(async move {
            let mut last_value = watcher.get();
            let mut values = Vec::new();
            while let Ok(value) = watcher.updated().await {
                values.push(value);
                if last_value == value {
                    return Err("value duplicated");
                }
                last_value = value;
            }
            Ok(values)
        });

        // wait for the task to get set up and polled till pending for once
        tokio::time::sleep(Duration::from_millis(100)).await;

        watchable.set(11).ok();
        tokio::time::sleep(Duration::from_millis(100)).await;
        let clone = watchable.clone();
        drop(clone); // this shouldn't trigger an update
        tokio::time::sleep(Duration::from_millis(100)).await;
        for i in 1..=10 {
            watchable.set(i + 11).ok();
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        drop(watchable);

        let values = task
            .await
            .expect("task panicked")
            .expect("value duplicated");
        assert_eq!(values, vec![11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21]);
    }

    #[test]
    fn test_has_watchers() {
        let a = Watchable::new(1u8);
        assert!(!a.has_watchers());
        let b = a.clone();
        assert!(!a.has_watchers());
        assert!(!b.has_watchers());

        let watcher = a.watch();
        assert!(a.has_watchers());
        assert!(b.has_watchers());

        drop(watcher);

        assert!(!a.has_watchers());
        assert!(!b.has_watchers());
    }

    #[tokio::test]
    async fn test_three_watchers_basic() {
        let watchable = Watchable::new(1u8);

        let mut w1 = watchable.watch();
        let mut w2 = watchable.watch();
        let mut w3 = watchable.watch();

        // All see the initial value

        assert_eq!(w1.get(), 1);
        assert_eq!(w2.get(), 1);
        assert_eq!(w3.get(), 1);

        // Change  value
        watchable.set(42).unwrap();

        // All watchers get notified
        assert_eq!(w1.updated().await.unwrap(), 42);
        assert_eq!(w2.updated().await.unwrap(), 42);
        assert_eq!(w3.updated().await.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_three_watchers_skip_intermediate() {
        let watchable = Watchable::new(0u8);
        let mut watcher = watchable.watch();

        watchable.set(1).ok();
        watchable.set(2).ok();
        watchable.set(3).ok();
        watchable.set(4).ok();

        let value = watcher.updated().await.unwrap();

        assert_eq!(value, 4);
    }

    #[tokio::test]
    async fn test_three_watchers_with_streams() {
        let watchable = Watchable::new(10u8);

        let mut stream1 = watchable.watch().stream();
        let mut stream2 = watchable.watch().stream();
        let mut stream3 = watchable.watch().stream_updates_only();

        assert_eq!(stream1.next().await.unwrap(), 10);
        assert_eq!(stream2.next().await.unwrap(), 10);

        // Update the value
        watchable.set(20).ok();

        // All streams see the update
        assert_eq!(stream1.next().await.unwrap(), 20);
        assert_eq!(stream2.next().await.unwrap(), 20);
        assert_eq!(stream3.next().await.unwrap(), 20);
    }

    #[tokio::test]
    async fn test_three_watchers_independent() {
        let watchable = Watchable::new(0u8);

        let mut fast_watcher = watchable.watch();
        let mut slow_watcher = watchable.watch();
        let mut lazy_watcher = watchable.watch();

        watchable.set(1).ok();
        assert_eq!(fast_watcher.updated().await.unwrap(), 1);

        // More updates happen
        watchable.set(2).ok();
        watchable.set(3).ok();

        assert_eq!(slow_watcher.updated().await.unwrap(), 3);
        assert_eq!(lazy_watcher.get(), 3);
    }

    #[tokio::test]
    async fn test_combine_three_watchers() {
        let a = Watchable::new(1u8);
        let b = Watchable::new(2u8);
        let c = Watchable::new(3u8);

        let mut combined = Triple::new(a.watch(), b.watch(), c.watch());

        assert_eq!(combined.get(), (1, 2, 3));

        // Update one
        b.set(20).ok();

        assert_eq!(combined.updated().await.unwrap(), (1, 20, 3));

        c.set(30).ok();
        assert_eq!(combined.updated().await.unwrap(), (1, 20, 30));
    }

    #[tokio::test]
    async fn test_three_watchers_disconnection() {
        let watchable = Watchable::new(5u8);

        // All connected
        let mut w1 = watchable.watch();
        let mut w2 = watchable.watch();
        let mut w3 = watchable.watch();

        // Drop the watchable
        drop(watchable);

        // All become disconnected
        assert!(!w1.is_connected());
        assert!(!w2.is_connected());
        assert!(!w3.is_connected());

        // Can still get last known value
        assert_eq!(w1.get(), 5);
        assert_eq!(w2.get(), 5);

        // But updates fail
        assert!(w3.updated().await.is_err());
    }

    #[tokio::test]
    async fn test_three_watchers_truly_concurrent() {
        use tokio::time::sleep;
        let watchable = Watchable::new(0u8);

        // Spawn three READER tasks
        let mut reader_handles = vec![];
        for i in 0..3 {
            let mut watcher = watchable.watch();
            let handle = tokio::spawn(async move {
                let mut values = vec![];
                // Collect up to 5 updates
                for _ in 0..5 {
                    if let Ok(value) = watcher.updated().await {
                        values.push(value);
                    } else {
                        break;
                    }
                }
                (i, values)
            });
            reader_handles.push(handle);
        }

        // Spawn three WRITER tasks that update concurrently
        let mut writer_handles = vec![];
        for i in 0..3 {
            let watchable_clone = watchable.clone();
            let handle = tokio::spawn(async move {
                for j in 0..5 {
                    let value = (i * 10) + j;
                    watchable_clone.set(value).ok();
                    sleep(Duration::from_millis(5)).await;
                }
            });
            writer_handles.push(handle);
        }

        // Wait for writers to finish
        for handle in writer_handles {
            handle.await.unwrap();
        }

        // Wait for readers and check results
        for handle in reader_handles {
            let (task_id, values) = handle.await.unwrap();
            println!("Reader {}: saw values {:?}", task_id, values);
            assert!(!values.is_empty());
        }
    }

    #[tokio::test]
    async fn test_peek() {
        let a = Watchable::new(vec![1, 2, 3]);
        let mut wa = a.watch();

        assert_eq!(wa.get(), vec![1, 2, 3]);
        assert_eq!(wa.peek(), &vec![1, 2, 3]);

        let mut wa_map = wa.map(|a| a.into_iter().map(|a| a * 2).collect::<Vec<_>>());

        assert_eq!(wa_map.get(), vec![2, 4, 6]);
        assert_eq!(wa_map.peek(), &vec![2, 4, 6]);

        let mut wb = a.watch();

        assert_eq!(wb.get(), vec![1, 2, 3]);
        assert_eq!(wb.peek(), &vec![1, 2, 3]);

        let mut wb_map = wb.map(|a| a.into_iter().map(|a| a * 2).collect::<Vec<_>>());

        assert_eq!(wb_map.get(), vec![2, 4, 6]);
        assert_eq!(wb_map.peek(), &vec![2, 4, 6]);

        let mut w_join = Join::new([wa_map, wb_map].into_iter());

        assert_eq!(w_join.get(), vec![vec![2, 4, 6], vec![2, 4, 6]]);
        assert_eq!(w_join.peek(), &vec![vec![2, 4, 6], vec![2, 4, 6]]);
    }

    #[tokio::test]
    async fn test_update_updates_peek() {
        let value = Watchable::new(42);
        let mut watcher = value.watch();

        assert_eq!(watcher.peek(), &42);
        assert!(!watcher.update());

        value.set(50).ok();

        assert_eq!(watcher.peek(), &42); // watcher wasn't updated yet
        assert!(watcher.update()); // Update returns true, because there was an update
        assert_eq!(watcher.peek(), &50);
        assert!(!watcher.update());

        let mut watcher_map = watcher.clone().map(|v| v * 2);

        assert_eq!(watcher_map.peek(), &100);
        assert!(!watcher_map.update());

        value.set(10).ok();

        assert_eq!(watcher_map.peek(), &100);
        assert!(watcher_map.update());
        assert_eq!(watcher_map.peek(), &20);
        assert!(!watcher_map.update());

        let value2 = Watchable::new(0);
        let mut watcher_join = Join::new([watcher, value2.watch()].into_iter());

        assert_eq!(watcher_join.peek(), &vec![10, 0]);
        assert!(!watcher_join.update());

        value.set(0).ok();
        value2.set(1).ok();

        assert_eq!(watcher_join.peek(), &vec![10, 0]);
        assert!(watcher_join.update());
        assert_eq!(watcher_join.peek(), &vec![0, 1]);
        assert!(!watcher_join.update());
    }

    #[tokio::test]
    async fn test_get_updates_peek() {
        let value = Watchable::new(42);
        let mut watcher = value.watch();

        assert_eq!(watcher.peek(), &42);
        assert!(!watcher.update());

        value.set(50).ok();

        assert_eq!(watcher.peek(), &42); // watcher wasn't updated yet
        assert_eq!(watcher.get(), 50); // Update returns true, because there was an update
        assert_eq!(watcher.peek(), &50);
        assert!(!watcher.update());

        let mut watcher_map = watcher.clone().map(|v| v * 2);

        assert_eq!(watcher_map.peek(), &100);
        assert!(!watcher_map.update());

        value.set(10).ok();

        assert_eq!(watcher_map.peek(), &100);
        assert_eq!(watcher_map.get(), 20);
        assert_eq!(watcher_map.peek(), &20);
        assert!(!watcher_map.update());

        let value2 = Watchable::new(0);
        let mut watcher_join = Join::new([watcher, value2.watch()].into_iter());

        assert_eq!(watcher_join.peek(), &vec![10, 0]);
        assert!(!watcher_join.update());

        value.set(0).ok();
        value2.set(1).ok();

        assert_eq!(watcher_join.peek(), &vec![10, 0]);
        assert_eq!(watcher_join.get(), vec![0, 1]);
        assert_eq!(watcher_join.peek(), &vec![0, 1]);
        assert!(!watcher_join.update());
    }

    #[tokio::test]
    async fn test_ensure_wakers_bounded() {
        use tokio::time::{interval, Duration};
        let watchable = Watchable::new(0);
        let mut watcher = watchable.watch();
        let max_tick = 1000;

        let handle = tokio::spawn(async move {
            let mut ticker = interval(Duration::from_nanos(1));
            let mut tick_no = 0;
            loop {
                tokio::select! {
                    _ = watcher.updated() => {}
                    _ = ticker.tick() => {
                        // We cancel the other future and start over again
                        tick_no += 1;
                        if tick_no > max_tick{
                            return
                        }
                    }
                }
                let num_wakers = watchable.shared.wakers.lock().unwrap().len();
                assert_eq!(num_wakers, 1);
            }
        });

        tokio::time::timeout(Duration::from_secs(1), handle)
            .await
            .unwrap()
            .unwrap()
    }
}
