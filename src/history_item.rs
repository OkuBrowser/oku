use glib::clone;
use glib::property::PropertySet;
use glib::subclass::object::ObjectImpl;
use glib::subclass::types::ObjectSubclass;
use glib::subclass::types::ObjectSubclassExt;
use glib::subclass::types::ObjectSubclassIsExt;
use glib::value::ToValue;
use glib::ParamSpec;
use glib::ParamSpecBuilderExt;
use glib::ParamSpecObject;
use glib::ParamSpecString;
use glib::Value;
use once_cell::sync::Lazy;
use std::cell::RefCell;
use uuid::Uuid;
use webkit2gtk::functions::uri_for_display;

pub mod imp {
    use super::*;

    #[derive(Default, Debug)]
    pub struct HistoryItem {
        pub(crate) id: RefCell<Uuid>,
        pub(crate) title: RefCell<String>,
        pub(crate) uri: RefCell<String>,
        pub(crate) timestamp: RefCell<String>,
        pub(crate) favicon: RefCell<Option<gdk::Texture>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for HistoryItem {
        const NAME: &'static str = "OkuHistoryItem";
        type Type = super::HistoryItem;
    }

    impl ObjectImpl for HistoryItem {
        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpecString::builder("id").readwrite().build(),
                    ParamSpecString::builder("title").readwrite().build(),
                    ParamSpecString::builder("uri").readwrite().build(),
                    ParamSpecString::builder("timestamp").readwrite().build(),
                    ParamSpecObject::builder::<gdk::Texture>("favicon")
                        .readwrite()
                        .build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "id" => {
                    let id = value.get::<String>().unwrap();
                    self.id.set(Uuid::parse_str(&id).unwrap());
                }
                "uri" => {
                    let uri = value.get::<String>().unwrap();
                    self.uri.set(
                        html_escape::encode_text(&uri_for_display(&uri).unwrap_or(uri.into()))
                            .to_string(),
                    );
                }
                "title" => {
                    let title = value.get::<String>().unwrap();
                    self.title.set(html_escape::encode_text(&title).to_string());
                }
                "timestamp" => {
                    let timestamp = value.get::<String>().unwrap();
                    self.timestamp
                        .set(html_escape::encode_text(&timestamp).to_string());
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
                "id" => obj.id().to_string().to_value(),
                "title" => obj.title().to_value(),
                "uri" => obj.uri().to_value(),
                "timestamp" => obj.timestamp().to_value(),
                "favicon" => obj.favicon().to_value(),
                _ => unimplemented!(),
            }
        }
    }
}

glib::wrapper! {
    pub struct HistoryItem(ObjectSubclass<imp::HistoryItem>);
}

impl HistoryItem {
    pub fn id(&self) -> Uuid {
        self.imp().id.borrow().to_owned()
    }
    pub fn title(&self) -> String {
        self.imp().title.borrow().to_string()
    }
    pub fn timestamp(&self) -> String {
        self.imp().timestamp.borrow().to_string()
    }
    pub fn uri(&self) -> String {
        self.imp().uri.borrow().to_string()
    }
    pub fn favicon(&self) -> Option<gdk::Texture> {
        self.imp().favicon.borrow().clone()
    }
    pub fn new(
        id: Uuid,
        title: String,
        uri: String,
        timestamp: String,
        favicon_database: &webkit2gtk::FaviconDatabase,
    ) -> Self {
        let history_item = glib::Object::builder::<Self>()
            .property("id", id.to_string())
            .property("title", title)
            .property("timestamp", timestamp)
            .property("uri", uri.clone())
            .build();

        favicon_database.favicon(
            &uri,
            Some(&gio::Cancellable::new()),
            clone!(
                #[weak]
                history_item,
                move |favicon_result| {
                    history_item.imp().favicon.set(favicon_result.ok());
                }
            ),
        );

        history_item
    }
}
