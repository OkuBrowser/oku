pub mod optional;
pub mod set;
pub mod timeout_map;
pub mod timeout_set;

use crate::common::InnerMap;
use futures::stream::{FusedStream, SelectAll};
use futures::{Stream, StreamExt};
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

/// Combining multiple streams into one, with each stream having a unique key.
pub struct StreamMap<K, S> {
    list: SelectAll<InnerMap<K, S>>,
    empty: bool,
    waker: Option<Waker>,
}

impl<K, T> Default for StreamMap<K, T>
where
    K: Clone + Unpin,
    T: Stream + Send + Unpin + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K, T> StreamMap<K, T>
where
    K: Clone + Unpin,
    T: Stream + Send + Unpin + 'static,
{
    /// Creates an empty [`StreamMap`]
    pub fn new() -> Self {
        Self {
            list: SelectAll::new(),
            empty: true,
            waker: None,
        }
    }
}

impl<K, T> StreamMap<K, T>
where
    K: Clone + PartialEq + Send + Unpin + 'static,
    T: Stream + Send + Unpin + 'static,
{
    /// Insert a stream into the map with a unique key.
    /// The function will return true if the map does not have the key present,
    /// otherwise it will return false
    pub fn insert(&mut self, key: K, stream: T) -> bool {
        if self.contains_key(&key) {
            return false;
        }

        let st = InnerMap::new(key, stream);
        self.list.push(st);

        if let Some(waker) = self.waker.take() {
            waker.wake();
        }

        self.empty = false;
        true
    }

    /// Mark stream with assigned key to wake up on successful yield.
    /// Will return false if stream does not exist or if value is the same as
    /// previously set.
    pub fn set_wake_on_success(&mut self, key: &K, wake_on_success: bool) -> bool {
        self.list
            .iter_mut()
            .find(|st| st.key().eq(key))
            .is_some_and(|st| st.set_wake_on_success(wake_on_success))
    }

    /// An iterator visiting all key-value pairs in arbitrary order.
    pub fn iter(&self) -> impl Iterator<Item = (&K, &T)> {
        self.list.iter().filter_map(|st| st.key_value())
    }

    /// An iterator visiting all key-value pairs mutably in arbitrary order.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&K, &mut T)> {
        self.list.iter_mut().filter_map(|st| st.key_value_mut())
    }

    /// An iterator visiting all key-value pairs with a pinned valued in arbitrary order
    pub fn iter_pin(&mut self) -> impl Iterator<Item = (&K, Pin<&mut T>)> {
        self.list.iter_mut().filter_map(|st| st.key_value_pin())
    }

    /// Returns an iterator visiting all keys in arbitrary order.
    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.list.iter().map(|st| st.key())
    }

    /// An iterator visiting all values in arbitrary order.
    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.list.iter().filter_map(|st| st.inner())
    }

    /// An iterator visiting all values mutably in arbitrary order.
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.list.iter_mut().filter_map(|st| st.inner_mut())
    }

    /// Returns `true` if the map contains a stream for the specified key.
    pub fn contains_key(&self, key: &K) -> bool {
        self.list.iter().any(|st| st.key().eq(key))
    }

    /// Clears the map.
    pub fn clear(&mut self) {
        self.list.clear();
    }

    /// Returns a reference to the stream corresponding to the key.
    pub fn get(&self, key: &K) -> Option<&T> {
        self.list
            .iter()
            .find(|st| st.key().eq(key))
            .and_then(|st| st.inner())
    }

    /// Returns a mutable stream to the value corresponding to the key.
    pub fn get_mut(&mut self, key: &K) -> Option<&mut T> {
        self.list
            .iter_mut()
            .find(|st| st.key().eq(key))
            .and_then(|st| st.inner_mut())
    }

    /// Returns a muable stream or default value if it does not exist.
    pub fn get_mut_or_default(&mut self, key: &K) -> &mut T
    where
        T: Default,
    {
        self.insert(key.clone(), T::default());
        self.get_mut(key).expect("valid entry")
    }

    /// Returns a pinned stream corresponding to the key.
    pub fn get_pinned(&mut self, key: &K) -> Option<Pin<&mut T>> {
        self.list
            .iter_mut()
            .find(|st| st.key().eq(key))
            .and_then(|st| st.inner_pin())
    }

    /// Removes a key from the map, returning the stream.
    pub fn remove(&mut self, key: &K) -> Option<T> {
        self.list
            .iter_mut()
            .find(|st| st.key().eq(key))
            .and_then(|st| st.take_inner())
    }

    /// Returns the number of streams in the map.
    pub fn len(&self) -> usize {
        self.list.iter().filter(|st| st.inner().is_some()).count()
    }

    /// Return `true` map contains no elements.
    pub fn is_empty(&self) -> bool {
        self.list.is_empty() || self.list.iter().all(|st| st.inner().is_none())
    }
}

impl<K, T> FromIterator<(K, T)> for StreamMap<K, T>
where
    K: Clone + PartialEq + Send + Unpin + 'static,
    T: Stream + Send + Unpin + 'static,
{
    fn from_iter<I: IntoIterator<Item = (K, T)>>(iter: I) -> Self {
        let mut maps = Self::new();
        for (key, val) in iter {
            maps.insert(key, val);
        }
        maps
    }
}

impl<K, T> Stream for StreamMap<K, T>
where
    K: Clone + PartialEq + Send + Unpin + 'static,
    T: Stream + Unpin + Send + 'static,
{
    type Item = (K, T::Item);

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = &mut *self;

        if this.list.is_empty() {
            self.waker = Some(cx.waker().clone());
            return Poll::Pending;
        }

        loop {
            match this.list.poll_next_unpin(cx) {
                Poll::Ready(Some((key, Some(item)))) => return Poll::Ready(Some((key, item))),
                // We continue in case there is any progress on the set of streams
                Poll::Ready(Some((key, None))) => {
                    this.remove(&key);
                }
                Poll::Ready(None) => {
                    // While we could allow the stream to continue to be pending, it would make more sense to notify that the stream
                    // is empty without needing to explicitly check while polling the actual "map" itself
                    // So we would mark a field to notify that the state is finished and return `Poll::Ready(None)` so the stream
                    // can be terminated while on the next poll, we could let it be return pending.
                    // We do this so that we are not returning `Poll::Ready(None)` each time the map is polled
                    // as that may be seen as UB and may cause an increase in cpu usage
                    if self.empty {
                        self.waker = Some(cx.waker().clone());
                        return Poll::Pending;
                    }

                    self.empty = true;
                    return Poll::Ready(None);
                }
                Poll::Pending => {
                    // Returning `None` does not mean the stream is actually terminated
                    self.waker = Some(cx.waker().clone());
                    return Poll::Pending;
                }
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.list.size_hint()
    }
}

impl<K, T> FusedStream for StreamMap<K, T>
where
    K: Clone + PartialEq + Send + Unpin + 'static,
    T: Stream + Unpin + Send + 'static,
{
    fn is_terminated(&self) -> bool {
        self.list.is_terminated()
    }
}

#[cfg(test)]
mod test {
    use crate::stream::StreamMap;
    use futures::stream::empty;
    use futures::{Stream, StreamExt};
    use std::pin::Pin;
    use std::task::{Context, Poll};

    struct Once<T> {
        value: T,
    }

    impl<T> Once<T> {
        pub fn new(value: T) -> Self {
            Self { value }
        }

        pub fn get(&self) -> &T {
            &self.value
        }

        pub fn set(&mut self, val: T) {
            self.value = val;
        }
    }

    impl<T> Stream for Once<T>
    where
        T: Unpin,
    {
        type Item = T;
        fn poll_next(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            Poll::Pending
        }
    }

    #[test]
    fn existing_key() {
        let mut map = StreamMap::new();
        assert!(map.insert(1, empty::<()>()));
        assert!(!map.insert(1, empty::<()>()));
    }

    #[test]
    fn poll_multiple_keyed_streams() {
        let mut map = StreamMap::new();
        map.insert(1, futures::stream::once(async { 10 }).boxed());
        map.insert(2, futures::stream::once(async { 20 }).boxed());

        map.insert(3, futures::stream::iter(vec![30, 40, 50]).boxed());

        futures::executor::block_on(async move {
            assert_eq!(map.next().await, Some((1, 10)));
            assert_eq!(map.next().await, Some((2, 20)));
            assert_eq!(map.next().await, Some((3, 30)));
            assert_eq!(map.next().await, Some((3, 40)));
            assert_eq!(map.next().await, Some((3, 50)));
            assert_eq!(map.next().await, None);
            let pending =
                futures::future::poll_fn(|cx| Poll::Ready(map.poll_next_unpin(cx).is_pending()))
                    .await;
            assert!(pending);
        })
    }

    #[test]
    fn get_from_map() {
        let mut map = StreamMap::new();
        map.insert(1, Once::new(10));
        map.insert(2, Once::new(20));

        {
            let value0 = map.get(&1).expect("valid entry").get();
            let value1 = map.get(&2).expect("valid entry").get();

            assert_eq!(value0, &10);
            assert_eq!(value1, &20);
        }

        {
            map.get_mut(&1).expect("valid entry").set(100);
            let value0 = map.get(&1).expect("valid entry").get();
            assert_eq!(*value0, 100);
        }
    }
}
