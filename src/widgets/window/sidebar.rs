use super::*;
use crate::database::DATABASE;
use crate::history_item::HistoryItem;
use crate::replica_item::ReplicaItem;
use crate::window_util::get_view_stack_page_by_name;
use crate::{widgets, NODE};
use glib::clone;
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
                    x.title.unwrap_or(String::new()),
                    x.uri,
                    x.timestamp.to_rfc2822(),
                    &favicon_database,
                )
            })
            .collect();
        history_store.remove_all();
        if items.len() > 0 {
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
        self.setup_history_page(&web_context);
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
        imp.side_box.set_margin_top(4);
        imp.side_box.append(&imp.side_view_switcher);
        imp.side_box.append(&imp.side_view_stack);
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
            imp,
            #[weak]
            web_context,
            move |history_model| {
                if let Some(item) = history_model.selected_item() {
                    let history_item = item.downcast_ref::<HistoryItem>().unwrap();
                    let new_view = this.new_tab_page(&web_context, None, None).0;
                    new_view.load_uri(&history_item.uri());
                    imp.history_model.unselect_all();
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

        imp.history_box.set_orientation(gtk::Orientation::Vertical);
        imp.history_box.append(&imp.history_scrolled_window);

        imp.history_box.set_orientation(gtk::Orientation::Vertical);
        imp.history_box.set_spacing(4);

        imp.side_view_stack.add_titled_with_icon(
            &imp.history_box,
            Some("history"),
            "History",
            "hourglass-symbolic",
        );
    }

    pub fn setup_replicas_page(&self) {
        let imp = self.imp();

        imp.add_replicas_button
            .set_start_icon_name(Some("folder-new"));
        imp.add_replicas_button.set_margin_start(4);
        imp.add_replicas_button.set_margin_end(4);
        imp.add_replicas_button.set_title("New replica");
        imp.add_replicas_button.add_css_class("card");
        imp.add_replicas_button.connect_activated(clone!(move |_| {
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

        imp.replicas_box.set_orientation(gtk::Orientation::Vertical);
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
