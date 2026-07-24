use crate::okunet::items::post_item::PostItem;
use crate::window_util::get_view_stack_page_by_name;
use crate::NODE;
use gio::prelude::ListModelExtManual;
use glib::clone;
use glib::closure;
use glib::object::Cast;
use glib::subclass::object::ObjectImpl;
use glib::subclass::types::ObjectSubclass;
use glib::subclass::types::ObjectSubclassExt;
use glib::subclass::types::ObjectSubclassIsExt;
use rayon::iter::FromParallelIterator;
use rayon::prelude::ParallelSliceMut;

use glib::Object;
use gtk::prelude::BoxExt;
use gtk::prelude::GObjectPropertyExpressionExt;
use gtk::prelude::OrientableExt;
use gtk::prelude::ScrollableExt;
use gtk::prelude::SelectionModelExt;
use gtk::prelude::WidgetExt;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};
use libadwaita::prelude::*;
use log::error;
use log::info;
use std::cell::Cell;
use std::cell::Ref;
use std::cell::RefCell;
use std::cmp::Reverse;
use std::rc::Rc;

pub mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct Net {
        // Home page
        pub(crate) posts_initialised: Cell<bool>,
        pub(crate) posts_store: RefCell<Option<Rc<gio::ListStore>>>,
        pub(crate) posts_factory: gtk::SignalListItemFactory,
        pub(crate) posts_model: gtk::SingleSelection,
        pub(crate) posts_view: gtk::ListView,
        pub(crate) posts_scrolled_window: gtk::ScrolledWindow,
        pub(crate) posts_placeholder: gtk::Label,
        pub(crate) posts_box: gtk::Box,
        // Main
        pub(crate) main_box: gtk::Box,
        pub(crate) view_stack: libadwaita::ViewStack,
        pub(crate) view_switcher: libadwaita::ViewSwitcher,
    }

    impl Net {}

    #[glib::object_subclass]
    impl ObjectSubclass for Net {
        const NAME: &'static str = "OkuNet";
        type Type = super::Net;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.set_layout_manager_type::<gtk::BoxLayout>();
            klass.set_accessible_role(gtk::AccessibleRole::Generic);
        }
    }

    impl ObjectImpl for Net {
        fn dispose(&self) {
            while let Some(child) = self.obj().first_child() {
                child.unparent();
            }
        }

        fn constructed(&self) {
            self.parent_constructed();

            self.obj().setup();
        }
    }
    impl WidgetImpl for Net {}
    impl BoxImpl for Net {}
}

glib::wrapper! {
    pub struct Net(ObjectSubclass<imp::Net>)
    @extends gtk::Box, gtk::Widget,
    @implements gtk::Accessible, gtk::Actionable, gtk::Orientable, gtk::Buildable, gtk::ConstraintTarget;
}

unsafe impl Send for Net {}
unsafe impl Sync for Net {}

impl Default for Net {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl Net {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn watch_posts(&self) {
        if let Some(node) = NODE.get() {
            self.imp().posts_initialised.set(true);
            let mut post_rx = node.okunet_post_sender.subscribe();
            loop {
                post_rx.borrow_and_update();
                info!("Posts updated … ");
                let this = self.clone();
                tokio::spawn(async move {
                    this.posts_updated().await;
                });
                match post_rx.changed().await {
                    Ok(_) => continue,
                    Err(e) => {
                        error!("{}", e);
                        break;
                    }
                }
            }
        }
    }

    pub fn watch(&self) {
        let this = self.clone();
        tokio::spawn(async move { this.watch_posts().await });
    }

    pub fn posts_store(&self) -> Ref<'_, gio::ListStore> {
        let posts_store = self.imp().posts_store.borrow();

        Ref::map(posts_store, |posts_store| {
            let posts_store: &gio::ListStore = posts_store.as_deref().unwrap();
            posts_store
        })
    }

    pub async fn posts_updated(&self) {
        if let Some(home_page) =
            get_view_stack_page_by_name("home".to_string(), &self.imp().view_stack)
        {
            home_page.child().set_sensitive(false);
        }

        let node = NODE.get().expect("Node initialised");
        tokio::spawn(node.refresh_users());

        let mut posts = Vec::from_par_iter(node.all_posts().await);
        posts.par_sort_unstable_by_key(|x| Reverse(x.entry.timestamp()));
        let posts_store = self.posts_store();
        let old_store = posts_store.snapshot();
        for (item_index, item) in old_store
            .iter()
            .filter_map(|x| x.clone().downcast::<PostItem>().ok())
            .enumerate()
        {
            match posts.iter().position(|x| PostItem::from(x) == item) {
                Some(post_index) => {
                    let post = &posts[post_index];
                    item.update(post.clone());
                    posts_store.remove(post_index as u32);
                }
                None => posts_store.remove(item_index as u32),
            }
        }
        let ctx = glib::MainContext::default();
        let this = self.clone();
        ctx.invoke(move || {
            let posts_store = this.posts_store();
            for x in posts.into_iter() {
                posts_store.append(&PostItem::new(&x));
            }
        });

        let items_changed =
            self.imp().posts_initialised.get() && old_store != posts_store.snapshot();

        if let Some(home_page) =
            get_view_stack_page_by_name("home".to_string(), &self.imp().view_stack)
        {
            if matches!(get_view_stack_page_by_name(
                self.imp().view_stack
                    .visible_child_name()
                    .unwrap_or_default()
                    .to_string(),
                    &self.imp().view_stack,
            ), Some(x) if x == home_page)
            {
                home_page.set_needs_attention(home_page.needs_attention() || items_changed);
            }

            home_page.child().set_sensitive(true);
        }
    }

    pub fn setup_home_page(&self) {
        let imp = self.imp();

        let posts_store = gio::ListStore::new::<crate::okunet::items::post_item::PostItem>();
        imp.posts_store.replace(Some(Rc::new(posts_store)));

        imp.posts_model.set_model(Some(&self.posts_store().clone()));
        imp.posts_model.set_autoselect(false);
        imp.posts_model.set_can_unselect(true);
        imp.posts_model.connect_selected_item_notify(clone!(
            #[weak(rename_to = _this)]
            self,
            #[weak]
            imp,
            move |posts_model| {
                if let Some(item) = posts_model.selected_item() {
                    let _post_item = item.downcast_ref::<PostItem>().unwrap();
                    // TODO: Open post in modal
                    imp.posts_model.unselect_all();
                }
            }
        ));

        imp.posts_factory.connect_setup(clone!(move |_, item| {
            let row = crate::widgets::okunet::post_row::PostRow::new();
            let list_item = item.downcast_ref::<gtk::ListItem>().unwrap();
            list_item.set_child(Some(&row));
            list_item
                .property_expression("item")
                .chain_property::<crate::okunet::items::post_item::PostItem>("url")
                .bind(&row, "url", gtk::Widget::NONE);
            list_item
                .property_expression("item")
                .chain_property::<crate::okunet::items::post_item::PostItem>("title")
                .bind(&row, "title", gtk::Widget::NONE);
            list_item
                .property_expression("item")
                .chain_property::<crate::okunet::items::post_item::PostItem>("body")
                .bind(&row, "body", gtk::Widget::NONE);
            list_item
                .property_expression("item")
                .chain_property::<crate::okunet::items::post_item::PostItem>("tags")
                .bind(&row, "tags", gtk::Widget::NONE);
            list_item
                .property_expression("item")
                .chain_property::<crate::okunet::items::post_item::PostItem>("author-id")
                .bind(&row, "author-id", gtk::Widget::NONE);
            list_item
                .property_expression("item")
                .chain_property::<crate::okunet::items::post_item::PostItem>("author-name")
                .bind(&row, "author-name", gtk::Widget::NONE);
        }));

        imp.posts_view.set_model(Some(&imp.posts_model));
        imp.posts_view.set_factory(Some(&imp.posts_factory));
        imp.posts_view.set_enable_rubberband(false);
        imp.posts_view
            .set_hscroll_policy(gtk::ScrollablePolicy::Minimum);
        imp.posts_view
            .set_vscroll_policy(gtk::ScrollablePolicy::Natural);
        imp.posts_view.set_vexpand(true);
        imp.posts_view.add_css_class("boxed-list-separate");
        imp.posts_view.add_css_class("navigation-sidebar");

        imp.posts_scrolled_window.set_child(Some(&imp.posts_view));
        imp.posts_scrolled_window
            .set_hscrollbar_policy(gtk::PolicyType::Never);
        imp.posts_scrolled_window.set_propagate_natural_height(true);
        imp.posts_scrolled_window.set_propagate_natural_width(true);
        self.posts_store()
            .property_expression("n-items")
            .chain_closure::<bool>(closure!(|_: Option<Object>, x: u32| { x == 0 }))
            .bind(&imp.posts_placeholder, "visible", gtk::Widget::NONE);
        imp.posts_placeholder
            .property_expression("visible")
            .chain_closure::<bool>(closure!(|_: Option<Object>, x: bool| { !x }))
            .bind(&imp.posts_scrolled_window, "visible", gtk::Widget::NONE);

        imp.posts_placeholder.set_label("No posts … ");
        imp.posts_placeholder.set_margin_top(24);
        imp.posts_placeholder.set_margin_bottom(24);
        imp.posts_placeholder.add_css_class("title-2");

        imp.posts_box.set_orientation(gtk::Orientation::Vertical);
        imp.posts_box.append(&imp.posts_placeholder);
        imp.posts_box.append(&imp.posts_scrolled_window);

        imp.view_stack.add_titled_with_icon(
            &imp.posts_box,
            Some("home"),
            "Home",
            "folder-remote-symbolic",
        );
    }

    pub fn setup(&self) {
        let imp = self.imp();

        imp.view_switcher.set_stack(Some(&imp.view_stack));

        self.setup_home_page();
        imp.view_stack
            .connect_visible_child_notify(clone!(move |view_stack| {
                if let Some(visible_page) = get_view_stack_page_by_name(
                    view_stack
                        .visible_child_name()
                        .unwrap_or_default()
                        .to_string(),
                    view_stack,
                ) {
                    visible_page.set_needs_attention(false);
                }
            }));

        imp.main_box.set_orientation(gtk::Orientation::Vertical);
        imp.main_box.set_hexpand(true);
        imp.main_box.set_vexpand(true);
        imp.main_box.set_spacing(8);
        imp.main_box.add_css_class("toolbar");
        imp.main_box.append(&imp.view_switcher);
        imp.main_box.append(&imp.view_stack);

        self.append(&imp.main_box);

        let this = self.clone();
        this.watch();
    }
}
