#![deny(missing_docs, unsafe_code)]
#![cfg_attr(not(feature = "doc"), forbid(unstable_features))]
#![cfg_attr(feature = "doc", feature(doc_cfg))]
//! A futures based crate for interacting with IndexedDB on browsers using webassembly.
//!
//! # Usage
//!
//! To use `idb`, you need to run following command from your project root:
//!
//! ```sh
//! cargo add idb
//! ```
//!
//! If you don't want to use `async`/`await` syntax, you can disable the `futures` feature using:
//!
//! ```sh
//! cargo add idb --no-default-features
//! ```
//!
//! After disabling the `futures` feature, you can use `on_success` and `on_error` methods on requests to attach
//! callbacks.
//!
//! ## Example
//!
//! To create a new database, you can use [`Factory::open`]:
//!
//! ```rust
//! use idb::{Database, DatabaseEvent, Error, Factory, IndexParams, KeyPath, ObjectStoreParams};
//!
//! async fn create_database() -> Result<Database, Error> {
//!     // Get a factory instance from global scope
//!     let factory = Factory::new()?;
//!
//!     // Create an open request for the database
//!     let mut open_request = factory.open("test", Some(1)).unwrap();
//!
//!     // Add an upgrade handler for database
//!     open_request.on_upgrade_needed(|event| {
//!         // Get database instance from event
//!         let database = event.database().unwrap();
//!
//!         // Prepare object store params
//!         let mut store_params = ObjectStoreParams::new();
//!         store_params.auto_increment(true);
//!         store_params.key_path(Some(KeyPath::new_single("id")));
//!
//!         // Create object store
//!         let store = database
//!             .create_object_store("employees", store_params)
//!             .unwrap();
//!
//!         // Prepare index params
//!         let mut index_params = IndexParams::new();
//!         index_params.unique(true);
//!
//!         // Create index on object store
//!         store
//!             .create_index("email", KeyPath::new_single("email"), Some(index_params))
//!             .unwrap();
//!     });
//!
//!     // `await` open request
//!     open_request.await
//! }
//! ```
//!
//! To add data to an object store, you can use [`ObjectStore::add`]:
//!
//! ```rust
//! use idb::{Database, Error, TransactionMode};
//! use serde::Serialize;
//! use serde_wasm_bindgen::Serializer;
//! use wasm_bindgen::JsValue;
//!
//! async fn add_data(database: &Database) -> Result<JsValue, Error> {
//!     // Create a read-write transaction
//!     let transaction = database.transaction(&["employees"], TransactionMode::ReadWrite)?;
//!
//!     // Get the object store
//!     let store = transaction.object_store("employees").unwrap();
//!
//!     // Prepare data to add
//!     let employee = serde_json::json!({
//!         "name": "John Doe",
//!         "email": "john@example.com",
//!     });
//!
//!     // Add data to object store
//!     let id = store
//!         .add(
//!             &employee.serialize(&Serializer::json_compatible()).unwrap(),
//!             None,
//!         )
//!         .unwrap()
//!         .await?;
//!
//!     // Commit the transaction
//!     transaction.commit()?.await?;
//!
//!     Ok(id)
//! }
//! ```
//!
//! To get data from an object store, you can use [`ObjectStore::get`]:
//!
//! ```rust
//! use idb::{Database, Error, TransactionMode};
//! use serde_json::Value;
//! use wasm_bindgen::JsValue;
//!
//! async fn get_data(database: &Database, id: JsValue) -> Result<Option<Value>, Error> {
//!     // Create a read-only transaction
//!     let transaction = database
//!         .transaction(&["employees"], TransactionMode::ReadOnly)
//!         .unwrap();
//!     
//!     // Get the object store
//!     let store = transaction.object_store("employees").unwrap();
//!
//!     // Get the stored data
//!     let stored_employee: Option<JsValue> = store.get(id)?.await?;
//!
//!     // Deserialize the stored data
//!     let stored_employee: Option<Value> = stored_employee
//!         .map(|stored_employee| serde_wasm_bindgen::from_value(stored_employee).unwrap());
//!     
//!     // Wait for the transaction to complete (alternatively, you can also commit the transaction)
//!     transaction.await?;
//!
//!     Ok(stored_employee)
//! }
//! ```
//!
//! For more examples on using other functionality, see the
//! [tests](https://github.com/devashishdxt/idb/tree/main/idb/tests) directory.
#[cfg(feature = "builder")]
#[cfg_attr(any(docsrs, feature = "doc"), doc(cfg(feature = "builder")))]
pub mod builder;
mod cursor;
mod database;
mod error;
pub mod event;
mod factory;
mod index;
mod key_range;
mod mappers;
mod object_store;
mod query;
pub mod request;
mod transaction;
mod utils;

pub use self::{
    cursor::{Cursor, CursorDirection, KeyCursor},
    database::Database,
    error::Error,
    event::{DatabaseEvent, Event, StoreEvent},
    factory::Factory,
    index::{Index, IndexParams},
    key_range::KeyRange,
    object_store::{KeyPath, ObjectStore, ObjectStoreParams},
    query::Query,
    request::Request,
    transaction::{Transaction, TransactionMode},
};
#[cfg(feature = "futures")]
#[cfg_attr(any(docsrs, feature = "doc"), doc(cfg(feature = "futures")))]
pub use self::{
    cursor::{ManagedCursor, ManagedKeyCursor},
    transaction::{TransactionFuture, TransactionResult},
};
