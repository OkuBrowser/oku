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
use gtk::prelude::GtkWindowExt;
use gtk::prelude::ListBoxRowExt;
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

use crate::database::DATABASE;
use crate::window_util::get_window_from_widget;

pub mod imp {

    use uuid::Uuid;

    use super::*;

    #[derive(Debug, Default)]
    pub struct HistoryRow {
        pub(crate) id: RefCell<Uuid>,
        pub(crate) title: RefCell<String>,
        pub(crate) uri: RefCell<String>,
        pub(crate) timestamp: RefCell<String>,
        pub(crate) favicon: gtk::Image,
        pub(crate) copy_url_button: gtk::Button,
        pub(crate) delete_button: gtk::Button,
        pub(crate) button_box: gtk::Box,
    }

    impl HistoryRow {}

    #[glib::object_subclass]
    impl ObjectSubclass for HistoryRow {
        const NAME: &'static str = "OkuHistoryRow";
        type Type = super::HistoryRow;
        type ParentType = libadwaita::ActionRow;

        fn class_init(klass: &mut Self::Class) {
            klass.set_layout_manager_type::<gtk::BoxLayout>();
            klass.set_accessible_role(gtk::AccessibleRole::Generic);
        }
    }

    impl ObjectImpl for HistoryRow {
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
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpecString::builder("id").build(),
                    ParamSpecString::builder("title-property").build(),
                    ParamSpecString::builder("uri").build(),
                    ParamSpecString::builder("timestamp").build(),
                    ParamSpecObject::builder::<gdk::Texture>("favicon").build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "id" => {
                    let id = value.get::<String>().unwrap();
                    self.obj().set_id(Uuid::parse_str(&id).unwrap());
                }
                "title-property" => {
                    let title = value.get::<&str>().unwrap();
                    self.obj().set_title_property(title);
                }
                "timestamp" => {
                    let timestamp = value.get::<&str>().unwrap();
                    self.obj().set_timestamp(timestamp);
                }
                "uri" => {
                    let uri = value.get::<&str>().unwrap();
                    self.obj().set_uri(uri);
                }
                "favicon" => {
                    let favicon = value.get::<gdk::Texture>().ok();
                    self.obj().set_favicon(favicon);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &ParamSpec) -> Value {
            match pspec.name() {
                "id" => self.obj().id().to_string().to_value(),
                "title-property" => self.obj().title_property().to_value(),
                "timestamp" => self.obj().timestamp().to_value(),
                "uri" => self.obj().uri().to_value(),
                "favicon" => self.obj().favicon().to_value(),
                _ => unimplemented!(),
            }
        }
    }
    impl WidgetImpl for HistoryRow {}
    impl ListBoxRowImpl for HistoryRow {}
    impl PreferencesRowImpl for HistoryRow {}
    impl ActionRowImpl for HistoryRow {}
}

glib::wrapper! {
    pub struct HistoryRow(ObjectSubclass<imp::HistoryRow>)
    @extends libadwaita::ActionRow, libadwaita::PreferencesRow, gtk::ListBoxRow, gtk::Widget,
    @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for HistoryRow {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl HistoryRow {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn setup(&self) {
        let imp = self.imp();

        imp.copy_url_button.set_icon_name("copy-symbolic");
        imp.copy_url_button.add_css_class("circular");
        imp.copy_url_button.add_css_class("linked");
        imp.copy_url_button.set_vexpand(false);
        imp.copy_url_button.set_hexpand(false);
        imp.copy_url_button.set_tooltip_text(Some("Copy URL"));
        imp.copy_url_button.connect_clicked(clone!(
            #[weak(rename_to = this)]
            self,
            move |_| {
                let clipboard = gdk::Display::default().unwrap().clipboard();
                clipboard.set_text(&this.uri());
                let window = get_window_from_widget(&this);
                let app = window.application().unwrap();
                let notification = gio::Notification::new("History URL copied");
                notification.set_body(Some(&format!(
                    "A URL from the browser history ({}) has been copied to the clipboard.",
                    this.uri()
                )));
                app.send_notification(None, &notification);
            }
        ));

        imp.delete_button.set_icon_name("user-trash-symbolic");
        imp.delete_button.add_css_class("circular");
        imp.delete_button.add_css_class("destructive-action");
        imp.delete_button.set_vexpand(false);
        imp.delete_button.set_hexpand(false);
        imp.delete_button.connect_clicked(clone!(
            #[weak(rename_to = this)]
            self,
            move |_| {
                if let Some(history_record) = DATABASE.get_history_record(this.id()).ok().flatten()
                {
                    if let Err(e) = DATABASE.delete_history_record(history_record) {
                        error!("{}", e)
                    }
                }
            }
        ));

        imp.button_box.append(&imp.copy_url_button);
        imp.button_box.append(&imp.delete_button);
        imp.button_box.set_homogeneous(false);
        imp.button_box.set_valign(gtk::Align::Center);
        imp.button_box.set_halign(gtk::Align::End);
        imp.button_box.add_css_class("linked");

        let content_box: gtk::Box = self.child().and_downcast().unwrap();
        content_box.set_hexpand(true);

        self.add_prefix(&imp.favicon);
        self.add_suffix(&imp.button_box);
        self.set_title_lines(1);
        self.set_subtitle_lines(2);
        self.add_css_class("caption");
        self.add_css_class("card");
    }

    pub fn id(&self) -> Uuid {
        self.imp().id.borrow().to_owned()
    }
    pub fn title_property(&self) -> String {
        self.imp().title.borrow().to_string()
    }
    pub fn timestamp(&self) -> String {
        self.imp().timestamp.borrow().to_string()
    }
    pub fn uri(&self) -> String {
        self.imp().uri.borrow().to_string()
    }
    pub fn favicon(&self) -> Option<gdk::Texture> {
        self.imp()
            .favicon
            .paintable()?
            .downcast::<gdk::Texture>()
            .ok()
    }

    pub fn set_id(&self, id: Uuid) {
        let imp = self.imp();

        imp.id.replace(id);
    }

    fn update_title(&self) {
        let encoded_title = &self.title_property();
        let encoded_uri = uri_for_display(&self.uri())
            .unwrap_or(self.uri().into())
            .to_string();
        let encoded_timestamp = &self.timestamp();
        if encoded_title.trim().is_empty() {
            self.set_title(&encoded_uri);
            self.set_subtitle(&encoded_timestamp);
        } else {
            self.set_title(&encoded_title);
            self.set_subtitle(&format!("{}\n{}", encoded_uri, encoded_timestamp));
        }
    }

    pub fn set_title_property(&self, title: &str) {
        let imp = self.imp();

        imp.title.replace(title.to_string());
        self.update_title();
    }

    pub fn set_uri(&self, uri: &str) {
        let imp = self.imp();

        imp.uri.replace(uri.to_string());
        self.update_title();
    }

    pub fn set_timestamp(&self, timestamp: &str) {
        let imp = self.imp();

        imp.timestamp.replace(timestamp.to_string());
        self.update_title();
    }

    pub fn set_favicon(&self, favicon: Option<gdk::Texture>) {
        let imp = self.imp();

        match favicon {
            Some(favicon) => {
                imp.favicon.set_paintable(Some(&favicon));
            }
            None => {
                imp.favicon.set_icon_name(Some("globe-symbolic"));
            }
        }
    }
}
