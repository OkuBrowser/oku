//! This module contains the definition of the `Event` trait and its implementations.
mod database;
mod store;

use crate::Error;

pub use self::{
    database::{
        DatabaseEvent, DeleteDatabaseRequestEvent, OpenDatabaseRequestEvent, VersionChangeEvent,
    },
    store::{
        AddStoreRequestEvent, ClearStoreRequestEvent, CountStoreRequestEvent,
        DeleteStoreRequestEvent, GetAllKeysStoreRequestEvent, GetAllStoreRequestEvent,
        GetKeyStoreRequestEvent, GetStoreRequestEvent, OpenCursorStoreRequestEvent,
        OpenKeyCursorStoreRequestEvent, PutStoreRequestEvent, StoreEvent, UpdateStoreRequestEvent,
    },
};

/// Trait implemented by all events.
pub trait Event {
    /// The type of the target of the event.
    type Target;

    /// Returns the target of the event.
    fn target(&self) -> Result<Self::Target, Error>;
}

impl Event for web_sys::Event {
    type Target = web_sys::EventTarget;

    fn target(&self) -> Result<Self::Target, Error> {
        self.target().ok_or(Error::EventTargetNotFound)
    }
}
