#![doc = include_str!("../README.md")]

// TODO: once backtrace lands stable, consider trying to serialize the backtrace too? not sure it
// makes sense though.

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct Error {
    description: String,
    source: Option<Box<Error>>,
}

impl Error {
    pub fn new<T>(e: &T) -> Error
    where
        T: ?Sized + std::error::Error,
    {
        Error {
            description: e.to_string(),
            source: e.source().map(|s| Box::new(Error::new(s))),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn 'static + std::error::Error)> {
        self.source.as_ref().map(|s| &**s as &(dyn 'static + std::error::Error))
    }

    fn description(&self) -> &str {
        &self.description
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.description)
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.description)
    }
}
