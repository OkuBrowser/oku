use serde::{Serialize, de};
use windows::core::BSTR;

use crate::{
    Variant, WMIConnection, WMIError, WMIResult, de::meta::struct_name_and_fields,
    result_enumerator::IWbemClassWrapper, ser::variant_ser::VariantSerializer,
};

impl WMIConnection {
    /// Wrapper for WMI's [ExecMethod](https://learn.microsoft.com/en-us/windows/win32/api/wbemcli/nf-wbemcli-iwbemservices-execmethod) function.
    ///
    /// This function is used internally by [`WMIConnection::exec_class_method`] and [`WMIConnection::exec_instance_method`],
    /// which are a higher-level abstraction, dealing with Rust data types instead of raw Variants, that should be preferred to use.
    ///
    /// In the case of a class ("static") method, `object_path` should be name of the class.
    ///
    /// Returns `None` if the method has no out parameters and a `void` return type, and an [`IWbemClassWrapper`] containing the output otherwise.
    /// A method with a return type other than `void` will always have a generic property named `ReturnValue` in the output class wrapper with the return value of the WMI method call.
    ///
    /// ```edition2021
    /// # use std::collections::HashMap;
    /// # use wmi::{Variant, WMIConnection, WMIResult};
    /// # fn main() -> WMIResult<()> {
    /// # let wmi_con = WMIConnection::new()?;
    /// let in_params = wmi_con
    ///     .get_object("Win32_Process")?
    ///     .get_method("Create")?
    ///     .unwrap()
    ///     .spawn_instance()?;
    /// in_params.put_property("CommandLine", "explorer.exe".to_string())?;
    ///
    /// // Because Create has a return value and out parameters, the Option returned will never be None.
    /// // Note: The Create call can be unreliable, so consider using another means of starting processes.
    /// let out = wmi_con.exec_method("Win32_Process", "Create", Some(&in_params))?.unwrap();
    /// println!("The return code of the Create call is {:?}", out.get_property("ReturnValue")?);
    /// # Ok(())
    /// # }
    /// ```
    pub fn exec_method(
        &self,
        object_path: impl AsRef<str>,
        method: impl AsRef<str>,
        in_params: Option<&IWbemClassWrapper>,
    ) -> WMIResult<Option<IWbemClassWrapper>> {
        let object_path = BSTR::from(object_path.as_ref());

        // In the case of a method with no out parameters and a VOID return type, there will be no out-parameters object
        let method = BSTR::from(method.as_ref());

        let mut output = None;
        unsafe {
            self.svc.ExecMethod(
                &object_path,
                &method,
                Default::default(),
                &self.ctx.0,
                in_params.as_ref().map(|param| &param.inner),
                Some(&mut output),
                None,
            )?;
        }

        Ok(output.map(IWbemClassWrapper::new))
    }

    /// Executes a method of a WMI class not tied to any specific instance. Examples include
    /// [Create](https://learn.microsoft.com/en-us/windows/win32/cimwin32prov/create-method-in-class-win32-process) of `Win32_Process`
    /// and [AddPrinterConnection](https://learn.microsoft.com/en-us/windows/win32/cimwin32prov/addprinterconnection-method-in-class-win32-printer) of `Win32_Printer`.
    ///
    /// A method with a return type other than `void` will always try to populate a generic property named `ReturnValue` in the output object with the return value of the WMI method call.
    /// If the method call has a `void` return type and no out parameters, the only acceptable type for `Out` is `()`.
    ///
    /// Arrays, Options, unknowns, and nested objects cannot be passed as input parameters due to limitations in how variants are constructed by `windows-rs`.
    ///
    /// This function uses [`WMIConnection::exec_method`] internally, with the name of the method class being the instance path, as is expected by WMI.
    ///
    /// ```edition2021
    /// # use serde::{Deserialize, Serialize};
    /// # use wmi::{Variant, WMIConnection, WMIResult};
    /// #[derive(Serialize)]
    /// # #[allow(non_snake_case)]
    /// struct CreateInput {
    ///     CommandLine: String
    /// }
    ///
    /// #[derive(Deserialize)]
    /// # #[allow(non_snake_case)]
    /// struct CreateOutput {
    ///     ReturnValue: u32,
    ///     ProcessId: u32
    /// }
    ///
    /// #[derive(Deserialize)]
    /// # #[allow(non_camel_case_types)]
    /// struct Win32_Process;
    ///
    /// # fn main() -> WMIResult<()> {
    /// # let wmi_con = WMIConnection::new()?;
    /// // Note: The Create call can be unreliable, so consider using another means of starting processes.
    /// let input = CreateInput {
    ///     CommandLine: "explorer.exe".to_string()
    /// };
    /// let output: CreateOutput = wmi_con
    ///     .exec_class_method::<Win32_Process, _>("Create", input)?;
    ///
    /// println!("The return code of the Create call is {}", output.ReturnValue);
    /// println!("The ID of the created process is: {}", output.ProcessId);
    /// # Ok(())
    /// # }
    /// ```
    pub fn exec_class_method<Class, Out>(
        &self,
        method: impl AsRef<str>,
        in_params: impl Serialize,
    ) -> WMIResult<Out>
    where
        Class: de::DeserializeOwned,
        Out: de::DeserializeOwned,
    {
        let (class, _) = struct_name_and_fields::<Class>()?;
        self.exec_instance_method::<Class, _>(class, method, in_params)
    }

    /// Executes a WMI method on a specific instance of a class. Examples include
    /// [GetSupportedSize](https://learn.microsoft.com/en-us/windows-hardware/drivers/storage/msft-Volume-getsupportedsizes) of `MSFT_Volume`
    /// and [Pause](https://learn.microsoft.com/en-us/windows/win32/cimwin32prov/pause-method-in-class-win32-printer) of `Win32_Printer`.
    ///
    /// `object_path` is the `__Path` variable of the class instance on which the method is being called, which can be obtained from a WMI query.
    ///
    /// A method with a return type other than `void` will always try to populate a generic property named `ReturnValue` in the output object with the return value of the WMI method call.
    /// If the method call has a `void` return type and no out parameters, the only acceptable type for `Out` is `()`.
    ///
    /// Arrays, Options, unknowns, and nested objects cannot be passed as input parameters due to limitations in how variants are constructed by `windows-rs`.
    ///
    /// ```edition2021
    /// # use serde::{Deserialize, Serialize};
    /// # use wmi::{FilterValue, Variant, WMIConnection, WMIResult};
    /// #[derive(Deserialize)]
    /// # #[allow(non_snake_case)]
    /// struct PrinterOutput {
    ///     ReturnValue: u32
    /// }
    ///
    /// #[derive(Deserialize)]
    /// # #[allow(non_camel_case_types, non_snake_case)]
    /// struct Win32_Printer {
    ///     __Path: String
    /// }
    ///
    /// # fn main() -> WMIResult<()> {
    /// # let wmi_con = WMIConnection::new()?;
    /// let printers: Vec<Win32_Printer> = wmi_con.query()?;
    ///
    /// for printer in printers {
    ///     let output: PrinterOutput = wmi_con.exec_instance_method::<Win32_Printer, _>(&printer.__Path, "Pause", ())?;
    ///     println!("Pausing the printer returned {}", output.ReturnValue);
    ///
    ///     let output: PrinterOutput = wmi_con.exec_instance_method::<Win32_Printer, _>(&printer.__Path, "Resume", ())?;
    ///     println!("Resuming the printer returned {}", output.ReturnValue);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn exec_instance_method<Class, Out>(
        &self,
        object_path: impl AsRef<str>,
        method: impl AsRef<str>,
        in_params: impl Serialize,
    ) -> WMIResult<Out>
    where
        Class: de::DeserializeOwned,
        Out: de::DeserializeOwned,
    {
        let (class, _) = struct_name_and_fields::<Class>()?;
        let method = method.as_ref();

        // See https://learn.microsoft.com/en-us/windows/win32/api/wbemcli/nf-wbemcli-iwbemclassobject-getmethod
        // GetMethod can only be called on a class definition, so we retrieve that before retrieving a specific object.
        let instance = match self.get_object(class)?.get_method(method)? {
            None => None,
            Some(method_class) => {
                let instance = method_class.spawn_instance()?;

                let serializer = VariantSerializer {
                    wmi: self,
                    instance: Some(instance),
                };

                match in_params.serialize(serializer) {
                    Ok(Variant::Object(instance)) => Some(instance),
                    Ok(other) => {
                        return Err(WMIError::ConvertVariantError(format!(
                            "Unexpected serializer output: {:?}",
                            other
                        )));
                    }
                    Err(e) => return Err(WMIError::ConvertVariantError(e.to_string())),
                }
            }
        };

        let output = self.exec_method(object_path, method, instance.as_ref())?;

        match output {
            Some(class_wrapper) => Ok(class_wrapper.into_desr()?),
            None => Out::deserialize(Variant::Empty),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Variant;
    use crate::tests::fixtures::wmi_con;
    use serde::{Deserialize, Serialize};
    use std::thread::sleep;
    use std::time::Duration;

    #[derive(Deserialize)]
    struct Win32_Process {
        __Path: String,
        HandleCount: u32,
    }

    #[derive(Debug, Serialize, Default)]
    pub struct Win32_ProcessStartup {
        CreateFlags: u32,
    }

    #[derive(Serialize)]
    struct CreateInput {
        CommandLine: String,
        ProcessStartupInformation: Win32_ProcessStartup,
    }

    #[derive(Deserialize)]
    struct CreateOutput {
        ReturnValue: u32,
        ProcessId: u32,
    }

    #[test]
    fn it_exec_methods_native() {
        let wmi_con = wmi_con();

        let in_params = wmi_con
            .get_object("Win32_Process")
            .unwrap()
            .get_method("Create")
            .unwrap()
            .unwrap()
            .spawn_instance()
            .unwrap();

        in_params
            .put_property("CommandLine", "explorer.exe".to_string())
            .unwrap();

        let out = wmi_con
            .exec_method("Win32_Process", "Create", Some(&in_params))
            .unwrap();

        let return_value = out.unwrap().get_property("ReturnValue").unwrap();

        assert!(matches!(return_value, Variant::UI4(0)));
    }

    #[test]
    fn it_exec_methods() {
        let wmi_con = wmi_con();
        const CREATE_SUSPENDED: u32 = 4;

        let in_params = CreateInput {
            CommandLine: "explorer.exe".to_string(),
            ProcessStartupInformation: Win32_ProcessStartup {
                CreateFlags: CREATE_SUSPENDED,
            },
        };
        let out: CreateOutput = wmi_con
            .exec_class_method::<Win32_Process, _>("Create", &in_params)
            .unwrap();

        assert_eq!(out.ReturnValue, 0);

        let query = format!(
            "SELECT * FROM Win32_Process WHERE ProcessId = {}",
            out.ProcessId
        );

        let process = &wmi_con.raw_query::<Win32_Process>(&query).unwrap()[0];

        // Since we started the process as suspended, it will not have any open handles.
        assert_eq!(process.HandleCount, 0);

        let _: () = wmi_con
            .exec_instance_method::<Win32_Process, _>(&process.__Path, "Terminate", ())
            .unwrap();

        // It can take a moment for the process to terminate, so we retry the query a few times.
        for _ in 0..10 {
            if wmi_con.raw_query::<Win32_Process>(&query).unwrap().len() == 0 {
                break;
            }
            sleep(Duration::from_millis(100));
        }

        assert!(wmi_con.raw_query::<Win32_Process>(&query).unwrap().len() == 0);
    }

    #[test]
    fn it_exec_with_u8_arrays() {
        let wmi_con = wmi_con();

        #[derive(Deserialize)]
        struct StdRegProv;

        #[derive(Deserialize, Serialize)]
        struct GetBinaryValue {
            sSubKeyName: String,
            sValueName: String,
        }

        #[derive(Deserialize)]
        struct GetBinaryValueOut {
            uValue: Vec<u8>,
        }

        let get_binary_value_params = GetBinaryValue {
            sSubKeyName: r#"SYSTEM\CurrentControlSet\Control\Windows"#.to_string(),
            sValueName: "FullProcessInformationSID".to_string(),
        };

        let value: GetBinaryValueOut = wmi_con
            .exec_class_method::<StdRegProv, _>("GetBinaryValue", &get_binary_value_params)
            .unwrap();

        assert!(value.uValue.len() > 0, "Expected to get a non-empty value");

        #[derive(Deserialize, Serialize)]
        struct SetBinaryValue {
            sSubKeyName: String,
            sValueName: String,
            uValue: Vec<u8>,
        }

        #[derive(Deserialize)]
        struct SetBinaryValueOut {
            ReturnValue: u32,
        }

        let test_value_name = format!("{}.test", get_binary_value_params.sValueName);
        let test_value = vec![0, 1, 2, 3];

        let set_binary_value_params = SetBinaryValue {
            sSubKeyName: get_binary_value_params.sSubKeyName,
            sValueName: test_value_name,
            uValue: test_value,
        };

        let value: SetBinaryValueOut = wmi_con
            .exec_class_method::<StdRegProv, _>("SetBinaryValue", &set_binary_value_params)
            .unwrap();

        assert_eq!(value.ReturnValue, 0);

        let get_test_binary_value_params = GetBinaryValue {
            sSubKeyName: set_binary_value_params.sSubKeyName,
            sValueName: set_binary_value_params.sValueName,
        };

        let value: GetBinaryValueOut = wmi_con
            .exec_class_method::<StdRegProv, _>("GetBinaryValue", &get_test_binary_value_params)
            .unwrap();

        assert_eq!(value.uValue, set_binary_value_params.uValue);

        wmi_con
            .exec_class_method::<StdRegProv, ()>("DeleteValue", &get_test_binary_value_params)
            .unwrap();
    }

    #[test]
    fn it_exec_with_object_arrays() {
        let wmi_con = wmi_con();

        #[derive(Deserialize)]
        struct StdRegProv;

        #[derive(Serialize)]
        struct GetSecurityDescriptor {
            sSubKeyName: String,
        }

        #[derive(Deserialize, Debug)]
        struct __Trustee {
            Name: String,
        }

        #[derive(Deserialize, Debug)]
        struct __ACE {
            Trustee: __Trustee,
        }

        #[derive(Deserialize, Debug)]
        struct __SecurityDescriptor {
            DACL: Vec<__ACE>,
        }

        #[derive(Deserialize, Debug)]
        struct GetSecurityDescriptorOut {
            Descriptor: Option<__SecurityDescriptor>,
        }

        let params = GetSecurityDescriptor {
            sSubKeyName: r#"SECURITY"#.to_string(),
        };

        let value: GetSecurityDescriptorOut = wmi_con
            .exec_class_method::<StdRegProv, _>("GetSecurityDescriptor", &params)
            .unwrap();

        let dacl = value.Descriptor.unwrap().DACL;

        assert!(dacl.len() > 0, "Expected to get a non-empty value");
        assert!(dacl.iter().find(|ac| ac.Trustee.Name == "SYSTEM").is_some());
    }
}
