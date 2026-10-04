//! This module implements a custom serializer type, [`VariantStructSerializer`],
//! to serialize a Rust struct into a HashMap mapping field name strings to [`Variant`] values
use std::{any::type_name, fmt::Display};

use crate::{Variant, WMIConnection, WMIError, result_enumerator::IWbemClassWrapper};
use serde::{
    Serialize, Serializer,
    ser::{Impossible, SerializeSeq, SerializeStruct},
};
use thiserror::Error;

macro_rules! serialize_variant_err_stub {
    ($signature:ident, $type:ty) => {
        fn $signature(self, _v: $type) -> Result<Self::Ok, Self::Error> {
            Err(VariantSerializerError::UnsupportedVariantType(
                type_name::<$type>().to_string(),
            ))
        }
    };
}

macro_rules! serialize_variant {
    ($signature:ident, $type:ty) => {
        fn $signature(self, v: $type) -> Result<Self::Ok, Self::Error> {
            Ok(Variant::from(v))
        }
    };
}

pub(crate) struct VariantSerializer<'a> {
    pub(crate) wmi: &'a WMIConnection,
    pub(crate) instance: Option<IWbemClassWrapper>,
}

impl<'a> Serializer for VariantSerializer<'a> {
    type Ok = Variant;
    type Error = VariantSerializerError;

    type SerializeSeq = VariantSeqSerializer<'a>;
    type SerializeTuple = Impossible<Self::Ok, Self::Error>;
    type SerializeTupleStruct = Impossible<Self::Ok, Self::Error>;
    type SerializeTupleVariant = Impossible<Self::Ok, Self::Error>;
    type SerializeMap = Impossible<Self::Ok, Self::Error>;
    type SerializeStruct = VariantInstanceSerializer<'a>;
    type SerializeStructVariant = Impossible<Self::Ok, Self::Error>;

    serialize_variant!(serialize_bool, bool);
    serialize_variant!(serialize_i8, i8);
    serialize_variant!(serialize_i16, i16);
    serialize_variant!(serialize_i32, i32);
    serialize_variant!(serialize_i64, i64);
    serialize_variant!(serialize_u8, u8);
    serialize_variant!(serialize_u16, u16);
    serialize_variant!(serialize_u32, u32);
    serialize_variant!(serialize_u64, u64);
    serialize_variant!(serialize_f32, f32);
    serialize_variant!(serialize_f64, f64);

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        // When starting from an instance, deserializing a unit means returning the original instance unmodified.
        Ok(self.instance.map(Variant::from).unwrap_or(Variant::Empty))
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        Ok(Variant::from(v.to_string()))
    }

    fn serialize_newtype_variant<T>(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        Err(VariantSerializerError::UnsupportedVariantType(format!(
            "{variant}::{name}"
        )))
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        let ser = self.serialize_struct(name, 0)?;

        ser.end()
    }

    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Ok(Variant::from(variant.to_string()))
    }

    // Generic serializer code not relevant to this use case

    serialize_variant_err_stub!(serialize_char, char);
    serialize_variant_err_stub!(serialize_bytes, &[u8]);

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        // we serialize to VT_NULL (explicit NULL semantic)  rather than VT_EMPTY
        // (default state or uninitialized semantic)
        Ok(Variant::Null)
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(VariantSeqSerializer {
            seq: Vec::with_capacity(len.unwrap_or_default()),
            wmi: self.wmi,
        })
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Err(VariantSerializerError::UnsupportedVariantType(
            "Tuple".to_string(),
        ))
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Err(VariantSerializerError::UnsupportedVariantType(
            name.to_string(),
        ))
    }

    fn serialize_tuple_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(VariantSerializerError::UnsupportedVariantType(format!(
            "{variant}::{name}"
        )))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Err(VariantSerializerError::UnsupportedVariantType(
            "Map".to_string(),
        ))
    }

    fn serialize_struct(
        self,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        // We are only given an initialized instance when called from `exec_method`,
        // with the instance matching the method signature class.
        // Otherwise, we use the name of the struct to create. See test for  `Win32_Process` with "Create" and `Win32_ProcessStartup`.
        let instance = match self.instance {
            Some(instance) => instance,
            None => self.wmi.get_object(name)?.spawn_instance()?,
        };

        let ser = VariantInstanceSerializer {
            wmi: self.wmi,
            instance,
        };

        Ok(ser)
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(VariantSerializerError::UnsupportedVariantType(format!(
            "{variant}::{name}"
        )))
    }
}

#[derive(Debug, Error)]
pub enum VariantSerializerError {
    #[error("Unknown error while serializing struct:\n{0}")]
    Unknown(String),
    #[error("{0} cannot be serialized to a Variant.")]
    UnsupportedVariantType(String),
    #[error("WMI error while serializing struct: \n {0}")]
    WMIError(#[from] WMIError),
}

impl serde::ser::Error for VariantSerializerError {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        VariantSerializerError::Unknown(msg.to_string())
    }
}

pub(crate) struct VariantInstanceSerializer<'a> {
    instance: IWbemClassWrapper,
    wmi: &'a WMIConnection,
}

impl<'a> SerializeStruct for VariantInstanceSerializer<'a> {
    type Ok = Variant;

    type Error = VariantSerializerError;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        let variant = value.serialize(VariantSerializer {
            wmi: self.wmi,
            instance: None,
        })?;

        self.instance.put_property(key, variant)?;

        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Variant::Object(self.instance))
    }
}

pub(crate) struct VariantSeqSerializer<'a> {
    seq: Vec<Variant>,
    wmi: &'a WMIConnection,
}

impl<'a> SerializeSeq for VariantSeqSerializer<'a> {
    type Ok = Variant;
    type Error = VariantSerializerError;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        let variant = value.serialize(VariantSerializer {
            wmi: self.wmi,
            instance: None,
        })?;

        self.seq.push(variant);

        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Variant::Array(self.seq))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::fixtures::wmi_con;
    use serde::Serialize;
    use std::ptr;
    use windows::Win32::System::Wmi::{CIM_FLAG_ARRAY, CIM_SINT64, CIM_UINT64};
    use windows::core::HSTRING;

    #[test]
    fn it_serialize_instance() {
        let wmi_con = wmi_con();

        #[derive(Serialize)]
        struct GetBinaryValue {
            sSubKeyName: String,
            sValueName: String,
        }

        let in_params = GetBinaryValue {
            sSubKeyName: r#"SYSTEM\CurrentControlSet\Control\Windows"#.to_string(),
            sValueName: "FullProcessInformationSID".to_string(),
        };

        // Similar to how `exec_class_method` creates these objects.
        let method_instance = wmi_con
            .get_object("StdRegProv")
            .unwrap()
            .get_method("GetBinaryValue")
            .unwrap()
            .unwrap()
            .spawn_instance()
            .unwrap();

        let instance_from_ser = in_params
            .serialize(VariantSerializer {
                wmi: &wmi_con,
                instance: Some(method_instance),
            })
            .unwrap();

        let instance_from_ser = match instance_from_ser {
            Variant::Object(instance_from_ser) => instance_from_ser,
            _ => panic!("Unexpected value {:?}", instance_from_ser),
        };

        let expected_instance = wmi_con
            .get_object("StdRegProv")
            .unwrap()
            .get_method("GetBinaryValue")
            .unwrap()
            .unwrap()
            .spawn_instance()
            .unwrap();

        assert_eq!(
            instance_from_ser.class().unwrap(),
            expected_instance.class().unwrap()
        );

        assert_eq!(
            instance_from_ser.get_property("sSubKeyName").unwrap(),
            Variant::String(in_params.sSubKeyName)
        );
    }

    fn spawn_instance(name: &str) -> IWbemClassWrapper {
        let wmi_con = wmi_con();
        wmi_con.get_object(&name).unwrap().spawn_instance().unwrap()
    }

    #[test]
    fn it_can_get_and_put_strings() {
        let prop = spawn_instance("Win32_PnPDevicePropertyString");
        let test_value = Variant::String("Some Title".to_string());

        prop.put_property("Data", test_value.clone()).unwrap();
        assert_eq!(prop.get_property("Data").unwrap(), test_value,);

        let prop = spawn_instance("Win32_PnPDevicePropertyStringArray");
        let test_value = Variant::Array(vec![
            Variant::String("X".to_string()),
            Variant::String("a".to_string()),
        ]);
        prop.put_property("Data", test_value.clone()).unwrap();
        assert_eq!(prop.get_property("Data").unwrap(), test_value,);
    }

    #[test]
    fn it_can_get_and_put_numbers_and_bool() {
        let prop = spawn_instance("Win32_PnPDevicePropertyBoolean");
        prop.put_property("Data", true).unwrap();
        assert_eq!(prop.get_property("Data").unwrap(), Variant::Bool(true));

        let prop = spawn_instance("Win32_PnPDevicePropertyUint8");
        prop.put_property("Data", u8::MAX).unwrap();
        assert_eq!(prop.get_property("Data").unwrap(), Variant::UI1(u8::MAX));

        let prop = spawn_instance("Win32_PnPDevicePropertyUint16");
        prop.put_property("Data", u16::MAX).unwrap();
        assert_eq!(prop.get_property("Data").unwrap(), Variant::UI2(u16::MAX));

        let prop = spawn_instance("Win32_PnPDevicePropertyUint32");
        prop.put_property("Data", u32::MAX).unwrap();
        assert_eq!(prop.get_property("Data").unwrap(), Variant::UI4(u32::MAX));

        let prop = spawn_instance("Win32_PnPDevicePropertyUint64");
        prop.put_property("Data", u64::MAX).unwrap();
        assert_eq!(prop.get_property("Data").unwrap(), Variant::UI8(u64::MAX));

        let prop = spawn_instance("Win32_PnPDevicePropertySint8");
        prop.put_property("Data", i8::MAX).unwrap();
        assert_eq!(prop.get_property("Data").unwrap(), Variant::I1(i8::MAX));
        prop.put_property("Data", i8::MIN).unwrap();
        assert_eq!(prop.get_property("Data").unwrap(), Variant::I1(i8::MIN));

        let prop = spawn_instance("Win32_PnPDevicePropertySint16");
        prop.put_property("Data", i16::MAX).unwrap();
        assert_eq!(prop.get_property("Data").unwrap(), Variant::I2(i16::MAX));
        prop.put_property("Data", i16::MIN).unwrap();
        assert_eq!(prop.get_property("Data").unwrap(), Variant::I2(i16::MIN));

        let prop = spawn_instance("Win32_PnPDevicePropertySint32");
        prop.put_property("Data", i32::MAX).unwrap();
        assert_eq!(prop.get_property("Data").unwrap(), Variant::I4(i32::MAX));
        prop.put_property("Data", i32::MIN).unwrap();
        assert_eq!(prop.get_property("Data").unwrap(), Variant::I4(i32::MIN));

        let prop = spawn_instance("Win32_PnPDevicePropertySint64");
        prop.put_property("Data", i64::MAX).unwrap();
        assert_eq!(prop.get_property("Data").unwrap(), Variant::I8(i64::MAX));
        prop.put_property("Data", i64::MIN).unwrap();
        assert_eq!(prop.get_property("Data").unwrap(), Variant::I8(i64::MIN));

        let prop = spawn_instance("Win32_PnPDevicePropertyReal32");
        prop.put_property("Data", 1.0f32).unwrap();
        assert_eq!(prop.get_property("Data").unwrap(), Variant::R4(1.0));

        let prop = spawn_instance("Win32_PnPDevicePropertyReal64");
        prop.put_property("Data", 1.0f64).unwrap();
        assert_eq!(prop.get_property("Data").unwrap(), Variant::R8(1.0));
    }

    #[test]
    fn it_can_get_and_put_arrays() {
        let prop = spawn_instance("Win32_PnPDevicePropertyBooleanArray");
        let test_value = vec![true, false, true];
        prop.put_property("Data", test_value.clone()).unwrap();
        assert_eq!(prop.get_property("Data").unwrap(), test_value.into());

        // Test with an empty array as well.
        let prop = spawn_instance("Win32_PnPDevicePropertyBooleanArray");
        let test_value = Variant::Array(vec![]);
        prop.put_property("Data", test_value.clone()).unwrap();
        assert_eq!(prop.get_property("Data").unwrap(), test_value.into());

        let prop = spawn_instance("Win32_PnPDevicePropertyBinary");
        let test_value = vec![1u8, 2, u8::MAX];
        prop.put_property("Data", test_value.clone()).unwrap();
        assert_eq!(prop.get_property("Data").unwrap(), test_value.into());

        let prop = spawn_instance("Win32_PnPDevicePropertyUint16Array");
        let test_value = vec![1u16, 2, u16::MAX];
        prop.put_property("Data", test_value.clone()).unwrap();
        assert_eq!(prop.get_property("Data").unwrap(), test_value.into());

        let prop = spawn_instance("Win32_PnPDevicePropertyUint32Array");
        let test_value = vec![1u32, 2, u32::MAX];
        prop.put_property("Data", test_value.clone()).unwrap();
        assert_eq!(prop.get_property("Data").unwrap(), test_value.into());

        let prop = spawn_instance("Win32_PnPDevicePropertySint8Array");
        let test_value = vec![1i8, i8::MIN, i8::MAX];
        prop.put_property("Data", test_value.clone()).unwrap();
        assert_eq!(prop.get_property("Data").unwrap(), test_value.into());

        let prop = spawn_instance("Win32_PnPDevicePropertySint16Array");
        let test_value = vec![1i16, i16::MIN, i16::MAX];
        prop.put_property("Data", test_value.clone()).unwrap();
        assert_eq!(prop.get_property("Data").unwrap(), test_value.into());

        let prop = spawn_instance("Win32_PnPDevicePropertySint32Array");
        let test_value = vec![1i32, i32::MIN, i32::MAX];
        prop.put_property("Data", test_value.clone()).unwrap();
        assert_eq!(prop.get_property("Data").unwrap(), test_value.into());

        let prop = spawn_instance("Win32_PnPDevicePropertyReal32Array");
        let test_value = vec![1.0f32, 2.0, -1.0];
        prop.put_property("Data", test_value.clone()).unwrap();
        assert_eq!(prop.get_property("Data").unwrap(), test_value.into());

        let prop = spawn_instance("Win32_PnPDevicePropertyReal64Array");
        let test_value = vec![1.0f64, 2.0, -1.0];
        prop.put_property("Data", test_value.clone()).unwrap();
        assert_eq!(prop.get_property("Data").unwrap(), test_value.into());
    }

    #[test]
    fn it_can_get_and_put_u64_i64_arrays() {
        // Since `Win32_PnPDeviceProperty{Uint64,Sint64}Array` are missing (documented, but do not exist in practice),
        // we create a new class and set custom properties with the needed array types.

        let wmi_con = wmi_con();
        let new_cls_obj = wmi_con.get_object("").unwrap();

        unsafe {
            new_cls_obj
                .inner
                .Put(
                    &HSTRING::from("uValue"),
                    0,
                    ptr::null(),
                    CIM_UINT64.0 | CIM_FLAG_ARRAY.0,
                )
                .unwrap()
        };

        let test_value = vec![1u64, 2, u64::MAX];
        new_cls_obj
            .put_property("uValue", test_value.clone())
            .unwrap();
        assert_eq!(
            new_cls_obj.get_property("uValue").unwrap(),
            test_value.into()
        );

        unsafe {
            new_cls_obj
                .inner
                .Put(
                    &HSTRING::from("iValue"),
                    0,
                    ptr::null(),
                    CIM_SINT64.0 | CIM_FLAG_ARRAY.0,
                )
                .unwrap()
        };

        let test_value = vec![1i64, i64::MIN, i64::MAX];
        new_cls_obj
            .put_property("iValue", test_value.clone())
            .unwrap();
        assert_eq!(
            new_cls_obj.get_property("iValue").unwrap(),
            test_value.into()
        );
    }

    #[test]
    fn it_serialize_instance_nested() {
        let wmi_con = wmi_con();

        #[derive(Debug, Serialize, Default)]
        pub struct Win32_ProcessStartup {
            pub Title: String,
            pub ShowWindow: Option<u16>,
            pub CreateFlags: Option<u32>,
        }

        #[derive(Serialize)]
        struct CreateInput {
            CommandLine: String,
            ProcessStartupInformation: Win32_ProcessStartup,
        }

        // Verify that `Win32_ProcessStartup` can be serialized.
        let startup_info = Win32_ProcessStartup {
            Title: "Pong".to_string(),
            ShowWindow: Some(3),
            CreateFlags: None,
        };

        let startup_info_instance = startup_info
            .serialize(VariantSerializer {
                wmi: &wmi_con,
                instance: None,
            })
            .unwrap();

        let startup_info_instance = match startup_info_instance {
            Variant::Object(startup_info_instance) => startup_info_instance,
            _ => panic!("Unexpected value {:?}", startup_info_instance),
        };

        assert_eq!(
            startup_info_instance.class().unwrap(),
            "Win32_ProcessStartup"
        );
        assert_eq!(
            startup_info_instance.get_property("Title").unwrap(),
            Variant::String(startup_info.Title.clone())
        );

        assert_eq!(
            startup_info_instance.get_property("ShowWindow").unwrap(),
            Variant::UI2(3)
        );
        assert_eq!(
            startup_info_instance.get_property("CreateFlags").unwrap(),
            Variant::Null
        );

        let create_params = CreateInput {
            CommandLine: r#"ping -n 3 127.0.0.1"#.to_string(),
            ProcessStartupInformation: startup_info,
        };

        // Similar to how `exec_class_method` creates these objects.
        let (method_in, method_out) = wmi_con
            .get_object("Win32_Process")
            .unwrap()
            .get_method_in_out("Create")
            .unwrap();

        let method_in = method_in.unwrap().spawn_instance().unwrap();
        let method_out = method_out.unwrap().spawn_instance().unwrap();

        let instance_from_ser = create_params
            .serialize(VariantSerializer {
                wmi: &wmi_con,
                instance: Some(method_in),
            })
            .unwrap();

        let instance_from_ser = match instance_from_ser {
            Variant::Object(instance_from_ser) => instance_from_ser,
            _ => panic!("Unexpected value {:?}", instance_from_ser),
        };

        assert_eq!(
            instance_from_ser.get_property("CommandLine").unwrap(),
            Variant::String(create_params.CommandLine)
        );

        assert!(matches!(
            instance_from_ser
                .get_property("ProcessStartupInformation")
                .unwrap(),
            Variant::Object(_)
        ));

        assert_eq!(
            method_out.get_property("ReturnValue").unwrap(),
            Variant::Null
        );
    }
}
