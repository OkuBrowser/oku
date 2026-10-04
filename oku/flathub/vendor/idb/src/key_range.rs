use wasm_bindgen::{JsCast, JsValue};
use web_sys::IdbKeyRange;

use crate::Error;

/// Represents a continuous interval over some data type that is used for keys.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyRange {
    inner: IdbKeyRange,
}

impl KeyRange {
    /// Returns a new [`KeyRange`] spanning only key.
    pub fn only(value: &JsValue) -> Result<Self, Error> {
        let inner = IdbKeyRange::only(value).map_err(Error::KeyRangeCreateFailed)?;

        Ok(Self { inner })
    }

    /// Returns a new [`KeyRange`] spanning from lower to upper. If `lower_open` is true, `lower` is not included in the
    /// range. If `upper_open` is true, `upper` is not included in the range.
    pub fn bound(
        lower: &JsValue,
        upper: &JsValue,
        lower_open: Option<bool>,
        upper_open: Option<bool>,
    ) -> Result<Self, Error> {
        let inner = IdbKeyRange::bound_with_lower_open_and_upper_open(
            lower,
            upper,
            lower_open.unwrap_or_default(),
            upper_open.unwrap_or_default(),
        )
        .map_err(Error::KeyRangeCreateFailed)?;

        Ok(Self { inner })
    }

    /// Returns a new [`KeyRange`] starting at key with no upper bound. If `lower_open` is true, key is not included in
    /// the range.
    pub fn lower_bound(lower: &JsValue, lower_open: Option<bool>) -> Result<Self, Error> {
        let inner = IdbKeyRange::lower_bound_with_open(lower, lower_open.unwrap_or_default())
            .map_err(Error::KeyRangeCreateFailed)?;

        Ok(Self { inner })
    }

    /// Returns a new [`KeyRange`] with no lower bound and ending at key. If `upper_open` is true, key is not included
    /// in the range.
    pub fn upper_bound(upper: &JsValue, upper_open: Option<bool>) -> Result<Self, Error> {
        let inner = IdbKeyRange::upper_bound_with_open(upper, upper_open.unwrap_or_default())
            .map_err(Error::KeyRangeCreateFailed)?;

        Ok(Self { inner })
    }

    /// Returns the range’s lower bound, or undefined if none.
    pub fn lower(&self) -> Result<JsValue, Error> {
        self.inner.lower().map_err(Error::KeyRangeBoundNotFound)
    }

    /// Returns the range’s upper bound, or undefined if none.
    pub fn upper(&self) -> Result<JsValue, Error> {
        self.inner.upper().map_err(Error::KeyRangeBoundNotFound)
    }

    /// Returns the range’s lower open flag.
    pub fn lower_open(&self) -> bool {
        self.inner.lower_open()
    }

    /// Returns the range’s upper open flag.
    pub fn upper_open(&self) -> bool {
        self.inner.upper_open()
    }

    /// Returns true if key is included in the range, and false otherwise.
    pub fn includes(&self, value: &JsValue) -> Result<bool, Error> {
        self.inner
            .includes(value)
            .map_err(Error::KeyRangeIncludesFailed)
    }
}

impl From<IdbKeyRange> for KeyRange {
    fn from(inner: IdbKeyRange) -> Self {
        Self { inner }
    }
}

impl From<KeyRange> for IdbKeyRange {
    fn from(key_range: KeyRange) -> Self {
        key_range.inner
    }
}

impl TryFrom<JsValue> for KeyRange {
    type Error = Error;

    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        value
            .dyn_into::<IdbKeyRange>()
            .map(Into::into)
            .map_err(|value| Error::UnexpectedJsType("IdbKeyRange", value))
    }
}

impl From<KeyRange> for JsValue {
    fn from(value: KeyRange) -> Self {
        value.inner.into()
    }
}
