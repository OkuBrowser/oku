use crate::optional::Optional;

/// A reusable future that is the equivalent to an `Option`.
///
/// By default, this future will be empty, which would return  [`Poll::Pending`] when polled,
/// but if a [`Future`] is supplied either upon construction via [`OptionalFuture::new`] or
/// is set via [`OptionalFuture::replace`], the future would then be polled once [`OptionalFuture`]
/// is polled. Once the future is polled to completion, the results will be returned, with
/// [`OptionalFuture`] being empty.
#[deprecated(note = "Use Optional instead")]
pub type OptionalFuture<F> = Optional<F>;
