use std::fmt;

use crate::{AnyError, StackError, StackErrorExt, meta, stack_error};

/// Provides extension methods to add context to [`StackError`]s.
pub trait StackResultExt<T, E> {
    /// Converts the result's error value to [`AnyError`] while providing additional context.
    #[track_caller]
    fn context(self, context: impl fmt::Display) -> Result<T, AnyError>;

    /// Converts the result's error value to [`AnyError`] while providing lazily-evaluated additional context.
    ///
    /// The `context` closure is only invoked if an error occurs.
    #[track_caller]
    fn with_context<F, C>(self, context: F) -> Result<T, AnyError>
    where
        F: FnOnce(&E) -> C,
        C: fmt::Display;
}

impl<T, E: StackError + 'static> StackResultExt<T, E> for Result<T, E> {
    fn context(self, context: impl fmt::Display) -> Result<T, AnyError> {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(e.into_any().context(context)),
        }
    }

    fn with_context<F, C>(self, context: F) -> Result<T, AnyError>
    where
        F: FnOnce(&E) -> C,
        C: fmt::Display,
    {
        match self {
            Ok(v) => Ok(v),
            Err(e) => {
                let context = context(&e);
                Err(e.into_any().context(context))
            }
        }
    }
}

/// Provides extension methods to add context to std errors.
///
/// You should only call these methods on results that contain errors which do not implement [`StackError`].
/// For results with `StackError`s, instead use the methods from [`StackResultExt`]. The latter will
/// preserve call-site metadata, whereas using the methods from the [`StdResultExt`] will lose the
/// call-site metadata when called on a `StackError` result.
pub trait StdResultExt<T, E> {
    /// Converts the result's error value to [`AnyError`] while providing additional context.
    #[track_caller]
    fn std_context(self, context: impl fmt::Display) -> Result<T, AnyError>;

    /// Converts the result's error value to [`AnyError`] while providing lazily-evaluated additional context.
    ///
    /// The `context` closure is only invoked if an error occurs.
    #[track_caller]
    fn with_std_context<F, C>(self, context: F) -> Result<T, AnyError>
    where
        F: FnOnce(&E) -> C,
        C: fmt::Display;

    /// Converts the result's error into [`AnyError`].
    ///
    /// You should make sure to only call this on results that contain an error which does *not*
    /// implement [`StackError`]. If it *does* implement [`StackError`] you can simply convert
    /// the result's error to [`AnyError`] with the `?` operator. If you use `anyerr` on
    /// `StackError`s, you will lose access to the error's call-site metadata.
    #[track_caller]
    fn anyerr(self) -> Result<T, AnyError>;
}

impl<T, E: std::error::Error + Send + Sync + 'static> StdResultExt<T, E> for Result<T, E> {
    fn std_context(self, context: impl fmt::Display) -> Result<T, AnyError> {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(AnyError::from_std(e).context(context)),
        }
    }

    fn with_std_context<F, C>(self, context: F) -> Result<T, AnyError>
    where
        F: FnOnce(&E) -> C,
        C: fmt::Display,
    {
        match self {
            Ok(v) => Ok(v),
            Err(e) => {
                let context = context(&e);
                Err(AnyError::from_std(e).context(context))
            }
        }
    }

    fn anyerr(self) -> Result<T, AnyError> {
        match self {
            Ok(v) => Ok(v),
            Err(err) => Err(AnyError::from_std(err)),
        }
    }
}

impl<T> StdResultExt<T, NoneError> for Option<T> {
    fn std_context(self, context: impl fmt::Display) -> Result<T, AnyError> {
        match self {
            Some(v) => Ok(v),
            None => Err(NoneError { meta: meta() }.into_any().context(context)),
        }
    }

    fn with_std_context<F, C>(self, context: F) -> Result<T, AnyError>
    where
        F: FnOnce(&NoneError) -> C,
        C: fmt::Display,
    {
        match self {
            Some(v) => Ok(v),
            None => {
                let err = NoneError { meta: meta() };
                let context = context(&err);
                Err(err.into_any().context(context))
            }
        }
    }

    fn anyerr(self) -> Result<T, AnyError> {
        match self {
            Some(v) => Ok(v),
            None => Err(NoneError { meta: meta() }.into_any()),
        }
    }
}

impl<T> StackResultExt<T, NoneError> for Option<T> {
    fn context(self, context: impl fmt::Display) -> Result<T, AnyError> {
        match self {
            Some(v) => Ok(v),
            None => Err(NoneError { meta: meta() }.into_any().context(context)),
        }
    }

    fn with_context<F, C>(self, context: F) -> Result<T, AnyError>
    where
        F: FnOnce(&NoneError) -> C,
        C: fmt::Display,
    {
        match self {
            Some(v) => Ok(v),
            None => {
                let err = NoneError { meta: meta() };
                let context = context(&err);
                Err(err.into_any().context(context))
            }
        }
    }
}

/// Error returned when converting [`Option`]s to an error.
#[stack_error(derive, add_meta)]
#[error("Expected some, found none")]
pub struct NoneError {}

/// A simple string error, providing a message and optionally a source.
#[stack_error(derive, add_meta)]
pub(crate) enum FromString {
    #[error("{message}")]
    WithSource { message: String, source: AnyError },
    #[error("{message}")]
    WithoutSource { message: String },
}
