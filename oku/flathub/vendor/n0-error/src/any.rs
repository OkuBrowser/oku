use std::{
    convert::Infallible,
    fmt::{self, Formatter},
    ops::Deref,
};

use crate::{ErrorRef, FromString, Meta, SourceFormat, StackError, StackErrorExt};

/// Type-erased error that can wrap a [`StackError`] or any [`std::error::Error`].
///
/// [`StackError`]s have a blanket impl to convert into [`AnyError`], while preserving
/// their call-site location info.
///
/// Errors that implement [`std::error::Error`] but not [`StackError`] can't convert to
/// [`AnyError`] automatically. Use either [`AnyError::from_std`] or [`crate::StdResultExt::anyerr`]
/// to convert std errors into [`AnyError`].
///
/// This is necessary unfortunately because if we had a blanket conversion from std errors to `AnyError`,
/// this blanket conversion would also apply to [`StackError`]s, thus losing their call-site location
/// info in the conversion.
pub struct AnyError(Inner);

enum Inner {
    Stack(Box<dyn StackError>),
    Std(Box<dyn std::error::Error + Send + Sync>, Meta),
}

impl AnyError {
    /// Creates an [`AnyError`] from a std error.
    ///
    /// This captures call-site metadata.
    #[track_caller]
    pub fn from_std(err: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::from_std_box(Box::new(err))
    }

    /// Creates an [`AnyError`] from a [`anyhow::Error`].
    #[track_caller]
    #[cfg(feature = "anyhow")]
    pub fn from_anyhow(err: anyhow::Error) -> Self {
        Self::from_std_box(err.into_boxed_dyn_error())
    }

    /// Creates an [`AnyError`] from a bosed std error.
    ///
    /// This captures call-site metadata.
    #[track_caller]
    pub fn from_std_box(err: Box<dyn std::error::Error + Send + Sync + 'static>) -> Self {
        Self(Inner::Std(err, Meta::default()))
    }

    /// Creates an [`AnyError`] from any `Display` value by formatting it into a message.
    #[track_caller]
    pub fn from_display(s: impl fmt::Display) -> Self {
        Self::from_string(s.to_string())
    }

    /// Creates an [`AnyError`] from a message string.
    #[track_caller]
    pub fn from_string(message: String) -> Self {
        FromString::WithoutSource {
            message,
            meta: Meta::default(),
        }
        .into_any()
    }

    /// Creates an [`AnyError`] from a [`StackError`].
    ///
    /// This preserves the error's call-site metadata.
    ///
    /// Equivalent to `AnyError::from(err)`.
    #[track_caller]
    pub fn from_stack(err: impl StackError + 'static) -> Self {
        Self::from_stack_box(Box::new(err))
    }

    /// Creates an [`AnyError`] from a boxed [`StackError`].
    #[track_caller]
    pub fn from_stack_box(err: Box<dyn StackError>) -> Self {
        Self(Inner::Stack(err))
    }

    /// Adds additional context on top of this error.
    #[track_caller]
    pub fn context(self, context: impl fmt::Display) -> AnyError {
        FromString::WithSource {
            message: context.to_string(),
            source: self,
            meta: Meta::default(),
        }
        .into_any()
    }

    /// Converts into boxed std error.
    pub fn into_boxed_dyn_error(self) -> Box<dyn std::error::Error + Send + Sync + 'static> {
        Box::new(self)
    }

    /// Downcast this error object by reference.
    pub fn downcast_ref<T: std::error::Error + 'static>(&self) -> Option<&T> {
        match &self.0 {
            Inner::Stack(err) => err.as_std().downcast_ref(),
            Inner::Std(err, _) => err.downcast_ref(),
        }
    }

    /// Downcast this error object by reference.
    pub fn downcast<T: std::error::Error + 'static>(self) -> Option<T> {
        match self.0 {
            Inner::Stack(err) => err.into_std().downcast().ok().map(|b| *b),
            Inner::Std(err, _) => err.downcast().ok().map(|b| *b),
        }
    }
}

impl fmt::Display for AnyError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let sources = f.alternate().then_some(SourceFormat::OneLine);
        write!(f, "{}", self.report().sources(sources))
    }
}

impl fmt::Debug for AnyError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            match &self.0 {
                Inner::Stack(error) => {
                    write!(f, "Stack({error:#?})")
                }
                Inner::Std(error, meta) => {
                    write!(f, "Std({error:#?}, {meta:?})")
                }
            }
        } else {
            write!(f, "{}", self.report().full())
        }
    }
}

impl StackError for AnyError {
    fn as_dyn(&self) -> &dyn StackError {
        self
    }
    fn into_std(self: Box<Self>) -> Box<dyn std::error::Error + Send + Sync> {
        self
    }

    fn as_std(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
        match &self.0 {
            Inner::Std(err, _) => err.as_ref(),
            Inner::Stack(err) => err.as_std(),
        }
    }

    fn meta(&self) -> Option<&Meta> {
        match &self.0 {
            Inner::Std(_, meta) => Some(meta),
            Inner::Stack(err) => err.meta(),
        }
    }

    fn source(&self) -> Option<ErrorRef<'_>> {
        self.as_ref().source()
    }

    fn is_transparent(&self) -> bool {
        self.as_ref().is_transparent()
    }

    fn as_ref<'a>(&'a self) -> ErrorRef<'a> {
        match &self.0 {
            Inner::Stack(error) => ErrorRef::Stack(error.deref()),
            Inner::Std(error, meta) => ErrorRef::std_with_meta(error.as_ref(), meta),
        }
    }

    fn fmt_message(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Inner::Stack(error) => error.fmt_message(f),
            Inner::Std(error, _) => fmt::Display::fmt(error, f),
        }
    }
}

impl std::error::Error for AnyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.as_std().source()
    }
}

impl From<String> for AnyError {
    #[track_caller]
    fn from(value: String) -> Self {
        Self::from_display(value)
    }
}

impl From<&str> for AnyError {
    #[track_caller]
    fn from(value: &str) -> Self {
        Self::from_display(value)
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for AnyError {
    #[track_caller]
    fn from(value: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Self::from_std_box(value)
    }
}

#[cfg(feature = "anyhow")]
impl From<anyhow::Error> for AnyError {
    #[track_caller]
    fn from(value: anyhow::Error) -> Self {
        Self::from_anyhow(value)
    }
}

impl From<std::io::Error> for AnyError {
    #[track_caller]
    fn from(value: std::io::Error) -> Self {
        Self::from_std(value)
    }
}

impl std::str::FromStr for AnyError {
    type Err = Infallible;

    #[track_caller]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from_display(s))
    }
}
