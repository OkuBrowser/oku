use wasm_bindgen::{JsCast, JsValue};
use web_sys::IdbIndexParameters;

use crate::Error;

/// Options when creating [`Index`](crate::Index).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct IndexParams {
    inner: IdbIndexParameters,
}

impl IndexParams {
    /// Creates a new instance of [`IndexParams`]
    pub fn new() -> Self {
        Default::default()
    }

    /// Sets the `unique` flag.
    pub fn unique(&mut self, unique: bool) -> &mut Self {
        self.inner.set_unique(unique);
        self
    }

    /// Sets the `multi_entry` flag.
    pub fn multi_entry(&mut self, multi_entry: bool) -> &mut Self {
        self.inner.set_multi_entry(multi_entry);
        self
    }
}

impl From<IdbIndexParameters> for IndexParams {
    fn from(inner: IdbIndexParameters) -> Self {
        Self { inner }
    }
}

impl From<IndexParams> for IdbIndexParameters {
    fn from(params: IndexParams) -> Self {
        params.inner
    }
}

impl TryFrom<JsValue> for IndexParams {
    type Error = Error;

    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        value
            .dyn_into::<IdbIndexParameters>()
            .map(Into::into)
            .map_err(|value| Error::UnexpectedJsType("IdbIndexParameters", value))
    }
}

impl From<IndexParams> for JsValue {
    fn from(value: IndexParams) -> Self {
        value.inner.into()
    }
}
