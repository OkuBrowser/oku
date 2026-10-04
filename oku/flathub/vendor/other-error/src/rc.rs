use core::ops::Deref;
use alloc::rc::Rc;

/// Reference-counted error for sharing within a single thread.
pub struct RcError<E> {
    inner_error: Rc<E>,
}

impl<E> Clone for RcError<E> {
    fn clone(&self) -> Self {
        Self {
            inner_error: self.inner_error.clone(),
        }
    }
}

impl<E> Deref for RcError<E> {
    type Target = E;
    fn deref(&self) -> &E {
        &self.inner_error
    }
}

impl<E: PartialEq> PartialEq for RcError<E> {
    fn eq(&self, other: &Self) -> bool {
        self.inner_error == other.inner_error
    }
}

impl<E: Eq> Eq for RcError<E> {}

impl<E> AsRef<E> for RcError<E> {
    fn as_ref(&self) -> &E {
        &self.inner_error
    }
}

impl<E> From<E> for RcError<E> {
    fn from(inner_error: E) -> Self {
        Self::new(inner_error)
    }
}

impl<E> RcError<E> {
    pub fn new(inner_error: E) -> Self {
        Self {
            inner_error: Rc::new(inner_error),
        }
    }
}

impl<E: core::fmt::Debug> core::fmt::Debug for RcError<E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(&self.inner_error, f)
    }
}

impl<E: core::fmt::Display> core::fmt::Display for RcError<E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(&self.inner_error, f)
    }
}

impl<E: core::error::Error> core::error::Error for RcError<E> {
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
        let err = RcError::new(MyError);
        assert_eq!(format!("{}", *err), "my error");
    }

    #[test]
    fn from_and_display() {
        let err: RcError<MyError> = MyError.into();
        assert_eq!(format!("{}", err), "my error");
        assert_eq!(format!("{:?}", err), "MyError");
    }

    #[test]
    fn clone_shares_allocation() {
        let err = RcError::new(MyError);
        let cloned = err.clone();
        assert!(Rc::ptr_eq(&err.inner_error, &cloned.inner_error));
    }
}