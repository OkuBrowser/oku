use glib::clone;
use glib::object::ObjectExt;
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
use std::cell::RefCell;
use std::sync::LazyLock;
use webkit2gtk::functions::uri_for_display;

pub mod imp {
    use super::*;

    #[derive(Default, Debug)]
    pub struct SuggestionItem {
        pub(crate) title: RefCell<String>,
        pub(crate) uri: RefCell<String>,
        pub(crate) favicon: RefCell<Option<gdk::Texture>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SuggestionItem {
        const NAME: &'static str = "OkuSuggestionItem";
        type Type = super::SuggestionItem;
    }

    impl ObjectImpl for SuggestionItem {
        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: LazyLock<Vec<ParamSpec>> = LazyLock::new(|| {
                vec![
                    ParamSpecString::builder("title").readwrite().build(),
                    ParamSpecString::builder("uri").readwrite().build(),
                    ParamSpecObject::builder::<gdk::Texture>("favicon")
                        .readwrite()
                        .build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
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
                "favicon" => {
                    let favicon = value.get::<gdk::Texture>().unwrap();
                    self.favicon.set(Some(favicon));
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &ParamSpec) -> Value {
            let obj = self.obj();
            match pspec.name() {
                "title" => obj.title().to_value(),
                "uri" => obj.uri().to_value(),
                "favicon" => obj.favicon().to_value(),
                _ => unimplemented!(),
            }
        }
    }
}

glib::wrapper! {
    pub struct SuggestionItem(ObjectSubclass<imp::SuggestionItem>);
}

impl SuggestionItem {
    pub fn title(&self) -> String {
        self.imp().title.borrow().to_string()
    }
    pub fn uri(&self) -> String {
        self.imp().uri.borrow().to_string()
    }
    pub fn favicon(&self) -> Option<gdk::Texture> {
        self.imp().favicon.borrow().clone()
    }
    pub fn new(title: String, uri: String, favicon_database: &webkit2gtk::FaviconDatabase) -> Self {
        let suggestion_item = glib::Object::builder::<Self>()
            .property("title", title)
            .property("uri", uri.clone())
            .build();

        favicon_database.favicon(
            &uri,
            Some(&gio::Cancellable::new()),
            clone!(
                #[weak]
                suggestion_item,
                move |favicon_result| {
                    if let Ok(favicon) = favicon_result {
                        suggestion_item.set_property("favicon", favicon);
                    }
                }
            ),
        );

        suggestion_item
    }
}
