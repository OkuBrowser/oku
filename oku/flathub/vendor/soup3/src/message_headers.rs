use glib::translate::*;
use glib::{GString, IntoGStr, IntoOptionalGStr};
use std::collections::HashMap;
use std::{fmt, ptr};

use crate::MessageHeaders;

impl MessageHeaders {
    #[doc(alias = "soup_message_headers_get_content_disposition")]
    pub fn content_disposition(&self) -> Option<(GString, HashMap<String, String>)> {
        let mut disposition = ptr::null_mut();
        let mut params = ptr::null_mut();
        unsafe {
            if bool::from_glib(ffi::soup_message_headers_get_content_disposition(
                mut_override(self.to_glib_none().0),
                &mut disposition,
                &mut params,
            )) {
                let params = if !params.is_null() {
                    HashMap::from_glib_full(params)
                } else {
                    HashMap::new()
                };
                Some((GString::from_glib_full(disposition), params))
            } else {
                None
            }
        }
    }

    #[doc(alias = "soup_message_headers_set_content_disposition")]
    pub fn set_content_disposition(
        &self,
        disposition: Option<impl IntoGStr>,
        params: Option<HashMap<String, String>>,
    ) {
        disposition.run_with_gstr(|disposition| unsafe {
            ffi::soup_message_headers_set_content_disposition(
                self.to_glib_none().0,
                disposition.to_glib_none().0,
                params.to_glib_none().0,
            )
        })
    }

    #[doc(alias = "soup_message_headers_get_content_type")]
    pub fn content_type(&self) -> Option<(GString, HashMap<String, String>)> {
        let mut params = ptr::null_mut();
        unsafe {
            let content_type = ffi::soup_message_headers_get_content_type(
                mut_override(self.to_glib_none().0),
                &mut params,
            );
            if !content_type.is_null() {
                let params = if !params.is_null() {
                    HashMap::from_glib_full(params)
                } else {
                    HashMap::new()
                };
                Some((GString::from_glib_none(content_type), params))
            } else {
                None
            }
        }
    }

    #[doc(alias = "soup_message_headers_set_content_type")]
    pub fn set_content_type(
        &self,
        content_type: Option<impl IntoGStr>,
        params: Option<HashMap<String, String>>,
    ) {
        content_type.run_with_gstr(|content_type| unsafe {
            ffi::soup_message_headers_set_content_type(
                self.to_glib_none().0,
                content_type.to_glib_none().0,
                params.to_glib_none().0,
            )
        })
    }
}

impl fmt::Debug for MessageHeaders {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct Map<'a>(&'a MessageHeaders);
        impl fmt::Debug for Map<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let mut map = f.debug_map();
                self.0.foreach(|name, value| _ = map.entry(&name, &value));
                map.finish()
            }
        }
        f.debug_tuple("MessageHeaders").field(&Map(self)).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MessageHeadersType;
    use std::collections::HashMap;

    #[test]
    fn content_type_can_be_set_and_read() {
        let headers = MessageHeaders::new(MessageHeadersType::Request);
        headers.set_content_type(Some(EXPECTED_CONTENT_TYPE), Some(expected_params()));

        let (content_type, params) = headers
            .content_type()
            .expect("content type header to be present");

        assert_eq!(EXPECTED_CONTENT_TYPE, content_type);
        assert_eq!(expected_params(), params);
    }

    #[test]
    fn content_disposition_can_be_set_and_read() {
        let headers = MessageHeaders::new(MessageHeadersType::Request);
        headers
            .set_content_disposition(Some(EXPECTED_CONTENT_DISPOSITION), Some(expected_params()));

        let (content_type, params) = headers
            .content_disposition()
            .expect("content disposition header to be present");

        assert_eq!(EXPECTED_CONTENT_DISPOSITION, content_type);
        assert_eq!(expected_params(), params);
    }

    const EXPECTED_CONTENT_TYPE: &str = "text/html";
    const EXPECTED_CONTENT_DISPOSITION: &str = "attachment";

    fn expected_params() -> HashMap<String, String> {
        [("param".to_owned(), "value".to_owned())]
            .into_iter()
            .collect()
    }
}
