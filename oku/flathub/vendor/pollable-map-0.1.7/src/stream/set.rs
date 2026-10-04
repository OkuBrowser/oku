use futures::{Stream, StreamExt};
use std::pin::Pin;

use super::StreamMap;
use futures::stream::FusedStream;
use std::task::{Context, Poll};

pub struct StreamSet<S> {
    id: i64,
    map: StreamMap<i64, S>,
}

impl<S> Default for StreamSet<S>
where
    S: Stream + Send + Unpin + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<S> StreamSet<S>
where
    S: Stream + Send + Unpin + 'static,
{
    /// Creates an empty ['StreamSet`]
    pub fn new() -> Self {
        Self {
            id: 0,
            map: StreamMap::default(),
        }
    }

    /// Insert a stream into the set of streams.
    pub fn insert(&mut self, stream: S) -> bool {
        self.id = self.id.wrapping_add(1);
        self.map.insert(self.id, stream)
    }

    /// An iterator visiting all streams in arbitrary order.
    pub fn iter(&self) -> impl Iterator<Item = &S> {
        self.map.iter().map(|(_, st)| st)
    }

    /// An iterator visiting all streams mutably in arbitrary order.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut S> {
        self.map.iter_mut().map(|(_, st)| st)
    }

    /// An iterator visiting all streams pinned valued in arbitrary order
    pub fn iter_pin(&mut self) -> impl Iterator<Item = Pin<&mut S>> {
        self.map.iter_pin().map(|(_, st)| st)
    }

    /// Clears the set.
    pub fn clear(&mut self) {
        self.map.clear();
    }

    /// Returns the number of streams in the set.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Return `true` map contains no elements.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

impl<S> FromIterator<S> for StreamSet<S>
where
    S: Stream + Send + Unpin + 'static,
{
    fn from_iter<I: IntoIterator<Item = S>>(iter: I) -> Self {
        let mut maps = Self::new();
        for st in iter {
            maps.insert(st);
        }
        maps
    }
}

impl<S> Stream for StreamSet<S>
where
    S: Stream + Send + Unpin + 'static,
{
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.map
            .poll_next_unpin(cx)
            .map(|output| output.map(|(_, item)| item))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.map.size_hint()
    }
}

impl<S> FusedStream for StreamSet<S>
where
    S: Stream + Send + Unpin + 'static,
{
    fn is_terminated(&self) -> bool {
        self.map.is_terminated()
    }
}

#[cfg(test)]
mod test {
    use crate::stream::set::StreamSet;
    use futures::StreamExt;

    #[test]
    fn valid_stream_set() {
        let mut list = StreamSet::new();
        assert!(list.insert(futures::stream::once(async { 0 }).boxed()));
        assert!(list.insert(futures::stream::once(async { 1 }).boxed()));

        futures::executor::block_on(async move {
            let val = list.next().await;
            assert_eq!(val, Some(0));
            let val = list.next().await;
            assert_eq!(val, Some(1));
        });
    }
}
