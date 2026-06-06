// Not used yet.

use glib::object::ObjectExt;
use glib::property::PropertySet;
use glib::subclass::object::ObjectImpl;
use glib::subclass::types::ObjectSubclass;
use glib::subclass::types::ObjectSubclassExt;
use glib::subclass::types::ObjectSubclassIsExt;
use glib::value::ToValue;
use glib::ParamSpec;
use glib::ParamSpecBoxed;
use glib::ParamSpecBuilderExt;
use glib::ParamSpecString;
use glib::Value;
use oku_core::database::posts::core::OkuPost;
use std::cell::RefCell;
use std::sync::LazyLock;
use webkit2gtk::functions::uri_for_display;

pub mod imp {

    use super::*;

    #[derive(Default, Debug)]
    pub struct PostItem {
        pub(crate) url: RefCell<String>,
        pub(crate) title: RefCell<String>,
        pub(crate) body: RefCell<String>,
        pub(crate) tags: RefCell<Vec<String>>,
        pub(crate) author_id: RefCell<String>,
        pub(crate) author_name: RefCell<Option<String>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PostItem {
        const NAME: &'static str = "OkuNetPostItem";
        type Type = super::PostItem;
    }

    impl ObjectImpl for PostItem {
        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: LazyLock<Vec<ParamSpec>> = LazyLock::new(|| {
                vec![
                    ParamSpecString::builder("url").readwrite().build(),
                    ParamSpecString::builder("title").readwrite().build(),
                    ParamSpecString::builder("body").readwrite().build(),
                    ParamSpecBoxed::builder::<Vec<String>>("tags")
                        .readwrite()
                        .build(),
                    ParamSpecString::builder("author_id").readwrite().build(),
                    ParamSpecString::builder("author_name").readwrite().build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "url" => {
                    let url = value.get::<String>().unwrap();
                    self.url.set(
                        html_escape::encode_text(&uri_for_display(&url).unwrap_or(url.into()))
                            .to_string(),
                    );
                }
                "title" => {
                    let title = value.get::<String>().unwrap();
                    self.title.set(html_escape::encode_text(&title).to_string());
                }
                "body" => {
                    let body = value.get::<String>().unwrap();
                    self.body.set(html_escape::encode_text(&body).to_string());
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
                    let author_id = value.get::<String>().unwrap();
                    self.author_id
                        .set(html_escape::encode_text(&author_id).to_string());
                }
                "author_name" => {
                    let author_name = value.get::<Option<String>>().ok().flatten();
                    match author_name {
                        Some(x) => self
                            .author_name
                            .set(Some(html_escape::encode_text(&x).to_string())),
                        None => self.author_name.set(None),
                    }
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &ParamSpec) -> Value {
            let obj = self.obj();
            match pspec.name() {
                "url" => obj.url().to_string().to_value(),
                "title" => obj.title().to_value(),
                "body" => obj.body().to_value(),
                "tags" => obj.tags().to_value(),
                "author_id" => obj.author_id().to_value(),
                "author_name" => obj.author_name().to_value(),
                _ => unimplemented!(),
            }
        }
    }
}

glib::wrapper! {
    pub struct PostItem(ObjectSubclass<imp::PostItem>);
}

unsafe impl Send for PostItem {}
unsafe impl Sync for PostItem {}

impl From<&OkuPost> for PostItem {
    fn from(value: &OkuPost) -> Self {
        glib::Object::builder::<Self>()
            .property("url", value.note.url.to_string())
            .property("title", &value.note.title)
            .property("body", &value.note.body)
            .property(
                "tags",
                value.note.tags.clone().into_iter().collect::<Vec<String>>(),
            )
            .property("author_id", oku_core::fs::util::fmt(value.entry.author()))
            .property("author_name", value.user().identity.map(|x| x.name))
            .build()
    }
}

impl From<OkuPost> for PostItem {
    fn from(value: OkuPost) -> Self {
        Self::from(&value)
    }
}

impl PostItem {
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
        self.imp().author_name.borrow().to_owned()
    }

    pub fn update(&self, post: OkuPost) {
        let ctx = glib::MainContext::default();
        let this = self.clone();
        ctx.invoke(move || {
            this.set_properties(&[
                ("url", &post.note.url.to_string()),
                ("title", &post.note.title),
                ("body", &post.note.body),
                (
                    "tags",
                    &post.note.tags.clone().into_iter().collect::<Vec<String>>(),
                ),
                ("author_id", &oku_core::fs::util::fmt(post.entry.author())),
                ("author_name", &post.user().identity.map(|x| x.name)),
            ]);
        });
    }
    pub fn new(post: &OkuPost) -> Self {
        Self::from(post)
    }
}
