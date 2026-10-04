
use either::Either;
use core::error::Error;
use core::fmt::{Debug, Display};
use core::ops::Deref;

/// Error that is one of two possible error types.
pub struct EitherError<L, R> {
    inner: Either<L, R>,
}

impl<L, R> AsRef<Either<L, R>> for EitherError<L, R> {
    fn as_ref(&self) -> &Either<L, R> {
        &self.inner
    }
}

impl<L, R> Deref for EitherError<L, R> {
    type Target = Either<L, R>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }   
}

impl<L, R> From<Either<L, R>> for EitherError<L, R> {
    fn from(inner: Either<L, R>) -> Self {
        Self::new(inner)
    }
}

impl<L, R> PartialEq for EitherError<L, R> where L: PartialEq, R: PartialEq {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<L, R> Eq for EitherError<L, R> where L: Eq, R: Eq {}

impl<L, R> EitherError<L, R> {
    pub fn new(inner: Either<L, R>) -> Self {
        Self { inner }
    }
    
    pub fn into_inner(self) -> Either<L, R> {
        self.inner
    }
}

impl<L: Debug, R: Debug> Debug for EitherError<L, R> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match &self.inner {
            Either::Left(err) => err.fmt(f),
            Either::Right(err) => err.fmt(f),
        }
    }
}

impl<L: Display, R: Display> Display for EitherError<L, R> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match &self.inner {
            Either::Left(err) => err.fmt(f),
            Either::Right(err) => err.fmt(f),
        }
    }
}

impl<L, R> Error for EitherError<L, R> where L: Error, R: Error {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.inner {
            Either::Left(err) => err.source(),
            Either::Right(err) => err.source(),
        }
    }
}

#[cfg(all(test, feature = "alloc"))]
mod tests {
    extern crate alloc;
    use super::*;
    use alloc::format;

    #[derive(Debug)]
    struct LeftErr;
    #[derive(Debug)]
    struct RightErr;

    impl Display for LeftErr {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.write_str("left")
        }
    }

    impl Display for RightErr {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.write_str("right")
        }
    }

    #[test]
    fn left_formats_left() {
        let err: EitherError<LeftErr, RightErr> = Either::Left(LeftErr).into();
        assert_eq!(format!("{}", err), "left");
        assert_eq!(format!("{:?}", err), "LeftErr");
    }

    #[test]
    fn right_formats_right() {
        let err: EitherError<LeftErr, RightErr> = Either::Right(RightErr).into();
        assert_eq!(format!("{}", err), "right");
        assert_eq!(format!("{:?}", err), "RightErr");
    }

    #[test]
    fn into_inner_round_trips() {
        let err = EitherError::<LeftErr, RightErr>::new(Either::Left(LeftErr));
        assert!(matches!(err.into_inner(), Either::Left(LeftErr)));
    }
}