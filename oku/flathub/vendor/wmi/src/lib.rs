//! # WMI-rs
//!
//! [WMI] is a management API for Windows-based operating systems.
//! This crate provides both a high-level Rust API (focused on data retrieval, event queries and method execution),
//! as well as a mid-level API for easy access to native WMI methods.
//!
//! This crate also uses `serde` for transforming between native WMI class objects and plain Rust structs.
//!
//! # Quickstart
//!
//! Before using WMI, a connection must be created.
//!
//! ```rust
//! # fn main() -> wmi::WMIResult<()> {
//! use wmi::WMIConnection;
//! let wmi_con = WMIConnection::new()?;
//! #   Ok(())
//! # }
//! ```
//!
//! There are multiple ways to get data from the OS using this crate.
//!
//! ## Operating on untyped `Variant`s
//!
//! WMI data model is based on COM's [`VARIANT`] Type, which is a struct capable of holding
//! many types of data.
//!
//! This crate provides the analogous [`wmi::Variant`] enum.
//!
//! Using this enum, we can execute a simple WMI query and inspect the results.
//!
//! ```edition2018
//! # fn main() -> wmi::WMIResult<()> {
//! use wmi::*;
//! let wmi_con = WMIConnection::new()?;
//! use std::collections::HashMap;
//! use wmi::Variant;
//! let results: Vec<HashMap<String, Variant>> = wmi_con.raw_query("SELECT * FROM Win32_OperatingSystem").unwrap();
//!
//! for os in results {
//!     println!("{:#?}", os);
//! }
//! #   Ok(())
//! # }
//! ```
//!
//! ## Using strongly typed data structures
//!
//! Using `serde`, it is possible to return a struct representing the the data.
//!
//! ```edition2018
//! # fn main() -> wmi::WMIResult<()> {
//! # use wmi::*;
//! # let wmi_con = WMIConnection::new()?;
//! use serde::Deserialize;
//! # #[cfg(feature = "chrono")]
//! use wmi::WMIDateTime;
//! # #[cfg(all(feature = "time", not(feature = "chrono")))]
//! # use wmi::WMIOffsetDateTime as WMIDateTime;
//!
//! #[derive(Deserialize, Debug)]
//! #[serde(rename = "Win32_OperatingSystem")]
//! #[serde(rename_all = "PascalCase")]
//! struct OperatingSystem {
//!     caption: String,
//!     debug: bool,
//!     last_boot_up_time: WMIDateTime,
//! }
//!
//! let results: Vec<OperatingSystem> = wmi_con.query()?;
//!
//! for os in results {
//!     println!("{:#?}", os);
//! }
//! #   Ok(())
//! # }
//! ```
//!
//! Because the name of the struct given to `serde` matches the [WMI class] name, the SQL query
//! can be inferred.
//!
//! [WMI]: https://docs.microsoft.com/en-us/windows/desktop/wmisdk/about-wmi
//! [Creating a WMI Application Using C++]: https://docs.microsoft.com/en-us/windows/desktop/wmisdk/creating-a-wmi-application-using-c-
//! [`VARIANT`]: https://docs.microsoft.com/en-us/windows/desktop/api/oaidl/ns-oaidl-tagvariant
//! [WMI class]: https://docs.microsoft.com/en-us/windows/desktop/cimwin32prov/win32-operatingsystem
//!
//! # High-level API functions
//!
//! Queries and data retrieval - [`WMIConnection::query`], [`WMIConnection::filtered_query`], [`WMIConnection::get`], [`WMIConnection::get_by_path`] and [`WMIConnection::associators`].
//!
//! Event listening - [`WMIConnection::notification`] and [`WMIConnection::filtered_notification`].
//!
//! Method calling - [`WMIConnection::exec_class_method`] and [`WMIConnection::exec_instance_method`].
//!
//! Most of these have `async` versions as well.
//!
//! # Mid-level API functions
//!
//! Queries and data retrieval - [`WMIConnection::get_object`], [`WMIConnection::exec_query`] and [`WMIConnection::exec_query_async`].
//!
//! Event listening - [`WMIConnection::exec_notification_query`] and [`WMIConnection::exec_notification_query_async`].
//!
//! Method calling - [`WMIConnection::exec_method`] and [`IWbemClassWrapper`].
//!
//! These try to keep the names of the underlying WMI machinery.
//!
//! # Subscribing to event notifications
//!
//! Using this crate you can subscribe to events notifications generated upon changes in WMI data and services.
//!
//! When querying for events, it is important to remember there are two types of event notifications. \
//! The first one is notifications about changes to the standard WMI data models. They are called **intrinsic events**. \
//! Events like [`__InstanceCreationEvent`] or [`__NamespaceDeletionEvent`] are examples of such events.
//!
//! The second type is notifications about changes made by providers. They are called **extrinsic events**.  \
//! Any WMI class deriving from the [`__ExtrinsicEvent`] class is an extrinsic event. \
//! An example of such events are [`Win32_ProcessStartTrace`] and [`Win32_VolumeChangeEvent`] classes.
//!
//! For more information about event queries, [see here](https://docs.microsoft.com/en-us/windows/win32/wmisdk/receiving-a-wmi-event#event-queries).\
//! You can use [WMI Code Creator] to see available events and create queries for them.
//!
//! The [`notification`] method returns an iterator that waits for any incoming events
//! resulting from the provided query. Loops reading from this iterator will not end until they are broken.
//!
//! An example of subscribing to an intrinsic event notification for every new [`Win32_Process`]
//! ```edition2018
//! # use wmi::*;
//! # #[cfg(not(feature = "test"))]
//! # fn main() {}
//! # #[cfg(feature = "test")]
//! # fn main() -> WMIResult<()>{
//! # use serde::Deserialize;
//! # use std::{collections::HashMap, time::Duration};
//! # let wmi_con = WMIConnection::new()?;
//! #[derive(Deserialize, Debug)]
//! #[serde(rename = "__InstanceCreationEvent")]
//! #[serde(rename_all = "PascalCase")]
//! struct NewProcessEvent {
//!     target_instance: Process
//! }
//!
//! #[derive(Deserialize, Debug)]
//! #[serde(rename = "Win32_Process")]
//! #[serde(rename_all = "PascalCase")]
//! struct Process {
//!     process_id: u32,
//!     name: String,
//!     executable_path: Option<String>,
//! }
//!
//! let mut filters = HashMap::<String, FilterValue>::new();
//!
//! filters.insert("TargetInstance".to_owned(), FilterValue::is_a::<Process>()?);
//!
//! let iterator = wmi_con.filtered_notification::<NewProcessEvent>(&filters, Some(Duration::from_secs(1)))?;
//! # tests::start_test_program();
//!
//! for result in iterator {
//!     let process = result?.target_instance;
//!     println!("New process!");
//!     println!("PID:        {}", process.process_id);
//!     println!("Name:       {}", process.name);
//!     println!("Executable: {:?}", process.executable_path);
//! #     break;
//! } // Loop will end only on error
//! # Ok(())
//! # }
//! ```
//!
//! An example of subscribing to an extrinsic event notification [`Win32_ProcessStartTrace`]
//! ```edition2018
//! # use wmi::*;
//! # #[cfg(not(feature = "test"))]
//! # fn main() {}
//! # #[cfg(feature = "test")]
//! # fn main() -> WMIResult<()> {
//! # tests::ignore_access_denied(run())
//! # }
//! # #[cfg(feature = "test")]
//! # fn run() -> WMIResult<()>{
//! # use serde::Deserialize;
//! # use std::{collections::HashMap, time::Duration};
//! # let wmi_con = WMIConnection::new()?;
//! #[derive(Deserialize, Debug)]
//! #[serde(rename = "Win32_ProcessStartTrace")]
//! #[serde(rename_all = "PascalCase")]
//! struct ProcessStartTrace {
//!     process_id: u32,
//!     process_name: String,
//! }
//!
//! let iterator = wmi_con.notification::<ProcessStartTrace>()?;
//! # tests::start_test_program();
//!
//! for result in iterator {
//!     let trace = result?;
//!     println!("Process started!");
//!     println!("PID:  {}", trace.process_id);
//!     println!("Name: {}", trace.process_name);
//! #    break;
//! } // Loop will end only on error
//! # Ok(())
//! # }
//! ```
//!
//! [`Win32_Process`]: https://docs.microsoft.com/en-us/windows/win32/cimwin32prov/win32-process
//! [`__InstanceCreationEvent`]: https://docs.microsoft.com/en-us/windows/win32/wmisdk/--instancecreationevent
//! [`__NamespaceDeletionEvent`]: https://docs.microsoft.com/en-us/windows/win32/wmisdk/--namespacedeletionevent
//! [`__ExtrinsicEvent`]: https://docs.microsoft.com/en-us/windows/win32/wmisdk/--extrinsicevent
//! [`Win32_ProcessStartTrace`]: https://docs.microsoft.com/en-us/previous-versions/windows/desktop/krnlprov/win32-processstarttrace
//! [`Win32_VolumeChangeEvent`]: https://docs.microsoft.com/en-us/windows/win32/cimwin32prov/win32-volumechangeevent
//! [WMI Code Creator]: https://www.microsoft.com/en-us/download/details.aspx?id=8572
//! [`notification`]: connection/struct.WMIConnection.html#method.notification
//!
//! # Executing Methods
//!
//! The crate also offers support for executing WMI methods on classes and instances.
//!
//! See [`WMIConnection::exec_class_method`], [`WMIConnection::exec_instance_method`] and [`WMIConnection::exec_method`]
//! for detailed examples.
//!
//! # Custom Authentication Levels
//!
//! Some WMI namespaces require specific authentication levels when accessing
//! sensitive system information. Use [`WMIConnection::set_proxy_blanket`] to configure this,
//! which maps directly to the Windows [`CoSetProxyBlanket`] function.
//!
//! **Default Behavior**:
//! - Local connections: [`AuthLevel::Call`] (message authentication)
//! - Remote connections: [`AuthLevel::PktPrivacy`] (packet encryption)
//!
//! ```no_run
//! # use wmi::*;
//! use serde::Deserialize;
//!
//! # fn main() -> WMIResult<()> {
//! // Access BitLocker data, which requires packet-level encryption.
//! // Note: admin privileges are required for BitLocker queries.
//! let wmi_con = WMIConnection::with_namespace_path(
//!     "ROOT\\CIMV2\\Security\\MicrosoftVolumeEncryption"
//! )?;
//! wmi_con.set_proxy_blanket(AuthLevel::PktPrivacy)?;
//!
//! #[derive(Deserialize, Debug)]
//! #[serde(rename = "Win32_EncryptableVolume")]
//! #[serde(rename_all = "PascalCase")]
//! struct EncryptableVolume {
//!     device_id: String,
//!     protection_status: Option<u32>,  // 0=Unprotected, 1=Protected, 2=Unknown
//! }
//!
//! let volumes: Vec<EncryptableVolume> = wmi_con.query()?;
//! # Ok(())
//! # }
//! ```
//!
//! [`CoSetProxyBlanket`]: https://learn.microsoft.com/en-us/windows/win32/api/combaseapi/nf-combaseapi-cosetproxyblanket
//!
//! # Internals
//!
//! [`WMIConnection`] is used to create and execute a WMI query, returning
//! [`IWbemClassWrapper`] which is a wrapper for a WMI object pointer.
//!
//! Then, `from_wbem_class_obj` is used to create a Rust struct with the equivalent data.
//!
//! Deserializing data from WMI and into Rust is done via `serde`.
//! More info can be found in `serde`'s documentation about [writing a data format].
//! The deserializer will either use the field names defined on the output struct,
//! or retrieve all field names from WMI if the output is a `HashMap`.
//!
//! [writing a data format]: https://serde.rs/data-format.html
//!
//! There are two main data structures (other than pointers to object) which convert native data to Rust data structures:
//! [`crate::Variant`] and [`SafeArrayAccessor`](safearray::SafeArrayAccessor).
//!
//! Most native objects has an equivalent wrapper struct which implements `Drop` for that data.
//!
//! # Async Query
//!
//! Async queries use WMI's native async support (but a runtime like `tokio`, `async-std` or `futures::executor::block_on` is still required).
//!
//! ```edition2018
//! # use futures::executor::block_on;
//! # block_on(exec_async_query()).unwrap();
//! # async fn exec_async_query() -> wmi::WMIResult<()> {
//! use wmi::*;
//! let wmi_con = WMIConnection::new()?;
//! use serde::Deserialize;
//!
//! #[derive(Deserialize, Debug)]
//! #[serde(rename = "Win32_OperatingSystem")]
//! #[serde(rename_all = "PascalCase")]
//! struct OperatingSystem {
//!     caption: String,
//!     debug: bool,
//! }
//!
//! let results: Vec<OperatingSystem> = wmi_con.async_query().await?;
//!
//! for os in results {
//!     println!("{:#?}", os);
//! }
//! #   Ok(())
//! # }
//! ```
//!
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unused_unsafe)]
#![allow(clippy::arc_with_non_send_sync)]
#![allow(clippy::needless_lifetimes)]
#![cfg(windows)]

mod connection;

#[cfg(feature = "chrono")]
mod datetime;

#[cfg(feature = "time")]
mod datetime_time;

mod context;
mod de;
mod duration;
mod method;
mod query;
mod result_enumerator;
pub mod safearray;
mod ser;
mod utils;
mod variant;

mod async_query;
mod query_sink;

mod notification;

#[cfg(any(test, feature = "test"))]
pub mod tests;

pub use connection::{AuthLevel, WMIConnection};

#[cfg(feature = "chrono")]
pub use datetime::WMIDateTime;

#[cfg(feature = "time")]
pub use datetime_time::WMIOffsetDateTime;

pub use context::{ContextValueType, WMIContext};
pub use duration::WMIDuration;
pub use query::{FilterValue, build_notification_query, build_query, quote_and_escape_wql_str};
pub use result_enumerator::IWbemClassWrapper;
pub use utils::{WMIError, WMIResult};
pub use variant::Variant;

#[doc = include_str!("../README.md")]
#[cfg(all(doctest, feature = "chrono"))]
pub struct ReadmeDoctests;
