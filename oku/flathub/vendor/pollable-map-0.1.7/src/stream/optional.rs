use crate::optional::Optional;

/// A reusable stream that is the equivalent to an `Option`.
///
/// By default, this future will be empty, which would return  [`Poll::Pending`] when polled,
/// but if a [`Stream`] is supplied either upon construction via [`OptionalStream::new`] or
/// is set via [`OptionalStream::replace`], the stream could later polled once [`OptionalStream`]
/// is polled. Once the stream is polled to completion, [`OptionalStream`] will be empty.
#[deprecated(note = "Use Optional instead")]
pub type OptionalStream<S> = Optional<S>;
