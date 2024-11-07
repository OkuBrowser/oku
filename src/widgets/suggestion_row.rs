use glib::object::Cast;
use glib::subclass::object::ObjectImpl;
use glib::subclass::types::ObjectSubclass;
use glib::subclass::types::ObjectSubclassExt;
use glib::subclass::types::ObjectSubclassIsExt;
use glib::value::ToValue;
use glib::ParamSpec;
use glib::ParamSpecObject;
use glib::ParamSpecString;
use glib::Value;
use gtk::prelude::WidgetExt;
use gtk::subclass::prelude::*;
use libadwaita::prelude::ActionRowExt;
use libadwaita::prelude::PreferencesRowExt;
use libadwaita::subclass::prelude::*;
use std::cell::RefCell;
use std::sync::LazyLock;
use webkit2gtk::functions::uri_for_display;

pub mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct SuggestionRow {
        pub(crate) title: RefCell<String>,
        pub(crate) uri: RefCell<String>,
        pub(crate) favicon: gtk::Image,
    }

    impl SuggestionRow {}

    #[glib::object_subclass]
    impl ObjectSubclass for SuggestionRow {
        const NAME: &'static str = "OkuSuggestionRow";
        type Type = super::SuggestionRow;
        type ParentType = libadwaita::ActionRow;

        fn class_init(klass: &mut Self::Class) {
            klass.set_layout_manager_type::<gtk::BoxLayout>();
            klass.set_accessible_role(gtk::AccessibleRole::Generic);
        }
    }

    impl ObjectImpl for SuggestionRow {
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
            static PROPERTIES: LazyLock<Vec<ParamSpec>> = LazyLock::new(|| {
                vec![
                    ParamSpecString::builder("title-property").build(),
                    ParamSpecString::builder("uri").build(),
                    ParamSpecObject::builder::<gdk::Texture>("favicon").build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "title-property" => {
                    let title = value.get::<&str>().unwrap();
                    self.obj().set_title_property(title);
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
                "title-property" => self.obj().title_property().to_value(),
                "uri" => self.obj().uri().to_value(),
                "favicon" => self.obj().favicon().to_value(),
                _ => unimplemented!(),
            }
        }
    }
    impl WidgetImpl for SuggestionRow {}
    impl ListBoxRowImpl for SuggestionRow {}
    impl PreferencesRowImpl for SuggestionRow {}
    impl ActionRowImpl for SuggestionRow {}
}

glib::wrapper! {
    pub struct SuggestionRow(ObjectSubclass<imp::SuggestionRow>)
    @extends libadwaita::ActionRow, libadwaita::PreferencesRow, gtk::ListBoxRow, gtk::Widget,
    @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for SuggestionRow {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl SuggestionRow {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn setup(&self) {
        let imp = self.imp();
        self.add_prefix(&imp.favicon);
        self.set_title_lines(1);
        self.set_subtitle_lines(1);
    }

    pub fn title_property(&self) -> String {
        self.imp().title.borrow().to_string()
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

    pub fn set_title_property(&self, title: &str) {
        let imp = self.imp();

        imp.title.replace(title.to_string());
        let encoded_title = &self.title_property();
        let encoded_uri = uri_for_display(&self.uri())
            .unwrap_or(self.uri().into())
            .to_string();
        if encoded_title.trim().is_empty() {
            self.set_title(&encoded_uri);
            self.set_subtitle(&String::new());
        } else {
            self.set_title(&encoded_title);
            self.set_subtitle(&encoded_uri);
        }
    }

    pub fn set_uri(&self, uri: &str) {
        let imp = self.imp();

        imp.uri.replace(uri.to_string());
        let encoded_title = self.title_property();
        let encoded_uri = uri_for_display(&self.uri())
            .unwrap_or(self.uri().into())
            .to_string();
        if encoded_title.trim().is_empty() {
            self.set_title(&encoded_uri);
            self.set_subtitle(&String::new());
        } else {
            self.set_title(&encoded_title);
            self.set_subtitle(&encoded_uri);
        }
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
