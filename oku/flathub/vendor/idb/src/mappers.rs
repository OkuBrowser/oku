use js_sys::Array;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::DomException;

use crate::{utils::array_to_vec, Cursor, Error, KeyCursor};

pub trait OutputMapper {
    type Output;

    fn map(value: Result<JsValue, Error>) -> Result<Self::Output, Error>;
}

pub struct NullCheckMapper;

impl OutputMapper for NullCheckMapper {
    type Output = JsValue;

    fn map(value: Result<JsValue, Error>) -> Result<Self::Output, Error> {
        value.and_then(|value| {
            if value.is_null() || value.is_undefined() {
                Err(Error::UnexpectedJsType("JsValue", value))
            } else {
                Ok(value)
            }
        })
    }
}

pub struct IgnoreMapper;

impl OutputMapper for IgnoreMapper {
    type Output = ();

    fn map(value: Result<JsValue, Error>) -> Result<Self::Output, Error> {
        value.map(|_| ())
    }
}

pub struct OptionMapper;

impl OutputMapper for OptionMapper {
    type Output = Option<JsValue>;

    fn map(value: Result<JsValue, Error>) -> Result<Self::Output, Error> {
        value.map(|value| {
            if value.is_null() || value.is_undefined() {
                None
            } else {
                Some(value)
            }
        })
    }
}

pub struct U32Mapper;

impl OutputMapper for U32Mapper {
    type Output = u32;

    fn map(value: Result<JsValue, Error>) -> Result<Self::Output, Error> {
        value.and_then(|value| {
            if value.is_null() || value.is_undefined() {
                Ok(0)
            } else {
                value
                    .as_f64()
                    .and_then(num_traits::cast)
                    .ok_or(Error::UnexpectedJsType("u32", value))
            }
        })
    }
}

pub struct VecMapper;

impl OutputMapper for VecMapper {
    type Output = Vec<JsValue>;

    fn map(value: Result<JsValue, Error>) -> Result<Self::Output, Error> {
        value.and_then(|value| {
            if value.is_null() || value.is_undefined() {
                Ok(vec![])
            } else {
                let array: Array = value
                    .dyn_into()
                    .map_err(|err| Error::UnexpectedJsType("Array", err))?;

                Ok(array_to_vec(array))
            }
        })
    }
}

pub struct CursorMapper;

impl OutputMapper for CursorMapper {
    type Output = Option<Cursor>;

    fn map(value: Result<JsValue, Error>) -> Result<Self::Output, Error> {
        value.and_then(|value| {
            if value.is_null() || value.is_undefined() {
                Ok(None)
            } else {
                let cursor = Cursor::try_from(value)?;
                Ok(Some(cursor))
            }
        })
    }
}

pub struct KeyCursorMapper;

impl OutputMapper for KeyCursorMapper {
    type Output = Option<KeyCursor>;

    fn map(value: Result<JsValue, Error>) -> Result<Self::Output, Error> {
        value.and_then(|value| {
            if value.is_null() || value.is_undefined() {
                Ok(None)
            } else {
                let cursor = KeyCursor::try_from(value)?;
                Ok(Some(cursor))
            }
        })
    }
}

pub struct ErrorMapper;

impl ErrorMapper {
    pub fn map(value: Result<Option<DomException>, Error>) -> Error {
        match value {
            Err(err) => err,
            Ok(None) => Error::DomExceptionNotFound,
            Ok(Some(exception)) => Error::DomException(exception),
        }
    }
}
