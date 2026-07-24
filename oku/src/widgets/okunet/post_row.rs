use glib::object::ObjectExt;
use glib::property::PropertySet;
use glib::subclass::object::ObjectImpl;
use glib::subclass::types::ObjectSubclass;
use glib::subclass::types::ObjectSubclassExt;
use glib::subclass::types::ObjectSubclassIsExt;
use glib::value::ToValue;
use glib::ParamSpec;
use glib::ParamSpecBuilderExt;
use glib::ParamSpecString;
use glib::Value;
use glib::{closure, Object};
use gtk::glib;
use gtk::prelude::BoxExt;
use gtk::prelude::ListBoxRowExt;
use gtk::prelude::WidgetExt;
use gtk::subclass::prelude::*;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::sync::LazyLock;

pub mod imp {
    use glib::ParamSpecBoxed;

    use super::*;

    #[derive(Debug, Default)]
    pub struct PostRow {
        // Data
        pub(crate) url: RefCell<String>,
        pub(crate) title: RefCell<String>,
        pub(crate) body: RefCell<String>,
        pub(crate) tags: RefCell<Vec<String>>,
        pub(crate) author_id: RefCell<String>,
        pub(crate) author_name: RefCell<Option<String>>,
        // Widgets
        pub(crate) url_label: gtk::Label,
        pub(crate) title_label: gtk::Label,
        pub(crate) body_label: gtk::Label,
        pub(crate) tags_label: gtk::Label,
        pub(crate) author_label: gtk::Label,
        pub(crate) author_avatar: libadwaita::Avatar,
        pub(crate) main: gtk::Box,
    }

    impl PostRow {}

    #[glib::object_subclass]
    impl ObjectSubclass for PostRow {
        const NAME: &'static str = "OkuPostRow";
        type Type = super::PostRow;
        type ParentType = gtk::ListBoxRow;

        fn class_init(klass: &mut Self::Class) {
            klass.set_layout_manager_type::<gtk::BoxLayout>();
            klass.set_accessible_role(gtk::AccessibleRole::Generic);
        }
    }

    impl ObjectImpl for PostRow {
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
                    ParamSpecString::builder("title").build(),
                    ParamSpecString::builder("body").build(),
                    ParamSpecBoxed::builder::<Vec<String>>("tags")
                        .readwrite()
                        .build(),
                    ParamSpecString::builder("author-id").build(),
                    ParamSpecString::builder("author-name").build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "url" => {
                    let url = value.get::<&str>().unwrap();
                    self.obj().set_url(url);
                }
                "title" => {
                    let title = value.get::<&str>().unwrap();
                    self.obj().set_title(title);
                }
                "body" => {
                    let body = value.get::<&str>().unwrap();
                    self.obj().set_body(body);
                }
                "tags" => {
                    let tags = value.get::<Vec<String>>().unwrap();
                    self.tags.set(
                        tags.iter()
                            .map(|x| html_escape::encode_text(x).to_string())
                            .collect(),
                    );
                }
                "author-id" => {
                    let author_id = value.get::<&str>().unwrap();
                    self.obj().set_author_id(author_id);
                }
                "author-name" => {
                    let author_name = value.get::<Option<&str>>().unwrap();
                    self.obj().set_author_name(&author_name);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &ParamSpec) -> Value {
            match pspec.name() {
                "url" => self.obj().url().to_value(),
                "title" => self.obj().title().to_value(),
                "body" => self.obj().body().to_value(),
                "tags" => self.obj().tags().to_value(),
                "author-id" => self.obj().author_id().to_value(),
                "author-name" => self.obj().author_name().to_value(),
                _ => unimplemented!(),
            }
        }
    }
    impl WidgetImpl for PostRow {}
    impl ListBoxRowImpl for PostRow {}
}

glib::wrapper! {
    pub struct PostRow(ObjectSubclass<imp::PostRow>)
    @extends gtk::ListBoxRow, gtk::Widget,
    @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

unsafe impl Send for PostRow {}
unsafe impl Sync for PostRow {}

impl Default for PostRow {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl PostRow {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn setup(&self) {
        let imp = self.imp();

        imp.author_avatar.set_show_initials(true);
        imp.author_avatar.set_size(32);

        self.bind_property("title", &imp.title_label, "label")
            .build();
        self.bind_property("url", &imp.url_label, "label").build();
        self.bind_property("body", &imp.body_label, "label").build();
        let this = self.clone();
        self.property_expression("author-name")
            .chain_closure::<String>(closure!(
                |_: Option<Object>, author_name: Option<String>| {
                    match author_name {
                        Some(x) => x,
                        None => this.author_id(),
                    }
                }
            ))
            .bind(&imp.author_label, "label", gtk::Widget::NONE);
        self.property_expression("tags")
            .chain_closure::<String>(closure!(|_: Option<Object>, tags: Vec<String>| {
                tags.join(", ")
            }))
            .bind(&imp.tags_label, "label", gtk::Widget::NONE);
        let this = self.clone();
        self.property_expression("author-name")
            .chain_closure::<String>(closure!(
                |_: Option<Object>, author_name: Option<String>| {
                    match author_name {
                        Some(x) => x,
                        None => this.author_id(),
                    }
                }
            ))
            .bind(&imp.author_avatar, "text", gtk::Widget::NONE);

        imp.main.append(&imp.title_label);
        imp.main.append(&imp.url_label);
        imp.main.append(&imp.author_label);
        imp.main.append(&imp.author_avatar);
        imp.main.append(&imp.body_label);
        imp.main.append(&imp.tags_label);
        imp.main.set_vexpand(true);
        imp.main.set_hexpand(true);
        imp.main.set_orientation(gtk::Orientation::Vertical);

        self.set_child(Some(&imp.main));
    }

    pub fn url(&self) -> String {
        self.imp().url.borrow().to_string()
    }
    pub fn title(&self) -> String {
        self.imp().title.borrow().to_string()
    }
    pub fn body(&self) -> String {
        self.imp().body.borrow().to_string()
    }
    pub fn tags(&self) -> Vec<String> {
        self.imp().tags.borrow().to_owned()
    }
    pub fn author_id(&self) -> String {
        self.imp().author_id.borrow().to_string()
    }
    pub fn author_name(&self) -> Option<String> {
        self.imp()
            .author_name
            .borrow()
            .clone()
            .map(|x| x.to_string())
    }

    fn set_url(&self, url: &str) {
        let imp = self.imp();

        imp.url.replace(url.to_string());
    }
    pub fn set_title(&self, title: &str) {
        let imp = self.imp();

        imp.title.replace(title.to_string());
    }
    pub fn set_body(&self, body: &str) {
        let imp = self.imp();

        imp.body.replace(body.to_string());
    }
    pub fn set_tags(&self, tags: &[String]) {
        let imp = self.imp();

        imp.tags.set(
            tags.iter()
                .map(|x| html_escape::encode_text(x).to_string())
                .collect(),
        );
    }
    pub fn set_author_id(&self, author_id: &str) {
        let imp = self.imp();

        imp.author_id.replace(author_id.to_string());
    }
    pub fn set_author_name(&self, author_name: &Option<&str>) {
        let imp = self.imp();

        imp.author_name
            .replace(author_name.map(|x| x.to_string()).clone());
    }
}
