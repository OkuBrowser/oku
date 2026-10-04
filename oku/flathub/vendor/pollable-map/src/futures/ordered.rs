use alloc::collections::VecDeque;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll, Waker};
use futures::Stream;

/// An unbounded queue of futures imposed a FIFO order while polling one future at a time
/// and returning the output to stream before popping the next future in queue to be polled.
#[pin_project::pin_project]
pub struct OrderedFutureSet<F> {
    queue: VecDeque<F>,
    #[pin]
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

    /// Push a future to the back of a pinned queue.
    pub fn push_pinned(self: Pin<&mut Self>, fut: F) {
        let this = self.project();
        this.queue.push_back(fut);
        if let Some(waker) = this.waker.take() {
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

    /// Remove a future from the front of a pinned queue.
    pub fn pop_front_pinned(self: Pin<&mut Self>) -> Option<F> {
        let this = self.project();
        let fut = this.queue.pop_front();
        if let Some(waker) = this.waker.take() {
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

    /// Remove a future from the back of a pinned queue.
    pub fn pop_back_pinned(self: Pin<&mut Self>) -> Option<F> {
        let this = self.project();
        let fut = this.queue.pop_back();
        if let Some(waker) = this.waker.take() {
            waker.wake();
        }
        fut
    }
}

impl<F> FromIterator<F> for OrderedFutureSet<F> {
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
    F: Future,
{
    type Item = F::Output;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        if this.current_future.as_ref().get_ref().is_none() {
            let Some(fut) = this.queue.pop_front() else {
                this.waker.replace(cx.waker().clone());
                return Poll::Pending;
            };
            this.current_future.set(Some(fut));
        }

        let fut = this
            .current_future
            .as_mut()
            .as_pin_mut()
            .expect("current future was initialized");

        match fut.poll(cx) {
            Poll::Ready(output) => {
                this.current_future.set(None);
                cx.waker().wake_by_ref();
                Poll::Ready(Some(output))
            }
            Poll::Pending => {
                this.waker.replace(cx.waker().clone());
                Poll::Pending
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (
            self.queue.len() + usize::from(self.current_future.is_some()),
            None,
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::futures::ordered::OrderedFutureSet;
    use alloc::vec;
    use alloc::vec::Vec;
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

    #[test]
    fn supports_unboxed_async_futures() {
        async fn value(value: u8) -> u8 {
            value
        }

        futures::executor::block_on(async move {
            let mut fifo = OrderedFutureSet::new();
            fifo.push(value(1));
            fifo.push(value(2));
            futures::pin_mut!(fifo);

            assert_eq!(fifo.as_mut().next().await, Some(1));
            assert_eq!(fifo.as_mut().next().await, Some(2));

            fifo.as_mut().push_pinned(value(3));
            assert_eq!(fifo.as_mut().next().await, Some(3));
        });
    }
}
