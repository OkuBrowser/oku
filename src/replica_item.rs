use glib::property::PropertySet;
use glib::subclass::object::ObjectImpl;
use glib::subclass::types::ObjectSubclass;
use glib::subclass::types::ObjectSubclassExt;
use glib::subclass::types::ObjectSubclassIsExt;
use glib::value::ToValue;
use glib::ParamSpec;
use glib::ParamSpecBoolean;
use glib::ParamSpecBuilderExt;
use glib::ParamSpecString;
use glib::Value;
use once_cell::sync::Lazy;
use std::cell::RefCell;

pub mod imp {
    use super::*;

    #[derive(Default, Debug)]
    pub struct ReplicaItem {
        pub(crate) id: RefCell<String>,
        pub(crate) writable: RefCell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ReplicaItem {
        const NAME: &'static str = "OkuReplicaItem";
        type Type = super::ReplicaItem;
    }

    impl ObjectImpl for ReplicaItem {
        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpecString::builder("id").readwrite().build(),
                    ParamSpecBoolean::builder("writable").readwrite().build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "id" => {
                    let id = value.get::<String>().unwrap();
                    self.id.set(html_escape::encode_text(&id).to_string());
                }
                "writable" => {
                    let writable = value.get::<bool>().unwrap();
                    self.writable.set(writable);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &ParamSpec) -> Value {
            let obj = self.obj();
            match pspec.name() {
                "id" => obj.id().to_value(),
                "writable" => obj.writable().to_value(),
                _ => unimplemented!(),
            }
        }
    }
}

glib::wrapper! {
    pub struct ReplicaItem(ObjectSubclass<imp::ReplicaItem>);
}

impl ReplicaItem {
    pub fn id(&self) -> String {
        self.imp().id.borrow().to_string()
    }
    pub fn writable(&self) -> bool {
        self.imp().writable.borrow().clone()
    }
    pub fn new(id: String, writable: bool) -> Self {
        let replica_item = glib::Object::builder::<Self>()
            .property("id", id)
            .property("writable", writable)
            .build();

        replica_item
    }
}
