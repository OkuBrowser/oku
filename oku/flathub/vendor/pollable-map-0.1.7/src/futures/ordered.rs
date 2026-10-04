use futures::Stream;
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

/// An unbounded queue of futures imposed a FIFO order while polling one future at a time
/// and returning the output to stream before popping the next future in queue to be polled.
pub struct OrderedFutureSet<F> {
    queue: VecDeque<F>,
    current_future: Option<F>,
    waker: Option<Waker>,
}

impl<F> Default for OrderedFutureSet<F> {
    fn default() -> Self {
        Self {
            queue: VecDeque::new(),
            current_future: None,
            waker: None,
        }
    }
}

impl<F> OrderedFutureSet<F> {
    /// Constructs a new, empty [`OrderedFutureSet`]
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a future to the back of the queue
    pub fn push(&mut self, fut: F) {
        self.queue.push_back(fut);
        if let Some(waker) = self.waker.take() {
            waker.wake();
        }
    }

    /// Remove a future from the front of the queue
    pub fn pop_front(&mut self) -> Option<F> {
        let fut = self.queue.pop_front();
        if let Some(waker) = self.waker.take() {
            waker.wake();
        }
        fut
    }

    /// Remove a future from the back of the queue
    pub fn pop_back(&mut self) -> Option<F> {
        let fut = self.queue.pop_back();
        if let Some(waker) = self.waker.take() {
            waker.wake();
        }
        fut
    }
}

impl<F> FromIterator<F> for OrderedFutureSet<F>
where
    F: Future + Send + Unpin + 'static,
{
    fn from_iter<T: IntoIterator<Item = F>>(iter: T) -> Self {
        let mut ordered = Self::new();
        for fut in iter {
            ordered.push(fut);
        }
        ordered
    }
}

impl<F> Stream for OrderedFutureSet<F>
where
    F: Future + Send + Unpin + 'static,
{
    type Item = F::Output;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let this = &mut *self;

        loop {
            if this.current_future.is_none() {
                let Some(fut) = this.queue.pop_front() else {
                    break;
                };
                this.current_future.replace(fut);
            }

            match this.current_future.as_mut() {
                Some(fut) => {
                    let output = futures::ready!(Pin::new(fut).poll(cx));
                    this.current_future.take();
                    cx.waker().wake_by_ref();
                    return Poll::Ready(Some(output));
                }
                None => {
                    this.waker.replace(cx.waker().clone());
                }
            }
        }

        this.waker.replace(cx.waker().clone());
        Poll::Pending
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.queue.len(), None)
    }
}

#[cfg(test)]
mod tests {
    use crate::futures::ordered::OrderedFutureSet;
    use futures::StreamExt;

    #[test]
    fn fifo_futures() {
        futures::executor::block_on(async move {
            let mut fifo = OrderedFutureSet::new();
            fifo.push(futures::future::ready(1));
            fifo.push(futures::future::ready(2));
            fifo.push(futures::future::ready(4));
            fifo.push(futures::future::ready(3));

            let items = fifo.take(4).collect::<Vec<u8>>().await;

            assert_eq!(items, vec![1, 2, 4, 3]);
        });
    }

    #[test]
    fn remove_front_entry() {
        futures::executor::block_on(async move {
            let mut fifo = OrderedFutureSet::new();
            fifo.push(futures::future::ready(1));
            fifo.push(futures::future::ready(2));
            fifo.push(futures::future::ready(4));
            fifo.push(futures::future::ready(3));

            let front_fut = fifo.pop_front();
            // TODO: Write a `Ready` future that supports `Eq` and `PartialEq` for tests
            //       to use `assert_eq(front_fut, Some(futures::future::ready(1)));`
            assert!(front_fut.is_some());

            let items = fifo.take(3).collect::<Vec<u8>>().await;

            assert_eq!(items, vec![2, 4, 3]);
        })
    }

    #[test]
    fn remove_back_entry() {
        futures::executor::block_on(async move {
            let mut fifo = OrderedFutureSet::new();
            fifo.push(futures::future::ready(1));
            fifo.push(futures::future::ready(2));
            fifo.push(futures::future::ready(4));
            fifo.push(futures::future::ready(3));

            let front_fut = fifo.pop_back();
            // TODO: Write a `Ready` future that supports `Eq` and `PartialEq` for tests
            //       to use `assert_eq(front_fut, Some(futures::future::ready(3)));`
            assert!(front_fut.is_some());

            let items = fifo.take(3).collect::<Vec<u8>>().await;

            assert_eq!(items, vec![1, 2, 4]);
        })
    }
}
