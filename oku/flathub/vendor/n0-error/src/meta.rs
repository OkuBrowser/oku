use std::{fmt, sync::OnceLock};

/// Wrapper around `std::panic::Location` used for display in reports.
#[derive(Clone, Copy)]
pub struct Location(&'static std::panic::Location<'static>);

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.0, f)
    }
}

impl fmt::Debug for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.0, f)
    }
}

/// Captured metadata for an error creation site.
///
/// Currently this only contains the call-site [`Location`].
#[derive(Clone)]
pub struct Meta {
    location: Option<Location>,
}

impl fmt::Display for Meta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(location) = self.location.as_ref() {
            write!(f, "{location}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for Meta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Meta")?;
        if let Some(location) = self.location.as_ref() {
            write!(f, "({location})")?;
        }
        Ok(())
    }
}

/// Creates new [`Meta`] capturing the caller location.
#[track_caller]
pub fn meta() -> Meta {
    Meta::default()
}

impl Default for Meta {
    #[track_caller]
    fn default() -> Self {
        Self {
            location: location(),
        }
    }
}

impl Meta {
    #[track_caller]
    /// Creates new [`Meta`] capturing the caller location.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the captured call-site location.
    pub fn location(&self) -> Option<&Location> {
        self.location.as_ref()
    }
}

#[cfg(test)]
static BACKTRACE_ENABLED: OnceLock<std::sync::RwLock<bool>> = OnceLock::new();

#[cfg(not(test))]
static BACKTRACE_ENABLED: OnceLock<bool> = OnceLock::new();

#[doc(hidden)]
pub fn backtrace_enabled() -> bool {
    let from_env = || {
        matches!(
            std::env::var("RUST_BACKTRACE").as_deref(),
            Ok("1") | Ok("full")
        ) || matches!(std::env::var("RUST_ERROR_LOCATION").as_deref(), Ok("1"))
    };
    #[cfg(test)]
    return *(BACKTRACE_ENABLED
        .get_or_init(|| std::sync::RwLock::new(from_env()))
        .read()
        .unwrap());

    #[cfg(not(test))]
    return *(BACKTRACE_ENABLED.get_or_init(from_env));
}

#[doc(hidden)]
#[cfg(test)]
pub fn set_backtrace_enabled(value: bool) {
    let mut inner = BACKTRACE_ENABLED
        .get_or_init(Default::default)
        .write()
        .unwrap();
    *inner = value;
}

#[track_caller]
fn location() -> Option<Location> {
    if backtrace_enabled() {
        Some(Location(std::panic::Location::caller()))
    } else {
        None
    }
}
