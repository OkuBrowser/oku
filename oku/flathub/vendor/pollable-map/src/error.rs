#[derive(Debug, PartialEq, Eq)]
pub struct TimedError;

impl core::fmt::Display for TimedError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Operation timed out")
    }
}

impl core::error::Error for TimedError {}
