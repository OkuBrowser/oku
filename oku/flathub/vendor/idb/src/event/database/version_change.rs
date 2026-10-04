use num_traits::ToPrimitive;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::IdbVersionChangeEvent;

use crate::{request::OpenDatabaseRequest, Database, DatabaseEvent, Error, Event};

/// Event triggered when the database version changes, as the result of an [`OpenDatabaseRequest::on_upgrade_needed`](crate::request::OpenDatabaseRequest::on_upgrade_needed) event
/// handler function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionChangeEvent {
    inner: IdbVersionChangeEvent,
}

impl VersionChangeEvent {
    /// Returns previous version of database.
    pub fn old_version(&self) -> Result<u32, Error> {
        self.inner
            .old_version()
            .to_u32()
            .ok_or(Error::NumberConversionError)
    }

    /// Returns new version of database.
    pub fn new_version(&self) -> Result<Option<u32>, Error> {
        self.inner
            .new_version()
            .map(|new| new.to_u32().ok_or(Error::NumberConversionError))
            .transpose()
    }
}

impl Event for VersionChangeEvent {
    type Target = OpenDatabaseRequest;

    fn target(&self) -> Result<Self::Target, Error> {
        let target = self.inner.target().ok_or(Error::EventTargetNotFound)?;
        OpenDatabaseRequest::try_from(target)
    }
}

impl DatabaseEvent for VersionChangeEvent {
    fn database(&self) -> Result<Database, Error> {
        let target = self.target()?;
        target.database()
    }
}

impl From<IdbVersionChangeEvent> for VersionChangeEvent {
    fn from(inner: IdbVersionChangeEvent) -> Self {
        Self { inner }
    }
}

impl From<VersionChangeEvent> for IdbVersionChangeEvent {
    fn from(event: VersionChangeEvent) -> Self {
        event.inner
    }
}

impl TryFrom<JsValue> for VersionChangeEvent {
    type Error = Error;

    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        value
            .dyn_into::<IdbVersionChangeEvent>()
            .map(Into::into)
            .map_err(|value| Error::UnexpectedJsType("IdbVersionChangeEvent", value))
    }
}

impl From<VersionChangeEvent> for JsValue {
    fn from(value: VersionChangeEvent) -> Self {
        value.inner.into()
    }
}
