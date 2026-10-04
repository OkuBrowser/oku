use crate::common::Timed;
use crate::error::TimedError;
use crate::optional::Optional;
use core::future::Future;
use core::ops::{Deref, DerefMut};
use core::pin::Pin;
use core::task::{Context, Poll};
use core::time::Duration;
use futures::Stream;

/// A reusable future or stream based on `Option` that will time out after a specific duration as elapse.
#[pin_project::pin_project]
pub struct TimeoutOptional<T> {
    duration: Duration,
    #[pin]
    task: Optional<Timed<T>>,
}

impl<T> Deref for TimeoutOptional<T> {
    type Target = Optional<Timed<T>>;
    fn deref(&self) -> &Self::Target {
        &self.task
    }
}

impl<T> DerefMut for TimeoutOptional<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.task
    }
}

impl<T> TimeoutOptional<T> {
    /// Construct a new [`TimeoutOptional`].
    pub fn new(duration: Duration) -> Self {
        Self {
            duration,
            task: Optional::default(),
        }
    }

    /// Construct a new [`TimeoutOptional`] with an existing [`Future`] or [`Stream`].
    pub fn new_with_task(duration: Duration, task: T) -> Self {
        Self {
            duration,
            task: Optional::new(Timed::new(task, duration)),
        }
    }

    /// Construct a new [`TimeoutOptional`] with an existing [`Future`].
    pub fn new_with_future(duration: Duration, task: T) -> Self
    where
        T: Future,
    {
        Self {
            duration,
            task: Optional::with_future(Timed::new(task, duration)),
        }
    }

    /// Construct a new [`TimeoutOptional`] with an existing [`Stream`].
    pub fn new_with_stream(duration: Duration, task: T) -> Self
    where
        T: Stream,
    {
        Self {
            duration,
            task: Optional::with_stream(Timed::new(task, duration)),
        }
    }

    /// Replaces the current the future or stream with a new one, returning the previous value if present.
    pub fn replace(&mut self, task: T) -> Option<T> {
        let prev = self.task.replace(Timed::new(task, self.duration));
        prev.map(|item| item.into_inner())
    }

    /// Replaces the current future or stream in place without moving the previous value.
    pub fn set(self: Pin<&mut Self>, task: T) {
        let this = self.project();
        this.task.set(Timed::new(task, *this.duration));
    }
}

impl<T: Future> Future for TimeoutOptional<T> {
    type Output = Result<T::Output, TimedError>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        this.task.poll(cx).map_err(|_| TimedError)
    }
}

impl<T: Stream> Stream for TimeoutOptional<T> {
    type Item = Result<T::Item, TimedError>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        this.task.poll_next(cx).map_err(|_| TimedError)
    }
}

#[cfg(test)]
mod test {
    use crate::optional::timeout::TimeoutOptional;
    use core::future::pending;
    use core::pin::Pin;
    use core::time::Duration;
    use futures::future::ready;

    #[test]
    fn test_timeout_optional_ready() {
        let mut task = TimeoutOptional::new_with_task(Duration::from_secs(1), ready(()));
        futures::executor::block_on(async move {
            let fut = Pin::new(&mut task);
            match fut.await {
                Ok(_) => assert!(task.is_none()),
                Err(e) => panic!("unexpected error: {e}"),
            }
        })
    }

    #[test]
    fn test_timeout_optional_timeout() {
        let mut task = TimeoutOptional::new_with_task(Duration::from_millis(10), pending::<()>());

        futures::executor::block_on(async move {
            let fut = Pin::new(&mut task);
            match fut.await {
                Ok(_) => unreachable!("should time out"),
                Err(_) => {
                    assert!(task.is_none());
                }
            }
        })
    }

    #[test]
    fn reusable_pinned_timeout_optional_future() {
        async fn value(value: i32) -> i32 {
            value
        }

        let task = TimeoutOptional::new_with_future(Duration::from_secs(1), value(0));
        futures::pin_mut!(task);

        futures::executor::block_on(async {
            assert_eq!(task.as_mut().await.expect("future should not time out"), 0);
            assert!(task.is_none());

            task.as_mut().set(value(1));
            assert!(task.is_some());

            assert_eq!(task.as_mut().await.expect("future should not time out"), 1);
            assert!(task.is_none());
        });
    }
}
