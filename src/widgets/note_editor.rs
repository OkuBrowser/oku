use crate::database::{Bookmark, DATABASE};
use crate::scheme_handlers::oku_path::OkuPath;
use crate::HOME_REPLICA_SET;
use crate::NODE;
use glib::clone;
use glib::subclass::object::ObjectImpl;
use glib::subclass::types::ObjectSubclass;
use glib::subclass::types::ObjectSubclassExt;
use glib::subclass::types::ObjectSubclassIsExt;
use glib::value::ToValue;
use glib::ParamSpec;
use glib::ParamSpecBoxed;
use glib::ParamSpecString;
use glib::Value;
use gtk::prelude::BoxExt;
use gtk::prelude::ButtonExt;
use gtk::prelude::WidgetExt;
use gtk::subclass::prelude::*;
use gtk::{glib, StringObject};
use libadwaita::prelude::PreferencesRowExt;
use libadwaita::prelude::*;
use libadwaita::subclass::dialog::AdwDialogImpl;
use log::error;
use oku_fs::database::posts::OkuNote;
use std::cell::RefCell;
use std::collections::HashSet;
use std::sync::atomic::Ordering;
use std::sync::LazyLock;
use webkit2gtk::functions::uri_for_display;
use webkit2gtk::prelude::WebViewExt;

pub mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct NoteEditor {
        pub(crate) url: RefCell<String>,
        pub(crate) title: RefCell<String>,
        pub(crate) body: RefCell<String>,
        pub(crate) tags: RefCell<Vec<String>>,
        pub(crate) main_box: gtk::Box,
        pub(crate) headerbar: libadwaita::HeaderBar,
        pub(crate) content_box: gtk::Box,
        pub(crate) url_entry: libadwaita::EntryRow,
        pub(crate) title_entry: libadwaita::EntryRow,
        pub(crate) tag_entry: libadwaita::EntryRow,
        pub(crate) row_list_box: gtk::ListBox,
        pub(crate) body_buffer: gtk::TextBuffer,
        pub(crate) body_entry: gtk::TextView,
        pub(crate) body_entry_label: gtk::Label,
        pub(crate) body_entry_label_overlay: gtk::Overlay,
        pub(crate) save_bookmark_button_content: libadwaita::ButtonContent,
        pub(crate) save_bookmark_button: gtk::Button,
        pub(crate) save_post_button_content: libadwaita::ButtonContent,
        pub(crate) save_post_button: gtk::Button,
        pub(crate) save_buttons: gtk::Box,
        pub(crate) tag_box: gtk::Box,
        pub(crate) tag_list: gtk::StringList,
        pub(crate) tag_factory: gtk::SignalListItemFactory,
        pub(crate) tag_model: gtk::SingleSelection,
        pub(crate) tag_view: gtk::ListView,
        pub(crate) tag_scrolled_window: gtk::ScrolledWindow,
    }

    impl NoteEditor {}

    #[glib::object_subclass]
    impl ObjectSubclass for NoteEditor {
        const NAME: &'static str = "OkuNoteEditor";
        type Type = super::NoteEditor;
        type ParentType = libadwaita::Dialog;
    }

    impl ObjectImpl for NoteEditor {
        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: LazyLock<Vec<ParamSpec>> = LazyLock::new(|| {
                vec![
                    ParamSpecString::builder("url").build(),
                    ParamSpecString::builder("title-property").build(),
                    ParamSpecString::builder("body").build(),
                    ParamSpecBoxed::builder::<Vec<String>>("tags").build(),
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
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &ParamSpec) -> Value {
            match pspec.name() {
                "url" => self.obj().url().to_string().to_value(),
                "title-property" => self.obj().title_property().to_value(),
                "body" => self.obj().body().to_value(),
                "tags" => self.obj().tags().to_value(),
                _ => unimplemented!(),
            }
        }
    }
    impl WidgetImpl for NoteEditor {}
    impl AdwDialogImpl for NoteEditor {}
}

glib::wrapper! {
    pub struct NoteEditor(ObjectSubclass<imp::NoteEditor>)
    @extends libadwaita::Dialog, gtk::Widget;
}

impl NoteEditor {
    pub fn new(window: Option<&super::window::Window>, bookmark: Option<Bookmark>) -> Self {
        let this: Self = glib::Object::builder::<Self>().build();
        let imp = this.imp();

        imp.url_entry
            .property_expression("text")
            .bind(&this, "url", gtk::Widget::NONE);
        imp.url_entry.set_title("URL");

        imp.title_entry.property_expression("text").bind(
            &this,
            "title-property",
            gtk::Widget::NONE,
        );
        imp.title_entry.set_title("Title");

        imp.tag_entry.set_title("Tag");
        imp.tag_entry.set_show_apply_button(true);
        imp.tag_entry.connect_apply(clone!(
            #[weak]
            this,
            #[weak]
            imp,
            move |tag_entry| {
                if !tag_entry.text().trim().is_empty()
                    && !imp
                        .tags
                        .borrow()
                        .contains(&tag_entry.text().trim().to_owned())
                {
                    this.append_tag(tag_entry.text().to_string());
                    tag_entry.set_text("");
                }
            }
        ));

        imp.row_list_box.append(&imp.url_entry);
        imp.row_list_box.append(&imp.title_entry);
        imp.row_list_box.append(&imp.tag_entry);
        imp.row_list_box.add_css_class("boxed-list");

        imp.body_entry.set_buffer(Some(&imp.body_buffer));
        imp.body_buffer
            .property_expression("text")
            .bind(&this, "body", gtk::Widget::NONE);
        imp.body_entry
            .set_vscroll_policy(gtk::ScrollablePolicy::Natural);
        imp.body_entry
            .set_hscroll_policy(gtk::ScrollablePolicy::Natural);
        imp.body_entry.set_left_margin(13);
        imp.body_entry.set_top_margin(24);
        imp.body_entry.set_bottom_margin(10);
        imp.body_entry.set_height_request(128);
        imp.body_entry.set_vexpand(true);
        imp.body_entry.set_hexpand(true);
        imp.body_entry.add_css_class("card");

        imp.body_entry_label.set_label("Body");
        imp.body_entry_label.add_css_class("subtitle");
        imp.body_entry_label.add_css_class("dim-label");
        imp.body_entry_label.add_css_class("caption");
        imp.body_entry_label.set_halign(gtk::Align::Start);
        imp.body_entry_label.set_valign(gtk::Align::Start);
        imp.body_entry_label.set_margin_start(13);
        imp.body_entry_label.set_margin_top(7);
        imp.body_entry_label_overlay
            .set_child(Some(&imp.body_entry));
        imp.body_entry_label_overlay
            .add_overlay(&imp.body_entry_label);

        imp.save_bookmark_button_content
            .set_icon_name("bookmark-filled-symbolic");
        imp.save_bookmark_button_content.set_label("Save bookmark");
        imp.save_bookmark_button
            .set_child(Some(&imp.save_bookmark_button_content));
        imp.save_bookmark_button.add_css_class("linked");
        imp.save_bookmark_button.add_css_class("pill");
        imp.save_bookmark_button.connect_clicked(clone!(
            #[weak]
            this,
            #[strong]
            bookmark,
            move |_| {
                if let Some(bookmark) = &bookmark {
                    if let Err(e) = DATABASE.delete_bookmark(bookmark.clone()) {
                        error!("{}", e)
                    }
                }
                if let Err(e) = DATABASE.upsert_bookmark(Bookmark {
                    url: this.url(),
                    title: this.title_property(),
                    body: this.body(),
                    tags: HashSet::from_iter(this.tags().into_iter()),
                }) {
                    error!("{}", e)
                }
                this.close();
            }
        ));

        imp.save_post_button_content
            .set_icon_name("people-symbolic");
        imp.save_post_button_content.set_label("Save to OkuNet");
        imp.save_post_button
            .set_child(Some(&imp.save_post_button_content));
        imp.save_post_button.add_css_class("linked");
        imp.save_post_button.add_css_class("pill");

        imp.save_buttons.append(&imp.save_bookmark_button);
        imp.save_buttons.append(&imp.save_post_button);
        imp.save_buttons.set_halign(gtk::Align::Center);
        imp.save_buttons.add_css_class("linked");

        this.setup_tag_list();

        imp.content_box.set_orientation(gtk::Orientation::Vertical);
        imp.content_box.set_spacing(8);
        imp.content_box.add_css_class("toolbar");
        imp.content_box.set_width_request(400);
        imp.content_box.append(&imp.row_list_box);
        imp.content_box.append(&imp.tag_box);
        imp.content_box.append(&imp.body_entry_label_overlay);
        imp.content_box.append(&imp.save_buttons);

        this.set_title("Note");
        imp.headerbar.add_css_class("flat");
        imp.main_box.set_orientation(gtk::Orientation::Vertical);
        imp.main_box.append(&imp.headerbar);
        imp.main_box.append(&imp.content_box);
        this.set_child(Some(&imp.main_box));

        if let Some(bookmark) = bookmark {
            imp.url_entry.set_text(&bookmark.url);
            imp.title_entry.set_text(&bookmark.title);
            imp.body_buffer.set_text(&bookmark.body);
            this.set_tags(bookmark.tags.into_iter().collect());
        } else if let Some(window) = window {
            let view = window.get_view();
            let url = view.uri().unwrap_or_default().to_string();
            let title = view.title().unwrap_or_default().to_string();
            imp.url_entry.set_text(&url);
            imp.title_entry.set_text(&title);

            glib::spawn_future_local(clone!(
                #[weak]
                this,
                #[weak]
                imp,
                async move {
                    if let Some(node) = NODE.get() {
                        let url = uri_for_display(&url)
                            .map(|x| x.to_string())
                            .unwrap_or(url)
                            .replacen("oku:", "", 1);
                        let post_at_url = if let Ok(oku_path) = OkuPath::parse(url.clone()) {
                            match oku_path {
                                OkuPath::User(author_id, Some(path)) => {
                                    if node.is_me(&author_id) {
                                        node.post(format!("{}.toml", path.to_string_lossy()).into())
                                            .await
                                            .ok()
                                    } else {
                                        None
                                    }
                                }
                                OkuPath::Me(Some(path)) => node
                                    .post(format!("{}.toml", path.to_string_lossy()).into())
                                    .await
                                    .ok(),
                                _ => None,
                            }
                        } else {
                            None
                        };
                        let post_from_url = {
                            let path = OkuNote::suggested_post_path_from_url(url);
                            node.post(format!("{}.toml", path).into()).await.ok()
                        };
                        if let Some(oku_post) = post_at_url.or(post_from_url) {
                            imp.url_entry.set_text(oku_post.note.url.as_ref());
                            imp.title_entry.set_text(&oku_post.note.title);
                            imp.body_buffer.set_text(&oku_post.note.body);
                            this.set_tags(oku_post.note.tags.into_iter().collect());
                        }
                    }
                }
            ));
        }

        match HOME_REPLICA_SET.load(Ordering::Relaxed) {
            true => {
                imp.save_post_button.connect_clicked(clone!(
                    #[weak]
                    this,
                    move |_| {
                        let ctx = glib::MainContext::default();
                        ctx.spawn_local_with_priority(
                            glib::source::Priority::HIGH,
                            clone!(
                                #[weak]
                                this,
                                async move {
                                    if let Some(node) = NODE.get() {
                                        match url::Url::parse(&this.url()) {
                                            Ok(parsed_url) => {
                                                match node
                                                    .create_or_modify_post(
                                                        None,
                                                        parsed_url,
                                                        this.title_property(),
                                                        this.body(),
                                                        HashSet::from_iter(this.tags().into_iter()),
                                                    )
                                                    .await
                                                {
                                                    Ok(_) => {
                                                        this.close();
                                                    }
                                                    Err(e) => {
                                                        error!("{}", e);
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                error!("{}", e);
                                            }
                                        }
                                    }
                                }
                            ),
                        );
                    }
                ));
            }
            false => {
                imp.save_post_button.set_sensitive(false);
                imp.save_post_button
                    .set_tooltip_text(Some("A home replica is required to post to OkuNet."));
            }
        }

        this.set_follows_content_size(true);
        this.set_visible(true);
        this.present(window);

        this
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
        self.imp().tags.borrow().clone()
    }
    pub fn set_url(&self, url: String) {
        let imp = self.imp();

        imp.url.replace(url);
    }
    pub fn set_title_property(&self, title: String) {
        let imp = self.imp();

        imp.title.replace(title);
    }
    pub fn set_body(&self, body: String) {
        let imp = self.imp();

        imp.body.replace(body);
    }
    pub fn set_tags(&self, tags: Vec<String>) {
        for tag in self.tags() {
            self.delete_tag(tag);
        }
        for tag in tags {
            self.append_tag(tag);
        }
    }
    pub fn append_tag(&self, tag: String) {
        let imp = self.imp();

        imp.tag_list.append(&tag);
        imp.tags.borrow_mut().push(tag);
    }
    pub fn delete_tag(&self, tag: String) {
        let imp = self.imp();

        if let Some(tag_position) = imp
            .tag_list
            .snapshot()
            .iter()
            .filter_map(|x| x.downcast_ref::<StringObject>())
            .position(|x| x.string() == tag)
        {
            imp.tag_list.remove(tag_position as u32);
            imp.tags.borrow_mut().retain(|x| *x != tag);
        }
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

        imp.tag_factory.connect_setup(clone!(
            #[weak(rename_to = this)]
            self,
            move |_, item| {
                let tag = crate::widgets::tag::Tag::new();
                let list_item = item.downcast_ref::<gtk::ListItem>().unwrap();
                list_item.set_child(Some(&tag));
                list_item
                    .property_expression("item")
                    .chain_property::<StringObject>("string")
                    .bind(&tag, "text", gtk::Widget::NONE);
                tag.imp().delete_button.connect_clicked(clone!(
                    #[weak]
                    this,
                    #[weak]
                    tag,
                    move |_| {
                        this.delete_tag(tag.text());
                    }
                ));
            }
        ));

        imp.tag_view.set_model(Some(&imp.tag_model));
        imp.tag_view.set_factory(Some(&imp.tag_factory));
        imp.tag_view.set_enable_rubberband(false);
        imp.tag_view
            .set_hscroll_policy(gtk::ScrollablePolicy::Minimum);
        imp.tag_view
            .set_vscroll_policy(gtk::ScrollablePolicy::Natural);
        imp.tag_view.set_vexpand(true);
        imp.tag_view.add_css_class("boxed-list-separate");
        imp.tag_view.add_css_class("navigation-sidebar");

        imp.tag_scrolled_window.set_child(Some(&imp.tag_view));
        imp.tag_scrolled_window
            .set_hscrollbar_policy(gtk::PolicyType::Never);
        imp.tag_scrolled_window.set_propagate_natural_height(true);
        imp.tag_scrolled_window.set_propagate_natural_width(true);

        imp.tag_box.set_orientation(gtk::Orientation::Horizontal);
        imp.tag_box.set_spacing(4);
        imp.tag_box.append(&imp.tag_scrolled_window);
    }
}
