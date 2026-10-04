use std::{
    fmt::{self, Formatter},
    sync::Arc,
};

use crate::{AnyError, Location, Meta, backtrace_enabled};

/// Output style for rendering error sources in a [`Report`].
#[derive(Debug, Copy, Clone)]
pub enum SourceFormat {
    /// Renders sources inline on a single line.
    OneLine,
    /// Renders sources on multiple lines
    MultiLine {
        /// If `true` include call-site info for each source.
        location: bool,
    },
}

/// Trait implemented by errors produced by this crate.
///
/// It extends `std::error::Error` semantics with optional error metadata,
/// and a `source` method where sources may *also* provide metadata.
pub trait StackError: fmt::Display + fmt::Debug + Send + Sync {
    /// Returns this error as a std error reference.
    fn as_std(&self) -> &(dyn std::error::Error + Send + Sync + 'static);

    /// Returns this error as a std error.
    fn into_std(self: Box<Self>) -> Box<dyn std::error::Error + Send + Sync>;

    /// Returns this error as a `dyn StackError`.
    fn as_dyn(&self) -> &dyn StackError;

    /// Returns metadata captured at creation time, if available.
    fn meta(&self) -> Option<&Meta>;

    /// Returns the next source in the chain, if any.
    fn source(&self) -> Option<ErrorRef<'_>>;

    /// Returns the next source in the chain, if any.
    fn fmt_message(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;

    /// Returns whether this error is transparent and should be skipped in reports.
    fn is_transparent(&self) -> bool;

    /// Returns this error as an [`ErrorRef`].
    ///
    /// See [`ErrorRef`] for details.
    fn as_ref(&self) -> ErrorRef<'_> {
        ErrorRef::Stack(self.as_dyn())
    }

    /// Returns an iterator over this error followed by its sources.
    fn stack(&self) -> Chain<'_> {
        Chain::stack(self.as_ref())
    }

    /// Returns an iterator over sources of this error (skipping `self`).
    fn sources(&self) -> Chain<'_> {
        Chain::sources(self.as_ref())
    }

    /// Returns a [`Report`] to output the error with configurable formatting.
    fn report(&self) -> Report<'_> {
        Report::new(self.as_dyn())
    }
}

/// Extension methods for [`StackError`]s that are [`Sized`].
pub trait StackErrorExt: StackError + Sized + 'static {
    /// Converts the error into [`AnyError`].
    ///
    /// This preserves the error's location metadata.
    #[track_caller]
    fn into_any(self) -> AnyError {
        AnyError::from_stack(self)
    }

    /// Converts the error into [`AnyError`], and adds additional error context.
    #[track_caller]
    fn context(self, context: impl fmt::Display) -> AnyError {
        self.into_any().context(context)
    }
}

impl<T: StackError + Sized + 'static> StackErrorExt for T {}

/// Reference to an error which can either be a std error or a stack error.
///
/// This provides a unified interface to either a std or a stack error. If it's a
/// stack error it allows to access the error metadata captured at the call site.
#[derive(Copy, Clone, Debug)]
pub enum ErrorRef<'a> {
    /// Std error (no location info).
    Std(&'a (dyn std::error::Error + 'static), Option<&'a Meta>),
    /// StackError (has location info).
    Stack(&'a dyn StackError),
}

impl<'a> ErrorRef<'a> {
    /// Creates a [`ErrorRef`] for a std error.
    pub fn std(err: &'a (dyn std::error::Error + 'static)) -> ErrorRef<'a> {
        ErrorRef::Std(err, None)
    }

    pub(crate) fn std_with_meta(
        err: &'a (dyn std::error::Error + 'static),
        meta: &'a Meta,
    ) -> ErrorRef<'a> {
        ErrorRef::Std(err, Some(meta))
    }

    /// Creates a [`ErrorRef`] for a `StackError`.
    pub fn stack(err: &dyn StackError) -> ErrorRef<'_> {
        ErrorRef::Stack(err)
    }

    /// Returns `true` if this error is transparent (i.e. directly forwards to its source).
    pub fn is_transparent(&self) -> bool {
        match self {
            ErrorRef::Std(_, _) => false,
            ErrorRef::Stack(error) => error.is_transparent(),
        }
    }

    /// Returns the error as a std error.
    pub fn as_std(self) -> &'a dyn std::error::Error {
        match self {
            ErrorRef::Std(error, _) => error,
            ErrorRef::Stack(error) => error.as_std(),
        }
    }

    /// Returns the next source in the source chain as a [`ErrorRef`].
    pub fn source(self) -> Option<ErrorRef<'a>> {
        match self {
            Self::Std(error, _) => error.source().map(ErrorRef::std),
            Self::Stack(error) => StackError::source(error),
        }
    }

    /// Returns the location where this error was created, if available.
    pub fn meta(&self) -> Option<&Meta> {
        match self {
            ErrorRef::Std(_, meta) => *meta,
            ErrorRef::Stack(error) => error.meta(),
        }
    }

    /// Formats the captured location, if present.
    pub(crate) fn fmt_location(&self, f: &mut Formatter) -> fmt::Result {
        fmt_location(self.meta().and_then(|m| m.location()), f)
    }

    /// Formats the error message.
    pub fn fmt_message(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ErrorRef::Std(error, _) => fmt::Display::fmt(error, f),
            ErrorRef::Stack(error) => error.fmt_message(f),
        }
    }

    /// Downcast this error object by reference.
    pub fn downcast_ref<T: std::error::Error + 'static>(self) -> Option<&'a T> {
        match self {
            ErrorRef::Std(error, _) => error.downcast_ref(),
            ErrorRef::Stack(error) => error.as_std().downcast_ref(),
        }
    }
}

impl<'a> fmt::Display for ErrorRef<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Std(error, _) => write!(f, "{error}"),
            Self::Stack(error) => write!(f, "{error}"),
        }
    }
}

/// A [`Report`] customizes how an error is displayed.
#[derive(Clone, Copy)]
pub struct Report<'a> {
    inner: &'a dyn StackError,
    location: bool,
    sources: Option<SourceFormat>,
}

impl<'a> Report<'a> {
    pub(crate) fn new(inner: &'a dyn StackError) -> Self {
        let location = backtrace_enabled();
        Self {
            inner,
            location,
            sources: None,
        }
    }

    /// Enables all available details.
    pub fn full(mut self) -> Self {
        let location = backtrace_enabled();
        self.location = location;
        self.sources = Some(SourceFormat::MultiLine { location });
        self
    }

    /// Sets whether location info is printed.
    ///
    /// Note that location info is only captured if environment variable
    /// `RUST_BACKTRACE` is set to `1` or `full`.
    pub fn location(mut self, value: bool) -> Self {
        self.location = value;
        self
    }

    /// Prints the error's sources.
    pub fn sources(mut self, format: Option<SourceFormat>) -> Self {
        self.sources = format;
        self
    }
}

impl<'a> fmt::Display for Report<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.sources {
            None => {
                let item = self
                    .inner
                    .stack()
                    .find(|item| !item.is_transparent())
                    .unwrap_or_else(|| self.inner.as_ref());
                item.fmt_message(f)?;
            }
            Some(format) => {
                let mut stack = self
                    .inner
                    .stack()
                    .filter(|s| self.location || !s.is_transparent())
                    .peekable();
                let mut is_first = true;
                while let Some(item) = stack.next() {
                    match format {
                        SourceFormat::OneLine => {
                            if !is_first {
                                write!(f, ": ")?;
                            }
                            item.fmt_message(f)?;
                        }
                        SourceFormat::MultiLine { location } => {
                            if !is_first {
                                write!(f, "    ")?;
                            }
                            item.fmt_message(f)?;
                            if location {
                                item.fmt_location(f)?;
                            }
                            if stack.peek().is_some() {
                                writeln!(f)?;
                                if is_first {
                                    writeln!(f, "Caused by:")?;
                                }
                            }
                        }
                    }
                    is_first = false;
                }
            }
        }
        Ok(())
    }
}

fn fmt_location(location: Option<&Location>, f: &mut Formatter) -> fmt::Result {
    if let Some(location) = location {
        write!(f, " ({})", location)?;
    }
    Ok(())
}

/// Iterator over the sources of an error.
pub struct Chain<'a> {
    item: Option<ErrorRef<'a>>,
    skip: bool,
}

impl<'a> Chain<'a> {
    /// Creates a chain over `self` and its sources.
    fn stack(item: ErrorRef<'a>) -> Self {
        Self {
            item: Some(item),
            skip: false,
        }
    }

    /// Creates a chain over only the sources, skipping `self`.
    fn sources(item: ErrorRef<'a>) -> Self {
        Self {
            item: Some(item),
            skip: true,
        }
    }
}

impl<'a> Iterator for Chain<'a> {
    type Item = ErrorRef<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let item = self.item?;
            self.item = item.source();
            if self.skip {
                self.skip = false;
            } else {
                return Some(item);
            }
        }
    }
}

macro_rules! impl_stack_error_for_std_error {
    ($ty:ty) => {
        impl StackError for $ty {
            fn as_std(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
                self
            }

            fn into_std(self: Box<Self>) -> Box<dyn std::error::Error + Send + Sync> {
                self
            }

            fn as_dyn(&self) -> &dyn StackError {
                self
            }

            fn meta(&self) -> Option<&Meta> {
                None
            }

            fn source(&self) -> Option<ErrorRef<'_>> {
                std::error::Error::source(self).map(ErrorRef::std)
            }

            fn fmt_message(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(self, f)
            }

            fn is_transparent(&self) -> bool {
                false
            }
        }
    };
}

impl_stack_error_for_std_error!(std::io::Error);
impl_stack_error_for_std_error!(std::fmt::Error);
impl_stack_error_for_std_error!(std::str::Utf8Error);
impl_stack_error_for_std_error!(std::string::FromUtf8Error);
impl_stack_error_for_std_error!(std::net::AddrParseError);
impl_stack_error_for_std_error!(std::array::TryFromSliceError);

#[cfg(feature = "anyhow")]
impl StackError for anyhow::Error {
    fn as_std(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
        std::convert::AsRef::as_ref(self)
    }

    fn into_std(self: Box<Self>) -> Box<dyn std::error::Error + Send + Sync> {
        self.into_boxed_dyn_error()
    }

    fn as_dyn(&self) -> &dyn StackError {
        self
    }

    fn meta(&self) -> Option<&Meta> {
        None
    }

    fn source(&self) -> Option<ErrorRef<'_>> {
        anyhow::Error::chain(self).next().map(ErrorRef::std)
    }

    fn fmt_message(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }

    fn is_transparent(&self) -> bool {
        false
    }
}

impl<T: StackError + std::error::Error + Sized + 'static> StackError for Arc<T> {
    fn as_std(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
        (**self).as_std()
    }

    fn into_std(self: Box<Self>) -> Box<dyn std::error::Error + Send + Sync> {
        self
    }

    fn as_dyn(&self) -> &dyn StackError {
        (**self).as_dyn()
    }

    fn meta(&self) -> Option<&Meta> {
        (**self).meta()
    }

    fn source(&self) -> Option<ErrorRef<'_>> {
        StackError::source(&**self)
    }

    fn fmt_message(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt_message(f)
    }

    fn is_transparent(&self) -> bool {
        (**self).is_transparent()
    }
}
