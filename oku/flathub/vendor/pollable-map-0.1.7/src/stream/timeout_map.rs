use crate::common::Timed;
use crate::stream::StreamMap;
use futures::stream::FusedStream;
use futures::{Stream, StreamExt};
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

pub struct TimeoutStreamMap<K, S> {
    duration: Duration,
    map: StreamMap<K, Timed<S>>,
}

impl<K, S> Deref for TimeoutStreamMap<K, S> {
    type Target = StreamMap<K, Timed<S>>;
    fn deref(&self) -> &Self::Target {
        &self.map
    }
}

impl<K, S> DerefMut for TimeoutStreamMap<K, S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.map
    }
}

impl<K, S> TimeoutStreamMap<K, S>
where
    K: Clone + PartialEq + Send + Unpin + 'static,
    S: Stream + Send + Unpin + 'static,
{
    /// Create an empty [`TimeoutStreamMap`]
    pub fn new(duration: Duration) -> Self {
        Self {
            duration,
            map: StreamMap::new(),
        }
    }

    /// Insert a stream into the map with a unique key.
    /// The function will return true if the map does not have the key present,
    /// otherwise it will return false
    pub fn insert(&mut self, key: K, stream: S) -> bool {
        self.map.insert(key, Timed::new(stream, self.duration))
    }
}

impl<K, S> Stream for TimeoutStreamMap<K, S>
where
    K: Clone + PartialEq + Send + Unpin + 'static,
    S: Stream + Send + Unpin + 'static,
{
    type Item = (K, std::io::Result<S::Item>);
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.map.poll_next_unpin(cx)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.map.size_hint()
    }
}

impl<K, S> FusedStream for TimeoutStreamMap<K, S>
where
    K: Clone + PartialEq + Send + Unpin + 'static,
    S: Stream + Send + Unpin + 'static,
{
    fn is_terminated(&self) -> bool {
        self.map.is_terminated()
    }
}

#[cfg(test)]
mod test {
    use crate::stream::timeout_map::TimeoutStreamMap;
    use futures::StreamExt;
    use std::time::Duration;

    #[test]
    fn timeout_map() {
        let mut list = TimeoutStreamMap::new(Duration::from_millis(100));
        assert!(list.insert(0, futures::stream::pending::<()>()));

        futures::executor::block_on(async move {
            let result = list.next().await;
            let Some((0, Err(e))) = result else {
                unreachable!("result is err");
            };

            assert_eq!(e.kind(), std::io::ErrorKind::TimedOut);
        });
    }

    #[test]
    fn valid_stream() {
        let mut list = TimeoutStreamMap::new(Duration::from_secs(10));
        assert!(list.insert(1, futures::stream::once(async { 0 }).boxed()));

        futures::executor::block_on(async move {
            let result = list.next().await;
            let Some((1, Ok(val))) = result else {
                unreachable!("result is err");
            };

            assert_eq!(val, 0);
        });
    }
}
