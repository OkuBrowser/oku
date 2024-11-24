use super::*;
use crate::bookmark_item::BookmarkItem;
use crate::database::DATABASE;
use crate::history_item::HistoryItem;
use crate::replica_item::ReplicaItem;
use crate::window_util::get_view_stack_page_by_name;
use crate::{widgets, NODE};
use glib::{clone, closure, Object};
use gtk::prelude::GtkWindowExt;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};
use libadwaita::prelude::*;
use log::error;
use oku_fs::iroh::docs::CapabilityKind;
use std::cell::Ref;
use std::rc::Rc;
use webkit2gtk::prelude::WebViewExt;
use webkit2gtk::WebContext;

impl Window {
    pub fn bookmarks_store(&self) -> Ref<gio::ListStore> {
        let bookmarks_store = self.imp().bookmarks_store.borrow();

        Ref::map(bookmarks_store, |bookmarks_store| {
            let bookmarks_store = bookmarks_store.as_deref().unwrap();
            bookmarks_store
        })
    }

    pub fn bookmarks_updated(&self) {
        let favicon_database = self
            .get_view()
            .network_session()
            .unwrap()
            .website_data_manager()
            .unwrap()
            .favicon_database()
            .unwrap();
        let bookmarks_store = self.bookmarks_store();
        let bookmarks = DATABASE.get_bookmarks().unwrap_or_default();
        let items: Vec<_> = bookmarks
            .into_iter()
            .map(|x| BookmarkItem::new(x.url, x.title, x.body, x.tags, &favicon_database))
            .collect();
        bookmarks_store.remove_all();
        if !items.is_empty() {
            for item in items.iter() {
                bookmarks_store.append(item);
            }
        }

        if let Some(bookmarks_page) =
            get_view_stack_page_by_name("bookmarks".to_string(), &self.imp().side_view_stack)
        {
            bookmarks_page.set_needs_attention(true)
        }
    }

    pub fn history_store(&self) -> Ref<gio::ListStore> {
        let history_store = self.imp().history_store.borrow();

        Ref::map(history_store, |history_store| {
            let history_store = history_store.as_deref().unwrap();
            history_store
        })
    }

    pub fn history_updated(&self) {
        let favicon_database = self
            .get_view()
            .network_session()
            .unwrap()
            .website_data_manager()
            .unwrap()
            .favicon_database()
            .unwrap();
        let history_store = self.history_store();
        let history_records = DATABASE.get_history_records().unwrap_or_default();
        let items: Vec<_> = history_records
            .into_iter()
            .map(|x| {
                HistoryItem::new(
                    x.id,
                    x.title.unwrap_or_default(),
                    x.uri,
                    x.timestamp.to_rfc2822(),
                    &favicon_database,
                )
            })
            .collect();
        history_store.remove_all();
        if !items.is_empty() {
            for item in items.iter() {
                history_store.append(item);
            }
        }

        if let Some(history_page) =
            get_view_stack_page_by_name("history".to_string(), &self.imp().side_view_stack)
        {
            history_page.set_needs_attention(true)
        }
    }

    pub fn replicas_store(&self) -> Ref<gio::ListStore> {
        let replicas_store = self.imp().replicas_store.borrow();

        Ref::map(replicas_store, |replicas_store| {
            let replicas_store = replicas_store.as_deref().unwrap();
            replicas_store
        })
    }

    pub fn replicas_updated(&self) {
        let ctx = glib::MainContext::default();
        ctx.spawn_local_with_priority(
            glib::source::Priority::HIGH,
            clone!(
                #[weak(rename_to = this)]
                self,
                async move {
                    if let Some(node) = NODE.get() {
                        if let Ok(mut replicas) = node.list_replicas().await {
                            let home_replica = node.home_replica().await;
                            let replicas_store = this.replicas_store();
                            for item_index in 0..replicas_store.n_items() {
                                let item: ReplicaItem =
                                    replicas_store.item(item_index).unwrap().downcast().unwrap();
                                match replicas.iter().position(|x| x.0.to_string() == item.id()) {
                                    Some(replica_index) => {
                                        let (replica, capability_kind) = replicas[replica_index];
                                        item.set_properties(&[
                                            ("id", &replica.to_string()),
                                            (
                                                "writable",
                                                &matches!(capability_kind, CapabilityKind::Write),
                                            ),
                                            (
                                                "home",
                                                &matches!(home_replica, Some(x) if x == replica),
                                            ),
                                        ]);
                                        replicas.remove(replica_index);
                                    }
                                    None => replicas_store.remove(item_index),
                                }
                            }
                            for (replica, capability_kind) in replicas.iter() {
                                replicas_store.append(&ReplicaItem::new(
                                    replica.to_string(),
                                    matches!(capability_kind, CapabilityKind::Write),
                                    matches!(home_replica, Some(x) if x == *replica),
                                ));
                            }
                        }
                    }
                }
            ),
        );
        if let Some(replicas_page) =
            get_view_stack_page_by_name("replicas".to_string(), &self.imp().side_view_stack)
        {
            replicas_page.set_needs_attention(true)
        }
    }

    pub fn setup_sidebar(&self, web_context: &WebContext) {
        let imp = self.imp();

        imp.side_view_switcher.set_stack(Some(&imp.side_view_stack));

        self.setup_replicas_page();
        self.setup_history_page(web_context);
        self.setup_bookmarks_page(web_context);
        self.setup_downloads_page();
        imp.side_view_stack
            .connect_visible_child_notify(clone!(move |side_view_stack| {
                if let Some(visible_page) = get_view_stack_page_by_name(
                    side_view_stack
                        .visible_child_name()
                        .unwrap_or_default()
                        .to_string(),
                    side_view_stack,
                ) {
                    visible_page.set_needs_attention(false);
                }
            }));

        imp.side_box.set_orientation(gtk::Orientation::Vertical);
        imp.side_box.set_spacing(8);
        imp.side_box.add_css_class("toolbar");
        imp.side_box.append(&imp.side_view_switcher);
        imp.side_box.append(&imp.side_view_stack);
    }

    pub fn setup_bookmarks_page(&self, web_context: &WebContext) {
        let imp = self.imp();

        let bookmarks_store = gio::ListStore::new::<BookmarkItem>();
        imp.bookmarks_store.replace(Some(Rc::new(bookmarks_store)));

        imp.bookmarks_model
            .set_model(Some(&self.bookmarks_store().clone()));
        imp.bookmarks_model.set_autoselect(false);
        imp.bookmarks_model.set_can_unselect(true);
        imp.bookmarks_model.connect_selected_item_notify(clone!(
            #[weak(rename_to = this)]
            self,
            #[weak]
            imp,
            #[weak]
            web_context,
            move |bookmarks_model| {
                if let Some(item) = bookmarks_model.selected_item() {
                    let bookmarks_item = item.downcast_ref::<BookmarkItem>().unwrap();
                    let new_view = this.new_tab_page(&web_context, None, None).0;
                    new_view.load_uri(&bookmarks_item.url());
                    imp.bookmarks_model.unselect_all();
                }
            }
        ));

        imp.bookmarks_factory.connect_setup(clone!(move |_, item| {
            let row = widgets::bookmark_row::BookmarkRow::new();
            let list_item = item.downcast_ref::<gtk::ListItem>().unwrap();
            list_item.set_child(Some(&row));
            list_item
                .property_expression("item")
                .chain_property::<BookmarkItem>("url")
                .bind(&row, "url", gtk::Widget::NONE);
            list_item
                .property_expression("item")
                .chain_property::<BookmarkItem>("title")
                .bind(&row, "title-property", gtk::Widget::NONE);
            list_item
                .property_expression("item")
                .chain_property::<BookmarkItem>("body")
                .bind(&row, "body", gtk::Widget::NONE);
            list_item
                .property_expression("item")
                .chain_property::<BookmarkItem>("tags")
                .bind(&row, "tags", gtk::Widget::NONE);
            list_item
                .property_expression("item")
                .chain_property::<BookmarkItem>("favicon")
                .bind(&row, "favicon", gtk::Widget::NONE);
        }));

        imp.bookmarks_view.set_model(Some(&imp.bookmarks_model));
        imp.bookmarks_view.set_factory(Some(&imp.bookmarks_factory));
        imp.bookmarks_view.set_enable_rubberband(false);
        imp.bookmarks_view
            .set_hscroll_policy(gtk::ScrollablePolicy::Minimum);
        imp.bookmarks_view
            .set_vscroll_policy(gtk::ScrollablePolicy::Natural);
        imp.bookmarks_view.set_vexpand(true);
        imp.bookmarks_view.add_css_class("boxed-list-separate");
        imp.bookmarks_view.add_css_class("navigation-sidebar");

        imp.bookmarks_scrolled_window
            .set_child(Some(&imp.bookmarks_view));
        imp.bookmarks_scrolled_window
            .set_hscrollbar_policy(gtk::PolicyType::Never);
        imp.bookmarks_scrolled_window
            .set_propagate_natural_height(true);
        imp.bookmarks_scrolled_window
            .set_propagate_natural_width(true);

        imp.bookmarks_label.set_label("Bookmarks");
        imp.bookmarks_label.set_margin_top(24);
        imp.bookmarks_label.set_margin_bottom(24);
        imp.bookmarks_label.add_css_class("title-1");

        self.setup_bookmarks_stack(web_context);

        imp.bookmarks_box
            .set_orientation(gtk::Orientation::Vertical);
        imp.bookmarks_box.set_spacing(4);
        imp.bookmarks_box.append(&imp.bookmarks_label);
        imp.bookmarks_box.append(&imp.bookmarks_search);
        imp.bookmarks_box.append(&imp.bookmarks_stack);

        imp.side_view_stack.add_titled_with_icon(
            &imp.bookmarks_box,
            Some("bookmarks"),
            "Bookmarks",
            "bookmark-filled-symbolic",
        );
    }

    pub fn setup_bookmarks_stack(&self, web_context: &WebContext) {
        let imp = self.imp();

        self.setup_bookmarks_search(web_context);
        imp.bookmarks_stack
            .set_transition_type(gtk::StackTransitionType::Crossfade);
        imp.bookmarks_stack
            .add_named(&imp.bookmarks_scrolled_window, Some("all"));
        imp.bookmarks_stack
            .add_named(&imp.bookmarks_search_scrolled_window, Some("search"));
        imp.bookmarks_search
            .set_placeholder_text(Some("Search bookmarks … "));
        imp.bookmarks_search
            .property_expression("text")
            .chain_closure::<String>(closure!(|_: Option<Object>, x: String| {
                match x.len() {
                    0 => "all".to_string(),
                    _ => "search".to_string(),
                }
            }))
            .bind(
                &imp.bookmarks_stack,
                "visible-child-name",
                gtk::Widget::NONE,
            );
    }

    pub fn setup_bookmarks_search(&self, web_context: &WebContext) {
        let imp = self.imp();

        let url_filter = gtk::StringFilter::new(Some(&gtk::PropertyExpression::new(
            BookmarkItem::static_type(),
            None::<&gtk::Expression>,
            "url",
        )));
        let title_filter = gtk::StringFilter::new(Some(&gtk::PropertyExpression::new(
            BookmarkItem::static_type(),
            None::<&gtk::Expression>,
            "title",
        )));
        let body_filter = gtk::StringFilter::new(Some(&gtk::PropertyExpression::new(
            BookmarkItem::static_type(),
            None::<&gtk::Expression>,
            "body",
        )));
        let tags_expression = &gtk::PropertyExpression::new(
            BookmarkItem::static_type(),
            None::<&gtk::Expression>,
            "tags",
        );
        let tags_closure_expression = gtk::ClosureExpression::new::<String>(
            [&tags_expression],
            closure!(|_: Option<Object>, x: Vec<String>| { x.join(",") }),
        );
        let tags_filter = gtk::StringFilter::new(Some(&tags_closure_expression));

        imp.bookmarks_search.property_expression("text").bind(
            &url_filter,
            "search",
            gtk::Widget::NONE,
        );
        imp.bookmarks_search.property_expression("text").bind(
            &title_filter,
            "search",
            gtk::Widget::NONE,
        );
        imp.bookmarks_search.property_expression("text").bind(
            &body_filter,
            "search",
            gtk::Widget::NONE,
        );
        imp.bookmarks_search.property_expression("text").bind(
            &tags_filter,
            "search",
            gtk::Widget::NONE,
        );

        let filter = gtk::AnyFilter::new();
        filter.append(url_filter);
        filter.append(title_filter);
        filter.append(body_filter);
        filter.append(tags_filter);

        imp.bookmarks_filter_model.set_filter(Some(&filter));
        imp.bookmarks_filter_model
            .set_model(Some(&self.bookmarks_store().clone()));
        imp.bookmarks_filter_model.set_incremental(true);
        imp.bookmarks_filter_selection_model
            .set_model(Some(&imp.bookmarks_filter_model));
        imp.bookmarks_filter_selection_model.set_autoselect(false);
        imp.bookmarks_filter_selection_model.set_can_unselect(true);
        imp.bookmarks_filter_selection_model
            .connect_selected_item_notify(clone!(
                #[weak(rename_to = this)]
                self,
                #[weak]
                web_context,
                move |bookmarks_filter_selection_model| {
                    if let Some(item) = bookmarks_filter_selection_model.selected_item() {
                        let bookmarks_item = item.downcast_ref::<BookmarkItem>().unwrap();
                        let new_view = this.new_tab_page(&web_context, None, None).0;
                        new_view.load_uri(&bookmarks_item.url());
                        bookmarks_filter_selection_model.unselect_all();
                    }
                }
            ));

        imp.bookmarks_search_factory
            .connect_setup(clone!(move |_, item| {
                let row = widgets::bookmark_row::BookmarkRow::new();
                let list_item = item.downcast_ref::<gtk::ListItem>().unwrap();
                list_item.set_child(Some(&row));
                list_item
                    .property_expression("item")
                    .chain_property::<BookmarkItem>("url")
                    .bind(&row, "url", gtk::Widget::NONE);
                list_item
                    .property_expression("item")
                    .chain_property::<BookmarkItem>("title")
                    .bind(&row, "title-property", gtk::Widget::NONE);
                list_item
                    .property_expression("item")
                    .chain_property::<BookmarkItem>("body")
                    .bind(&row, "body", gtk::Widget::NONE);
                list_item
                    .property_expression("item")
                    .chain_property::<BookmarkItem>("tags")
                    .bind(&row, "tags", gtk::Widget::NONE);
                list_item
                    .property_expression("item")
                    .chain_property::<BookmarkItem>("favicon")
                    .bind(&row, "favicon", gtk::Widget::NONE);
            }));

        imp.bookmarks_search_view
            .set_model(Some(&imp.bookmarks_filter_selection_model));
        imp.bookmarks_search_view
            .set_factory(Some(&imp.bookmarks_search_factory));
        imp.bookmarks_search_view.set_enable_rubberband(false);
        imp.bookmarks_search_view
            .set_hscroll_policy(gtk::ScrollablePolicy::Minimum);
        imp.bookmarks_search_view
            .set_vscroll_policy(gtk::ScrollablePolicy::Natural);
        imp.bookmarks_search_view.set_vexpand(true);
        imp.bookmarks_search_view
            .add_css_class("boxed-list-separate");
        imp.bookmarks_search_view
            .add_css_class("navigation-sidebar");

        imp.bookmarks_search_scrolled_window
            .set_child(Some(&imp.bookmarks_search_view));
        imp.bookmarks_search_scrolled_window
            .set_hscrollbar_policy(gtk::PolicyType::Never);
        imp.bookmarks_search_scrolled_window
            .set_propagate_natural_height(true);
        imp.bookmarks_search_scrolled_window
            .set_propagate_natural_width(true);
    }

    pub fn setup_history_page(&self, web_context: &WebContext) {
        let imp = self.imp();

        let history_store = gio::ListStore::new::<HistoryItem>();
        imp.history_store.replace(Some(Rc::new(history_store)));

        imp.history_model
            .set_model(Some(&self.history_store().clone()));
        imp.history_model.set_autoselect(false);
        imp.history_model.set_can_unselect(true);
        imp.history_model.connect_selected_item_notify(clone!(
            #[weak(rename_to = this)]
            self,
            #[weak]
            web_context,
            move |history_model| {
                if let Some(item) = history_model.selected_item() {
                    let history_item = item.downcast_ref::<HistoryItem>().unwrap();
                    let new_view = this.new_tab_page(&web_context, None, None).0;
                    new_view.load_uri(&history_item.uri());
                    history_model.unselect_all();
                }
            }
        ));

        imp.history_factory.connect_setup(clone!(move |_, item| {
            let row = widgets::history_row::HistoryRow::new();
            let list_item = item.downcast_ref::<gtk::ListItem>().unwrap();
            list_item.set_child(Some(&row));
            list_item
                .property_expression("item")
                .chain_property::<HistoryItem>("id")
                .bind(&row, "id", gtk::Widget::NONE);
            list_item
                .property_expression("item")
                .chain_property::<HistoryItem>("title")
                .bind(&row, "title-property", gtk::Widget::NONE);
            list_item
                .property_expression("item")
                .chain_property::<HistoryItem>("uri")
                .bind(&row, "uri", gtk::Widget::NONE);
            list_item
                .property_expression("item")
                .chain_property::<HistoryItem>("favicon")
                .bind(&row, "favicon", gtk::Widget::NONE);
            list_item
                .property_expression("item")
                .chain_property::<HistoryItem>("timestamp")
                .bind(&row, "timestamp", gtk::Widget::NONE);
        }));

        imp.history_view.set_model(Some(&imp.history_model));
        imp.history_view.set_factory(Some(&imp.history_factory));
        imp.history_view.set_enable_rubberband(false);
        imp.history_view
            .set_hscroll_policy(gtk::ScrollablePolicy::Minimum);
        imp.history_view
            .set_vscroll_policy(gtk::ScrollablePolicy::Natural);
        imp.history_view.set_vexpand(true);
        imp.history_view.add_css_class("boxed-list-separate");
        imp.history_view.add_css_class("navigation-sidebar");

        imp.history_scrolled_window
            .set_child(Some(&imp.history_view));
        imp.history_scrolled_window
            .set_hscrollbar_policy(gtk::PolicyType::Never);
        imp.history_scrolled_window
            .set_propagate_natural_height(true);
        imp.history_scrolled_window
            .set_propagate_natural_width(true);

        imp.history_label.set_label("History");
        imp.history_label.set_margin_top(24);
        imp.history_label.set_margin_bottom(24);
        imp.history_label.add_css_class("title-1");

        self.setup_history_stack(web_context);

        imp.history_box.set_orientation(gtk::Orientation::Vertical);
        imp.history_box.set_spacing(4);
        imp.history_box.append(&imp.history_label);
        imp.history_box.append(&imp.history_search);
        imp.history_box.append(&imp.history_stack);

        imp.side_view_stack.add_titled_with_icon(
            &imp.history_box,
            Some("history"),
            "History",
            "hourglass-symbolic",
        );
    }

    pub fn setup_history_stack(&self, web_context: &WebContext) {
        let imp = self.imp();

        self.setup_history_search(web_context);
        imp.history_stack
            .set_transition_type(gtk::StackTransitionType::Crossfade);
        imp.history_stack
            .add_named(&imp.history_scrolled_window, Some("all"));
        imp.history_stack
            .add_named(&imp.history_search_scrolled_window, Some("search"));
        imp.history_search
            .set_placeholder_text(Some("Search history entries … "));
        imp.history_search
            .property_expression("text")
            .chain_closure::<String>(closure!(|_: Option<Object>, x: String| {
                match x.len() {
                    0 => "all".to_string(),
                    _ => "search".to_string(),
                }
            }))
            .bind(&imp.history_stack, "visible-child-name", gtk::Widget::NONE);
    }

    pub fn setup_history_search(&self, web_context: &WebContext) {
        let imp = self.imp();

        let uri_filter = gtk::StringFilter::new(Some(&gtk::PropertyExpression::new(
            HistoryItem::static_type(),
            None::<&gtk::Expression>,
            "uri",
        )));
        let title_filter = gtk::StringFilter::new(Some(&gtk::PropertyExpression::new(
            HistoryItem::static_type(),
            None::<&gtk::Expression>,
            "title",
        )));

        imp.history_search.property_expression("text").bind(
            &uri_filter,
            "search",
            gtk::Widget::NONE,
        );
        imp.history_search.property_expression("text").bind(
            &title_filter,
            "search",
            gtk::Widget::NONE,
        );

        let filter = gtk::AnyFilter::new();
        filter.append(uri_filter);
        filter.append(title_filter);

        imp.history_filter_model.set_filter(Some(&filter));
        imp.history_filter_model
            .set_model(Some(&self.history_store().clone()));
        imp.history_filter_model.set_incremental(true);
        imp.history_filter_selection_model
            .set_model(Some(&imp.history_filter_model));
        imp.history_filter_selection_model.set_autoselect(false);
        imp.history_filter_selection_model.set_can_unselect(true);
        imp.history_filter_selection_model
            .connect_selected_item_notify(clone!(
                #[weak(rename_to = this)]
                self,
                #[weak]
                web_context,
                move |history_filter_selection_model| {
                    if let Some(item) = history_filter_selection_model.selected_item() {
                        let history_item = item.downcast_ref::<HistoryItem>().unwrap();
                        let new_view = this.new_tab_page(&web_context, None, None).0;
                        new_view.load_uri(&history_item.uri());
                        history_filter_selection_model.unselect_all();
                    }
                }
            ));

        imp.history_search_factory
            .connect_setup(clone!(move |_, item| {
                let row = widgets::history_row::HistoryRow::new();
                let list_item = item.downcast_ref::<gtk::ListItem>().unwrap();
                list_item.set_child(Some(&row));
                list_item
                    .property_expression("item")
                    .chain_property::<HistoryItem>("id")
                    .bind(&row, "id", gtk::Widget::NONE);
                list_item
                    .property_expression("item")
                    .chain_property::<HistoryItem>("title")
                    .bind(&row, "title-property", gtk::Widget::NONE);
                list_item
                    .property_expression("item")
                    .chain_property::<HistoryItem>("uri")
                    .bind(&row, "uri", gtk::Widget::NONE);
                list_item
                    .property_expression("item")
                    .chain_property::<HistoryItem>("favicon")
                    .bind(&row, "favicon", gtk::Widget::NONE);
                list_item
                    .property_expression("item")
                    .chain_property::<HistoryItem>("timestamp")
                    .bind(&row, "timestamp", gtk::Widget::NONE);
            }));

        imp.history_search_view
            .set_model(Some(&imp.history_filter_selection_model));
        imp.history_search_view
            .set_factory(Some(&imp.history_search_factory));
        imp.history_search_view.set_enable_rubberband(false);
        imp.history_search_view
            .set_hscroll_policy(gtk::ScrollablePolicy::Minimum);
        imp.history_search_view
            .set_vscroll_policy(gtk::ScrollablePolicy::Natural);
        imp.history_search_view.set_vexpand(true);
        imp.history_search_view.add_css_class("boxed-list-separate");
        imp.history_search_view.add_css_class("navigation-sidebar");

        imp.history_search_scrolled_window
            .set_child(Some(&imp.history_search_view));
        imp.history_search_scrolled_window
            .set_hscrollbar_policy(gtk::PolicyType::Never);
        imp.history_search_scrolled_window
            .set_propagate_natural_height(true);
        imp.history_search_scrolled_window
            .set_propagate_natural_width(true);
    }

    pub fn setup_replicas_page(&self) {
        let imp = self.imp();

        imp.add_replicas_button_content
            .set_icon_name("folder-new-symbolic");
        imp.add_replicas_button_content.set_label("New replica");
        imp.add_replicas_button_content.add_css_class("card");

        imp.add_replicas_button
            .set_child(Some(&imp.add_replicas_button_content));
        imp.add_replicas_button.set_margin_start(4);
        imp.add_replicas_button.set_margin_end(4);
        imp.add_replicas_button.connect_clicked(clone!(move |_| {
            let ctx = glib::MainContext::default();
            ctx.spawn_local_with_priority(
                glib::source::Priority::HIGH,
                clone!(async move {
                    if let Some(node) = NODE.get() {
                        match node.create_replica().await {
                            Ok(_) => (),
                            Err(e) => {
                                error!("{}", e)
                            }
                        }
                    }
                }),
            );
        }));

        let replicas_store = gio::ListStore::new::<crate::replica_item::ReplicaItem>();
        imp.replicas_store.replace(Some(Rc::new(replicas_store)));

        imp.replicas_model
            .set_model(Some(&self.replicas_store().clone()));
        imp.replicas_model.set_autoselect(false);
        imp.replicas_model.set_can_unselect(true);
        imp.replicas_model.connect_selected_item_notify(clone!(
            #[weak(rename_to = this)]
            self,
            #[weak]
            imp,
            move |replicas_model| {
                if let Some(item) = replicas_model.selected_item() {
                    let replica_item = item.downcast_ref::<ReplicaItem>().unwrap();
                    let clipboard = gdk::Display::default().unwrap().clipboard();
                    clipboard.set_text(&replica_item.id());
                    let app = this.application().unwrap();
                    let notification = gio::Notification::new("Replica ID copied");
                    notification.set_body(Some(&format!(
                        "Replica ID {} has been copied to the clipboard.",
                        replica_item.id()
                    )));
                    app.send_notification(None, &notification);
                    imp.replicas_model.unselect_all();
                }
            }
        ));

        imp.replicas_factory.connect_setup(clone!(move |_, item| {
            let row = widgets::replica_row::ReplicaRow::new();
            let list_item = item.downcast_ref::<gtk::ListItem>().unwrap();
            list_item.set_child(Some(&row));
            list_item
                .property_expression("item")
                .chain_property::<crate::replica_item::ReplicaItem>("id")
                .bind(&row, "id", gtk::Widget::NONE);
            list_item
                .property_expression("item")
                .chain_property::<crate::replica_item::ReplicaItem>("writable")
                .bind(&row, "writable", gtk::Widget::NONE);
            list_item
                .property_expression("item")
                .chain_property::<crate::replica_item::ReplicaItem>("home")
                .bind(&row, "home", gtk::Widget::NONE);
        }));

        imp.replicas_view.set_model(Some(&imp.replicas_model));
        imp.replicas_view.set_factory(Some(&imp.replicas_factory));
        imp.replicas_view.set_enable_rubberband(false);
        imp.replicas_view
            .set_hscroll_policy(gtk::ScrollablePolicy::Minimum);
        imp.replicas_view
            .set_vscroll_policy(gtk::ScrollablePolicy::Natural);
        imp.replicas_view.set_vexpand(true);
        imp.replicas_view.add_css_class("boxed-list-separate");
        imp.replicas_view.add_css_class("navigation-sidebar");

        imp.replicas_scrolled_window
            .set_child(Some(&imp.replicas_view));
        imp.replicas_scrolled_window
            .set_hscrollbar_policy(gtk::PolicyType::Never);
        imp.replicas_scrolled_window
            .set_propagate_natural_height(true);
        imp.replicas_scrolled_window
            .set_propagate_natural_width(true);

        imp.replicas_label.set_label("Replicas");
        imp.replicas_label.set_margin_top(24);
        imp.replicas_label.set_margin_bottom(24);
        imp.replicas_label.add_css_class("title-1");

        imp.replicas_box.set_orientation(gtk::Orientation::Vertical);
        imp.replicas_box.append(&imp.replicas_label);
        imp.replicas_box.append(&imp.add_replicas_button);
        imp.replicas_box.append(&imp.replicas_scrolled_window);

        imp.side_view_stack.add_titled_with_icon(
            &imp.replicas_box,
            Some("replicas"),
            "Replicas",
            "folder-remote-symbolic",
        );
    }

    pub fn setup_sidebar_button_clicked(&self) {
        let imp = self.imp();

        // Sidebar button clicked
        imp.sidebar_button.connect_clicked(clone!(
            #[weak]
            imp,
            move |_| {
                imp.split_view
                    .set_show_sidebar(!imp.split_view.shows_sidebar());
            }
        ));
        let action_library = gio::ActionEntry::builder("library")
            .activate(clone!(move |window: &Self, _, _| {
                let imp = window.imp();
                imp.split_view
                    .set_show_sidebar(!imp.split_view.shows_sidebar());
            }))
            .build();
        self.add_action_entries([action_library]);
    }
}
