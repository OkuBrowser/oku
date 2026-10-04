#![cfg(target_arch = "wasm32")]

mod builder;
mod cursor;
mod database;
mod factory;
mod index;
mod object_store;
mod open_request;
mod transaction;

use wasm_bindgen_test::wasm_bindgen_test_configure;

wasm_bindgen_test_configure!(run_in_browser);
