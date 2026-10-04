use crate::{
    WMIError, WMIResult,
    connection::WMIConnection,
    de::meta::struct_name_and_fields,
    de::wbem_class_de::from_wbem_class_obj,
    result_enumerator::{IWbemClassWrapper, QueryResultEnumerator},
};
use serde::de;
use std::{collections::HashMap, time::Duration};
use windows::Win32::System::Wmi::{
    WBEM_FLAG_FORWARD_ONLY, WBEM_FLAG_RETURN_IMMEDIATELY, WBEM_FLAG_RETURN_WBEM_COMPLETE,
};
use windows::core::BSTR;

#[non_exhaustive]
pub enum FilterValue {
    Bool(bool),
    Number(i64),
    Str(&'static str),
    String(String),
    StrLike(&'static str),
    StringLike(String),
    IsA(&'static str),
}

impl From<String> for FilterValue {
    fn from(value: String) -> Self {
        FilterValue::String(value)
    }
}

impl From<&'static str> for FilterValue {
    fn from(value: &'static str) -> Self {
        FilterValue::Str(value)
    }
}

impl From<i64> for FilterValue {
    fn from(value: i64) -> Self {
        FilterValue::Number(value)
    }
}

impl From<bool> for FilterValue {
    fn from(value: bool) -> Self {
        FilterValue::Bool(value)
    }
}

impl FilterValue {
    /// Create a [FilterValue::IsA] variant form a given type
    ///
    /// ```edition2018
    /// # use std::collections::HashMap;
    /// # use wmi::FilterValue;
    /// # use serde::Deserialize;
    /// # fn main() -> wmi::WMIResult<()> {
    /// #[derive(Deserialize)]
    /// struct Win32_OperatingSystem {}
    ///
    /// let mut filters = HashMap::<String, FilterValue>::new();
    /// filters.insert("TargetInstance".to_owned(), FilterValue::is_a::<Win32_OperatingSystem>()?);
    /// # Ok(())
    /// # }
    /// ```
    ///
    pub fn is_a<'de, T>() -> WMIResult<Self>
    where
        T: serde::Deserialize<'de>,
    {
        let (name, _) = struct_name_and_fields::<T>()?;
        Ok(Self::IsA(name))
    }
}

/// Build an SQL query for the given filters, over the given type (using its name and fields).
/// For example, for:
///
/// # Examples
///
/// For a struct such as:
///
/// ```edition2018
/// # use wmi::*;
/// # use serde::Deserialize;
/// #[derive(Deserialize, Debug)]
/// #[serde(rename = "Win32_OperatingSystem")]
/// #[serde(rename_all = "PascalCase")]
/// struct OperatingSystem {
///     caption: String,
///     debug: bool,
/// }
/// ```
///
/// The resulting query (with no filters) will look like:
/// ```
/// "SELECT Caption, Debug FROM Win32_OperatingSystem";
/// ```
///
pub fn build_query<'de, T>(filters: Option<&HashMap<String, FilterValue>>) -> WMIResult<String>
where
    T: de::Deserialize<'de>,
{
    let (name, fields, optional_where_clause) = get_query_segments::<T>(filters)?;

    let query_text = format!(
        "SELECT {} FROM {} {}",
        fields.join(","),
        name,
        optional_where_clause
    );

    Ok(query_text)
}

/// Build an SQL query for an event notification subscription with the given filters and within polling time, over the given type (using its fields).
/// For example, for:
///
/// # Examples
///
/// For a struct such as:
///
/// ```edition2018
/// # use wmi::*;
/// # use serde::Deserialize;
/// #[derive(Deserialize, Debug)]
/// #[serde(rename = "Win32_ProcessStartTrace")]
/// #[serde(rename_all = "PascalCase")]
/// struct ProcessStartTrace {
///     process_id: u32,
///     process_name: String,
/// }
/// ```
///
/// The resulting query with no filters and no within polling time will look like:
/// ```
/// "SELECT * FROM Win32_ProcessStartTrace";
/// ```
///
/// Conversely, the resulting query with filters and with within polling time will look like:
/// ```
/// "SELECT * FROM Win32_ProcessStartTrace WITHIN 10 WHERE ProcessName = 'explorer.exe'";
/// ```
///
pub fn build_notification_query<'de, T>(
    filters: Option<&HashMap<String, FilterValue>>,
    within: Option<Duration>,
) -> WMIResult<String>
where
    T: de::Deserialize<'de>,
{
    let (name, _, optional_where_clause) = get_query_segments::<T>(filters)?;

    let optional_within_caluse = match within {
        Some(within) => format!("WITHIN {} ", within.as_secs_f64()),
        None => String::new(),
    };

    let query_text = format!(
        "SELECT * FROM {} {}{}",
        name, optional_within_caluse, optional_where_clause
    );

    Ok(query_text)
}

fn get_query_segments<'de, T>(
    filters: Option<&HashMap<String, FilterValue>>,
) -> WMIResult<(&'static str, &'static [&'static str], String)>
where
    T: de::Deserialize<'de>,
{
    let (name, fields) = struct_name_and_fields::<T>()?;

    let optional_where_clause = match filters {
        None => String::new(),
        Some(filters) => {
            if filters.is_empty() {
                String::new()
            } else {
                let mut conditions = vec![];

                for (field, filter) in filters {
                    let value = match filter {
                        FilterValue::Bool(b) => {
                            if *b {
                                "true".to_owned()
                            } else {
                                "false".to_owned()
                            }
                        }
                        FilterValue::Number(n) => format!("{}", n),
                        FilterValue::Str(s) => quote_and_escape_wql_str(s),
                        FilterValue::String(s) => quote_and_escape_wql_str(s),
                        FilterValue::StrLike(s) => {
                            conditions.push(format!(
                                "{} LIKE {}",
                                field,
                                quote_and_escape_wql_str(s)
                            ));
                            continue;
                        }
                        FilterValue::StringLike(s) => {
                            conditions.push(format!(
                                "{} LIKE {}",
                                field,
                                quote_and_escape_wql_str(s)
                            ));
                            continue;
                        }
                        FilterValue::IsA(s) => {
                            conditions.push(format!(
                                "{} ISA {}",
                                field,
                                quote_and_escape_wql_str(s)
                            ));
                            continue;
                        }
                    };

                    conditions.push(format!("{} = {}", field, value));
                }

                // Just to make testing easier.
                conditions.sort();

                format!("WHERE {}", conditions.join(" AND "))
            }
        }
    };

    Ok((name, fields.unwrap_or(&["*"]), optional_where_clause))
}

/// Quote/escape a string for WQL.
///
/// [2.2.1 WQL Query] references [DMTF-DSP0004] ("CIM") which, in reading section "4.11.1 String Constants",
/// seems to only require that `\` and `"` be escaped.  It's underspecified in CIM what happens with unicode
/// values - perhaps `\xNNNN` escaping would be appropriate for a more general purpose CIM string escaping
/// function - but in my testing on Windows 10.0.19041.572, it seems that such values do *not* need to be
/// escaped for WQL, and are treated as their expected unicode values just fine.
///
/// [2.2.1 WQL Query]:  https://docs.microsoft.com/en-us/openspecs/windows_protocols/ms-wmi/6c8a38f4-4ee1-47cb-99f1-b42718a575ce
/// [DMTF-DSP0004]:     https://www.dmtf.org/sites/default/files/standards/documents/DSP0004V2.3_final.pdf
///
/// ```edition2018
/// # use wmi::quote_and_escape_wql_str;
/// assert_eq!(quote_and_escape_wql_str(r#"C:\Path\With"In Name"#), r#""C:\\Path\\With\"In Name""#);
/// ```
pub fn quote_and_escape_wql_str(s: impl AsRef<str>) -> String {
    let s = s.as_ref();
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for ch in s.chars() {
        match ch {
            '\"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            ch => o.push(ch),
        }
    }
    o.push('"');
    o
}

impl WMIConnection {
    /// Execute the given query and return an iterator of WMI pointers.
    /// It's better to use the other query methods (like [`WMIConnection::query`] and [`WMIConnection::filtered_query`]), since this is a relatively low level API.
    ///
    pub fn exec_query(
        &self,
        query: impl AsRef<str>,
    ) -> WMIResult<impl Iterator<Item = WMIResult<IWbemClassWrapper>> + 'static> {
        let query_language = BSTR::from("WQL");
        let query = BSTR::from(query.as_ref());

        let enumerator = unsafe {
            self.svc.ExecQuery(
                &query_language,
                &query,
                WBEM_FLAG_FORWARD_ONLY | WBEM_FLAG_RETURN_IMMEDIATELY,
                &self.ctx.0,
            )?
        };

        Ok(QueryResultEnumerator::new(enumerator))
    }

    /// Execute a free-text query and deserialize the results.
    /// Can be used either with a struct (like [`WMIConnection::query`] and [`WMIConnection::filtered_query`]),
    /// but also with a generic map.
    ///
    /// ```edition2018
    /// # fn main() -> wmi::WMIResult<()> {
    /// # use std::collections::HashMap;
    /// # use wmi::*;
    /// # let con = WMIConnection::new()?;
    /// let results: Vec<HashMap<String, Variant>> = con.raw_query("SELECT Name FROM Win32_OperatingSystem")?;
    /// #   Ok(())
    /// # }
    /// ```
    pub fn raw_query<T>(&self, query: impl AsRef<str>) -> WMIResult<Vec<T>>
    where
        T: de::DeserializeOwned,
    {
        let enumerator = self.exec_query(query)?;

        enumerator
            .map(|item| match item {
                Ok(wbem_class_obj) => wbem_class_obj.into_desr(),
                Err(e) => Err(e),
            })
            .collect()
    }

    /// Query all the objects of type T.
    ///
    /// ```edition2018
    /// # fn main() -> wmi::WMIResult<()> {
    /// use wmi::*;
    /// use serde::Deserialize;
    ///
    /// let con = WMIConnection::new()?;
    ///
    /// #[derive(Deserialize, Debug)]
    /// struct Win32_Process {
    ///     Name: String,
    /// }
    ///
    /// let procs: Vec<Win32_Process> = con.query()?;
    /// #   Ok(())
    /// # }
    /// ```
    pub fn query<T>(&self) -> WMIResult<Vec<T>>
    where
        T: de::DeserializeOwned,
    {
        let query_text = build_query::<T>(None)?;

        self.raw_query(query_text)
    }

    /// Query all the objects of type T, while filtering according to `filters`.
    ///
    /// ```edition2018
    /// # fn main() -> wmi::WMIResult<()> {
    /// # use std::collections::HashMap;
    /// # use wmi::*;
    /// # let con = WMIConnection::new()?;
    /// use serde::Deserialize;
    /// #[derive(Deserialize, Debug)]
    /// struct Win32_Process {
    ///     Name: String,
    /// }
    ///
    /// let mut filters = HashMap::new();
    ///
    /// filters.insert("Name".to_owned(), FilterValue::Str("cargo.exe"));
    ///
    /// let results = con.filtered_query::<Win32_Process>(&filters).unwrap();
    ///
    /// assert!(results.len() >= 1);
    /// #   Ok(())
    /// # }
    /// ```
    pub fn filtered_query<T>(&self, filters: &HashMap<String, FilterValue>) -> WMIResult<Vec<T>>
    where
        T: de::DeserializeOwned,
    {
        let query_text = build_query::<T>(Some(filters))?;

        self.raw_query(query_text)
    }

    /// Get a single object of type T.
    /// If none are found, an error is returned.
    /// If more than one object is found, all but the first are ignored.
    ///
    /// ```edition2018
    /// # fn main() -> wmi::WMIResult<()> {
    /// # let con = WMIConnection::new()?;
    /// # use wmi::*;
    /// use serde::Deserialize;
    /// #[derive(Deserialize)]
    /// struct Win32_OperatingSystem {
    ///     Name: String,
    /// }
    ///
    /// let os = con.get::<Win32_OperatingSystem>()?;
    /// #   Ok(())
    /// # }
    /// ```
    pub fn get<T>(&self) -> WMIResult<T>
    where
        T: de::DeserializeOwned,
    {
        let results = self.query()?;

        results.into_iter().next().ok_or(WMIError::ResultEmpty)
    }

    /// Get a WMI object by path, and return a wrapper around a WMI pointer.
    /// This function is used internally by [`WMIConnection::get_by_path`], which is a higher-level abstraction.
    /// It can also be used to get class objects, see [`WMIConnection::exec_method`] for more info.
    ///
    /// ```edition2018
    /// # fn main() -> wmi::WMIResult<()> {
    /// # use wmi::*;
    /// # use serde::Deserialize;
    /// # let con = WMIConnection::new()?;
    /// let raw_os = con.get_object(r#"\\.\root\cimv2:Win32_OperatingSystem=@"#)?;
    /// assert_eq!(raw_os.class()?, "Win32_OperatingSystem");
    ///
    /// #[derive(Deserialize)]
    /// struct Win32_OperatingSystem {
    ///     Name: String,
    /// }
    ///
    /// let os: Win32_OperatingSystem = raw_os.into_desr()?;
    /// println!("{}", os.Name);
    /// #   Ok(())
    /// # }
    /// ```
    pub fn get_object(&self, object_path: impl AsRef<str>) -> WMIResult<IWbemClassWrapper> {
        let object_path = BSTR::from(object_path.as_ref());

        let mut pcls_obj = None;

        unsafe {
            self.svc.GetObject(
                &object_path,
                WBEM_FLAG_RETURN_WBEM_COMPLETE,
                &self.ctx.0,
                Some(&mut pcls_obj),
                None,
            )?;
        }

        let pcls_ptr = pcls_obj.ok_or(WMIError::NullPointerResult)?;

        let pcls_wrapper = IWbemClassWrapper::new(pcls_ptr);

        Ok(pcls_wrapper)
    }

    /// Create an instance of existing class
    ///
    /// # Example
    /// ```no_run
    /// # use wmi::*;
    /// # fn main() -> WMIResult<()> {
    /// let wmi_con = WMIConnection::with_namespace_path("root\\standardcimv2")?;
    /// let rule_class = wmi_con.get_object("MSFT_NetFirewallHyperVRule")?;
    /// let instance = rule_class.spawn_instance()?;
    /// instance.put_property("ElementName", "Blocking outbound rule")?;
    /// instance.put_property("InstanceID", "{ed7dee72-7ca3-4728-ad16-e6ee5c465c98}")?;
    /// instance.put_property("Action", 4)?;
    /// instance.put_property("Enabled", 1)?;
    /// instance.put_property("Direction", 2)?;
    /// wmi_con.put_instance(&instance)?;
    /// # Ok(())
    /// # }
    /// ````
    pub fn put_instance(&self, instance: &IWbemClassWrapper) -> WMIResult<()> {
        unsafe {
            self.svc.PutInstance(
                &instance.inner,
                WBEM_FLAG_RETURN_WBEM_COMPLETE,
                &self.ctx.0,
                None,
            )?;
        }
        Ok(())
    }

    /// Delete an instance at path
    ///
    /// # Example
    /// ```no_run
    /// # use wmi::*;
    /// # fn main() -> WMIResult<()> {
    /// let wmi_con = WMIConnection::with_namespace_path("root\\standardcimv2")?;
    /// let rule_path = r#"MSFT_NetFirewallHyperVRule.InstanceID="{ed7dee72-7ca3-4728-ad16-e6ee5c465c98}""#;
    /// wmi_con.delete_instance(rule_path)?;
    /// # Ok(())
    /// # }
    /// ````
    pub fn delete_instance(&self, path: &str) -> WMIResult<()> {
        let path = BSTR::from(path);
        unsafe {
            self.svc
                .DeleteInstance(&path, WBEM_FLAG_RETURN_WBEM_COMPLETE, &self.ctx.0, None)?
        };
        Ok(())
    }

    /// Get a WMI object by path, and return a deserialized object.
    /// This is useful when the type of the object at the path in known at compile time.
    ///
    /// ```edition2018
    /// # fn main() -> wmi::WMIResult<()> {
    /// # use wmi::*;
    /// # use serde::Deserialize;
    /// # let con = WMIConnection::new()?;
    /// #[derive(Deserialize)]
    /// struct Win32_OperatingSystem {
    ///     Name: String,
    /// }
    /// let os = con.get_by_path::<Win32_OperatingSystem>(r#"\\.\root\cimv2:Win32_OperatingSystem=@"#)?;
    /// #   Ok(())
    /// # }
    /// ```
    ///
    /// It's possible to have a path where the type of the WMI object is not known at compile time.
    /// Either use [`WMIConnection::get_object`] and [`IWbemClassWrapper::class`] to find out the real type of the object,
    /// or if the object is only of a few possible types, deserialize it to an enum:
    ///
    /// ```edition2018
    /// # fn main() -> wmi::WMIResult<()> {
    /// # use std::collections::HashMap;
    /// # use wmi::*;
    /// # use serde::Deserialize;
    /// # let con = WMIConnection::new()?;
    ///
    /// #[derive(Deserialize, Debug, PartialEq)]
    /// struct Win32_Group {
    ///     __Path: String,
    /// }
    ///
    /// let mut filters = HashMap::new();
    /// filters.insert("Name".into(), "Administrators".into());
    ///
    ///
    /// let admin_group: Win32_Group = con
    ///     .filtered_query(&filters)?
    ///     .into_iter()
    ///     .next()
    ///     .unwrap();
    ///
    /// #[derive(Deserialize, Debug, PartialEq)]
    /// struct Win32_Account {
    ///     __Path: String,
    /// }
    ///
    /// #[derive(Deserialize, Debug, PartialEq)]
    /// struct Win32_UserAccount {
    ///     Caption: String,
    /// }
    ///
    /// #[derive(Deserialize, Debug, PartialEq)]
    /// struct Win32_SystemAccount {
    ///     Caption: String,
    /// }
    ///
    /// #[derive(Deserialize, Debug, PartialEq)]
    /// struct Win32_GroupUser{ }
    ///
    /// // Groups contain multiple types of objects, all inheriting from `Win32_Account`.
    /// let accounts_in_group: Vec<Win32_Account> = con.associators::<_, Win32_GroupUser>(&admin_group.__Path)?;
    ///
    /// #[derive(Deserialize, Debug)]
    /// enum User {
    ///     #[serde(rename = "Win32_SystemAccount")]
    ///     System(Win32_SystemAccount),
    ///     #[serde(rename = "Win32_UserAccount")]
    ///     User(Win32_UserAccount),
    ///     #[serde(rename = "Win32_Group")]
    ///     Group(Win32_Group),
    /// }
    ///
    /// for account in accounts_in_group {
    ///     let user: User = con.get_by_path(&account.__Path)?;
    ///     println!("{:?}", user);
    /// }
    /// #   Ok(())
    /// # }
    /// ```
    pub fn get_by_path<T>(&self, object_path: &str) -> WMIResult<T>
    where
        T: de::DeserializeOwned,
    {
        let wbem_class_obj = self.get_object(object_path)?;

        from_wbem_class_obj(wbem_class_obj)
    }

    /// Query all the associators of type T of the given object.
    /// The `object_path` argument can be provided by querying an object wih it's `__Path` property.
    /// `AssocClass` must be have the name as the connecting association class between the original object and the results.
    /// See <https://docs.microsoft.com/en-us/windows/desktop/cimwin32prov/win32-diskdrivetodiskpartition> for example.
    ///
    /// ```edition2018
    /// # fn main() -> wmi::WMIResult<()> {
    /// # use wmi::*;
    /// # use serde::Deserialize;
    /// # let con = WMIConnection::new()?;
    ///
    /// #[derive(Deserialize, Debug)]
    /// struct Win32_DiskDrive {
    ///     // `__Path` is a WMI-internal property used to uniquely identify objects.
    ///     __Path: String,
    ///     Caption: String,
    /// }
    ///
    /// #[derive(Deserialize, Debug)]
    /// struct Win32_DiskPartition {
    ///     Caption: String,
    /// }
    ///
    /// // There's no need to specify any fields here, since only the name of the struct is used by `associators`.
    /// #[derive(Deserialize, Debug)]
    /// struct Win32_DiskDriveToDiskPartition {}
    ///
    /// let disk = con.get::<Win32_DiskDrive>()?;
    /// let results = con.associators::<Win32_DiskPartition, Win32_DiskDriveToDiskPartition>(&disk.__Path)?;
    /// #   Ok(())
    /// # }
    /// ```
    pub fn associators<ResultClass, AssocClass>(
        &self,
        object_path: &str,
    ) -> WMIResult<Vec<ResultClass>>
    where
        ResultClass: de::DeserializeOwned,
        AssocClass: de::DeserializeOwned,
    {
        let (class_name, _fields) = struct_name_and_fields::<ResultClass>()?;
        let (association_class, _) = struct_name_and_fields::<AssocClass>()?;

        // See more at:
        // https://docs.microsoft.com/en-us/windows/desktop/wmisdk/associators-of-statement
        let query = format!(
            "ASSOCIATORS OF {{{object_path}}} WHERE AssocClass = {association_class} ResultClass = {class_name}",
            object_path = object_path,
            association_class = association_class,
            class_name = class_name
        );

        self.raw_query(query)
    }
}

#[allow(non_snake_case)]
#[allow(non_camel_case_types)]
#[allow(dead_code)]
#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use std::collections::HashMap;
    use windows::Win32::System::Wmi::WBEM_E_INVALID_QUERY;

    use crate::tests::fixtures::*;
    use crate::{Variant, WMIError};

    #[test]
    fn it_works_native_wrapper() {
        let wmi_con = wmi_con();

        let enumerator = wmi_con
            .exec_query("SELECT * FROM Win32_OperatingSystem")
            .unwrap();

        let res = enumerator.into_iter().next().unwrap();
        let w = res.unwrap();
        let mut props = w.list_properties().unwrap();

        props.sort();

        assert_eq!(props.len(), 64);
        assert_eq!(props[..2], ["BootDevice", "BuildNumber"]);
        assert_eq!(props[props.len() - 2..], ["Version", "WindowsDirectory"]);

        let result = serde_json::to_string_pretty(&w);

        assert!(result.is_ok());
        assert!(result.unwrap().len() > 2);

        let desc_prop_name = "Description";
        let description = w.get_property(desc_prop_name).unwrap();

        let _original_desc = match description {
            Variant::String(s) => s,
            _ => panic!("Unexpected variant returned {:?}", description),
        };

        w.put_property(desc_prop_name, "test").unwrap();

        let new_desc = w.get_property(desc_prop_name).unwrap();

        assert_eq!(new_desc, Variant::String("test".to_string()));
    }

    #[test]
    fn it_fails_gracefully() {
        let wmi_con = wmi_con();

        let enumerator = wmi_con
            .exec_query("SELECT NoSuchField FROM Win32_OperatingSystem")
            .unwrap();

        for res in enumerator {
            assert!(res.is_err())
        }
    }

    #[test]
    fn it_fails_gracefully_with_invalid_sql() {
        let wmi_con = wmi_con();

        let enumerator = wmi_con.exec_query("42").unwrap();

        // Show how to detect which error had occurred.
        for res in enumerator {
            match res {
                Ok(_) => assert!(false),
                Err(wmi_err) => match wmi_err {
                    WMIError::HResultError { hres } => {
                        assert_eq!(hres, WBEM_E_INVALID_QUERY.0);
                    }
                    _ => assert!(false),
                },
            }
        }
    }

    #[test]
    fn it_can_query_a_struct() {
        let wmi_con = wmi_con();

        #[derive(Deserialize, Debug)]
        struct Win32_OperatingSystem {
            Caption: String,
        }

        let results = wmi_con.query::<Win32_OperatingSystem>().unwrap();

        for os in results {
            assert!(os.Caption.starts_with("Microsoft Windows"));
        }
    }

    #[test]
    fn it_can_query_a_hashmap() {
        let wmi_con = wmi_con();

        let results: Vec<HashMap<String, Variant>> = wmi_con
            .raw_query("SELECT Name FROM Win32_OperatingSystem")
            .unwrap();

        let results_as_json = serde_json::to_string(&results).unwrap();
        assert!(results_as_json.starts_with(r#"[{"Name":"Microsoft Windows"#));
    }

    #[test]
    fn it_builds_correct_query_for_newtype_struct() {
        #[derive(Deserialize, Debug)]
        struct Win32_OperatingSystem(pub HashMap<String, Variant>);

        let query = build_query::<Win32_OperatingSystem>(None).unwrap();
        let select_part = r#"SELECT * FROM Win32_OperatingSystem "#.to_owned();

        assert_eq!(query, select_part);
    }

    #[test]
    fn it_can_query_a_newtype_struct() {
        let wmi_con = wmi_con();

        #[derive(Deserialize, Debug)]
        struct Win32_OperatingSystem(pub HashMap<String, Variant>);

        let results = wmi_con.query::<Win32_OperatingSystem>().unwrap();

        for os in results {
            match os.0.get("Caption").unwrap() {
                Variant::String(s) => assert!(s.starts_with("Microsoft Windows")),
                _ => assert!(false),
            }
        }
    }

    #[test]
    fn con_query_flatten() {
        // Due to serde#1346, it's not possible to use `query` with a struct that has a `flatten` field,
        // so we need to either use `raw_query` or a newtype struct.

        let wmi_con = wmi_con();

        #[derive(Deserialize, Debug)]
        struct Win32_OperatingSystem {
            Caption: String,
            Name: String,

            #[serde(flatten)]
            extra: HashMap<String, Variant>,
        }

        let system: Vec<Win32_OperatingSystem> = wmi_con
            .raw_query("SELECT * FROM Win32_OperatingSystem")
            .unwrap();
        let system = system.into_iter().next().unwrap();
        assert_ne!(system.Name, "");
        assert!(system.extra.contains_key("BuildNumber"));

        #[derive(Deserialize, Debug)]
        #[serde(rename = "Win32_OperatingSystem")]
        struct Win32_OperatingSystemWrapper(pub Win32_OperatingSystem);
        let system = wmi_con.query::<Win32_OperatingSystemWrapper>().unwrap();
        let system = system.into_iter().next().unwrap();

        assert_ne!(system.0.Name, "");
        assert!(system.0.extra.contains_key("BuildNumber"));
    }

    #[test]
    fn it_fails_gracefully_when_querying_a_struct() {
        let wmi_con = wmi_con();

        #[derive(Deserialize, Debug)]
        struct Win32_OperatingSystem {
            NoSuchField: String,
        }

        let result = wmi_con.query::<Win32_OperatingSystem>();

        assert!(result.is_err());
    }

    #[test]
    fn it_builds_correct_query_without_filters() {
        #[derive(Deserialize, Debug)]
        struct Win32_OperatingSystem {
            Caption: String,
        }

        let query = build_query::<Win32_OperatingSystem>(None).unwrap();
        let select_part = r#"SELECT Caption FROM Win32_OperatingSystem "#.to_owned();

        assert_eq!(query, select_part);
    }

    #[test]
    fn it_builds_correct_notification_query_without_filters() {
        #[derive(Deserialize, Debug)]
        struct Win32_ProcessStartTrace {
            Caption: String,
        }

        let query = build_notification_query::<Win32_ProcessStartTrace>(None, None).unwrap();
        let select_part = r#"SELECT * FROM Win32_ProcessStartTrace "#.to_owned();

        assert_eq!(query, select_part);
    }

    #[test]
    fn it_builds_correct_query() {
        #[derive(Deserialize, Debug)]
        struct Win32_OperatingSystem {
            Caption: String,
        }

        let mut filters = HashMap::new();

        filters.insert("C1".to_owned(), FilterValue::Str("a"));
        filters.insert("C2".to_owned(), FilterValue::String("b".to_owned()));
        filters.insert("C3".to_owned(), FilterValue::Number(42));
        filters.insert("C4".to_owned(), FilterValue::Bool(false));
        filters.insert(
            "C5".to_owned(),
            FilterValue::String(r#"with " and \ chars"#.to_owned()),
        );
        filters.insert("C6".to_owned(), FilterValue::IsA("Class"));
        filters.insert(
            "C7".to_owned(),
            FilterValue::is_a::<Win32_OperatingSystem>().unwrap(),
        );
        filters.insert("C8".to_owned(), FilterValue::StrLike("c"));
        filters.insert("C9".to_owned(), FilterValue::StringLike("d".to_owned()));

        let query = build_query::<Win32_OperatingSystem>(Some(&filters)).unwrap();
        let select_part = r#"SELECT Caption FROM Win32_OperatingSystem "#.to_owned();
        let where_part = r#"WHERE C1 = "a" AND C2 = "b" AND C3 = 42 AND C4 = false AND C5 = "with \" and \\ chars" AND C6 ISA "Class" AND C7 ISA "Win32_OperatingSystem" AND C8 LIKE "c" AND C9 LIKE "d""#;

        assert_eq!(query, select_part + where_part);
    }

    #[test]
    fn it_builds_correct_notification_query() {
        #[derive(Deserialize, Debug)]
        struct Win32_ProcessStartTrace {
            Caption: String,
        }

        let mut filters = HashMap::new();

        filters.insert("C1".to_owned(), FilterValue::Str("a"));
        filters.insert("C2".to_owned(), FilterValue::String("b".to_owned()));
        filters.insert("C3".to_owned(), FilterValue::Number(42));
        filters.insert("C4".to_owned(), FilterValue::Bool(false));
        filters.insert(
            "C5".to_owned(),
            FilterValue::String(r#"with " and \ chars"#.to_owned()),
        );
        filters.insert("C6".to_owned(), FilterValue::IsA("Class"));
        filters.insert(
            "C7".to_owned(),
            FilterValue::is_a::<Win32_ProcessStartTrace>().unwrap(),
        );
        filters.insert("C8".to_owned(), FilterValue::StrLike("c"));
        filters.insert("C9".to_owned(), FilterValue::StringLike("d".to_owned()));

        let query = build_notification_query::<Win32_ProcessStartTrace>(
            Some(&filters),
            Some(Duration::from_secs_f64(10.5)),
        )
        .unwrap();
        let select_part = r#"SELECT * FROM Win32_ProcessStartTrace "#.to_owned();
        let within_part = r#"WITHIN 10.5 "#;
        let where_part = r#"WHERE C1 = "a" AND C2 = "b" AND C3 = 42 AND C4 = false AND C5 = "with \" and \\ chars" AND C6 ISA "Class" AND C7 ISA "Win32_ProcessStartTrace" AND C8 LIKE "c" AND C9 LIKE "d""#;

        assert_eq!(query, select_part + within_part + where_part);
    }

    #[test]
    fn it_can_filter() {
        let wmi_con = wmi_con();

        #[derive(Deserialize, Debug)]
        struct Win32_Process {
            Name: String,
        }

        let mut filters = HashMap::new();

        filters.insert("Name".to_owned(), FilterValue::Str("cargo.exe"));

        let results = wmi_con.filtered_query::<Win32_Process>(&filters).unwrap();

        assert!(results.len() >= 1);

        for proc in results {
            assert_eq!(proc.Name, "cargo.exe");
        }
    }

    #[test]
    fn it_can_query_all_classes() {
        let wmi_con = wmi_con();
        let classes = [
            "CIM_ComputerSystem",
            "Win32_OperatingSystem",
            "Win32_TimeZone",
            "Win32_ComputerSystem",
            "Win32_NetworkAdapter",
            "Win32_NetworkAdapterConfiguration",
            "Win32_LogicalDisk",
            "Win32_PhysicalMemory",
            "Win32_StartupCommand",
            "Win32_NetworkLoginProfile",
            "Win32_Share",
            "Win32_MappedLogicalDisk",
            "Win32_DiskDrive",
            "Win32_IP4RouteTable",
            "Win32_Group",
            // Only works under 64bit.
            // "Win32_ShadowCopy",
        ];

        for class_name in classes.iter() {
            let results: Vec<HashMap<String, Variant>> = wmi_con
                .raw_query(format!("SELECT * FROM {}", class_name))
                .unwrap();

            for res in results {
                match res.get("Caption") {
                    Some(Variant::String(_)) | Some(Variant::Null) => assert!(true),
                    _ => assert!(false),
                }
            }
        }

        // Associators. TODO: Support this in the desr logic (so a Disk can have `Partitions`).
        let associators_classes = [
            "Win32_DiskDriveToDiskPartition",
            "Win32_LogicalDiskToPartition",
        ];

        for class_name in associators_classes.iter() {
            let results: Vec<HashMap<String, Variant>> = wmi_con
                .raw_query(format!("SELECT * FROM {}", class_name))
                .unwrap();

            for res in results {
                match res.get("Antecedent") {
                    Some(Variant::String(s)) => assert_ne!(s, ""),
                    _ => assert!(false),
                }
            }
        }

        let results: Vec<HashMap<String, Variant>> =
            wmi_con.raw_query("SELECT * FROM Win32_GroupUser").unwrap();

        for res in results {
            match res.get("GroupComponent") {
                Some(Variant::String(s)) => assert_ne!(s, ""),
                _ => assert!(false),
            }

            match res.get("PartComponent") {
                Some(Variant::String(s)) => assert_ne!(s, ""),
                _ => assert!(false),
            }
        }
    }

    #[test]
    fn con_get_return_a_single_object() {
        let wmi_con = wmi_con();

        #[derive(Deserialize, Debug)]
        struct Win32_Process {
            Name: String,
        }

        let proc = wmi_con.get::<Win32_Process>().unwrap();

        assert_ne!(proc.Name, "");
    }

    #[test]
    fn con_error_for_query_without_struct() {
        let wmi_con = wmi_con();

        let res: Result<Vec<HashMap<String, Variant>>, _> = wmi_con.query();

        assert!(res.is_err());
    }

    #[test]
    fn it_can_query_associators() {
        let wmi_con = wmi_con();

        #[derive(Deserialize, Debug)]
        struct Win32_DiskDrive {
            __Path: String,
            Caption: String,
        }

        #[derive(Deserialize, Debug)]
        struct Win32_DiskPartition {
            Caption: String,
        }

        #[derive(Deserialize, Debug)]
        struct Win32_DiskDriveToDiskPartition {}

        let disk = wmi_con.get::<Win32_DiskDrive>().unwrap();

        let results = wmi_con
            .associators::<Win32_DiskPartition, Win32_DiskDriveToDiskPartition>(&disk.__Path)
            .unwrap();

        assert!(results.len() >= 1);

        for part in results {
            // We want to check that the output is in the format "Disk #1, Partition #1".
            // However, it is localised so we simply check if there are two or more '#'.
            // This means there are at least two sublevels in the hierarchy being enumerated.
            assert!(part.Caption.chars().filter(|x| *x == '#').count() >= 2);
        }
    }

    #[test]
    fn it_can_query_correct_variant_types() {
        let wmi_con = wmi_con();
        let mut results: Vec<HashMap<String, Variant>> = wmi_con
            .raw_query("SELECT CPUScore FROM Win32_WinSAT")
            .unwrap();

        match results.pop().unwrap().values().next() {
            Some(&Variant::R4(_v)) => assert!(true),
            _ => assert!(false),
        }

        let mut results: Vec<HashMap<String, Variant>> = wmi_con
            .raw_query("SELECT FreePhysicalMemory FROM Win32_OperatingSystem")
            .unwrap();

        match results.pop().unwrap().values().next() {
            Some(&Variant::UI8(_v)) => assert!(true),
            _ => {
                assert!(false)
            }
        }

        let mut results: Vec<HashMap<String, Variant>> = wmi_con
            .raw_query("SELECT MaxNumberOfProcesses FROM Win32_OperatingSystem")
            .unwrap();

        match results.pop().unwrap().values().next() {
            Some(&Variant::UI4(_v)) => assert!(true),
            _ => assert!(false),
        }

        let mut results: Vec<HashMap<String, Variant>> = wmi_con
            .raw_query("SELECT ForegroundApplicationBoost FROM Win32_OperatingSystem")
            .unwrap();

        match results.pop().unwrap().values().next() {
            Some(&Variant::UI1(_v)) => assert!(true),
            _ => assert!(false),
        }

        let mut results: Vec<HashMap<String, Variant>> = wmi_con
            .raw_query("SELECT Roles FROM CIM_ComputerSystem")
            .unwrap();

        match results.pop().unwrap().values().next() {
            Some(Variant::Array(v)) => match v.get(0) {
                None | Some(Variant::String(_)) => assert!(true),
                _ => assert!(false),
            },
            _ => assert!(false),
        }

        let mut results: Vec<HashMap<String, Variant>> = wmi_con
            .raw_query("SELECT * FROM CIM_ComputerSystem")
            .unwrap();

        match results.pop().unwrap().get("OEMLogoBitmap") {
            Some(Variant::Array(v)) => {
                assert!(v.is_empty());
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn it_can_query_floats() {
        let wmi_con = wmi_con();

        #[derive(Deserialize, Debug)]
        struct Win32_WinSAT {
            CPUScore: f32,
        }

        let sat = wmi_con.get::<Win32_WinSAT>().unwrap();
        assert!(sat.CPUScore >= 0.0);
    }

    #[test]
    fn it_can_query_arrays() {
        let wmi_con = wmi_con();

        #[derive(Deserialize, Debug)]
        struct Win32_OperatingSystem {
            MUILanguages: Vec<String>,
        }

        let os = wmi_con.get::<Win32_OperatingSystem>().unwrap();
        assert!(!os.MUILanguages.is_empty());

        #[derive(Deserialize, Debug)]
        struct Win32_ComputerSystem {
            BootStatus: Vec<u16>,
            PowerManagementCapabilities: Vec<u16>,
        }

        let cs = wmi_con.get::<Win32_ComputerSystem>().unwrap();
        assert!(!cs.BootStatus.is_empty());
        assert!(cs.PowerManagementCapabilities.is_empty());
    }

    #[test]
    fn it_can_query_slashes_and_unicode() {
        let tmp_dir = tempdir::TempDir::new("PlayStation™Now").unwrap();
        let wmi_con = wmi_con();
        let tmp_dir_path = tmp_dir.path().to_string_lossy().to_string();

        #[derive(Deserialize, Debug)]
        struct Win32_Directory {
            Name: String,
        }

        let mut filters = HashMap::new();
        filters.insert(
            String::from("Name"),
            FilterValue::String(tmp_dir_path.clone()),
        );
        let directory = wmi_con
            .filtered_query::<Win32_Directory>(&filters)
            .unwrap()
            .pop()
            .unwrap();
        assert_eq!(directory.Name.to_lowercase(), tmp_dir_path.to_lowercase());
    }

    #[test]
    fn con_get_return_an_object_by_path() {
        let wmi_con = wmi_con();

        #[derive(Deserialize, Debug, PartialEq)]
        struct Win32_Process {
            __Path: String,
            Name: String,
            ProcessId: i64,
        }

        let procs = wmi_con.query::<Win32_Process>().unwrap();

        let proc = &procs[3];

        let proc_by_path = wmi_con.get_by_path::<Win32_Process>(&proc.__Path).unwrap();

        assert_eq!(&proc_by_path, proc);

        let proc_by_path_hashmap: HashMap<String, Variant> =
            wmi_con.get_by_path(&proc.__Path).unwrap();

        assert_eq!(
            proc_by_path_hashmap.get("ProcessId").unwrap(),
            &Variant::UI4(proc.ProcessId as _)
        );
    }

    #[test]
    fn con_get_return_an_object_by_path_from_actual_path() {
        let wmi_con = wmi_con();

        #[derive(Deserialize, Debug, PartialEq)]
        struct Win32_OperatingSystem {
            Caption: String,
        }

        let os = wmi_con
            .get_by_path::<Win32_OperatingSystem>(r#"\\.\root\cimv2:Win32_OperatingSystem=@"#)
            .unwrap();

        assert!(os.Caption.contains("Microsoft Windows"));
    }

    #[test]
    fn con_get_return_a_raw_object_by_path_from_actual_path() {
        let wmi_con = wmi_con();

        #[derive(Deserialize, Debug, PartialEq)]
        struct Win32_Account {
            __Path: String,
        }

        #[derive(Deserialize, Debug, PartialEq)]
        struct Win32_Group {
            __Path: String,
            Caption: String,
        }

        #[derive(Deserialize, Debug, PartialEq)]
        struct Win32_UserAccount {
            Caption: String,
        }

        #[derive(Deserialize, Debug, PartialEq)]
        struct Win32_SystemAccount {
            Caption: String,
        }

        #[derive(Deserialize, Debug, PartialEq)]
        struct Win32_GroupUser {}

        let mut filters = HashMap::new();
        filters.insert("Name".into(), "Administrators".into());

        let group: Win32_Group = wmi_con
            .filtered_query(&filters)
            .unwrap()
            .into_iter()
            .next()
            .unwrap();

        let accounts_in_group: Vec<Win32_Account> = wmi_con
            .associators::<_, Win32_GroupUser>(&group.__Path)
            .unwrap();

        #[derive(Deserialize, Debug)]
        enum User {
            #[serde(rename = "Win32_SystemAccount")]
            System(Win32_SystemAccount),
            #[serde(rename = "Win32_UserAccount")]
            User(Win32_UserAccount),
            #[serde(rename = "Win32_Group")]
            Group(Win32_Group),
        }

        for account in accounts_in_group {
            let raw_account = wmi_con.get_object(&account.__Path).unwrap();

            // Completely dynamic.
            match raw_account.class().unwrap().as_str() {
                "Win32_UserAccount" | "Win32_SystemAccount" | "Win32_Group" => {
                    // OK.
                }
                _ => panic!(),
            };

            let _account_as_hashmap: HashMap<String, Variant> = raw_account.into_desr().unwrap();

            // Enum based desr.
            let _raw_account: User = wmi_con.get_by_path(&account.__Path).unwrap();
        }
    }

    #[test]
    fn test_put_and_delete_instance() {
        let wmi_con = WMIConnection::with_namespace_path("root\\standardcimv2").unwrap();
        let rule_class = wmi_con.get_object("MSFT_NetFirewallHyperVRule").unwrap();
        let instance = rule_class.spawn_instance().unwrap();
        instance
            .put_property("ElementName", "Blocking outbound rule")
            .unwrap();
        instance
            .put_property("InstanceID", "{ed7dee72-7ca3-4728-ad16-e6ee5c465c98}")
            .unwrap();
        instance.put_property("Action", 4).unwrap();
        instance.put_property("Enabled", 0).unwrap();
        instance.put_property("Direction", 2).unwrap();
        wmi_con.put_instance(&instance).unwrap();

        let rule_path =
            r#"MSFT_NetFirewallHyperVRule.InstanceID="{ed7dee72-7ca3-4728-ad16-e6ee5c465c98}""#;
        wmi_con.delete_instance(rule_path).unwrap();
    }
}
