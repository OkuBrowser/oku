use crate::config::Config;
use crate::database::DATABASE;
use crate::widgets::address_entry::AddressEntry;
use crate::widgets::settings::core::apply_appearance_config;
use crate::{APP_ID, HOME_REPLICA_SET, NODE};
use glib::clone;
use gtk::prelude::GtkWindowExt;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};
use libadwaita::subclass::application_window::AdwApplicationWindowImpl;
use libadwaita::{prelude::*, ResponseAppearance};
use log::{error, info};
use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::Ordering;
use webkit2gtk::prelude::WebViewExt;
use webkit2gtk::{NetworkSession, WebContext};

pub mod imp {
    use gtk::EventControllerFocus;

    use super::*;

    #[derive(Debug, Default)]
    pub struct Window {
        pub(crate) config: Config,
        // Window parameters
        pub(crate) is_private: Cell<bool>,
        pub(crate) bookmarks_sidebar_initialised: Cell<bool>,
        pub(crate) history_sidebar_initialised: Cell<bool>,
        pub(crate) replicas_sidebar_initialised: Cell<bool>,
        pub(crate) style_provider: RefCell<gtk::CssProvider>,
        // OkuNet fetch overlay
        pub(crate) okunet_fetch_overlay_box: gtk::Box,
        pub(crate) okunet_fetch_overlay_spinner: libadwaita::Spinner,
        pub(crate) okunet_fetch_overlay_label: gtk::Label,
        pub(crate) okunet_fetch_overlay_animation: RefCell<Option<libadwaita::SpringAnimation>>,
        // Fullscreen overlay
        pub(crate) fullscreen_overlay_box: gtk::Box,
        pub(crate) fullscreen_overlay_label: gtk::Label,
        pub(crate) fullscreen_overlay_animation: RefCell<Option<libadwaita::SpringAnimation>>,
        // Navigation bar
        pub(crate) nav_entry: AddressEntry,
        pub(crate) nav_entry_focus: EventControllerFocus,
        pub(crate) suggestions_store: RefCell<Option<Rc<gio::ListStore>>>,
        pub(crate) suggestions_factory: gtk::SignalListItemFactory,
        pub(crate) suggestions_model: gtk::SingleSelection,
        pub(crate) suggestions_view: gtk::ListView,
        pub(crate) suggestions_scrolled_window: libadwaita::ClampScrollable,
        pub(crate) suggestions_popover: gtk::Popover,
        // Left header buttons
        pub(crate) back_button: gtk::Button,
        pub(crate) forward_button: gtk::Button,
        pub(crate) navigation_buttons: gtk::Box,
        pub(crate) refresh_button: gtk::Button,
        pub(crate) add_tab_button: gtk::Button,
        pub(crate) sidebar_button: gtk::Button,
        pub(crate) left_header_buttons: gtk::Box,
        // Right header buttons
        pub(crate) overview_button: libadwaita::TabButton,
        pub(crate) note_button: gtk::Button,
        pub(crate) find_button: gtk::Button,
        pub(crate) replicas_button: gtk::Button,
        pub(crate) menu_button: gtk::Button,
        pub(crate) right_header_buttons: gtk::Box,
        // HeaderBar
        pub(crate) headerbar: libadwaita::HeaderBar,
        // Menu popover
        pub(crate) zoomout_button: gtk::Button,
        pub(crate) zoomin_button: gtk::Button,
        pub(crate) zoom_buttons: gtk::Box,
        pub(crate) zoomreset_button: gtk::Button,
        pub(crate) zoom_percentage: gtk::Label,
        pub(crate) fullscreen_button: gtk::Button,
        pub(crate) print_button: gtk::Button,
        pub(crate) screenshot_button: gtk::Button,
        pub(crate) new_window_button: gtk::Button,
        pub(crate) new_private_window_button: gtk::Button,
        pub(crate) settings_button: gtk::Button,
        pub(crate) about_button: gtk::Button,
        pub(crate) shortcuts_button: gtk::Button,
        pub(crate) menu_box: gtk::Box,
        pub(crate) menu_popover: gtk::Popover,
        // Find popover
        pub(crate) find_box: gtk::Box,
        pub(crate) find_popover: gtk::Popover,
        pub(crate) previous_find_button: gtk::Button,
        pub(crate) next_find_button: gtk::Button,
        pub(crate) find_buttons: gtk::Box,
        pub(crate) find_case_insensitive: gtk::ToggleButton,
        pub(crate) find_at_word_starts: gtk::ToggleButton,
        pub(crate) find_treat_medial_capital_as_word_start: gtk::ToggleButton,
        pub(crate) find_backwards: gtk::ToggleButton,
        pub(crate) find_wrap_around: gtk::ToggleButton,
        pub(crate) find_option_buttons: gtk::Box,
        pub(crate) find_search_entry: gtk::SearchEntry,
        pub(crate) total_matches_label: gtk::Label,
        pub(crate) find_middle_box: gtk::Box,
        // Tabs
        pub(crate) tab_bar: libadwaita::TabBar,
        pub(crate) tab_view: libadwaita::TabView,
        pub(crate) tab_bar_revealer: gtk::Revealer,
        // Main content
        pub(crate) main_overlay: gtk::Overlay,
        pub(crate) main_box: gtk::Box,
        pub(crate) fullscreen_box: libadwaita::ToolbarView,
        pub(crate) fullscreen_motion_controller: gtk::EventControllerMotion,
        pub(crate) tab_overview: libadwaita::TabOverview,
        pub(crate) split_view: libadwaita::OverlaySplitView,
        // Sidebar content
        pub(crate) side_box: gtk::Box,
        pub(crate) side_view_stack: libadwaita::ViewStack,
        pub(crate) side_view_switcher: libadwaita::ViewSwitcher,
        // Bookmarks
        pub(crate) bookmarks_box: gtk::Box,
        pub(crate) bookmarks_store: RefCell<Option<Rc<gio::ListStore>>>,
        pub(crate) bookmarks_factory: gtk::SignalListItemFactory,
        pub(crate) bookmarks_model: gtk::SingleSelection,
        pub(crate) bookmarks_view: gtk::ListView,
        pub(crate) bookmarks_scrolled_window: gtk::ScrolledWindow,
        pub(crate) bookmarks_label: gtk::Label,
        pub(crate) bookmarks_placeholder: gtk::Label,
        pub(crate) bookmarks_all_box: gtk::Box,
        pub(crate) bookmarks_stack: gtk::Stack,
        pub(crate) bookmarks_search: gtk::SearchEntry,
        pub(crate) bookmarks_filter_model: gtk::FilterListModel,
        pub(crate) bookmarks_filter_selection_model: gtk::SingleSelection,
        pub(crate) bookmarks_search_factory: gtk::SignalListItemFactory,
        pub(crate) bookmarks_search_view: gtk::ListView,
        pub(crate) bookmarks_search_scrolled_window: gtk::ScrolledWindow,
        pub(crate) bookmarks_search_box: gtk::Box,
        pub(crate) bookmarks_search_placeholder: gtk::Label,
        // History
        pub(crate) history_box: gtk::Box,
        pub(crate) history_store: RefCell<Option<Rc<gio::ListStore>>>,
        pub(crate) history_factory: gtk::SignalListItemFactory,
        pub(crate) history_model: gtk::SingleSelection,
        pub(crate) history_view: gtk::ListView,
        pub(crate) history_scrolled_window: gtk::ScrolledWindow,
        pub(crate) history_label: gtk::Label,
        pub(crate) history_placeholder: gtk::Label,
        pub(crate) history_all_box: gtk::Box,
        pub(crate) history_stack: gtk::Stack,
        pub(crate) history_search: gtk::SearchEntry,
        pub(crate) history_filter_model: gtk::FilterListModel,
        pub(crate) history_filter_selection_model: gtk::SingleSelection,
        pub(crate) history_search_factory: gtk::SignalListItemFactory,
        pub(crate) history_search_view: gtk::ListView,
        pub(crate) history_search_scrolled_window: gtk::ScrolledWindow,
        pub(crate) history_search_box: gtk::Box,
        pub(crate) history_search_placeholder: gtk::Label,
        // Replicas
        pub(crate) add_replicas_button: gtk::Button,
        pub(crate) add_replicas_button_content: libadwaita::ButtonContent,
        pub(crate) replicas_box: gtk::Box,
        pub(crate) replicas_store: RefCell<Option<Rc<gio::ListStore>>>,
        pub(crate) replicas_factory: gtk::SignalListItemFactory,
        pub(crate) replicas_model: gtk::SingleSelection,
        pub(crate) replicas_view: gtk::ListView,
        pub(crate) replicas_scrolled_window: gtk::ScrolledWindow,
        pub(crate) replicas_label: gtk::Label,
        pub(crate) replicas_placeholder: gtk::Label,
        // Downloads
        pub(crate) downloads_box: gtk::Box,
        pub(crate) downloads_store: RefCell<Option<Rc<gio::ListStore>>>,
        pub(crate) downloads_factory: gtk::SignalListItemFactory,
        pub(crate) downloads_model: gtk::SingleSelection,
        pub(crate) downloads_view: gtk::ListView,
        pub(crate) downloads_scrolled_window: gtk::ScrolledWindow,
        pub(crate) downloads_label: gtk::Label,
        pub(crate) downloads_placeholder: gtk::Label,
        // Miscellaneous
        pub(crate) progress_animation: RefCell<Option<libadwaita::SpringAnimation>>,
        pub(crate) progress_bar: gtk::ProgressBar,
        pub(crate) url_status_outer_box: gtk::Box,
        pub(crate) url_status_box: gtk::Box,
        pub(crate) url_status: gtk::Label,
        pub(crate) network_session: RefCell<webkit2gtk::NetworkSession>,
    }

    impl Window {}

    #[glib::object_subclass]
    impl ObjectSubclass for Window {
        const NAME: &'static str = "OkuWindow";
        type Type = super::Window;
        type ParentType = libadwaita::ApplicationWindow;
    }

    impl ObjectImpl for Window {}
    impl WidgetImpl for Window {}
    impl WindowImpl for Window {}
    impl ApplicationWindowImpl for Window {}
    impl AdwApplicationWindowImpl for Window {}
}

glib::wrapper! {
    pub struct Window(ObjectSubclass<imp::Window>)
    @extends libadwaita::ApplicationWindow, gtk::ApplicationWindow, gtk::Window, gtk::Widget,
    @implements gio::ActionMap, gio::ActionGroup;
}

unsafe impl Send for Window {}
unsafe impl Sync for Window {}

impl Window {
    pub fn new(
        app: &libadwaita::Application,
        style_provider: &gtk::CssProvider,
        web_context: &WebContext,
        is_private: bool,
    ) -> Self {
        let style_manager = app.style_manager();

        let this: Self = glib::Object::builder::<Self>()
            .property("application", app)
            .build();
        this.set_can_focus(true);
        this.set_icon_name(Some(APP_ID));

        let imp = this.imp();
        imp.is_private.set(is_private);
        match imp.is_private.get() {
            true => {
                this.set_title(Some("Oku — Private"));
                this.add_css_class("devel");
                imp.network_session.replace(NetworkSession::new_ephemeral());
            }
            false => {
                this.set_title(Some("Oku"));
            }
        }
        imp.style_provider.replace(style_provider.clone());

        this.setup_headerbar();
        this.setup_menu_popover();
        this.setup_find_popover();
        this.setup_tabs();
        this.setup_main_content(web_context);
        this.setup_overview_button_clicked();
        this.setup_note_button_clicked();
        this.setup_find_button_clicked();
        this.setup_replicas_button_clicked();
        this.setup_tab_indicator();
        this.setup_add_tab_button_clicked(web_context);
        this.setup_sidebar_button_clicked();
        this.setup_tab_signals(web_context, &style_manager);
        this.setup_navigation_signals();
        this.setup_suggestions_popover();
        this.setup_menu_buttons_clicked(web_context);
        this.setup_new_view_signals(web_context, &style_manager);
        this.setup_network_session();
        this.setup_actions(web_context);
        this.setup_config(&style_manager);

        if imp.tab_view.n_pages() == 0 {
            let initial_web_view = this.new_tab_page(web_context, None, None).0;
            initial_web_view.load_uri("oku:home");
        }

        this.present();
        this.watch_all();

        this
    }

    fn setup_config(&self, style_manager: &libadwaita::StyleManager) {
        let imp = self.imp();

        // Window appearance
        apply_appearance_config(style_manager, self);
        imp.config.connect_notify_local(
            Some("colour-per-domain"),
            clone!(
                #[weak]
                style_manager,
                #[weak(rename_to = this)]
                self,
                move |_, _| {
                    apply_appearance_config(&style_manager, &this);
                }
            ),
        );
        imp.config.connect_notify_local(
            Some("colour-scheme"),
            clone!(
                #[weak]
                style_manager,
                #[weak(rename_to = this)]
                self,
                move |_, _| {
                    apply_appearance_config(&style_manager, &this);
                }
            ),
        );
        imp.config.connect_notify_local(
            Some("palette"),
            clone!(
                #[weak]
                style_manager,
                #[weak(rename_to = this)]
                self,
                move |_, _| {
                    apply_appearance_config(&style_manager, &this);
                }
            ),
        );

        // Window properties
        let config = imp.config.imp();
        let (mut previous_width, mut previous_height) = (config.width(), config.height());
        if previous_width == 0 && previous_height == 0 {
            (previous_width, previous_height) = (1000, 700);
        }
        config.set_width(previous_width);
        config.set_height(previous_height);

        self.set_properties(&[
            ("default-width", &config.width()),
            ("default-height", &config.height()),
            ("maximized", &config.is_maximised()),
            ("fullscreened", &config.is_fullscreen()),
        ]);

        self.bind_property("default-width", &imp.config, "width")
            .bidirectional()
            .build();
        self.bind_property("default-height", &imp.config, "height")
            .bidirectional()
            .build();
        self.bind_property("maximized", &imp.config, "is-maximised")
            .bidirectional()
            .build();
        self.bind_property("fullscreened", &imp.config, "is-fullscreen")
            .bidirectional()
            .build();
    }

    fn setup_actions(&self, web_context: &WebContext) {
        let action_close_window = gio::ActionEntry::builder("close-window")
            .activate(clone!(move |window: &Self, _, _| window.close()))
            .build();
        self.add_action_entries([action_close_window]);

        let action_inspector = gio::ActionEntry::builder("inspector")
            .activate(clone!(move |window: &Self, _, _| {
                if window.imp().tab_view.n_pages() == 0 {
                    return;
                }
                let web_view = window.get_view();
                if let Some(inspector) = web_view.inspector() {
                    inspector.show()
                }
            }))
            .build();
        self.add_action_entries([action_inspector]);

        let action_open_file = gio::ActionEntry::builder("open-file")
            .activate(clone!(
                #[weak]
                web_context,
                move |window: &Self, _, _| {
                    let file_dialog = gtk::FileDialog::builder().build();
                    file_dialog.open(
                        Some(window),
                        Some(&gio::Cancellable::new()),
                        clone!(
                            #[weak]
                            web_context,
                            #[weak]
                            window,
                            move |destination| {
                                if let Ok(destination) = destination {
                                    let new_view = window.new_tab_page(&web_context, None, None).0;
                                    new_view.load_uri(&format!(
                                        "file://{}",
                                        destination.path().unwrap().to_str().unwrap()
                                    ));
                                }
                            }
                        ),
                    )
                }
            ))
            .build();
        self.add_action_entries([action_open_file]);

        let action_save = gio::ActionEntry::builder("save")
            .activate(clone!(move |window: &Self, _, _| {
                if window.imp().tab_view.n_pages() == 0 {
                    return;
                }
                let web_view = window.get_view();
                let dialog = libadwaita::AlertDialog::new(
                    Some("Save page?"),
                    Some(&format!(
                        "Would you like to save '{}'?",
                        web_view.uri().unwrap_or_default()
                    )),
                );
                dialog.add_responses(&[("cancel", "Cancel"), ("save", "Save")]);
                dialog.set_response_appearance("cancel", ResponseAppearance::Default);
                dialog.set_response_appearance("save", ResponseAppearance::Suggested);
                dialog.set_default_response(Some("cancel"));
                dialog.set_close_response("cancel");
                dialog.connect_response(
                    None,
                    clone!(
                        #[weak]
                        window,
                        #[weak]
                        web_view,
                        move |_, response| {
                            match response {
                                "cancel" => (),
                                "save" => {
                                    let mhtml_filter = gtk::FileFilter::new();
                                    mhtml_filter.add_pattern("*.mhtml");
                                    let filter_store = gio::ListStore::new::<gtk::FileFilter>();
                                    filter_store.append(&mhtml_filter);
                                    let file_dialog = gtk::FileDialog::builder()
                                        .accept_label("Save")
                                        .initial_name("page.mhtml")
                                        .filters(&filter_store)
                                        .initial_folder(&gio::File::for_path(
                                            glib::user_special_dir(
                                                glib::enums::UserDirectory::Downloads,
                                            )
                                            .unwrap(),
                                        ))
                                        .title(format!(
                                            "Select destination for '{}'",
                                            web_view.uri().unwrap_or_default()
                                        ))
                                        .build();
                                    file_dialog.save(
                                        Some(&window),
                                        Some(&gio::Cancellable::new()),
                                        clone!(
                                            #[weak]
                                            web_view,
                                            move |destination| {
                                                if let Ok(destination) = destination {
                                                    web_view.save_to_file(
                                                        &destination,
                                                        webkit2gtk::SaveMode::Mhtml,
                                                        Some(&gio::Cancellable::new()),
                                                        |_| {},
                                                    );
                                                }
                                            }
                                        ),
                                    )
                                }
                                _ => {
                                    unreachable!()
                                }
                            }
                        }
                    ),
                );
                dialog.present(Some(window));
            }))
            .build();
        self.add_action_entries([action_save]);

        let action_next_tab = gio::ActionEntry::builder("next-tab")
            .activate(clone!(move |window: &Self, _, _| {
                let tab_view = &window.imp().tab_view;
                tab_view.select_next_page();
            }))
            .build();
        self.add_action_entries([action_next_tab]);
        let action_previous_tab = gio::ActionEntry::builder("previous-tab")
            .activate(clone!(move |window: &Self, _, _| {
                let tab_view = &window.imp().tab_view;
                tab_view.select_previous_page();
            }))
            .build();
        self.add_action_entries([action_previous_tab]);
        let action_current_tab_left = gio::ActionEntry::builder("current-tab-left")
            .activate(clone!(move |window: &Self, _, _| {
                let tab_view = &window.imp().tab_view;
                if let Some(current_page) = tab_view.selected_page() {
                    tab_view.reorder_backward(&current_page);
                }
            }))
            .build();
        self.add_action_entries([action_current_tab_left]);
        let action_current_tab_right = gio::ActionEntry::builder("current-tab-right")
            .activate(clone!(move |window: &Self, _, _| {
                let tab_view = &window.imp().tab_view;
                if let Some(current_page) = tab_view.selected_page() {
                    tab_view.reorder_forward(&current_page);
                }
            }))
            .build();
        self.add_action_entries([action_current_tab_right]);
        let action_duplicate_current_tab = gio::ActionEntry::builder("duplicate-current-tab")
            .activate(clone!(
                #[weak]
                web_context,
                move |window: &Self, _, _| {
                    let web_view = window.get_view();
                    let new_view = window.new_tab_page(&web_context, None, None).0;
                    new_view.load_uri(&web_view.uri().unwrap_or("oku:home".into()))
                }
            ))
            .build();
        self.add_action_entries([action_duplicate_current_tab]);
        let action_tab_overview = gio::ActionEntry::builder("tab-overview")
            .activate(clone!(move |window: &Self, _, _| {
                let tab_overview = &window.imp().tab_overview;
                tab_overview.set_open(!tab_overview.is_open())
            }))
            .build();
        self.add_action_entries([action_tab_overview]);
    }

    fn setup_main_content(&self, web_context: &WebContext) {
        let imp = self.imp();

        self.setup_url_status();
        self.setup_progress_bar();
        self.setup_fullscreen_overlay();
        self.setup_okunet_fetch_overlay();

        imp.main_overlay.set_vexpand(true);
        imp.main_overlay.set_child(Some(&imp.tab_view));
        imp.main_overlay.add_overlay(&imp.progress_bar);
        imp.main_overlay.add_overlay(&imp.okunet_fetch_overlay_box);
        imp.main_overlay.add_overlay(&imp.fullscreen_overlay_box);
        imp.main_overlay.add_overlay(&imp.url_status_outer_box);

        imp.main_box.set_orientation(gtk::Orientation::Vertical);
        imp.main_box.set_vexpand(false);
        imp.main_box.append(&imp.tab_bar_revealer);
        imp.main_box.append(&imp.main_overlay);

        imp.fullscreen_box.add_top_bar(&imp.headerbar);
        imp.fullscreen_box.set_content(Some(&imp.main_box));
        imp.fullscreen_motion_controller.connect_motion(clone!(
            #[weak(rename_to = this)]
            self,
            #[weak]
            imp,
            move |_fullscreen_motion_controller, _x, y| {
                match this.is_fullscreen() {
                    true => imp.fullscreen_box.set_reveal_top_bars(
                        y <= imp.fullscreen_box.top_bar_height().max(5).into(),
                    ),
                    false => imp.fullscreen_box.set_reveal_top_bars(true),
                }
            }
        ));
        imp.fullscreen_box
            .add_controller(imp.fullscreen_motion_controller.clone());

        self.setup_sidebar(web_context);
        imp.split_view.set_content(Some(&imp.fullscreen_box));
        imp.split_view.set_sidebar(Some(&imp.side_box));
        imp.split_view.set_max_sidebar_width(400.0);
        imp.split_view.set_collapsed(true);
        imp.split_view.set_pin_sidebar(true);

        imp.tab_overview.set_enable_new_tab(true);
        imp.tab_overview.set_enable_search(true);
        imp.tab_overview.set_view(Some(&imp.tab_view));
        imp.tab_overview.set_child(Some(&imp.split_view));

        self.set_content(Some(&imp.tab_overview));
    }

    pub fn setup_fullscreen_overlay(&self) {
        let imp = self.imp();

        imp.fullscreen_overlay_label
            .set_label("Press F11 to exit fullscreen … ");
        imp.fullscreen_overlay_label.add_css_class("title-1");
        imp.fullscreen_overlay_box
            .append(&imp.fullscreen_overlay_label);
        imp.fullscreen_overlay_box.set_valign(gtk::Align::Center);
        imp.fullscreen_overlay_box.set_halign(gtk::Align::Center);
        imp.fullscreen_overlay_box.set_can_focus(false);
        imp.fullscreen_overlay_box.set_can_target(false);
        imp.fullscreen_overlay_box.add_css_class("osd");
        imp.fullscreen_overlay_box.add_css_class("toolbar");
        imp.fullscreen_overlay_box.set_opacity(0.0);

        self.connect_fullscreened_notify(clone!(
            #[weak]
            imp,
            move |this| {
                match this.is_fullscreen() {
                    true => this.show_fullscreen_overlay(),
                    false => imp.fullscreen_overlay_box.set_opacity(0.0),
                }
            }
        ));
    }

    pub fn show_fullscreen_overlay(&self) {
        let imp = self.imp();

        if let Some(animation) = imp.fullscreen_overlay_animation.borrow().as_ref() {
            animation.pause()
        }
        let animation = libadwaita::SpringAnimation::new(
            &imp.fullscreen_overlay_box,
            0.0,
            1.0,
            libadwaita::SpringParams::new(0.75, 75.0, 100.0),
            libadwaita::CallbackAnimationTarget::new(clone!(
                #[weak]
                imp,
                move |v| {
                    imp.fullscreen_overlay_box.set_opacity(1.0 - v);
                }
            )),
        );
        animation.set_clamp(true);
        animation.play();
        imp.fullscreen_overlay_animation.replace(Some(animation));
    }

    pub fn toggle_fullscreen(&self) {
        match self.is_fullscreen() {
            true => {
                let this = self.clone();
                glib::spawn_future_local(this.get_view().evaluate_javascript_future(
                    "document.exitFullscreen();",
                    None,
                    None,
                ));
                self.set_fullscreened(false);
            }
            false => {
                self.set_fullscreened(true);
            }
        }
    }

    pub async fn watch_bookmarks(&self) {
        self.imp().bookmarks_sidebar_initialised.set(true);
        let mut bookmark_rx = DATABASE.bookmark_sender.subscribe();
        loop {
            bookmark_rx.borrow_and_update();
            info!("Bookmarks updated … ");
            let this = self.clone();
            tokio::task::spawn_blocking(move || this.bookmarks_updated());
            match bookmark_rx.changed().await {
                Ok(_) => continue,
                Err(e) => {
                    error!("{}", e);
                    break;
                }
            }
        }
    }

    pub async fn watch_history(&self) {
        self.imp().history_sidebar_initialised.set(true);
        let mut history_rx = DATABASE.history_sender.subscribe();
        loop {
            history_rx.borrow_and_update();
            info!("History updated … ");
            let this = self.clone();
            tokio::task::spawn_blocking(move || this.history_updated());
            match history_rx.changed().await {
                Ok(_) => continue,
                Err(e) => {
                    error!("{}", e);
                    break;
                }
            }
        }
    }

    pub async fn watch_replicas(&self) {
        if let Some(node) = NODE.get() {
            self.imp().replicas_sidebar_initialised.set(true);
            let mut replica_rx = node.replica_sender.subscribe();
            loop {
                replica_rx.borrow_and_update();
                info!("Replicas updated … ");
                let this = self.clone();
                tokio::spawn(async move { this.replicas_updated().await });
                HOME_REPLICA_SET.store(node.home_replica().await.is_some(), Ordering::Relaxed);
                match replica_rx.changed().await {
                    Ok(_) => continue,
                    Err(e) => {
                        error!("{}", e);
                        break;
                    }
                }
            }
        }
    }

    pub fn setup_okunet_fetch_overlay(&self) {
        let imp = self.imp();

        imp.okunet_fetch_overlay_label
            .set_label("Fetching from the OkuNet … ");
        imp.okunet_fetch_overlay_box
            .append(&imp.okunet_fetch_overlay_label);
        imp.okunet_fetch_overlay_box
            .append(&imp.okunet_fetch_overlay_spinner);
        imp.okunet_fetch_overlay_box.set_valign(gtk::Align::Start);
        imp.okunet_fetch_overlay_box.set_halign(gtk::Align::Start);
        imp.okunet_fetch_overlay_box.set_margin_start(4);
        imp.okunet_fetch_overlay_box.set_margin_top(4);
        imp.okunet_fetch_overlay_box.set_margin_bottom(4);
        imp.okunet_fetch_overlay_box.set_margin_end(4);
        imp.okunet_fetch_overlay_box.set_can_focus(false);
        imp.okunet_fetch_overlay_box.set_can_target(false);
        imp.okunet_fetch_overlay_box.add_css_class("card");
        imp.okunet_fetch_overlay_box.add_css_class("toolbar");
        imp.okunet_fetch_overlay_box.set_opacity(0.0);

        // imp.okunet_fetch_overlay_box
        //     .property_expression("opacity")
        //     .chain_closure::<bool>(closure!(|_: Option<glib::Object>, x: f64| { x == 0.0 }))
        //     .bind(
        //         &imp.nav_entry.imp().okunet_refresh_button,
        //         "sensitive",
        //         gtk::Widget::NONE,
        //     );
    }

    pub fn show_okunet_fetch_overlay(&self) {
        let imp = self.imp();

        if let Some(animation) = imp.okunet_fetch_overlay_animation.borrow().as_ref() {
            animation.pause()
        }
        let animation = libadwaita::SpringAnimation::new(
            &imp.okunet_fetch_overlay_box,
            imp.okunet_fetch_overlay_box.opacity(),
            1.0,
            libadwaita::SpringParams::new(1.0, 15.0, 100.0),
            libadwaita::CallbackAnimationTarget::new(clone!(
                #[weak]
                imp,
                move |v| {
                    imp.okunet_fetch_overlay_box.set_opacity(v);
                }
            )),
        );
        animation.set_clamp(true);
        animation.play();
        imp.okunet_fetch_overlay_animation.replace(Some(animation));
    }

    pub fn hide_okunet_fetch_overlay(&self) {
        let imp = self.imp();

        if let Some(animation) = imp.okunet_fetch_overlay_animation.borrow().as_ref() {
            animation.pause()
        }
        let animation = libadwaita::SpringAnimation::new(
            &imp.okunet_fetch_overlay_box,
            imp.okunet_fetch_overlay_box.opacity(),
            1.0,
            libadwaita::SpringParams::new(1.0, 15.0, 100.0),
            libadwaita::CallbackAnimationTarget::new(clone!(
                #[weak]
                imp,
                move |v| {
                    imp.okunet_fetch_overlay_box.set_opacity(1.0 - v);
                }
            )),
        );
        animation.set_clamp(true);
        animation.play();
        imp.okunet_fetch_overlay_animation.replace(Some(animation));
    }

    pub async fn watch_okunet_fetch(&self) {
        if let Some(node) = NODE.get() {
            let mut okunet_fetch_rx = node.okunet_fetch_sender.subscribe();
            loop {
                let this = self.clone();
                match *okunet_fetch_rx.borrow_and_update() {
                    true => glib::MainContext::default()
                        .invoke(move || this.show_okunet_fetch_overlay()),
                    false => glib::MainContext::default()
                        .invoke(move || this.hide_okunet_fetch_overlay()),
                }
                match okunet_fetch_rx.changed().await {
                    Ok(_) => continue,
                    Err(e) => {
                        error!("{}", e);
                        break;
                    }
                }
            }
        }
    }

    pub fn watch_all(&self) {
        let this = self.clone();
        tokio::spawn(async move { this.watch_bookmarks().await });
        let this = self.clone();
        tokio::spawn(async move { this.watch_history().await });
        let this = self.clone();
        tokio::spawn(async move { this.watch_replicas().await });
        let this = self.clone();
        tokio::spawn(async move { this.watch_okunet_fetch().await });
    }
}
