use crate::bookmark_item::BookmarkItem;
use crate::database::Bookmark;
use crate::database::DATABASE;
use crate::window_util::get_window_from_widget;
use glib::clone;
use glib::object::Cast;
use glib::object::CastNone;
use glib::subclass::object::ObjectImpl;
use glib::subclass::types::ObjectSubclass;
use glib::subclass::types::ObjectSubclassExt;
use glib::subclass::types::ObjectSubclassIsExt;
use glib::value::ToValue;
use glib::ParamSpec;
use glib::ParamSpecBoxed;
use glib::ParamSpecObject;
use glib::ParamSpecString;
use glib::Value;
use gtk::prelude::ButtonExt;
use gtk::prelude::ListBoxRowExt;
use gtk::prelude::WidgetExt;
use gtk::prelude::{BoxExt, GObjectPropertyExpressionExt};
use gtk::subclass::prelude::*;
use libadwaita::prelude::ActionRowExt;
use libadwaita::prelude::PreferencesRowExt;
use libadwaita::subclass::prelude::*;
use log::error;
use std::cell::RefCell;
use std::collections::HashSet;
use std::sync::LazyLock;
use webkit2gtk::functions::uri_for_display;

pub mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct BookmarkRow {
        pub(crate) url: RefCell<String>,
        pub(crate) title: RefCell<String>,
        pub(crate) body: RefCell<String>,
        pub(crate) tags: RefCell<Vec<String>>,
        pub(crate) favicon: gtk::Image,
        pub(crate) edit_button: gtk::Button,
        pub(crate) delete_button: gtk::Button,
        pub(crate) button_box: gtk::Box,
    }

    impl BookmarkRow {}

    #[glib::object_subclass]
    impl ObjectSubclass for BookmarkRow {
        const NAME: &'static str = "OkuBookmarkRow";
        type Type = super::BookmarkRow;
        type ParentType = libadwaita::ActionRow;

        fn class_init(klass: &mut Self::Class) {
            klass.set_layout_manager_type::<gtk::BoxLayout>();
            klass.set_accessible_role(gtk::AccessibleRole::Generic);
        }
    }

    impl ObjectImpl for BookmarkRow {
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
                    ParamSpecString::builder("url").build(),
                    ParamSpecString::builder("title-property").build(),
                    ParamSpecString::builder("body").build(),
                    ParamSpecBoxed::builder::<Vec<String>>("tags").build(),
                    ParamSpecObject::builder::<gdk::Texture>("favicon").build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "url" => {
                    let url = value.get::<String>().unwrap();
                    self.obj().set_url(url);
                }
                "title-property" => {
                    let title = value.get::<String>().unwrap();
                    self.obj().set_title_property(title);
                }
                "body" => {
                    let body = value.get::<String>().unwrap();
                    self.obj().set_body(body);
                }
                "tags" => {
                    let tags = value.get::<Vec<String>>().unwrap();
                    self.obj().set_tags(tags);
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
                "url" => self.obj().url().to_value(),
                "title-property" => self.obj().title_property().to_value(),
                "body" => self.obj().body().to_value(),
                "tags" => self.obj().tags().to_value(),
                "favicon" => self.obj().favicon().to_value(),
                _ => unimplemented!(),
            }
        }
    }
    impl WidgetImpl for BookmarkRow {}
    impl ListBoxRowImpl for BookmarkRow {}
    impl PreferencesRowImpl for BookmarkRow {}
    impl ActionRowImpl for BookmarkRow {}
}

glib::wrapper! {
    pub struct BookmarkRow(ObjectSubclass<imp::BookmarkRow>)
    @extends libadwaita::ActionRow, libadwaita::PreferencesRow, gtk::ListBoxRow, gtk::Widget,
    @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for BookmarkRow {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl From<&BookmarkItem> for BookmarkRow {
    fn from(bookmark_item: &BookmarkItem) -> Self {
        let obj = Self::default();
        bookmark_item
            .property_expression("url")
            .bind(&obj, "url", gtk::Widget::NONE);
        bookmark_item
            .property_expression("title")
            .bind(&obj, "title-property", gtk::Widget::NONE);
        bookmark_item
            .property_expression("body")
            .bind(&obj, "body", gtk::Widget::NONE);
        bookmark_item
            .property_expression("tags")
            .bind(&obj, "tags", gtk::Widget::NONE);
        bookmark_item
            .property_expression("favicon")
            .bind(&obj, "favicon", gtk::Widget::NONE);
        obj
    }
}

impl BookmarkRow {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn setup(&self) {
        let imp = self.imp();

        imp.edit_button.set_icon_name("editor-symbolic");
        imp.edit_button.add_css_class("circular");
        imp.edit_button.add_css_class("linked");
        imp.edit_button.set_vexpand(false);
        imp.edit_button.set_hexpand(false);
        imp.edit_button.connect_clicked(clone!(
            #[weak(rename_to = this)]
            self,
            move |_| {
                crate::widgets::note_editor::NoteEditor::new(
                    Some(&get_window_from_widget(&this)),
                    Some(Bookmark {
                        url: this.url(),
                        title: this.title_property(),
                        body: this.body(),
                        tags: HashSet::from_iter(this.tags().into_iter()),
                    }),
                );
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
                if let Some(bookmark) = DATABASE.get_bookmark(this.url()).ok().flatten() {
                    if let Err(e) = DATABASE.delete_bookmark(bookmark) {
                        error!("{}", e)
                    }
                }
            }
        ));

        imp.button_box.append(&imp.edit_button);
        imp.button_box.append(&imp.delete_button);
        imp.button_box.set_homogeneous(false);
        imp.button_box.set_valign(gtk::Align::Center);
        imp.button_box.set_halign(gtk::Align::End);
        imp.button_box.add_css_class("linked");

        let content_box: gtk::Box = self.child().and_downcast().unwrap();
        content_box.set_hexpand(true);

        self.add_prefix(&imp.favicon);
        self.add_suffix(&imp.button_box);
        self.set_margin_bottom(4);
        self.set_title_lines(1);
        self.set_subtitle_lines(2);
        self.add_css_class("caption");
        self.add_css_class("card");
    }

    pub fn url(&self) -> String {
        self.imp().url.borrow().to_string()
    }
    pub fn title_property(&self) -> String {
        self.imp().title.borrow().to_string()
    }
    pub fn body(&self) -> String {
        self.imp().body.borrow().to_string()
    }
    pub fn tags(&self) -> Vec<String> {
        self.imp().tags.borrow().to_owned()
    }
    pub fn favicon(&self) -> Option<gdk::Texture> {
        self.imp()
            .favicon
            .paintable()?
            .downcast::<gdk::Texture>()
            .ok()
    }

    pub fn set_url(&self, url: String) {
        let imp = self.imp();

        imp.url.replace(url);
        self.update_title();
    }

    fn update_title(&self) {
        let encoded_title = &self.title_property();
        let encoded_url = uri_for_display(&self.url())
            .unwrap_or(self.url().into())
            .to_string();
        let encoded_tags = &self.tags().join(", ");
        if encoded_title.trim().is_empty() {
            self.set_title(&encoded_url);
            self.set_subtitle(encoded_tags);
        } else {
            self.set_title(encoded_title);
            if encoded_tags.trim().is_empty() {
                self.set_subtitle(&encoded_url.to_string());
            } else {
                self.set_subtitle(&format!("{}\n{}", encoded_url, encoded_tags));
            }
        }
    }

    pub fn set_title_property(&self, title: String) {
        let imp = self.imp();

        imp.title.replace(title);
        self.update_title();
    }

    pub fn set_body(&self, body: String) {
        let imp = self.imp();

        imp.body.replace(body);
        self.update_title();
    }

    pub fn set_tags(&self, tags: Vec<String>) {
        let imp = self.imp();

        imp.tags.replace(tags);
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
