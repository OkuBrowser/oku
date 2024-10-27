use crate::database::DATABASE;
use crate::window_util::get_window_from_widget;
use gdk::prelude::DisplayExt;
use gio::prelude::ApplicationExt;
use glib::clone;
use glib::object::Cast;
use glib::object::CastNone;
use glib::subclass::object::ObjectImpl;
use glib::subclass::types::ObjectSubclass;
use glib::subclass::types::ObjectSubclassExt;
use glib::subclass::types::ObjectSubclassIsExt;
use glib::value::ToValue;
use glib::ParamSpec;
use glib::ParamSpecObject;
use glib::ParamSpecString;
use glib::Value;
use gtk::prelude::BoxExt;
use gtk::prelude::ButtonExt;
use gtk::prelude::GObjectPropertyExpressionExt;
use gtk::prelude::GtkWindowExt;
use gtk::prelude::ListBoxRowExt;
use gtk::prelude::OrientableExt;
use gtk::prelude::WidgetExt;
use gtk::subclass::prelude::*;
use libadwaita::prelude::ActionRowExt;
use libadwaita::prelude::PreferencesRowExt;
use libadwaita::subclass::prelude::*;
use log::error;
use once_cell::sync::Lazy;
use std::cell::RefCell;
use uuid::Uuid;
use webkit2gtk::functions::uri_for_display;

pub mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct Tag {
        pub(crate) text: RefCell<String>,
        pub(crate) text_label: gtk::Label,
        pub(crate) delete_button: gtk::Button,
    }

    impl Tag {}

    #[glib::object_subclass]
    impl ObjectSubclass for Tag {
        const NAME: &'static str = "OkuTag";
        type Type = super::Tag;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.set_layout_manager_type::<gtk::BoxLayout>();
            klass.set_accessible_role(gtk::AccessibleRole::Generic);
        }
    }

    impl ObjectImpl for Tag {
        fn dispose(&self) {
            while let Some(child) = self.obj().first_child() {
                child.unparent();
            }
        }

        fn constructed(&self) {
            self.parent_constructed();

            self.obj().setup();
        }

        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> =
                Lazy::new(|| vec![ParamSpecString::builder("text").build()]);
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "text" => {
                    let text = value.get::<String>().unwrap();
                    self.obj().set_text(text);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &ParamSpec) -> Value {
            match pspec.name() {
                "text" => self.obj().text().to_value(),
                _ => unimplemented!(),
            }
        }
    }
    impl WidgetImpl for Tag {}
    impl BoxImpl for Tag {}
}

glib::wrapper! {
    pub struct Tag(ObjectSubclass<imp::Tag>)
    @extends gtk::Box, gtk::Widget,
    @implements gtk::Accessible, gtk::Actionable, gtk::Orientable, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for Tag {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl Tag {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn setup(&self) {
        let imp = self.imp();

        self.property_expression("text")
            .bind(&imp.text_label, "label", gtk::Widget::NONE);
        imp.text_label.set_xalign(0.0);
        imp.text_label.set_ellipsize(pango::EllipsizeMode::End);
        imp.text_label.set_hexpand(true);

        imp.delete_button.set_icon_name("window-close-symbolic");
        imp.delete_button.add_css_class("flat");
        imp.delete_button.add_css_class("circular");

        self.add_css_class("toolbar");
        self.add_css_class("osd");
        self.set_orientation(gtk::Orientation::Horizontal);
        self.append(&imp.text_label);
        self.append(&imp.delete_button);
    }
    pub fn text(&self) -> String {
        self.imp().text.borrow().to_owned()
    }

    pub fn set_text(&self, text: String) {
        let imp = self.imp();

        imp.text.replace(text);
    }
}
