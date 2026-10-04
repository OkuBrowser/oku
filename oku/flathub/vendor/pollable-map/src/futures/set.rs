use core::future::Future;
use core::pin::Pin;

use super::FutureMap;
use core::task::{Context, Poll};
use futures::stream::FusedStream;
use futures::{Stream, StreamExt};

pub struct FutureSet<S> {
    id: i64,
    map: FutureMap<i64, S>,
}

impl<S> Default for FutureSet<S>
where
    S: Future,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<S> FutureSet<S>
where
    S: Future,
{
    /// Creates an empty [`FutureSet`]
    pub fn new() -> Self {
        Self {
            id: 0,
            map: FutureMap::default(),
        }
    }

    /// Insert a future into the set of futures.
    pub fn insert(&mut self, fut: S) -> bool {
        self.id = self.id.wrapping_add(1);
        self.map.insert(self.id, fut)
    }

    /// An iterator visiting all futures in arbitrary order.
    pub fn iter(&self) -> impl Iterator<Item = &S> {
        self.map.iter().map(|(_, st)| st)
    }

    /// An iterator visiting all futures pinned valued in arbitrary order
    pub fn iter_pin(&mut self) -> impl Iterator<Item = Pin<&mut S>> {
        self.map.iter_pin().map(|(_, st)| st)
    }

    /// Clears the set.
    pub fn clear(&mut self) {
        self.map.clear();
    }

    /// Returns the number of futures in the set.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Return `true` map contains no elements.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

impl<S> FutureSet<S>
where
    S: Future + Unpin,
{
    /// An iterator visiting all futures mutably in arbitrary order.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut S> {
        self.map.iter_mut().map(|(_, st)| st)
    }
}

impl<S> FromIterator<S> for FutureSet<S>
where
    S: Future,
{
    fn from_iter<I: IntoIterator<Item = S>>(iter: I) -> Self {
        let mut maps = Self::new();
        for st in iter {
            maps.insert(st);
        }
        maps
    }
}

impl<S> Stream for FutureSet<S>
where
    S: Future,
{
    type Item = S::Output;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.map
            .poll_next_unpin(cx)
            .map(|output| output.map(|(_, item)| item))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.map.size_hint()
    }
}

impl<S> FusedStream for FutureSet<S>
where
    S: Future,
{
    fn is_terminated(&self) -> bool {
        self.map.is_terminated()
    }
}

#[cfg(test)]
mod test {
    use crate::futures::set::FutureSet;
    use futures::StreamExt;

    #[test]
    fn valid_future_set() {
        let mut list = FutureSet::new();
        assert!(list.insert(futures::future::ready(0)));
        assert!(list.insert(futures::future::ready(1)));

        futures::executor::block_on(async move {
            let val = list.next().await;
            assert_eq!(val, Some(0));
            let val = list.next().await;
            assert_eq!(val, Some(1));
        });
    }

    #[test]
    fn supports_unboxed_async_future() {
        let mut set = FutureSet::new();
        assert!(set.insert(async { 42 }));

        futures::executor::block_on(async move {
            assert_eq!(set.next().await, Some(42));
        });
    }
}
