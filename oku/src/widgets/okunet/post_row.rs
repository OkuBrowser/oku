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
use gtk::prelude::WidgetExt;
use gtk::subclass::prelude::*;
use libadwaita::prelude::PreferencesRowExt;
use std::cell::RefCell;
use std::sync::LazyLock;

pub mod imp {
    use glib::ParamSpecBoxed;

    use super::*;

    #[derive(Debug, Default)]
    pub struct PostRow {
        pub(crate) url: RefCell<String>,
        pub(crate) title: RefCell<String>,
        pub(crate) body: RefCell<String>,
        pub(crate) tags: RefCell<Vec<String>>,
        pub(crate) author_id: RefCell<String>,
        pub(crate) author_name: RefCell<String>,
        pub(crate) avatar: libadwaita::Avatar,
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
                    ParamSpecString::builder("author_id").build(),
                    ParamSpecString::builder("author_name").build(),
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
                "author_id" => {
                    let author_id = value.get::<&str>().unwrap();
                    self.obj().set_author_id(author_id);
                }
                "author_name" => {
                    let author_name = value.get::<&str>().unwrap();
                    self.obj().set_author_name(author_name);
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
                "author_id" => self.obj().author_id().to_value(),
                "author_name" => self.obj().author_name().to_value(),
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
        let _imp = self.imp();
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
    pub fn author_name(&self) -> String {
        self.imp().author_name.borrow().to_string()
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
    pub fn set_author_name(&self, author_name: &str) {
        let imp = self.imp();

        imp.author_name.replace(author_name.to_string());
    }
}
