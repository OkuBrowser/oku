#[macro_use]
mod macros;

pub use self::{
    delete_database_request::DeleteDatabaseRequestFuture,
    open_database_request::OpenDatabaseRequestFuture,
};

mod open_database_request {
    use wasm_bindgen::JsValue;
    use web_sys::Event;

    use crate::{request::OpenDatabaseRequest, Database, Error, Request};

    impl_database_request_future!(
        OpenDatabaseRequestFuture,
        crate::request::OpenDatabaseRequest,
        crate::Database,
        "Future returned by [`OpenDatabaseRequest::into_future`](crate::request::OpenDatabaseRequest::into_future)."
    );

    fn success_callback(event: Event) -> Result<Database, Error> {
        let target = event.target().ok_or_else(|| Error::EventTargetNotFound)?;
        let request = OpenDatabaseRequest::try_from(target)?;

        let result = request.result()?;

        if result.is_undefined() {
            Err(Error::UnexpectedJsType("database", JsValue::UNDEFINED))
        } else if result.is_null() {
            Err(Error::UnexpectedJsType("database", JsValue::NULL))
        } else {
            result.try_into()
        }
    }
}

mod delete_database_request {
    use web_sys::Event;

    use crate::{request::DeleteDatabaseRequest, Error, Request};

    impl_database_request_future!(
        DeleteDatabaseRequestFuture,
        crate::request::DeleteDatabaseRequest,
        (),
        "Future returned by [`DeleteDatabaseRequest::into_future`](crate::request::DeleteDatabaseRequest::into_future)."
    );

    fn success_callback(event: Event) -> Result<(), Error> {
        let target = event.target().ok_or_else(|| Error::EventTargetNotFound)?;
        let request = DeleteDatabaseRequest::try_from(target)?;

        let result = request.result()?;

        if result.is_null() || result.is_undefined() {
            Ok(())
        } else {
            Err(Error::UnexpectedJsType("null", result))
        }
    }
}
