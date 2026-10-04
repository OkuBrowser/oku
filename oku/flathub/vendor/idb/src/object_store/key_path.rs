use js_sys::Array;
use wasm_bindgen::{JsCast, JsValue};

use crate::Error;

/// Represents key path of an object store
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyPath {
    /// Key path with single key
    Single(String),
    /// Key path with multiple keys
    Array(Vec<String>),
}

impl KeyPath {
    /// Creates new single key path
    pub fn new_single(key_path: &str) -> Self {
        Self::Single(key_path.to_owned())
    }

    /// Creates new multiple key path
    pub fn new_array<'a>(key_path_array: impl IntoIterator<Item = &'a str>) -> Self {
        Self::Array(key_path_array.into_iter().map(ToOwned::to_owned).collect())
    }
}

impl From<KeyPath> for JsValue {
    fn from(key_path: KeyPath) -> Self {
        match key_path {
            KeyPath::Single(key_path) => JsValue::from_str(&key_path),
            KeyPath::Array(key_path_array) => {
                let key_path: Array = key_path_array
                    .iter()
                    .map(|s| JsValue::from_str(s))
                    .collect();
                key_path.into()
            }
        }
    }
}

impl TryFrom<JsValue> for KeyPath {
    type Error = Error;

    fn try_from(key_path: JsValue) -> Result<Self, Self::Error> {
        if key_path.is_string() {
            let key_path = key_path.as_string().ok_or(Error::InvalidKeyPath)?;

            Ok(KeyPath::Single(key_path))
        } else {
            let key_path: Array = key_path
                .dyn_into()
                .map_err(|value| Error::UnexpectedJsType("Array", value))?;

            let mut key_paths = vec![];

            for i in 0..key_path.length() {
                if let Some(k) = key_path.get(i).as_string() {
                    key_paths.push(k);
                }
            }

            Ok(KeyPath::Array(key_paths))
        }
    }
}
