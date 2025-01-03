use crate::window_util::get_window_from_widget;
use crate::MOUNT_DIR;
use crate::NODE;
use crate::REPLICAS_MOUNTED;
use gdk::prelude::DisplayExt;
use gio::prelude::ApplicationExt;
use glib::clone;
use glib::object::CastNone;
use glib::subclass::object::ObjectImpl;
use glib::subclass::types::ObjectSubclass;
use glib::subclass::types::ObjectSubclassExt;
use glib::subclass::types::ObjectSubclassIsExt;
use glib::value::ToValue;
use glib::ParamSpec;
use glib::ParamSpecBoolean;
use glib::ParamSpecString;
use glib::Value;
use gtk::prelude::BoxExt;
use gtk::prelude::ButtonExt;
use gtk::prelude::GtkWindowExt;
use gtk::prelude::ListBoxRowExt;
use gtk::prelude::WidgetExt;
use gtk::subclass::prelude::*;
use libadwaita::prelude::ActionRowExt;
use libadwaita::prelude::PreferencesRowExt;
use libadwaita::subclass::prelude::*;
use log::error;
use oku_fs::iroh_base::ticket::Ticket;
use oku_fs::iroh_docs::rpc::client::docs::ShareMode;
use oku_fs::iroh_docs::NamespaceId;
use std::cell::RefCell;
use std::sync::atomic::Ordering;
use std::sync::LazyLock;

pub mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct ReplicaRow {
        pub(crate) id: RefCell<String>,
        pub(crate) writable: RefCell<bool>,
        pub(crate) home: RefCell<bool>,
        pub(crate) home_button: gtk::Button,
        pub(crate) open_button: gtk::Button,
        pub(crate) read_ticket_button: gtk::Button,
        pub(crate) write_ticket_button: gtk::Button,
        pub(crate) fetch_button: gtk::Button,
        pub(crate) sync_button: gtk::Button,
        pub(crate) delete_button: gtk::Button,
        pub(crate) button_box: gtk::Box,
    }

    impl ReplicaRow {}

    #[glib::object_subclass]
    impl ObjectSubclass for ReplicaRow {
        const NAME: &'static str = "OkuReplicaRow";
        type Type = super::ReplicaRow;
        type ParentType = libadwaita::ActionRow;

        fn class_init(klass: &mut Self::Class) {
            klass.set_layout_manager_type::<gtk::BoxLayout>();
            klass.set_accessible_role(gtk::AccessibleRole::Generic);
        }
    }

    impl ObjectImpl for ReplicaRow {
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
                    ParamSpecString::builder("id").build(),
                    ParamSpecBoolean::builder("writable").build(),
                    ParamSpecBoolean::builder("home").build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "id" => {
                    let id = value.get::<&str>().unwrap();
                    self.obj().set_id(id);
                }
                "writable" => {
                    let writable = value.get::<bool>().unwrap();
                    self.obj().set_writable(writable);
                }
                "home" => {
                    let home = value.get::<bool>().unwrap();
                    self.obj().set_home(home);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &ParamSpec) -> Value {
            match pspec.name() {
                "id" => self.obj().id().to_value(),
                "writable" => self.obj().writable().to_value(),
                "home" => self.obj().home().to_value(),
                _ => unimplemented!(),
            }
        }
    }
    impl WidgetImpl for ReplicaRow {}
    impl ListBoxRowImpl for ReplicaRow {}
    impl PreferencesRowImpl for ReplicaRow {}
    impl ActionRowImpl for ReplicaRow {}
}

glib::wrapper! {
    pub struct ReplicaRow(ObjectSubclass<imp::ReplicaRow>)
    @extends libadwaita::ActionRow, libadwaita::PreferencesRow, gtk::ListBoxRow, gtk::Widget,
    @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for ReplicaRow {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl ReplicaRow {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn setup(&self) {
        let imp = self.imp();

        imp.home_button.set_icon_name("user-home-symbolic");
        imp.home_button.add_css_class("circular");
        imp.home_button.set_vexpand(false);
        imp.home_button.set_hexpand(false);
        imp.home_button.set_valign(gtk::Align::Center);
        imp.home_button.connect_clicked(clone!(
            #[weak(rename_to = this)]
            self,
            move |_| {
                if let Some(node) = NODE.get() {
                    let new_home_replica = match this.home() {
                        false => Some(NamespaceId::from(
                            oku_fs::fs::util::parse_array_hex_or_base32::<32>(&this.id())
                                .unwrap_or_default(),
                        )),
                        true => None,
                    };
                    if let Err(e) = node.set_home_replica(&new_home_replica) {
                        error!("{}", e);
                    }
                }
            }
        ));

        imp.open_button.set_icon_name("external-link-symbolic");
        imp.open_button.add_css_class("circular");
        imp.open_button.set_vexpand(false);
        imp.open_button.set_hexpand(false);
        imp.open_button
            .set_visible(REPLICAS_MOUNTED.load(Ordering::Relaxed));
        imp.open_button.connect_clicked(clone!(
            #[weak(rename_to = this)]
            self,
            move |_| {
                let _ = open::that_detached(MOUNT_DIR.to_path_buf().join(this.id()));
            }
        ));

        imp.read_ticket_button.set_icon_name("ticket-symbolic");
        imp.read_ticket_button.add_css_class("linked");
        imp.read_ticket_button.set_vexpand(false);
        imp.read_ticket_button.set_hexpand(false);
        imp.read_ticket_button
            .set_tooltip_text(Some("Create read-only ticket"));
        imp.read_ticket_button.connect_clicked(clone!(
            #[weak(rename_to = this)]
            self,
            move |_| {
                let ctx = glib::MainContext::default();
                ctx.spawn_local_with_priority(
                    glib::source::Priority::HIGH,
                    clone!(
                        #[weak]
                        this,
                        async move {
                            if let Some(node) = NODE.get() {
                                if let Ok(ticket) = node
                                    .create_document_ticket(
                                        &NamespaceId::from(oku_fs::fs::util::parse_array_hex_or_base32::<32>(&this.id()).unwrap_or_default()),
                                        &ShareMode::Read,
                                    )
                                    .await
                                {
                                    let clipboard = gdk::Display::default().unwrap().clipboard();
                                    clipboard.set_text(&ticket.serialize());
                                    let window = get_window_from_widget(&this);
                                    let app = window.application().unwrap();
                                    let notification = gio::Notification::new("Read-only replica ticket copied");
                                    notification.set_body(Some(&format!("A read-only ticket for a replica ({}) has been copied to the clipboard.", this.id())));
                                    app.send_notification(None, &notification);
                                }
                            }
                        }
                    ),
                );
            }
        ));

        imp.write_ticket_button
            .set_icon_name("ticket-special-symbolic");
        imp.write_ticket_button.add_css_class("linked");
        imp.write_ticket_button.add_css_class("destructive-action");
        imp.write_ticket_button.set_vexpand(false);
        imp.write_ticket_button.set_hexpand(false);
        imp.write_ticket_button
            .set_tooltip_text(Some("Create read & write ticket"));
        imp.write_ticket_button.connect_clicked(clone!(
            #[weak(rename_to = this)]
            self,
            move |_| {
                let ctx = glib::MainContext::default();
                ctx.spawn_local_with_priority(
                    glib::source::Priority::HIGH,
                    clone!(
                        #[weak]
                        this,
                        async move {
                            if let Some(node) = NODE.get() {
                                if let Ok(ticket) = node
                                    .create_document_ticket(
                                        &NamespaceId::from(oku_fs::fs::util::parse_array_hex_or_base32::<32>(&this.id()).unwrap_or_default()),
                                        &ShareMode::Write,
                                    )
                                    .await
                                {
                                    let clipboard = gdk::Display::default().unwrap().clipboard();
                                    clipboard.set_text(&ticket.serialize());
                                    let window = get_window_from_widget(&this);
                                    let app = window.application().unwrap();
                                    let notification = gio::Notification::new("Read & write replica ticket copied");
                                    notification.set_body(Some(&format!("A read & write ticket for a replica ({}) has been copied to the clipboard.", this.id())));
                                    app.send_notification(None, &notification);
                                }
                            }
                        }
                    ),
                );
            }
        ));

        imp.fetch_button
            .set_icon_name("arrow-pointing-at-line-down-symbolic");
        imp.fetch_button.add_css_class("linked");
        imp.fetch_button.add_css_class("warning");
        imp.fetch_button.set_vexpand(false);
        imp.fetch_button.set_hexpand(false);
        imp.fetch_button
            .set_tooltip_text(Some("Fetch entire replica"));
        imp.fetch_button.connect_clicked(clone!(
            #[weak(rename_to = this)]
            self,
            move |_| {
                let ctx = glib::MainContext::default();
                ctx.spawn_local_with_priority(
                    glib::source::Priority::HIGH,
                    clone!(
                        #[weak]
                        this,
                        async move {
                            if let Some(node) = NODE.get() {
                                match node
                                    .fetch_replica_by_id(
                                        &NamespaceId::from(
                                            oku_fs::fs::util::parse_array_hex_or_base32::<32>(
                                                &this.id(),
                                            )
                                            .unwrap_or_default(),
                                        ),
                                        &None,
                                    )
                                    .await
                                {
                                    Ok(_) => (),
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

        imp.sync_button.set_icon_name("update-symbolic");
        imp.sync_button.add_css_class("linked");
        imp.sync_button.add_css_class("warning");
        imp.sync_button.set_vexpand(false);
        imp.sync_button.set_hexpand(false);
        imp.sync_button
            .set_tooltip_text(Some("Sync last-fetched files"));
        imp.sync_button.connect_clicked(clone!(
            #[weak(rename_to = this)]
            self,
            move |_| {
                let ctx = glib::MainContext::default();
                ctx.spawn_local_with_priority(
                    glib::source::Priority::HIGH,
                    clone!(
                        #[weak]
                        this,
                        async move {
                            if let Some(node) = NODE.get() {
                                match node
                                    .sync_replica(&NamespaceId::from(
                                        oku_fs::fs::util::parse_array_hex_or_base32::<32>(
                                            &this.id(),
                                        )
                                        .unwrap_or_default(),
                                    ))
                                    .await
                                {
                                    Ok(_) => (),
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

        imp.delete_button.set_icon_name("user-trash-symbolic");
        imp.delete_button.add_css_class("circular");
        imp.delete_button.add_css_class("destructive-action");
        imp.delete_button.set_vexpand(false);
        imp.delete_button.set_hexpand(false);
        imp.delete_button.connect_clicked(clone!(
            #[weak(rename_to = this)]
            self,
            move |_| {
                let ctx = glib::MainContext::default();
                ctx.spawn_local_with_priority(
                    glib::source::Priority::HIGH,
                    clone!(
                        #[weak]
                        this,
                        async move {
                            if let Some(node) = NODE.get() {
                                match node
                                    .delete_replica(&NamespaceId::from(
                                        oku_fs::fs::util::parse_array_hex_or_base32::<32>(
                                            &this.id(),
                                        )
                                        .unwrap_or_default(),
                                    ))
                                    .await
                                {
                                    Ok(_) => (),
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

        imp.button_box.append(&imp.open_button);
        imp.button_box.append(&imp.read_ticket_button);
        imp.button_box.append(&imp.write_ticket_button);
        imp.button_box.append(&imp.fetch_button);
        imp.button_box.append(&imp.sync_button);
        imp.button_box.append(&imp.delete_button);
        imp.button_box.set_homogeneous(false);
        imp.button_box.set_valign(gtk::Align::Center);
        imp.button_box.set_halign(gtk::Align::End);
        imp.button_box.add_css_class("linked");

        let content_box: gtk::Box = self.child().and_downcast().unwrap();
        content_box.set_hexpand(true);

        self.add_prefix(&imp.home_button);
        self.add_suffix(&imp.button_box);
        self.set_margin_bottom(4);
        self.set_title_lines(1);
        self.add_css_class("caption");
        self.add_css_class("card");
    }

    pub fn id(&self) -> String {
        self.imp().id.borrow().to_string()
    }

    pub fn writable(&self) -> bool {
        *self.imp().writable.borrow()
    }

    pub fn home(&self) -> bool {
        *self.imp().home.borrow()
    }

    pub fn set_id(&self, id: &str) {
        let imp = self.imp();

        imp.id.replace(id.to_string());
        self.set_title(&oku_fs::fs::util::fmt_short(NamespaceId::from(
            oku_fs::fs::util::parse_array_hex_or_base32::<32>(id).unwrap_or_default(),
        )));
    }

    pub fn set_writable(&self, writable: bool) {
        let imp = self.imp();

        imp.writable.replace(writable);
        imp.write_ticket_button.set_visible(writable);
    }

    pub fn set_home(&self, home: bool) {
        let imp = self.imp();

        imp.home.replace(home);
        let (old_class, new_class) = match home {
            true => ("accent", "warning"),
            false => ("warning", "accent"),
        };
        imp.home_button.remove_css_class(old_class);
        imp.home_button.add_css_class(new_class);
    }
}
