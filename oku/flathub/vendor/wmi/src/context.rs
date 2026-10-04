use crate::{WMIConnection, WMIResult};
use log::debug;
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::System::{
    Com::{CLSCTX_INPROC_SERVER, CoCreateInstance},
    Wmi::{IWbemContext, WbemContext},
};
use windows::core::BSTR;

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum ContextValueType {
    String(String),
    I4(i32),
    R8(f64),
    Bool(bool),
}

impl From<ContextValueType> for VARIANT {
    fn from(value: ContextValueType) -> Self {
        match value {
            ContextValueType::Bool(b) => Self::from(b),
            ContextValueType::I4(i4) => Self::from(i4),
            ContextValueType::R8(r8) => Self::from(r8),
            ContextValueType::String(str) => Self::from(BSTR::from(str)),
        }
    }
}

/// Provides access to WMI's context, available via [`WMIConnection::ctx`].
#[derive(Clone, Debug)]
pub struct WMIContext(pub(crate) IWbemContext);

impl WMIContext {
    /// Creates a new instances of [`WMIContext`]
    pub(crate) fn new() -> WMIResult<WMIContext> {
        debug!("Calling CoCreateInstance for CLSID_WbemContext");

        let ctx = unsafe { CoCreateInstance(&WbemContext, None, CLSCTX_INPROC_SERVER)? };

        debug!("Got context {:?}", ctx);

        Ok(WMIContext(ctx))
    }

    /// Sets the specified named context value for use in providing additional context information to queries.
    ///
    /// Note the context values will persist across subsequent queries until [`WMIContext::delete_all`] is called.
    pub fn set_value(&mut self, key: &str, value: impl Into<ContextValueType>) -> WMIResult<()> {
        let value = value.into();
        unsafe { self.0.SetValue(&BSTR::from(key), 0, &value.into())? };

        Ok(())
    }

    /// Clears all named values from the underlying context object.
    pub fn delete_all(&mut self) -> WMIResult<()> {
        unsafe { self.0.DeleteAll()? };

        Ok(())
    }
}

impl WMIConnection {
    /// Returns a mutable reference to the [`WMIContext`] object
    pub fn ctx(&mut self) -> &mut WMIContext {
        &mut self.ctx
    }
}

macro_rules! impl_from_type {
    ($target_type:ty, $variant:ident) => {
        impl From<$target_type> for ContextValueType {
            fn from(value: $target_type) -> Self {
                Self::$variant(value.into())
            }
        }
    };
}

impl_from_type!(&str, String);
impl_from_type!(i32, I4);
impl_from_type!(f64, R8);
impl_from_type!(bool, Bool);

#[allow(non_snake_case)]
#[allow(non_camel_case_types)]
#[allow(dead_code)]
#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[test]
    fn verify_ctx_values_used() {
        let mut wmi_con = WMIConnection::with_namespace_path("ROOT\\StandardCimv2").unwrap();

        #[derive(Deserialize, PartialEq, Eq, PartialOrd, Ord, Debug)]
        struct MSFT_NetAdapter {
            InterfaceName: String,
        }

        let mut orig_adapters = wmi_con.query::<MSFT_NetAdapter>().unwrap();
        assert!(!orig_adapters.is_empty());

        // With 'IncludeHidden' set to 'true', expect the response to contain additional adapters
        wmi_con.ctx().set_value("IncludeHidden", true).unwrap();
        let all_adapters = wmi_con.query::<MSFT_NetAdapter>().unwrap();
        assert!(all_adapters.len() > orig_adapters.len());

        wmi_con.ctx().delete_all().unwrap();
        let mut adapters = wmi_con.query::<MSFT_NetAdapter>().unwrap();
        adapters.sort();
        orig_adapters.sort();
        assert_eq!(adapters, orig_adapters);
    }

    #[tokio::test]
    async fn async_verify_ctx_values_used() {
        let mut wmi_con = WMIConnection::with_namespace_path("ROOT\\StandardCimv2").unwrap();

        #[derive(Deserialize, PartialEq, Eq, PartialOrd, Ord, Debug)]
        struct MSFT_NetAdapter {
            InterfaceName: String,
        }

        let mut orig_adapters = wmi_con.async_query::<MSFT_NetAdapter>().await.unwrap();
        assert!(!orig_adapters.is_empty());

        // With 'IncludeHidden' set to 'true', expect the response to contain additional adapters
        wmi_con.ctx().set_value("IncludeHidden", true).unwrap();
        let all_adapters = wmi_con.async_query::<MSFT_NetAdapter>().await.unwrap();
        assert!(all_adapters.len() > orig_adapters.len());

        wmi_con.ctx().delete_all().unwrap();
        let mut adapters = wmi_con.async_query::<MSFT_NetAdapter>().await.unwrap();
        adapters.sort();
        orig_adapters.sort();
        assert_eq!(adapters, orig_adapters);
    }
}
