macro_rules! impl_database_event {
    ($type: ident, $base_event: ty, $target: ty, $doc: expr) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct $type {
            inner: $base_event,
        }

        impl crate::Event for $type {
            type Target = $target;

            fn target(&self) -> Result<Self::Target, crate::Error> {
                let target = self
                    .inner
                    .target()
                    .ok_or(crate::Error::EventTargetNotFound)?;
                <$target>::try_from(target)
            }
        }

        impl crate::DatabaseEvent for $type {
            fn database(&self) -> Result<crate::Database, crate::Error> {
                let target = <Self as crate::Event>::target(self)?;
                target.database()
            }
        }

        impl From<$base_event> for $type {
            fn from(inner: $base_event) -> Self {
                Self { inner }
            }
        }

        impl From<$type> for $base_event {
            fn from(event: $type) -> Self {
                event.inner
            }
        }

        impl TryFrom<wasm_bindgen::JsValue> for $type {
            type Error = crate::Error;

            fn try_from(value: wasm_bindgen::JsValue) -> Result<Self, Self::Error> {
                wasm_bindgen::JsCast::dyn_into::<$base_event>(value)
                    .map(Into::into)
                    .map_err(|value| crate::Error::UnexpectedJsType(stringify!($base_event), value))
            }
        }

        impl From<$type> for wasm_bindgen::JsValue {
            fn from(value: $type) -> Self {
                value.inner.into()
            }
        }
    };
}
