use crate::CONFIG_DIR;
use glib::subclass::object::ObjectImpl;
use glib::subclass::types::ObjectSubclass;
use glib::value::ToValue;
use glib::ParamSpec;
use glib::ParamSpecBoolean;
use glib::ParamSpecBuilderExt;
use glib::Value;
use glib::{ParamSpecEnum, ParamSpecInt};
use log::error;
use serde::Deserialize;
use serde::Serialize;
use std::cell::RefCell;
use std::sync::LazyLock;

pub mod enums;
pub mod imp;

glib::wrapper! {
    pub struct Config(ObjectSubclass<imp::Config>);
}

impl Default for Config {
    fn default() -> Self {
        let config = imp::Config::new();
        glib::Object::builder::<Self>()
            .property("colour-per-domain", config.colour_per_domain())
            .property("colour-scheme", config.colour_scheme())
            .property("palette", config.palette())
            .property("width", config.width())
            .property("height", config.height())
            .property("is-maximised", config.is_maximised())
            .property("is-fullscreen", config.is_fullscreen())
            .build()
    }
}
