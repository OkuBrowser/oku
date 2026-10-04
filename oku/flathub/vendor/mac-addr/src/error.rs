use core::fmt;

/// Error returned when parsing a MAC address with [`core::str::FromStr`].
#[derive(Copy, Debug, PartialEq, Eq, Clone)]
pub enum ParseMacAddrError {
    /// Input contained more than 6 components.
    TooManyComponents,
    /// Input contained fewer than 6 components.
    TooFewComponents,
    /// A component was invalid hex or empty.
    InvalidComponent,
}

impl fmt::Display for ParseMacAddrError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match *self {
            ParseMacAddrError::TooManyComponents => "Too many components in a MAC address string",
            ParseMacAddrError::TooFewComponents => "Too few components in a MAC address string",
            ParseMacAddrError::InvalidComponent => "Invalid component in a MAC address string",
        };
        f.write_str(s)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ParseMacAddrError {}
