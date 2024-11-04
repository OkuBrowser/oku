use glib::clone;
use glib::property::PropertySet;
use glib::subclass::object::ObjectImpl;
use glib::subclass::types::ObjectSubclass;
use glib::subclass::types::ObjectSubclassExt;
use glib::subclass::types::ObjectSubclassIsExt;
use glib::value::ToValue;
use glib::ParamSpec;
use glib::ParamSpecBoxed;
use glib::ParamSpecBuilderExt;
use glib::ParamSpecObject;
use glib::ParamSpecString;
use glib::Value;
use once_cell::sync::Lazy;
use std::cell::RefCell;
use webkit2gtk::functions::uri_for_display;

pub mod imp {
    use super::*;

    #[derive(Default, Debug)]
    pub struct BookmarkItem {
        pub(crate) url: RefCell<String>,
        pub(crate) title: RefCell<String>,
        pub(crate) body: RefCell<String>,
        pub(crate) tags: RefCell<Vec<String>>,
        pub(crate) favicon: RefCell<Option<gdk::Texture>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for BookmarkItem {
        const NAME: &'static str = "OkuBookmarkItem";
        type Type = super::BookmarkItem;
    }

    impl ObjectImpl for BookmarkItem {
        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpecString::builder("url").readwrite().build(),
                    ParamSpecString::builder("title").readwrite().build(),
                    ParamSpecString::builder("body").readwrite().build(),
                    ParamSpecBoxed::builder::<Vec<String>>("tags")
                        .readwrite()
                        .build(),
                    ParamSpecObject::builder::<gdk::Texture>("favicon")
                        .readwrite()
                        .build(),
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
                "favicon" => {
                    let favicon = value.get::<gdk::Texture>().ok();
                    self.favicon.set(favicon);
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
                "favicon" => obj.favicon().to_value(),
                _ => unimplemented!(),
            }
        }
    }
}

glib::wrapper! {
    pub struct BookmarkItem(ObjectSubclass<imp::BookmarkItem>);
}

impl BookmarkItem {
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
    pub fn favicon(&self) -> Option<gdk::Texture> {
        self.imp().favicon.borrow().clone()
    }
    pub fn new(
        url: String,
        title: String,
        body: String,
        tags: Vec<String>,
        favicon_database: &webkit2gtk::FaviconDatabase,
    ) -> Self {
        let bookmark_item = glib::Object::builder::<Self>()
            .property("url", url.clone())
            .property("title", title)
            .property("body", body)
            .property("tags", tags.clone())
            .build();

        favicon_database.favicon(
            &url,
            Some(&gio::Cancellable::new()),
            clone!(
                #[weak]
                bookmark_item,
                move |favicon_result| {
                    bookmark_item.imp().favicon.set(favicon_result.ok());
                }
            ),
        );

        bookmark_item
    }
}
