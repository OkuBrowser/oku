use wasm_bindgen::JsValue;

use crate::KeyRange;

/// Specifies a query when fetching data from object store
#[derive(Debug, Clone, PartialEq)]
pub enum Query {
    /// Denotes a single key
    Key(JsValue),
    /// Denotes a key range
    KeyRange(KeyRange),
}

impl From<JsValue> for Query {
    fn from(value: JsValue) -> Self {
        Query::Key(value)
    }
}

impl From<KeyRange> for Query {
    fn from(value: KeyRange) -> Self {
        Query::KeyRange(value)
    }
}

impl From<Query> for JsValue {
    fn from(value: Query) -> Self {
        match value {
            Query::Key(value) => value,
            Query::KeyRange(value) => value.into(),
        }
    }
}
