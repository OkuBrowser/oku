macro_rules! impl_database_request {
    ($type: ident, $event_type: ty, $doc: expr) => {
        #[doc = $doc]
        #[derive(Debug)]
        pub struct $type {
            inner: DatabaseRequest,
        }

        impl $type {
            /// Returns the database associated with this request
            pub fn database(&self) -> Result<crate::Database, crate::Error> {
                self.inner.database()
            }

            /// Adds an event handler for `blocked` event.
            pub fn on_blocked<F>(&mut self, callback: F)
            where
                F: FnOnce(crate::event::VersionChangeEvent) + 'static,
            {
                self.inner.on_blocked(callback)
            }

            /// Adds an event handler for `upgradeneeded` event.
            pub fn on_upgrade_needed<F>(&mut self, callback: F)
            where
                F: FnOnce(crate::event::VersionChangeEvent) + 'static,
            {
                self.inner.on_upgrade_needed(callback)
            }

            /// Release memory management of the callbacks to JS GC.
            ///
            /// > Note: This may leak memory. Read more about it
            /// > [here](https://docs.rs/wasm-bindgen/latest/wasm_bindgen/closure/struct.Closure.html#method.into_js_value).
            pub fn forget_callbacks(&mut self) {
                self.inner.forget_callbacks()
            }
        }

        impl crate::Request for $type {
            type Event = $event_type;

            fn result(&self) -> Result<wasm_bindgen::JsValue, crate::Error> {
                self.inner.result()
            }

            fn error(&self) -> Result<Option<web_sys::DomException>, crate::Error> {
                self.inner.error()
            }

            fn source(&self) -> Result<js_sys::Object, crate::Error> {
                self.inner.source()
            }

            fn transaction(&self) -> Option<crate::Transaction> {
                self.inner.transaction()
            }

            fn ready_state(&self) -> Result<crate::request::RequestReadyState, crate::Error> {
                self.inner.ready_state()
            }

            fn on_success<F>(&mut self, callback: F)
            where
                F: FnOnce($event_type) + 'static,
            {
                self.inner.on_success(move |event| callback(event.into()))
            }

            fn on_error<F>(&mut self, callback: F)
            where
                F: FnOnce($event_type) + 'static,
            {
                self.inner.on_error(move |event| callback(event.into()))
            }
        }

        impl From<DatabaseRequest> for $type {
            fn from(value: DatabaseRequest) -> Self {
                Self { inner: value }
            }
        }

        impl From<$type> for DatabaseRequest {
            fn from(value: $type) -> Self {
                value.inner
            }
        }

        impl TryFrom<web_sys::EventTarget> for $type {
            type Error = crate::Error;

            fn try_from(target: web_sys::EventTarget) -> Result<Self, Self::Error> {
                Ok(Self {
                    inner: target.try_into()?,
                })
            }
        }

        impl From<web_sys::IdbOpenDbRequest> for $type {
            fn from(inner: web_sys::IdbOpenDbRequest) -> Self {
                Self {
                    inner: inner.into(),
                }
            }
        }

        impl From<$type> for web_sys::IdbOpenDbRequest {
            fn from(request: $type) -> Self {
                request.inner.into()
            }
        }

        impl TryFrom<wasm_bindgen::JsValue> for $type {
            type Error = crate::Error;

            fn try_from(value: wasm_bindgen::JsValue) -> Result<Self, Self::Error> {
                Ok(Self {
                    inner: value.try_into()?,
                })
            }
        }

        impl From<$type> for wasm_bindgen::JsValue {
            fn from(value: $type) -> Self {
                value.inner.into()
            }
        }
    };
}
