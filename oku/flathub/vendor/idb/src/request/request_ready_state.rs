use wasm_bindgen::JsValue;
use web_sys::IdbRequestReadyState;

use crate::Error;

/// State of a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestReadyState {
    /// Request is still ongoing.
    Pending,
    /// Request has already completed.
    Done,
}

impl From<RequestReadyState> for IdbRequestReadyState {
    fn from(state: RequestReadyState) -> Self {
        match state {
            RequestReadyState::Pending => IdbRequestReadyState::Pending,
            RequestReadyState::Done => IdbRequestReadyState::Done,
        }
    }
}

impl TryFrom<IdbRequestReadyState> for RequestReadyState {
    type Error = Error;

    fn try_from(value: IdbRequestReadyState) -> Result<Self, Self::Error> {
        match value {
            IdbRequestReadyState::Pending => Ok(RequestReadyState::Pending),
            IdbRequestReadyState::Done => Ok(RequestReadyState::Done),
            _ => Err(Error::InvalidReqeustReadyState),
        }
    }
}

impl TryFrom<JsValue> for RequestReadyState {
    type Error = Error;

    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        IdbRequestReadyState::from_js_value(&value)
            .ok_or(Error::InvalidCursorDirection)?
            .try_into()
    }
}

impl From<RequestReadyState> for JsValue {
    fn from(direction: RequestReadyState) -> Self {
        let inner: IdbRequestReadyState = direction.into();
        inner.into()
    }
}
