//! # Migration to GTK4 api
//!
//! Upstream webkit2gtk project has a document to help with the migration to the GTK4 api:
//! <https://webkitgtk.org/reference/webkit2gtk/unstable/migrating-to-webkitgtk-6.0.html>

#![cfg_attr(docsrs, feature(doc_cfg))]

macro_rules! assert_initialized_main_thread {
    () => {};
}

macro_rules! skip_assert_initialized {
    () => {};
}

pub use ffi;
pub use gdk;
pub use gio;
pub use glib;
pub use gtk;
pub use javascriptcore;
pub use soup;

mod website_data_manager;

#[allow(unused_imports)]
mod auto;
pub use crate::auto::*;

pub mod prelude {
    #[doc(hidden)]
    pub use gtk::prelude::*;
    #[doc(hidden)]
    pub use soup::prelude::*;

    pub use super::auto::traits::*;
}

pub mod functions {
    pub use super::auto::functions::*;
}
