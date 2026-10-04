#[macro_use]
mod macros;

mod version_change;

use crate::{Database, Error, Event};

pub use self::version_change::VersionChangeEvent;

/// Trait for defining events triggered when a database is opened or deleted.
pub trait DatabaseEvent: Event {
    /// Returns the database that triggered the event.
    fn database(&self) -> Result<Database, Error>;
}

impl DatabaseEvent for web_sys::Event {
    fn database(&self) -> Result<Database, Error> {
        let target = self.target().ok_or_else(|| Error::EventTargetNotFound)?;
        Database::try_from(target)
    }
}

impl_database_event!(
    OpenDatabaseRequestEvent,
    web_sys::Event,
    crate::request::OpenDatabaseRequest,
    "Event for [`OpenDatabaseRequest`](crate::request::OpenDatabaseRequest) handlers."
);
impl_database_event!(
    DeleteDatabaseRequestEvent,
    web_sys::Event,
    crate::request::DeleteDatabaseRequest,
    "Event for [`DeleteDatabaseRequest`](crate::request::DeleteDatabaseRequest) handlers."
);
