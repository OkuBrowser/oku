//! Re-exports of abstractions and implementations deemed useful and good by number 0 engineers.
//!
//! Read up more on our challenges with rust's async: <https://www.iroh.computer/blog/async-rust-challenges-in-iroh>
//!
//! This library also allows importing a single [`task`] and [`time`] module that'll work
//! in `wasm*-*-unknown` targets, using `wasm_bindgen` and `wasm_bindgen_futures`, but mirroring
//! the `tokio` API with only minor differences.
//!
//! ## Feature flags
//!
//! * `serde`: Enables serde support for the [`time::SystemTime`] type when building for WebAssembly.

#![deny(missing_docs, rustdoc::broken_intra_doc_links)]
#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(n0_future_docsrs, feature(doc_auto_cfg))]

mod maybe_future;

pub mod task;
pub mod time;

// futures-* re-exports

pub use futures_buffered::*;
pub use futures_lite::{io, pin, ready, stream, Future, FutureExt, Stream, StreamExt};
pub use futures_util::{future::Either, Sink, SinkExt, TryFutureExt, TryStreamExt};
pub use maybe_future::MaybeFuture;

/// Implementation and types for splitting a `Stream + Sink`.
/// See [`split::split`].
pub mod split {
    pub use futures_util::stream::{SplitSink, SplitStream};

    use crate::{Sink, Stream};

    /// Splits a `Stream + Sink` object into separate `Sink` and `Stream`
    /// objects.
    ///
    /// This can be useful when you want to split ownership between tasks, or
    /// allow direct interaction between the two objects (e.g. via
    /// `Sink::send_all`).
    pub fn split<S, SinkItem>(stream_sink: S) -> (SplitSink<S, SinkItem>, SplitStream<S>)
    where
        S: Stream + Sized + Sink<SinkItem>,
    {
        use futures_util::stream::StreamExt as _;
        stream_sink.split()
    }
}

/// Re-exports boxed versions of [`Future`] and [`Stream`] traits
/// that are `Send` in non-wasm and `!Send` in wasm.
///
/// If you don't want this type of target-dependend `Send` and `!Send`,
/// use [`stream::Boxed`]/[`stream::BoxedLocal`] and
/// [`future::Boxed`]/[`future::BoxedLocal`].
///
/// [`Future`]: futures_lite::Future
/// [`Stream`]: futures_lite::Stream
/// [`stream::Boxed`]: crate::stream::Boxed
/// [`stream::BoxedLocal`]: crate::stream::BoxedLocal
/// [`future::Boxed`]: crate::future::Boxed
/// [`future::BoxedLocal`]: crate::future::BoxedLocal
pub mod boxed {
    #[cfg(not(wasm_browser))]
    pub use futures_lite::future::Boxed as BoxFuture;
    #[cfg(wasm_browser)]
    pub use futures_lite::future::BoxedLocal as BoxFuture;
    #[cfg(not(wasm_browser))]
    pub use futures_lite::stream::Boxed as BoxStream;
    #[cfg(wasm_browser)]
    pub use futures_lite::stream::BoxedLocal as BoxStream;
}

/// Combinators for the [`Future`] trait.
pub mod future {
    use std::task::Poll;

    pub use futures_lite::future::*;

    use super::pin;

    /// Poll a future once and return the output if ready.
    ///
    /// Evaluates and consumes the future, returning the resulting output if the future is
    /// ready after the first call to [`Future::poll`].
    ///
    /// If poll instead returns [`Poll::Pending`], `None` is returned.
    ///
    /// This method is useful in cases where immediacy is more important than waiting for a
    /// result. It is also convenient for quickly obtaining the value of a future that is
    /// known to always resolve immediately.
    pub fn now_or_never<T, F: Future<Output = T>>(fut: F) -> Option<T> {
        pin!(fut);
        let waker = std::task::Waker::noop();
        let mut cx = std::task::Context::from_waker(waker);
        match fut.poll(&mut cx) {
            Poll::Ready(res) => Some(res),
            Poll::Pending => None,
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_now_or_never_smoke() {
            let fut = std::future::ready(0);
            assert_eq!(now_or_never(fut), Some(0));

            let fut = std::future::pending::<isize>();
            assert_eq!(now_or_never(fut), None);
        }
    }
}
