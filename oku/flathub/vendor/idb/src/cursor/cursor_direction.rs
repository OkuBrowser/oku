use wasm_bindgen::JsValue;
use web_sys::IdbCursorDirection;

use crate::Error;

/// Specifies the cursor direction.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum CursorDirection {
    /// `Next` causes the cursor to be opened at the start of the source. When iterated, the cursor yields all records,
    /// including duplicates, in monotonically increasing order of keys.
    #[default]
    Next,
    /// `NextUnique` causes the cursor to be opened at the start of the source. When iterated, the cursor does not yield
    /// records with the same key, but otherwise yields all records, in monotonically increasing order of keys.
    NextUnique,
    /// `Prev` causes the cursor to be opened at the end of the source. When iterated, the cursor yields all records,
    /// including duplicates, in monotonically decreasing order of keys.
    Prev,
    /// `PrevUnique` causes the cursor to be opened at the end of the source. When iterated, the cursor does not yield
    /// records with the same key, but otherwise yields all records, in monotonically decreasing order of keys.
    PrevUnique,
}

impl TryFrom<IdbCursorDirection> for CursorDirection {
    type Error = Error;

    fn try_from(direction: IdbCursorDirection) -> Result<Self, Self::Error> {
        match direction {
            IdbCursorDirection::Next => Ok(CursorDirection::Next),
            IdbCursorDirection::Nextunique => Ok(CursorDirection::NextUnique),
            IdbCursorDirection::Prev => Ok(CursorDirection::Prev),
            IdbCursorDirection::Prevunique => Ok(CursorDirection::PrevUnique),
            _ => Err(Error::InvalidCursorDirection),
        }
    }
}

impl From<CursorDirection> for IdbCursorDirection {
    fn from(direction: CursorDirection) -> Self {
        match direction {
            CursorDirection::Next => IdbCursorDirection::Next,
            CursorDirection::NextUnique => IdbCursorDirection::Nextunique,
            CursorDirection::Prev => IdbCursorDirection::Prev,
            CursorDirection::PrevUnique => IdbCursorDirection::Prevunique,
        }
    }
}

impl TryFrom<JsValue> for CursorDirection {
    type Error = Error;

    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        IdbCursorDirection::from_js_value(&value)
            .ok_or(Error::InvalidCursorDirection)?
            .try_into()
    }
}

impl From<CursorDirection> for JsValue {
    fn from(direction: CursorDirection) -> Self {
        let inner: IdbCursorDirection = direction.into();
        inner.into()
    }
}
