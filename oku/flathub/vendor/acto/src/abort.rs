use smol_str::SmolStr;
use std::{
    any::Any,
    fmt::{self, Debug, Display},
};

pub enum PanicOrAbort {
    Panic(Box<dyn PanicInfo>),
    Abort(ActoAborted),
}

impl Display for PanicOrAbort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PanicOrAbort::Panic(p) => write!(f, "Actor panicked: {}", p.cause()),
            PanicOrAbort::Abort(_) => write!(f, "Actor aborted via Acto"),
        }
    }
}

impl Debug for PanicOrAbort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Panic(arg0) => f.debug_tuple("Panic").field(arg0).finish(),
            Self::Abort(arg0) => Debug::fmt(arg0, f),
        }
    }
}

pub trait PanicInfo: Debug + Send + 'static {
    fn payload(&self) -> Option<&(dyn Any + Send + 'static)>;
    fn is_cancelled(&self) -> bool;
    fn cause(&self) -> String;
}

/// This error is returned when an actor has been aborted.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ActoAborted(SmolStr);

impl ActoAborted {
    pub fn new(name: impl AsRef<str>) -> Self {
        Self(name.into())
    }
}

impl std::error::Error for ActoAborted {}

impl fmt::Display for ActoAborted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Actor aborted: {}", self.0)
    }
}
