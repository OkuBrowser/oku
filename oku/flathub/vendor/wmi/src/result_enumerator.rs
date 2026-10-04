use crate::{
    Variant, WMIError, WMIResult, de::wbem_class_de::from_wbem_class_obj,
    safearray::safe_array_to_vec_of_strings,
};
use log::trace;
use serde::{
    Serialize, de,
    ser::{Error, SerializeMap},
};
use std::ptr::{self, NonNull};
use windows::Win32::System::Ole::SafeArrayDestroy;
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::System::Wmi::{
    CIMTYPE_ENUMERATION, IEnumWbemClassObject, IWbemClassObject, WBEM_FLAG_ALWAYS,
    WBEM_FLAG_NONSYSTEM_ONLY, WBEM_INFINITE,
};
use windows::{
    Win32::System::Wmi::WBEM_CONDITION_FLAG_TYPE,
    core::{BSTR, HSTRING, PCWSTR},
};

/// A wrapper around a [IWbemClassObject](https://learn.microsoft.com/en-us/windows/win32/api/wbemcli/nn-wbemcli-iwbemclassobject).
///
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IWbemClassWrapper {
    pub inner: IWbemClassObject,
}

impl IWbemClassWrapper {
    pub fn new(inner: IWbemClassObject) -> Self {
        Self { inner }
    }

    /// Return the names of all the properties of the object.
    /// See more at <https://learn.microsoft.com/en-us/windows/win32/api/wbemcli/nf-wbemcli-iwbemclassobject-getnames>.
    pub fn list_properties(&self) -> WMIResult<Vec<String>> {
        let p_names = unsafe {
            self.inner.GetNames(
                None,
                WBEM_CONDITION_FLAG_TYPE(WBEM_FLAG_ALWAYS.0 | WBEM_FLAG_NONSYSTEM_ONLY.0),
                ptr::null_mut(),
            )
        }?;

        let p_names = NonNull::new(p_names).ok_or(WMIError::NullPointerResult)?;

        let res = unsafe { safe_array_to_vec_of_strings(p_names) };

        unsafe { SafeArrayDestroy(p_names.as_ptr()) }?;

        res
    }

    /// Get the value of a property.
    /// See more at <https://learn.microsoft.com/en-us/windows/win32/api/wbemcli/nf-wbemcli-iwbemclassobject-get>.
    pub fn get_property(&self, property_name: &str) -> WMIResult<Variant> {
        let name_prop = HSTRING::from(property_name);

        let mut vt_prop = VARIANT::default();

        let mut cim_type = 0;

        unsafe {
            self.inner.Get(
                PCWSTR::from_raw(name_prop.as_ptr()),
                0,
                &mut vt_prop,
                Some(&mut cim_type),
                None,
            )?;
        }

        let property_value = Variant::from_variant(&vt_prop)?;

        property_value.convert_into_cim_type(CIMTYPE_ENUMERATION(cim_type))
    }

    /// Set the value of a property.
    /// See more at <https://learn.microsoft.com/en-us/windows/win32/api/wbemcli/nf-wbemcli-iwbemclassobject-put>.
    pub fn put_property(&self, property_name: &str, value: impl Into<Variant>) -> WMIResult<()> {
        let name_prop = HSTRING::from(property_name);

        let value = value.into();
        let vt_prop: VARIANT = value.try_into()?;

        // "In every other case, vtType must be 0 (zero)"
        // See more at <https://learn.microsoft.com/en-us/windows/win32/api/wbemcli/nf-wbemcli-iwbemclassobject-put>.
        let cim_type = 0;

        unsafe {
            self.inner
                .Put(PCWSTR::from_raw(name_prop.as_ptr()), 0, &vt_prop, cim_type)?;
        }

        Ok(())
    }

    /// Get the input signature class for the named method.
    /// See [`crate::WMIConnection::exec_method`] for a usage example.
    /// See more at <https://learn.microsoft.com/en-us/windows/win32/api/wbemcli/nf-wbemcli-iwbemclassobject-getmethod>.
    ///
    /// The method may have no input parameters, such as in this case: <https://learn.microsoft.com/en-us/windows/win32/cimwin32prov/reboot-method-in-class-win32-operatingsystem>.
    /// In such cases, `None` is returned.
    pub fn get_method(&self, name: impl AsRef<str>) -> WMIResult<Option<IWbemClassWrapper>> {
        let method = BSTR::from(name.as_ref());

        // Retrieve the input signature of the WMI method.
        // The fields of the resulting IWbemClassObject will have the names and types of the WMI method's input parameters
        let mut input_signature = None;

        unsafe {
            self.inner.GetMethod(
                &method,
                Default::default(),
                &mut input_signature,
                std::ptr::null_mut(),
            )?;
        }

        Ok(input_signature.map(IWbemClassWrapper::new))
    }

    /// Get the input and output signature classes for the named method.
    /// See [`crate::WMIConnection::exec_method`] for a usage example.
    /// Note: GetMethod can only be called on a class definition.
    /// See more at <https://learn.microsoft.com/en-us/windows/win32/api/wbemcli/nf-wbemcli-iwbemclassobject-getmethod>.
    ///
    /// The method may have no in or out parameters, such as in this case: <https://learn.microsoft.com/en-us/windows/win32/cimwin32prov/reboot-method-in-class-win32-operatingsystem>.
    /// In such cases, `None` is returned.
    pub fn get_method_in_out(
        &self,
        name: impl AsRef<str>,
    ) -> WMIResult<(Option<IWbemClassWrapper>, Option<IWbemClassWrapper>)> {
        let method = BSTR::from(name.as_ref());

        // Retrieve the input signature of the WMI method.
        // The fields of the resulting IWbemClassObject will have the names and types of the WMI method's input parameters
        let mut input_signature = None;
        let mut output_signature = None;

        unsafe {
            self.inner.GetMethod(
                &method,
                Default::default(),
                &mut input_signature,
                &mut output_signature,
            )?;
        }

        Ok((
            input_signature.map(IWbemClassWrapper::new),
            output_signature.map(IWbemClassWrapper::new),
        ))
    }

    /// Create a new instance a class. See [`crate::WMIConnection::exec_method`] for a usage example.
    /// See more at <https://learn.microsoft.com/en-us/windows/win32/api/wbemcli/nf-wbemcli-iwbemclassobject-spawninstance>.
    pub fn spawn_instance(&self) -> WMIResult<IWbemClassWrapper> {
        let inst = unsafe { self.inner.SpawnInstance(Default::default())? };
        let inst = IWbemClassWrapper::new(inst);

        Ok(inst)
    }

    pub fn path(&self) -> WMIResult<String> {
        self.get_property("__Path").and_then(Variant::try_into)
    }

    pub fn class(&self) -> WMIResult<String> {
        self.get_property("__Class").and_then(Variant::try_into)
    }

    pub fn into_desr<T>(self) -> WMIResult<T>
    where
        T: de::DeserializeOwned,
    {
        from_wbem_class_obj(self)
    }
}

impl Serialize for IWbemClassWrapper {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let properties = self.list_properties().map_err(Error::custom)?;
        let mut s = serializer.serialize_map(Some(properties.len()))?;
        for property in properties.iter() {
            let value = self.get_property(property).unwrap();
            s.serialize_entry(property, &value)?;
        }
        s.end()
    }
}

pub(crate) struct QueryResultEnumerator {
    p_enumerator: IEnumWbemClassObject,
}

impl QueryResultEnumerator {
    pub(crate) fn new(p_enumerator: IEnumWbemClassObject) -> Self {
        Self { p_enumerator }
    }
}

impl Iterator for QueryResultEnumerator {
    type Item = WMIResult<IWbemClassWrapper>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut objs = [None; 1];
        let mut return_value = 0;

        let res = unsafe {
            self.p_enumerator
                .Next(WBEM_INFINITE, &mut objs, &mut return_value)
        };

        if let Err(e) = res.ok() {
            return Some(Err(e.into()));
        }

        if return_value == 0 {
            return None;
        }

        trace!(
            "Got enumerator {:?} and obj {:?}",
            self.p_enumerator, &objs[0]
        );

        let [obj] = objs;
        let pcls_ptr = obj.ok_or(WMIError::NullPointerResult);

        match pcls_ptr {
            Err(e) => Some(Err(e)),
            Ok(pcls_ptr) => Some(Ok(IWbemClassWrapper::new(pcls_ptr))),
        }
    }
}
