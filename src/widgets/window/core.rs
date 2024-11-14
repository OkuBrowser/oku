use crate::config::Config;
use crate::widgets::settings::core::apply_appearance_config;
use crate::window_util::get_window_from_widget;
use crate::APP_ID;
use glib::clone;
use gtk::prelude::GtkWindowExt;
use gtk::subclass::prelude::*;
use gtk::EventControllerFocus;
use gtk::{gio, glib};
use libadwaita::subclass::application_window::AdwApplicationWindowImpl;
use libadwaita::{prelude::*, ResponseAppearance};
use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;
use webkit2gtk::prelude::WebViewExt;
use webkit2gtk::{NetworkSession, WebContext};

pub mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct Window {
        pub(crate) config: Config,
        // Window parameters
        pub(crate) is_private: Cell<bool>,
        pub(crate) style_provider: RefCell<gtk::CssProvider>,
        // Navigation bar
        pub(crate) nav_entry: gtk::SearchEntry,
        pub(crate) nav_entry_focus: RefCell<EventControllerFocus>,
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
        // History
        pub(crate) history_box: gtk::Box,
        pub(crate) history_store: RefCell<Option<Rc<gio::ListStore>>>,
        pub(crate) history_factory: gtk::SignalListItemFactory,
        pub(crate) history_model: gtk::SingleSelection,
        pub(crate) history_view: gtk::ListView,
        pub(crate) history_scrolled_window: gtk::ScrolledWindow,
        pub(crate) history_label: gtk::Label,
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
        // Downloads
        pub(crate) downloads_box: gtk::Box,
        pub(crate) downloads_store: RefCell<Option<gio::ListStore>>,
        pub(crate) downloads_factory: gtk::SignalListItemFactory,
        pub(crate) downloads_model: gtk::SingleSelection,
        pub(crate) downloads_view: gtk::ListView,
        pub(crate) downloads_scrolled_window: gtk::ScrolledWindow,
        pub(crate) downloads_label: gtk::Label,
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
        this.setup_main_content(&web_context);
        this.setup_overview_button_clicked();
        this.setup_note_button_clicked();
        this.setup_find_button_clicked();
        this.setup_replicas_button_clicked();
        this.setup_tab_indicator();
        this.setup_add_tab_button_clicked(&web_context);
        this.setup_sidebar_button_clicked();
        this.setup_tab_signals(&web_context, &style_manager);
        this.setup_navigation_signals();
        this.setup_suggestions_popover();
        this.setup_menu_buttons_clicked(&web_context);
        this.setup_new_view_signals(&web_context, &style_manager);
        this.setup_network_session();
        this.setup_actions(&web_context);
        this.setup_config(&style_manager);

        if imp.tab_view.n_pages() == 0 {
            let initial_web_view = this.new_tab_page(&web_context, None, None).0;
            initial_web_view.load_uri("oku:home");
        }

        this.present();

        this
    }

    fn setup_config(&self, style_manager: &libadwaita::StyleManager) {
        let imp = self.imp();

        // Window appearance
        apply_appearance_config(&style_manager, self);
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
            (previous_width, previous_height) = (768, 576);
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
                                        .title(&format!(
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

        imp.main_overlay.set_vexpand(true);
        imp.main_overlay.set_child(Some(&imp.tab_view));
        imp.main_overlay.add_overlay(&imp.progress_bar);
        imp.main_overlay.add_overlay(&imp.url_status_outer_box);

        imp.main_box.set_orientation(gtk::Orientation::Vertical);
        imp.main_box.set_vexpand(false);
        imp.main_box.append(&imp.headerbar);
        imp.main_box.append(&imp.tab_bar_revealer);
        imp.main_box.append(&imp.main_overlay);

        self.setup_sidebar(&web_context);
        imp.split_view.set_content(Some(&imp.main_box));
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
}
