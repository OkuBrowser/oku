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
use glib::ParamSpecString;
use glib::Value;
use glib::{closure, Object};
use glib::{ParamSpecBoxed, ParamSpecUInt64};
use gtk::glib;
use gtk::prelude::BoxExt;
use gtk::prelude::ListBoxRowExt;
use gtk::prelude::WidgetExt;
use gtk::subclass::prelude::*;
use gtk::StringObject;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::sync::LazyLock;

pub mod imp {
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
        pub(crate) timestamp: RefCell<u64>,
        // Widgets
        // Tags
        pub(crate) tag_box: gtk::Box,
        pub(crate) tag_list: gtk::StringList,
        pub(crate) tag_factory: gtk::SignalListItemFactory,
        pub(crate) tag_model: gtk::SingleSelection,
        pub(crate) tag_view: gtk::ListView,
        pub(crate) tag_scrolled_window: gtk::ScrolledWindow,
        // Top box
        pub(crate) author_name_label: gtk::Label,
        pub(crate) author_id_label: gtk::Label,
        pub(crate) author_avatar: libadwaita::Avatar,
        pub(crate) timestamp_label: gtk::Label,
        pub(crate) post_top_box: gtk::Box,
        // Middle box
        pub(crate) url_label: gtk::Label,
        pub(crate) title_label: gtk::Label,
        pub(crate) post_middle_box: gtk::Box,
        // Body box
        pub(crate) body_label: gtk::Label,
        pub(crate) post_body_box: gtk::Box,
        // Main
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
                    ParamSpecUInt64::builder("timestamp").build(),
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
                    self.obj().set_tags(&tags);
                }
                "author-id" => {
                    let author_id = value.get::<&str>().unwrap();
                    self.obj().set_author_id(author_id);
                }
                "author-name" => {
                    let author_name = value.get::<Option<&str>>().unwrap();
                    self.obj().set_author_name(&author_name);
                }
                "timestamp" => {
                    let timestamp = value.get::<u64>().unwrap();
                    self.obj().set_timestamp(&timestamp);
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
                "timestamp" => self.obj().timestamp().to_value(),
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

    pub fn setup_tag_list(&self) {
        let imp = self.imp();

        imp.tag_model.set_model(Some(&imp.tag_list));
        imp.tag_model.set_autoselect(false);
        imp.tag_model.set_can_unselect(true);
        imp.tag_model
            .connect_selected_item_notify(clone!(move |tag_model| {
                tag_model.unselect_all();
            }));

        imp.tag_factory.connect_setup(clone!(move |_, item| {
            let tag = crate::widgets::tag::Tag::new();
            tag.set_deletable(&false);
            let list_item = item.downcast_ref::<gtk::ListItem>().unwrap();
            list_item.set_child(Some(&tag));
            list_item
                .property_expression("item")
                .chain_property::<StringObject>("string")
                .bind(&tag, "text", gtk::Widget::NONE);
        }));

        imp.tag_view.set_model(Some(&imp.tag_model));
        imp.tag_view.set_factory(Some(&imp.tag_factory));
        imp.tag_view
            .set_layout_manager(Some(libadwaita::WrapLayout::new()));
        imp.tag_view.set_enable_rubberband(false);
        imp.tag_view
            .set_hscroll_policy(gtk::ScrollablePolicy::Minimum);
        imp.tag_view
            .set_vscroll_policy(gtk::ScrollablePolicy::Natural);
        imp.tag_view.set_vexpand(true);
        imp.tag_view.add_css_class("boxed-list-separate");
        imp.tag_view.add_css_class("navigation-sidebar");
        imp.tag_view
            .set_layout_manager(Some(libadwaita::WrapLayout::new()));

        imp.tag_scrolled_window.set_child(Some(&imp.tag_view));
        imp.tag_scrolled_window.set_propagate_natural_height(true);
        imp.tag_scrolled_window.set_propagate_natural_width(true);

        imp.tag_box.set_orientation(gtk::Orientation::Horizontal);
        imp.tag_box.set_spacing(4);
        imp.tag_box.append(&imp.tag_scrolled_window);
    }

    pub fn setup(&self) {
        let imp = self.imp();

        imp.author_avatar.set_show_initials(true);
        imp.author_avatar.set_size(32);

        self.bind_property("title", &imp.title_label, "label")
            .build();
        self.bind_property("url", &imp.url_label, "label").build();
        self.bind_property("body", &imp.body_label, "label").build();
        self.bind_property("author-name", &imp.author_name_label, "label")
            .build();
        self.property_expression("author-id")
            .chain_closure::<String>(closure!(|_: Option<Object>, author_id: String| {
                format!("@{author_id}")
            }))
            .bind(&imp.author_id_label, "label", gtk::Widget::NONE);
        self.property_expression("timestamp")
            .chain_closure::<String>(closure!(|_: Option<Object>, timestamp: u64| {
                chrono::DateTime::from_timestamp_micros(timestamp.try_into().unwrap_or(0))
                    .map(|x| x.to_rfc2822())
                    .unwrap_or_default()
            }))
            .bind(&imp.author_id_label, "label", gtk::Widget::NONE);
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

        self.setup_tag_list();

        imp.author_id_label.add_css_class("dimmed");
        imp.timestamp_label.add_css_class("dimmed");
        imp.url_label.add_css_class("dimmed");

        imp.post_top_box.append(&imp.author_avatar);
        imp.post_top_box.append(&imp.author_name_label);
        imp.post_top_box.append(&imp.author_id_label);
        imp.post_top_box.append(&imp.timestamp_label);
        imp.post_top_box.set_vexpand(true);
        imp.post_top_box.set_hexpand(true);
        imp.post_top_box
            .set_orientation(gtk::Orientation::Horizontal);

        imp.post_middle_box.append(&imp.title_label);
        imp.post_middle_box.append(&imp.url_label);
        imp.post_middle_box.set_vexpand(true);
        imp.post_middle_box.set_hexpand(true);
        imp.post_middle_box
            .set_orientation(gtk::Orientation::Horizontal);

        imp.post_body_box.append(&imp.tag_box);
        imp.post_body_box.append(&imp.body_label);
        imp.post_body_box.set_vexpand(true);
        imp.post_body_box.set_hexpand(true);
        imp.post_body_box
            .set_orientation(gtk::Orientation::Vertical);

        imp.main.append(&imp.post_top_box);
        imp.main.append(&imp.post_middle_box);
        imp.main.append(&imp.post_body_box);
        imp.main.set_vexpand(true);
        imp.main.set_hexpand(true);
        imp.main.set_orientation(gtk::Orientation::Vertical);

        self.set_child(Some(&imp.main));
        self.set_vexpand(true);
        self.set_hexpand(true);
        self.add_css_class("card");
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
    pub fn timestamp(&self) -> u64 {
        *self.imp().timestamp.borrow()
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
        imp.tag_list.splice(
            0,
            imp.tag_list.n_items(),
            &imp.tags
                .borrow()
                .iter()
                .map(|x| x.as_str())
                .collect::<Vec<&str>>()[..],
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
    pub fn set_timestamp(&self, timestamp: &u64) {
        let imp = self.imp();
        imp.timestamp.set(*timestamp);
    }
}
