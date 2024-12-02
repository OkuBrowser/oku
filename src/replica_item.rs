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
use std::cell::RefCell;
use std::sync::LazyLock;

pub mod imp {
    use super::*;

    #[derive(Default, Debug)]
    pub struct ReplicaItem {
        pub(crate) id: RefCell<String>,
        pub(crate) writable: RefCell<bool>,
        pub(crate) home: RefCell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ReplicaItem {
        const NAME: &'static str = "OkuReplicaItem";
        type Type = super::ReplicaItem;
    }

    impl ObjectImpl for ReplicaItem {
        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: LazyLock<Vec<ParamSpec>> = LazyLock::new(|| {
                vec![
                    ParamSpecString::builder("id").readwrite().build(),
                    ParamSpecBoolean::builder("writable").readwrite().build(),
                    ParamSpecBoolean::builder("home").readwrite().build(),
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
                "home" => {
                    let home = value.get::<bool>().unwrap();
                    self.home.set(home);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &ParamSpec) -> Value {
            let obj = self.obj();
            match pspec.name() {
                "id" => obj.id().to_value(),
                "writable" => obj.writable().to_value(),
                "home" => obj.home().to_value(),
                _ => unimplemented!(),
            }
        }
    }
}

glib::wrapper! {
    pub struct ReplicaItem(ObjectSubclass<imp::ReplicaItem>);
}

unsafe impl Send for ReplicaItem {}
unsafe impl Sync for ReplicaItem {}

impl ReplicaItem {
    pub fn id(&self) -> String {
        self.imp().id.borrow().to_string()
    }
    pub fn writable(&self) -> bool {
        *self.imp().writable.borrow()
    }
    pub fn home(&self) -> bool {
        *self.imp().home.borrow()
    }
    pub fn new(id: String, writable: bool, home: bool) -> Self {
        let replica_item = glib::Object::builder::<Self>()
            .property("id", id)
            .property("writable", writable)
            .property("home", home)
            .build();

        replica_item
    }
}
