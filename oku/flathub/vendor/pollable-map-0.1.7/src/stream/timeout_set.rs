use crate::common::Timed;
use crate::stream::set::StreamSet;
use futures::stream::FusedStream;
use futures::{Stream, StreamExt};
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

pub struct TimeoutStreamSet<S> {
    duration: Duration,
    set: StreamSet<Timed<S>>,
}

impl<S> Deref for TimeoutStreamSet<S> {
    type Target = StreamSet<Timed<S>>;
    fn deref(&self) -> &Self::Target {
        &self.set
    }
}

impl<S> DerefMut for TimeoutStreamSet<S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.set
    }
}

impl<S> TimeoutStreamSet<S>
where
    S: Stream + Send + Unpin + 'static,
{
    /// Create an empty ['TimeoutStreamSet']
    pub fn new(duration: Duration) -> Self {
        Self {
            duration,
            set: StreamSet::new(),
        }
    }

    /// Insert a stream into the set of streams.
    pub fn insert(&mut self, stream: S) -> bool {
        self.set.insert(Timed::new(stream, self.duration))
    }
}

impl<S> Stream for TimeoutStreamSet<S>
where
    S: Stream + Send + Unpin + 'static,
{
    type Item = std::io::Result<S::Item>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.set.poll_next_unpin(cx)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.set.size_hint()
    }
}

impl<S> FusedStream for TimeoutStreamSet<S>
where
    S: Stream + Send + Unpin + 'static,
{
    fn is_terminated(&self) -> bool {
        self.set.is_terminated()
    }
}

#[cfg(test)]
mod test {
    use crate::stream::timeout_set::TimeoutStreamSet;
    use futures::StreamExt;
    use std::time::Duration;

    #[test]
    fn timeout_set() {
        let mut list = TimeoutStreamSet::new(Duration::from_millis(100));
        assert!(list.insert(futures::stream::pending::<()>()));

        futures::executor::block_on(async move {
            let result = list.next().await;
            let Some(Err(e)) = result else {
                unreachable!("result is err");
            };

            assert_eq!(e.kind(), std::io::ErrorKind::TimedOut);
        });
    }

    #[test]
    fn valid_stream() {
        let mut list = TimeoutStreamSet::new(Duration::from_secs(10));
        assert!(list.insert(futures::stream::once(async { 0 }).boxed()));

        futures::executor::block_on(async move {
            let result = list.next().await;
            let Some(Ok(val)) = result else {
                unreachable!("result is err");
            };

            assert_eq!(val, 0);
        });
    }
}
