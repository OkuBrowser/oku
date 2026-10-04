use core::ops::Deref;
use alloc::sync::Arc;

/// Error that is sharable between threads
pub struct ArcError<E> {
    inner_error: Arc<E>,
}

impl<E> Clone for ArcError<E> {
    fn clone(&self) -> Self {
        Self {
            inner_error: self.inner_error.clone(),
        }
    }
}

impl<E> Deref for ArcError<E> {
    type Target = E;
    fn deref(&self) -> &E {
        &self.inner_error
    }
}

impl<E: PartialEq> PartialEq for ArcError<E> {
    fn eq(&self, other: &Self) -> bool {
        self.inner_error == other.inner_error
    }
}

impl<E: Eq> Eq for ArcError<E> {}

impl<E> AsRef<E> for ArcError<E> {
    fn as_ref(&self) -> &E {
        &self.inner_error
    }
}

impl<E> From<E> for ArcError<E> {
    fn from(inner_error: E) -> Self {
        Self::new(inner_error)
    }
}

impl<E> ArcError<E> {
    pub fn new(inner_error: E) -> Self {
        Self {
            inner_error: Arc::new(inner_error),
        }
    }
}

impl<E: core::fmt::Debug> core::fmt::Debug for ArcError<E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(&self.inner_error, f)
    }
}

impl<E: core::fmt::Display> core::fmt::Display for ArcError<E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(&self.inner_error, f)
    }
}

impl<E: core::error::Error> core::error::Error for ArcError<E> {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        self.inner_error.source()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;

    #[derive(Debug)]
    struct MyError;

    impl core::fmt::Display for MyError {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.write_str("my error")
        }
    }

    #[test]
    fn new_and_deref() {
        let err = ArcError::new(MyError);
        assert_eq!(format!("{}", *err), "my error");
    }

    #[test]
    fn from_and_display() {
        let err: ArcError<MyError> = MyError.into();
        assert_eq!(format!("{}", err), "my error");
        assert_eq!(format!("{:?}", err), "MyError");
    }

    #[test]
    fn clone_shares_allocation() {
        let err = ArcError::new(MyError);
        let cloned = err.clone();
        assert!(Arc::ptr_eq(&err.inner_error, &cloned.inner_error));
    }
}