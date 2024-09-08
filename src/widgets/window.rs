use super::settings::apply_config;
use crate::suggestion_item::SuggestionItem;
use crate::window_util::{
    connect, get_title, get_view_from_page, new_webkit_settings, update_favicon, update_nav_bar,
    update_title,
};
use crate::{CONFIG, DATA_DIR, HISTORY_MANAGER, MOUNT_DIR, VERSION};
use chrono::Utc;
use glib::clone;
use gtk::subclass::prelude::*;
use gtk::EventControllerFocus;
use gtk::{gio, glib};
use libadwaita::subclass::application_window::AdwApplicationWindowImpl;
use libadwaita::{prelude::*, ResponseAppearance};
use std::cell::RefCell;
use std::cell::{Cell, Ref};
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use tracing::{error, info, warn};
use webkit2gtk::functions::{
    uri_for_display, user_media_permission_is_for_audio_device,
    user_media_permission_is_for_display_device, user_media_permission_is_for_video_device,
};
use webkit2gtk::prelude::PermissionRequestExt;
use webkit2gtk::prelude::PolicyDecisionExt;
use webkit2gtk::prelude::WebViewExt;
use webkit2gtk::{FindOptions, NavigationPolicyDecision, PolicyDecisionType, WebContext, WebView};
use webkit2gtk::{LoadEvent, NavigationType};

pub mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct Window {
        // Window parameters
        pub(crate) _is_private: Cell<bool>,
        pub(crate) style_provider: RefCell<gtk::CssProvider>,
        // Navigation bar
        pub(crate) nav_entry: gtk::SearchEntry,
        pub(crate) nav_entry_focus: RefCell<EventControllerFocus>,
        pub(crate) suggestions_store: RefCell<Option<Rc<gio::ListStore>>>,
        pub(crate) suggestions_factory: gtk::SignalListItemFactory,
        pub(crate) suggestions_model: gtk::SingleSelection,
        pub(crate) suggestions_view: gtk::ListView,
        pub(crate) suggestions_scrolled_window: gtk::ScrolledWindow,
        pub(crate) suggestions_popover: gtk::Popover,
        // Left header buttons
        pub(crate) back_button: gtk::Button,
        pub(crate) forward_button: gtk::Button,
        pub(crate) navigation_buttons: gtk::Box,
        pub(crate) add_tab_button: gtk::Button,
        pub(crate) refresh_button: gtk::Button,
        pub(crate) left_header_buttons: gtk::Box,
        // Right header buttons
        pub(crate) overview_button: libadwaita::TabButton,
        pub(crate) downloads_button: gtk::Button,
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
        pub(crate) history_button: gtk::Button,
        pub(crate) settings_button: gtk::Button,
        pub(crate) about_button: gtk::Button,
        pub(crate) menu_box: gtk::Box,
        pub(crate) menu_popover: gtk::Popover,
        // Downloads popover
        pub(crate) downloads_box: gtk::Box,
        pub(crate) downloads_popover: gtk::Popover,
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
        // Miscellaneous
        pub(crate) progress_animation: RefCell<Option<libadwaita::SpringAnimation>>,
        pub(crate) progress_bar: gtk::ProgressBar,
        pub(crate) url_status_outer_box: gtk::Box,
        pub(crate) url_status_box: gtk::Box,
        pub(crate) url_status: gtk::Label,
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
    @extends libadwaita::ApplicationWindow, gtk::Window, gtk::Widget,
    @implements gio::ActionMap, gio::ActionGroup;
}

impl Window {
    pub fn new(app: &libadwaita::Application, web_context: &WebContext) -> Self {
        let style_manager = app.style_manager();

        let this: Self = glib::Object::builder::<Self>()
            .property("application", app)
            .build();
        this.set_can_focus(true);
        this.set_title(Some("Oku"));
        this.set_icon_name(Some("com.github.dirout.oku"));

        this.setup_css_providers();
        this.setup_headerbar();
        this.setup_menu_popover();
        this.setup_downloads_popover();
        this.setup_find_popover();
        this.setup_tabs();
        this.setup_main_content();
        this.setup_overview_button_clicked();
        this.setup_downloads_button_clicked();
        this.setup_find_button_clicked();
        this.setup_replicas_button_clicked();
        this.setup_tab_indicator();
        this.setup_add_tab_button_clicked(&web_context);
        this.setup_tab_signals(&web_context, &style_manager);
        this.setup_navigation_signals();
        this.setup_suggestions_popover();
        this.setup_menu_buttons_clicked(&web_context);
        this.setup_new_view_signals(&web_context, &style_manager);

        let imp = this.imp();

        if imp.tab_view.n_pages() == 0 && app.windows().len() <= 1 {
            let initial_web_view = this.new_tab_page(&web_context, None, None).0;
            initial_web_view.load_uri("about:blank");
        }
        this.set_content(Some(&imp.tab_overview));
        apply_config(&style_manager, &this);
        this.set_visible(true);

        this
    }

    fn setup_css_providers(&self) {
        let imp = self.imp();

        gtk::style_context_add_provider_for_display(
            &gdk::Display::default().unwrap(),
            &*imp.style_provider.borrow(),
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    /// Adapted from Geopard (https://github.com/ranfdev/Geopard)
    fn set_progress_animated(&self, progress: f64) {
        let imp = self.imp();

        if let Some(animation) = imp.progress_animation.borrow().as_ref() {
            animation.pause()
        }
        if progress == 0.0 {
            imp.progress_bar.set_fraction(0.0);
            return;
        }
        let progress_bar = imp.progress_bar.clone();
        let animation = libadwaita::SpringAnimation::new(
            &imp.progress_bar,
            imp.progress_bar.fraction(),
            progress,
            libadwaita::SpringParams::new(1.0, 1.0, 100.0),
            libadwaita::CallbackAnimationTarget::new(move |v| {
                progress_bar.set_fraction(v);
                progress_bar.set_opacity(1.0 - v);
            }),
        );
        animation.play();
        imp.progress_animation.replace(Some(animation));
    }

    /// Update the load progress indicator under the navigation bar
    ///
    /// # Arguments
    ///
    /// * `nav_entry` - The navigation bar of the browser
    ///
    /// * `web_view` - The WebKit instance for the current tab
    pub fn update_load_progress(&self, web_view: &webkit2gtk::WebView) {
        let load_progress = web_view.estimated_load_progress();
        if load_progress as i64 == 1 {
            self.set_progress_animated(0.00)
        } else {
            self.set_progress_animated(load_progress)
        }
    }

    /// Get the WebKit instance for the current tab
    pub fn get_view(&self) -> webkit2gtk::WebView {
        let imp = self.imp();

        if let Some(current_page) = imp.tab_view.selected_page() {
            let current_page_number = imp.tab_view.page_position(&current_page);
            let specific_page = imp.tab_view.nth_page(current_page_number);
            specific_page.child().downcast().unwrap()
        } else {
            let web_view = webkit2gtk::WebView::new();
            web_view.load_uri("about:blank");
            web_view
        }
    }

    fn setup_url_status(&self) {
        let imp = self.imp();

        imp.url_status.set_hexpand(false);
        imp.url_status.set_xalign(0.0);
        imp.url_status.set_ellipsize(pango::EllipsizeMode::End);
        imp.url_status.set_margin_top(6);
        imp.url_status.set_margin_bottom(6);
        imp.url_status.set_margin_start(6);
        imp.url_status.set_margin_end(6);
        imp.url_status.set_selectable(false);
        imp.url_status.set_can_focus(false);
        imp.url_status.set_can_target(false);
        imp.url_status.set_focusable(false);

        imp.url_status_box.set_hexpand(false);
        imp.url_status_box.add_css_class("background");
        imp.url_status_box.set_can_focus(false);
        imp.url_status_box.set_can_target(false);
        imp.url_status_box.set_focusable(false);
        imp.url_status_box.append(&imp.url_status);
        imp.url_status_box.set_visible(false);

        imp.url_status_outer_box
            .set_orientation(gtk::Orientation::Vertical);
        imp.url_status_outer_box.set_valign(gtk::Align::End);
        imp.url_status_outer_box.set_hexpand(false);
        imp.url_status_outer_box.set_can_focus(false);
        imp.url_status_outer_box.set_can_target(false);
        imp.url_status_outer_box.set_focusable(false);
        imp.url_status_outer_box.append(&imp.url_status_box);
    }

    fn setup_navigation_buttons(&self) {
        let imp = self.imp();

        // Navigation bar
        imp.nav_entry
            .add_controller(imp.nav_entry_focus.borrow().clone());
        imp.nav_entry.set_can_focus(true);
        imp.nav_entry.set_focusable(true);
        imp.nav_entry.set_focus_on_click(true);
        imp.nav_entry.set_editable(true);
        imp.nav_entry.set_hexpand(true);
        imp.nav_entry
            .set_placeholder_text(Some("Enter an address … "));
        imp.nav_entry.set_input_purpose(gtk::InputPurpose::Url);
        imp.nav_entry.set_halign(gtk::Align::Fill);

        // Back button
        imp.back_button.set_can_focus(true);
        imp.back_button.set_receives_default(true);
        imp.back_button.set_icon_name("go-previous");
        imp.back_button.add_css_class("linked");

        // Forward button
        imp.forward_button.set_can_focus(true);
        imp.forward_button.set_receives_default(true);
        imp.forward_button.set_icon_name("go-next");
        imp.forward_button.add_css_class("linked");

        // All navigation buttons
        imp.navigation_buttons.append(&imp.back_button);
        imp.navigation_buttons.append(&imp.forward_button);
        imp.navigation_buttons.add_css_class("linked");
    }

    fn setup_left_headerbar(&self) {
        self.setup_navigation_buttons();
        let imp = self.imp();

        // Add Tab button
        imp.add_tab_button.set_can_focus(true);
        imp.add_tab_button.set_receives_default(true);
        imp.add_tab_button.set_icon_name("tab-new");

        // Refresh button
        imp.refresh_button.set_can_focus(true);
        imp.refresh_button.set_receives_default(true);
        imp.refresh_button.set_icon_name("view-refresh");

        // Left header buttons
        imp.left_header_buttons.append(&imp.navigation_buttons);
        imp.left_header_buttons.append(&imp.refresh_button);
        imp.left_header_buttons.append(&imp.add_tab_button);
    }

    fn setup_right_headerbar(&self) {
        let imp = self.imp();

        // Overview button
        imp.overview_button.set_can_focus(true);
        imp.overview_button.set_receives_default(true);
        imp.overview_button.set_view(Some(&imp.tab_view));

        // Downloads button
        imp.downloads_button.set_can_focus(true);
        imp.downloads_button.set_receives_default(true);
        imp.downloads_button
            .set_icon_name("folder-download-symbolic");

        // Find button
        imp.find_button.set_can_focus(true);
        imp.find_button.set_receives_default(true);
        imp.find_button.set_icon_name("edit-find");

        // Replica menu button
        imp.replicas_button.set_can_focus(true);
        imp.replicas_button.set_receives_default(true);
        imp.replicas_button.set_icon_name("file-cabinet-symbolic");

        // Menu button
        imp.menu_button.set_can_focus(true);
        imp.menu_button.set_receives_default(true);
        imp.menu_button.set_icon_name("document-properties");

        imp.right_header_buttons.append(&imp.overview_button);
        imp.right_header_buttons.append(&imp.downloads_button);
        imp.right_header_buttons.append(&imp.find_button);
        imp.right_header_buttons.append(&imp.replicas_button);
        imp.right_header_buttons.append(&imp.menu_button);
    }

    fn setup_headerbar(&self) {
        self.setup_left_headerbar();
        self.setup_right_headerbar();
        let imp = self.imp();
        // HeaderBar
        imp.headerbar.set_can_focus(true);
        imp.headerbar.set_title_widget(Some(&imp.nav_entry));
        imp.headerbar.pack_start(&imp.left_header_buttons);
        imp.headerbar.pack_end(&imp.right_header_buttons);
    }

    fn setup_menu_popover(&self) {
        let imp = self.imp();

        // Zoom out button
        imp.zoomout_button.set_can_focus(true);
        imp.zoomout_button.set_receives_default(true);
        imp.zoomout_button.set_icon_name("zoom-out");
        imp.zoomout_button.add_css_class("linked");

        // Zoom in button
        imp.zoomin_button.set_can_focus(true);
        imp.zoomin_button.set_receives_default(true);
        imp.zoomin_button.set_icon_name("zoom-in");
        imp.zoomin_button.add_css_class("linked");

        // Both zoom buttons
        imp.zoom_buttons.append(&imp.zoomout_button);
        imp.zoom_buttons.append(&imp.zoomin_button);
        imp.zoom_buttons.add_css_class("linked");

        // Zoom percentage label
        imp.zoom_percentage.set_text("100%");
        imp.zoom_percentage.set_margin_start(4);
        imp.zoom_percentage.set_margin_end(4);

        // Zoom reset button
        imp.zoomreset_button.set_can_focus(true);
        imp.zoomreset_button.set_receives_default(true);
        imp.zoomreset_button.set_icon_name("zoom-original");

        // Fullscreen button
        imp.fullscreen_button.set_can_focus(true);
        imp.fullscreen_button.set_receives_default(true);
        imp.fullscreen_button.set_icon_name("view-fullscreen");

        // Print button
        imp.print_button.set_can_focus(true);
        imp.print_button.set_receives_default(true);
        imp.print_button.set_icon_name("document-print");

        // Screenshot button
        imp.screenshot_button.set_can_focus(true);
        imp.screenshot_button.set_receives_default(true);
        imp.screenshot_button.set_icon_name("camera-photo");

        // New Window button
        imp.new_window_button.set_can_focus(true);
        imp.new_window_button.set_receives_default(true);
        imp.new_window_button.set_icon_name("window-new");

        // History button
        imp.history_button.set_can_focus(true);
        imp.history_button.set_receives_default(true);
        imp.history_button.set_icon_name("document-open-recent");

        // Settings button
        imp.settings_button.set_can_focus(true);
        imp.settings_button.set_receives_default(true);
        imp.settings_button.set_icon_name("preferences-system");

        // About button
        imp.about_button.set_can_focus(true);
        imp.about_button.set_receives_default(true);
        imp.about_button.set_icon_name("help-about");

        // Menu popover
        imp.menu_box.set_hexpand(true);
        imp.menu_box.append(&imp.zoom_buttons);
        imp.menu_box.append(&imp.zoom_percentage);
        imp.menu_box.append(&imp.zoomreset_button);
        imp.menu_box.append(&imp.fullscreen_button);
        imp.menu_box.append(&imp.print_button);
        imp.menu_box.append(&imp.screenshot_button);
        imp.menu_box.append(&imp.new_window_button);
        imp.menu_box.append(&imp.history_button);
        imp.menu_box.append(&imp.settings_button);
        imp.menu_box.append(&imp.about_button);
        imp.menu_box.add_css_class("toolbar");

        imp.menu_popover.set_child(Some(&imp.menu_box));
        imp.menu_popover.set_parent(&imp.menu_button);
        imp.menu_popover.set_autohide(true);
    }

    fn setup_downloads_popover(&self) {
        let imp = self.imp();

        imp.downloads_popover.set_child(Some(&imp.downloads_box));
        imp.downloads_popover.set_parent(&imp.downloads_button);
        imp.downloads_popover.set_autohide(true);
    }

    fn get_find_options(&self) -> FindOptions {
        let imp = self.imp();

        let mut find_options = FindOptions::empty();

        find_options.set(
            webkit2gtk::FindOptions::CASE_INSENSITIVE,
            imp.find_case_insensitive.is_active(),
        );
        find_options.set(
            webkit2gtk::FindOptions::AT_WORD_STARTS,
            imp.find_at_word_starts.is_active(),
        );
        find_options.set(
            webkit2gtk::FindOptions::TREAT_MEDIAL_CAPITAL_AS_WORD_START,
            imp.find_treat_medial_capital_as_word_start.is_active(),
        );
        find_options.set(
            webkit2gtk::FindOptions::BACKWARDS,
            imp.find_backwards.is_active(),
        );
        find_options.set(
            webkit2gtk::FindOptions::WRAP_AROUND,
            imp.find_wrap_around.is_active(),
        );

        find_options
    }

    fn setup_find_signals(&self) {
        let imp = self.imp();

        imp.find_search_entry.connect_search_changed(clone!(
            #[weak]
            imp,
            #[weak(rename_to = this)]
            self,
            move |find_search_entry| {
                let web_view = this.get_view();
                let find_controller = web_view.find_controller().unwrap();
                let find_options = this.get_find_options();
                find_controller.search(&find_search_entry.text(), find_options.bits(), u32::MAX);
                imp.find_search_entry.connect_activate(clone!(
                    #[weak]
                    find_controller,
                    move |_find_search_entry| find_controller.search_next()
                ));
                imp.find_search_entry.connect_next_match(clone!(
                    #[weak]
                    find_controller,
                    move |_find_search_entry| find_controller.search_next()
                ));
                imp.find_search_entry.connect_previous_match(clone!(
                    #[weak]
                    find_controller,
                    move |_find_search_entry| find_controller.search_previous()
                ));
                imp.find_search_entry.connect_stop_search(clone!(
                    #[weak]
                    imp,
                    #[weak]
                    find_controller,
                    move |_find_search_entry| {
                        imp.total_matches_label.set_text("");
                        find_controller.search_finish()
                    }
                ));
                imp.next_find_button.connect_clicked(clone!(
                    #[weak]
                    find_controller,
                    move |_next_find_button| find_controller.search_next()
                ));
                imp.previous_find_button.connect_clicked(clone!(
                    #[weak]
                    find_controller,
                    move |_previous_find_button| find_controller.search_previous()
                ));
                imp.find_case_insensitive.connect_clicked(clone!(
                    #[weak]
                    imp,
                    #[weak]
                    find_controller,
                    move |_find_case_insensitive| {
                        imp.total_matches_label.set_text("");
                        find_controller.search_finish()
                    }
                ));
                imp.find_at_word_starts.connect_clicked(clone!(
                    #[weak]
                    imp,
                    #[weak]
                    find_controller,
                    move |find_at_word_starts| {
                        if find_at_word_starts.is_active() {
                            imp.find_treat_medial_capital_as_word_start
                                .set_sensitive(true);
                        } else {
                            imp.find_treat_medial_capital_as_word_start
                                .set_sensitive(false);
                        }
                        imp.total_matches_label.set_text("");
                        find_controller.search_finish()
                    }
                ));
                imp.find_treat_medial_capital_as_word_start
                    .connect_clicked(clone!(
                        #[weak]
                        imp,
                        #[weak]
                        find_controller,
                        move |_find_treat_medial_capital_as_word_start| {
                            imp.total_matches_label.set_text("");
                            find_controller.search_finish()
                        }
                    ));
                imp.find_backwards.connect_clicked(clone!(
                    #[weak]
                    find_controller,
                    #[weak]
                    imp,
                    move |find_backwards| {
                        if find_backwards.is_active() {
                            imp.next_find_button.set_icon_name("go-up");
                            imp.previous_find_button.set_icon_name("go-down");
                        } else {
                            imp.next_find_button.set_icon_name("go-down");
                            imp.previous_find_button.set_icon_name("go-up");
                        }
                        imp.total_matches_label.set_text("");
                        find_controller.search_finish()
                    }
                ));
                imp.find_wrap_around.connect_clicked(clone!(
                    #[weak]
                    imp,
                    #[weak]
                    find_controller,
                    move |_find_wrap_around| {
                        imp.total_matches_label.set_text("");
                        find_controller.search_finish()
                    }
                ));
            }
        ));
    }

    fn setup_find_box(&self) {
        let imp = self.imp();

        self.setup_find_signals();

        imp.find_search_entry.set_can_focus(true);
        imp.find_search_entry.set_focusable(true);
        imp.find_search_entry.set_focus_on_click(true);
        imp.find_search_entry.set_editable(true);
        imp.find_search_entry.set_hexpand(true);
        imp.find_search_entry
            .set_placeholder_text(Some("Search in page … "));
        imp.find_search_entry
            .set_input_purpose(gtk::InputPurpose::Url);
        imp.find_search_entry.set_halign(gtk::Align::Fill);
        imp.find_search_entry.set_margin_start(4);
        imp.find_search_entry.set_margin_end(4);

        imp.find_middle_box.append(&imp.find_search_entry);
        imp.find_middle_box.append(&imp.total_matches_label);
        imp.find_middle_box.set_margin_start(2);
        imp.find_middle_box.set_margin_end(2);

        imp.previous_find_button.set_can_focus(true);
        imp.previous_find_button.set_receives_default(true);
        imp.previous_find_button.set_icon_name("go-up");
        imp.previous_find_button.add_css_class("linked");

        imp.next_find_button.set_can_focus(true);
        imp.next_find_button.set_receives_default(true);
        imp.next_find_button.set_icon_name("go-down");
        imp.next_find_button.add_css_class("linked");

        imp.find_buttons.append(&imp.previous_find_button);
        imp.find_buttons.append(&imp.next_find_button);
        imp.find_buttons.add_css_class("linked");
        imp.find_buttons.set_margin_start(2);
        imp.find_buttons.set_margin_end(2);

        imp.find_case_insensitive.set_can_focus(true);
        imp.find_case_insensitive.set_receives_default(true);
        imp.find_case_insensitive
            .set_icon_name("format-text-strikethrough");
        imp.find_case_insensitive.add_css_class("linked");
        imp.find_case_insensitive
            .set_tooltip_text(Some("Ignore case when searching"));

        imp.find_at_word_starts.set_can_focus(true);
        imp.find_at_word_starts.set_receives_default(true);
        imp.find_at_word_starts.set_icon_name("go-first");
        imp.find_at_word_starts.add_css_class("linked");
        imp.find_at_word_starts
            .set_tooltip_text(Some("Search text only at the start of words"));

        imp.find_treat_medial_capital_as_word_start
            .set_can_focus(true);
        imp.find_treat_medial_capital_as_word_start
            .set_receives_default(true);
        imp.find_treat_medial_capital_as_word_start
            .set_icon_name("format-text-underline");
        imp.find_treat_medial_capital_as_word_start
            .add_css_class("linked");
        imp.find_treat_medial_capital_as_word_start
            .set_tooltip_text(Some(
                "Treat capital letters in the middle of words as word start",
            ));
        imp.find_treat_medial_capital_as_word_start
            .set_sensitive(false);

        imp.find_backwards.set_can_focus(true);
        imp.find_backwards.set_receives_default(true);
        imp.find_backwards.set_icon_name("media-seek-backward");
        imp.find_backwards.add_css_class("linked");
        imp.find_backwards
            .set_tooltip_text(Some("Search backwards"));

        imp.find_wrap_around.set_can_focus(true);
        imp.find_wrap_around.set_receives_default(true);
        imp.find_wrap_around.set_icon_name("media-playlist-repeat");
        imp.find_wrap_around.add_css_class("linked");
        imp.find_wrap_around
            .set_tooltip_text(Some("Wrap around the document when searching"));

        imp.find_option_buttons.append(&imp.find_case_insensitive);
        imp.find_option_buttons.append(&imp.find_at_word_starts);
        imp.find_option_buttons
            .append(&imp.find_treat_medial_capital_as_word_start);
        imp.find_option_buttons.append(&imp.find_backwards);
        imp.find_option_buttons.append(&imp.find_wrap_around);
        imp.find_option_buttons.add_css_class("linked");
        imp.find_option_buttons.set_margin_start(2);
        imp.find_option_buttons.set_margin_end(2);

        imp.find_box.set_orientation(gtk::Orientation::Horizontal);
        imp.find_box.set_hexpand(true);
        imp.find_box.append(&imp.find_option_buttons);
        imp.find_box.append(&imp.find_middle_box);
        imp.find_box.append(&imp.find_buttons);
    }

    fn setup_find_popover(&self) {
        let imp = self.imp();

        self.setup_find_box();
        imp.find_popover.set_child(Some(&imp.find_box));
        imp.find_popover.set_parent(&imp.find_button);
        imp.find_popover.set_autohide(true);
    }

    fn setup_tabs(&self) {
        let imp = self.imp();

        imp.tab_view.set_vexpand(true);
        imp.tab_view.set_visible(true);

        imp.tab_bar.set_autohide(true);
        imp.tab_bar.set_expand_tabs(true);
        imp.tab_bar.set_view(Some(&imp.tab_view));

        imp.tab_bar_revealer.set_child(Some(&imp.tab_bar));
        imp.tab_bar_revealer
            .set_transition_type(gtk::RevealerTransitionType::SwingDown);
        imp.tab_bar_revealer.set_reveal_child(true);
    }

    fn setup_progress_bar(&self) {
        let imp = self.imp();

        imp.progress_bar.add_css_class("osd");
        imp.progress_bar.set_valign(gtk::Align::Start);
    }

    fn setup_main_content(&self) {
        let imp = self.imp();

        self.setup_url_status();
        self.setup_progress_bar();

        imp.main_overlay.set_vexpand(true);
        imp.main_overlay.set_child(Some(&imp.tab_view));
        imp.main_overlay.add_overlay(&imp.progress_bar);
        imp.main_overlay.add_overlay(&imp.url_status_outer_box);

        imp.main_box.set_orientation(gtk::Orientation::Vertical);
        imp.main_box.set_vexpand(true);
        imp.main_box.append(&imp.headerbar);
        imp.main_box.append(&imp.tab_bar_revealer);
        imp.main_box.append(&imp.main_overlay);

        imp.tab_overview.set_enable_new_tab(true);
        imp.tab_overview.set_enable_search(true);
        imp.tab_overview.set_view(Some(&imp.tab_view));
        imp.tab_overview.set_child(Some(&imp.main_box));
    }

    fn setup_overview_button_clicked(&self) {
        let imp = self.imp();

        imp.overview_button.connect_clicked(clone!(
            #[weak(rename_to = tab_overview)]
            imp.tab_overview,
            move |_| {
                tab_overview.set_open(!tab_overview.is_open());
            }
        ));
    }

    fn setup_downloads_button_clicked(&self) {
        let imp = self.imp();

        imp.downloads_button.connect_clicked(clone!(
            #[weak(rename_to = downloads_popover)]
            imp.downloads_popover,
            move |_| {
                downloads_popover.popup();
            }
        ));
    }

    fn setup_find_button_clicked(&self) {
        let imp = self.imp();

        imp.find_button.connect_clicked(clone!(
            #[weak(rename_to = find_popover)]
            imp.find_popover,
            move |_| {
                find_popover.popup();
            }
        ));
    }

    fn setup_replicas_button_clicked(&self) {
        let imp = self.imp();

        imp.replicas_button.connect_clicked(clone!(move |_| {
            let _ = open::that(MOUNT_DIR.to_path_buf());
        }));
    }

    /// Create a new WebKit instance for the current tab
    ///
    /// # Arguments
    ///  
    /// * `ipfs` - An IPFS client
    fn new_view(
        &self,
        web_context: &WebContext,
        related_view: Option<&webkit2gtk::WebView>,
        initial_request: Option<&webkit2gtk::URIRequest>,
    ) -> webkit2gtk::WebView {
        let web_settings: webkit2gtk::Settings = new_webkit_settings();
        let web_view = if let Some(related_view) = related_view {
            WebView::builder()
                .settings(&web_settings)
                .related_view(related_view)
                .build()
        } else {
            WebView::builder()
                .web_context(web_context)
                .settings(&web_settings)
                .build()
        };
        web_view.set_vexpand(true);
        let network_session = web_view.network_session().unwrap();
        let data_manager = network_session.website_data_manager().unwrap();
        let security_manager = web_context.security_manager().unwrap();
        let extensions_path = format!("{}/web-extensions/", DATA_DIR.to_string_lossy());

        data_manager.set_favicons_enabled(true);

        security_manager.register_uri_scheme_as_secure("ipfs");
        security_manager.register_uri_scheme_as_secure("ipns");
        security_manager.register_uri_scheme_as_secure("tor");
        security_manager.register_uri_scheme_as_secure("hive");
        security_manager.register_uri_scheme_as_cors_enabled("ipfs");
        security_manager.register_uri_scheme_as_cors_enabled("ipns");
        security_manager.register_uri_scheme_as_cors_enabled("tor");
        security_manager.register_uri_scheme_as_cors_enabled("hive");

        web_settings.set_user_agent_with_application_details(Some("Oku"), Some(VERSION.unwrap()));
        web_settings.set_enable_write_console_messages_to_stdout(true);
        web_context.set_web_process_extensions_directory(&extensions_path);
        web_view.set_width_request(1024);
        web_view.set_height_request(640);
        if let Some(initial_request) = initial_request {
            web_view.load_request(initial_request)
        } else {
            web_view.load_uri("about:blank");
        }
        let rgba = gdk::RGBA::new(1.00, 1.00, 1.00, 0.00);
        web_view.set_background_color(&rgba);
        web_view.set_visible(true);

        web_view
    }

    /// Create a new entry in the TabBar
    ///
    /// # Arguments
    ///
    /// * `ipfs` - An IPFS client
    pub fn new_tab_page(
        &self,
        web_context: &WebContext,
        related_view: Option<&webkit2gtk::WebView>,
        initial_request: Option<&webkit2gtk::URIRequest>,
    ) -> (webkit2gtk::WebView, libadwaita::TabPage) {
        let imp = self.imp();

        let new_view = self.new_view(&web_context, related_view, initial_request);
        let new_page = imp.tab_view.append(&new_view);
        new_page.set_title("New Tab");
        new_page.set_icon(Some(&gio::ThemedIcon::new("content-loading-symbolic")));
        new_page.set_live_thumbnail(true);
        imp.tab_view.set_selected_page(&new_page);
        new_page.set_indicator_icon(Some(&gio::ThemedIcon::new("view-pin-symbolic")));
        new_page.set_indicator_activatable(true);
        imp.nav_entry.grab_focus();

        (new_view, new_page)
    }

    fn setup_tab_indicator(&self) {
        let imp = self.imp();

        // Indicator logic
        imp.tab_view.connect_indicator_activated(clone!(
            #[weak(rename_to = tab_view)]
            imp.tab_view,
            move |_, current_page| {
                let current_view = get_view_from_page(current_page);
                if !current_view.is_playing_audio() && !current_view.is_muted() {
                    tab_view.set_page_pinned(&current_page, !current_page.is_pinned());
                } else {
                    current_view.set_is_muted(!current_view.is_muted());
                }
            }
        ));
    }

    fn setup_add_tab_button_clicked(&self, web_context: &WebContext) {
        let imp = self.imp();

        // Add Tab button clicked
        imp.add_tab_button.connect_clicked(clone!(
            #[weak(rename_to = this)]
            self,
            #[weak]
            web_context,
            move |_| {
                this.new_tab_page(&web_context, None, None);
            }
        ));
    }

    pub fn setup_tab_signals(
        &self,
        web_context: &WebContext,
        style_manager: &libadwaita::StyleManager,
    ) {
        let imp = self.imp();

        imp.tab_overview.connect_create_tab(clone!(
            #[weak(rename_to = this)]
            self,
            #[weak]
            web_context,
            #[upgrade_or_panic]
            move |_| this.new_tab_page(&web_context, None, None).1
        ));

        // Selected tab changed
        imp.tab_view.connect_selected_page_notify(clone!(
            #[weak(rename_to = nav_entry)]
            imp.nav_entry,
            #[weak(rename_to = refresh_button)]
            imp.refresh_button,
            #[weak(rename_to = back_button)]
            imp.back_button,
            #[weak(rename_to = forward_button)]
            imp.forward_button,
            #[weak(rename_to = zoom_percentage)]
            imp.zoom_percentage,
            #[weak]
            style_manager,
            #[weak(rename_to = this)]
            self,
            move |_| {
                let web_view = this.get_view();
                update_nav_bar(&nav_entry, &web_view);
                this.set_title(Some(&get_title(&web_view)));
                this.update_load_progress(&web_view);
                if web_view.is_loading() {
                    refresh_button.set_icon_name("process-stop")
                } else {
                    refresh_button.set_icon_name("view-refresh")
                }
                back_button.set_sensitive(web_view.can_go_back());
                forward_button.set_sensitive(web_view.can_go_forward());
                this.update_domain_color(&web_view, &style_manager);
                zoom_percentage.set_text(&format!("{:.0}%", web_view.zoom_level() * 100.0))
            }
        ));
    }

    fn setup_navigation_signals(&self) {
        let imp = self.imp();

        // Back button clicked
        imp.back_button.connect_clicked(clone!(
            #[weak(rename_to = this)]
            self,
            move |_| {
                let web_view = this.get_view();
                web_view.go_back()
            }
        ));

        // Forward button clicked
        imp.forward_button.connect_clicked(clone!(
            #[weak(rename_to = this)]
            self,
            move |_| {
                let web_view = this.get_view();
                web_view.go_forward()
            }
        ));

        // Refresh button clicked
        imp.refresh_button.connect_clicked(clone!(
            #[weak(rename_to = this)]
            self,
            move |_| {
                let web_view = this.get_view();
                if web_view.is_loading() {
                    web_view.stop_loading()
                } else {
                    web_view.reload()
                }
            }
        ));

        // User hit return key in navbar, prompting navigation
        imp.nav_entry.connect_activate(clone!(
            #[weak(rename_to = nav_entry)]
            imp.nav_entry,
            #[weak(rename_to = this)]
            self,
            move |_| {
                let web_view = this.get_view();
                connect(&nav_entry, &web_view);
            }
        ));

        imp.nav_entry_focus.borrow().connect_enter(clone!(
            #[weak]
            imp,
            move |_| {
                imp.nav_entry.select_region(0, -1);
            }
        ));

        imp.nav_entry_focus.borrow().connect_leave(clone!(
            #[weak]
            imp,
            #[weak(rename_to = this)]
            self,
            move |_| {
                let suggestions_store = this.suggestions_store();
                suggestions_store.remove_all();
                if imp.suggestions_popover.is_visible() {
                    imp.suggestions_popover.popdown();
                }
            }
        ));

        imp.nav_entry.connect_search_changed(clone!(
            #[weak]
            imp,
            #[weak(rename_to = this)]
            self,
            move |_nav_entry| {
                if imp.nav_entry_focus.borrow().contains_focus() {
                    let favicon_database = this
                        .get_view()
                        .network_session()
                        .unwrap()
                        .website_data_manager()
                        .unwrap()
                        .favicon_database()
                        .unwrap();

                    let mut suggestion_items = Vec::new();
                    if let Ok(history_manager) = HISTORY_MANAGER.try_lock() {
                        suggestion_items = history_manager
                            .get_suggestions(&favicon_database, imp.nav_entry.text().to_string());
                        drop(history_manager);
                    }
                    let suggestions_store = this.suggestions_store();
                    suggestions_store.remove_all();
                    if imp.suggestions_popover.is_visible() {
                        imp.suggestions_popover.popdown();
                    }
                    if suggestion_items.len() > 0 {
                        for suggestion_item in suggestion_items.iter() {
                            suggestions_store.append(suggestion_item);
                        }
                        imp.suggestions_popover.popup();
                    }
                }
            }
        ));
    }

    pub fn suggestions_store(&self) -> Ref<gio::ListStore> {
        let suggestions_store = self.imp().suggestions_store.borrow();

        Ref::map(suggestions_store, |suggestions_store| {
            let suggestions_store = suggestions_store.as_deref().unwrap();
            suggestions_store
        })
    }

    pub fn setup_suggestions_popover(&self) {
        let imp = self.imp();

        let suggestions_store = gio::ListStore::new::<crate::suggestion_item::SuggestionItem>();
        imp.suggestions_store
            .replace(Some(Rc::new(suggestions_store)));

        imp.suggestions_model
            .set_model(Some(&self.suggestions_store().clone()));
        imp.suggestions_model.set_autoselect(false);
        imp.suggestions_model.connect_selected_item_notify(clone!(
            #[weak]
            imp,
            move |suggestions_model| {
                if let Some(item) = suggestions_model.selected_item() {
                    let suggestion_item = item.downcast_ref::<SuggestionItem>().unwrap();
                    let encoded_uri = suggestion_item.uri();
                    let decoded_uri = html_escape::decode_html_entities(&encoded_uri);
                    imp.nav_entry
                        .set_text(&uri_for_display(&decoded_uri).unwrap_or(decoded_uri.into()));
                }
            }
        ));

        imp.suggestions_factory
            .connect_setup(clone!(move |_, item| {
                let row = super::suggestion_row::SuggestionRow::new();
                let list_item = item.downcast_ref::<gtk::ListItem>().unwrap();
                list_item.set_child(Some(&row));
                list_item
                    .property_expression("item")
                    .chain_property::<crate::suggestion_item::SuggestionItem>("title")
                    .bind(&row, "title-property", gtk::Widget::NONE);
                list_item
                    .property_expression("item")
                    .chain_property::<crate::suggestion_item::SuggestionItem>("uri")
                    .bind(&row, "uri", gtk::Widget::NONE);
                list_item
                    .property_expression("item")
                    .chain_property::<crate::suggestion_item::SuggestionItem>("favicon")
                    .bind(&row, "favicon", gtk::Widget::NONE);
            }));

        imp.suggestions_view.set_model(Some(&imp.suggestions_model));
        imp.suggestions_view
            .set_factory(Some(&imp.suggestions_factory));
        imp.suggestions_view.set_enable_rubberband(false);
        imp.suggestions_view
            .set_hscroll_policy(gtk::ScrollablePolicy::Natural);
        imp.suggestions_view
            .set_vscroll_policy(gtk::ScrollablePolicy::Natural);
        imp.suggestions_view.set_overflow(gtk::Overflow::Visible);

        imp.suggestions_scrolled_window
            .set_child(Some(&imp.suggestions_view));
        imp.suggestions_scrolled_window
            .set_hscrollbar_policy(gtk::PolicyType::Never);
        imp.suggestions_scrolled_window.set_max_content_height(400);
        imp.suggestions_scrolled_window
            .set_propagate_natural_height(true);
        imp.suggestions_scrolled_window
            .set_propagate_natural_width(true);

        imp.suggestions_popover
            .set_child(Some(&imp.suggestions_scrolled_window));
        imp.suggestions_popover.set_parent(&imp.nav_entry);
        imp.suggestions_popover.add_css_class("menu");
        imp.suggestions_popover.add_css_class("suggestions");
        imp.suggestions_popover.set_has_arrow(false);
        imp.suggestions_popover.set_autohide(false);
        imp.suggestions_popover.set_can_focus(false);
    }

    pub fn setup_zoom_buttons_clicked(&self) {
        let imp = self.imp();

        // Zoom-in button clicked
        imp.zoomin_button.connect_clicked(clone!(
            #[weak(rename_to = this)]
            self,
            move |_| {
                let web_view = this.get_view();
                let current_zoom_level = web_view.zoom_level();
                web_view.set_zoom_level(current_zoom_level + 0.1);
            }
        ));

        // Zoom-out button clicked
        imp.zoomout_button.connect_clicked(clone!(
            #[weak(rename_to = this)]
            self,
            move |_| {
                let web_view = this.get_view();
                let current_zoom_level = web_view.zoom_level();
                web_view.set_zoom_level(current_zoom_level - 0.1);
            }
        ));

        // Reset Zoom button clicked
        imp.zoomreset_button.connect_clicked(clone!(
            #[weak(rename_to = this)]
            self,
            move |_| {
                let web_view = this.get_view();
                web_view.set_zoom_level(1.0);
            }
        ));
    }

    /// Create new browser window when a tab is dragged off
    ///
    /// # Arguments
    ///
    /// * `ipfs` - An IPFS client
    fn create_window_from_drag(
        &self,
        web_context: &WebContext,
    ) -> std::option::Option<libadwaita::TabView> {
        let application = self.application().unwrap().downcast().unwrap();
        let new_window = self::Window::new(&application, &web_context);
        Some(new_window.imp().tab_view.to_owned())
    }

    fn about_dialog(&self) {
        let about_dialog = libadwaita::AboutDialog::builder()
            .version(VERSION.unwrap())
            .application_name("Oku")
            .developer_name("Emil Sayahi")
            .application_icon("com.github.dirout.oku")
            .license_type(gtk::License::Agpl30)
            .build();
        about_dialog.present(Some(self));
    }

    pub fn setup_menu_buttons_clicked(&self, web_context: &WebContext) {
        self.setup_zoom_buttons_clicked();
        let imp = self.imp();

        imp.menu_button.connect_clicked(clone!(
            #[weak(rename_to = menu_popover)]
            imp.menu_popover,
            move |_| {
                menu_popover.popup();
            }
        ));

        // Enter Fullscreen button clicked
        imp.fullscreen_button.connect_clicked(clone!(
            #[weak(rename_to = this)]
            self,
            move |fullscreen_button| {
                let web_view = this.get_view();
                if !this.is_fullscreen() {
                    fullscreen_button.set_icon_name("view-restore");
                    web_view.evaluate_javascript(
                        "document.documentElement.requestFullscreen();",
                        None,
                        None,
                        Some(&gio::Cancellable::new()),
                        move |_| {},
                    )
                } else {
                    fullscreen_button.set_icon_name("view-fullscreen");
                    web_view.evaluate_javascript(
                        "document.exitFullscreen();",
                        None,
                        None,
                        Some(&gio::Cancellable::new()),
                        move |_| {},
                    )
                }
            }
        ));

        // Print button clicked
        imp.print_button.connect_clicked(clone!(
            #[weak(rename_to = this)]
            self,
            move |_print_button| {
                let web_view = this.get_view();
                web_view.evaluate_javascript(
                    "window.print();",
                    None,
                    None,
                    Some(&gio::Cancellable::new()),
                    move |_| {},
                )
            }
        ));

        // Screenshot button clicked
        imp.screenshot_button.connect_clicked(clone!(
            #[weak(rename_to = this)]
            self,
            move |_| {
                let web_view = this.get_view();
                let dialog = libadwaita::AlertDialog::new(
                    Some("Take a screenshot?"),
                    Some("Do you wish to save a screenshot of the current page?"),
                );
                dialog.add_responses(&[
                    ("cancel", "Cancel"),
                    ("visible", "Screenshot visible area"),
                    ("full", "Screenshot full document"),
                ]);
                dialog.set_response_appearance(
                    "cancel",
                    ResponseAppearance::Default,
                );
                dialog.set_response_appearance(
                    "visible",
                    ResponseAppearance::Suggested,
                );
                dialog.set_response_appearance(
                    "full",
                    ResponseAppearance::Suggested,
                );
                dialog.set_default_response(Some("cancel"));
                dialog.set_close_response("cancel");
                dialog.connect_response(
                    None,
                    clone!(
                        #[weak]
                        web_view,
                        #[weak]
                        this,
                        move |_, response| {
                            if response != "cancel" {
                                let snapshot_region = match response {
                                    "visible" => webkit2gtk::SnapshotRegion::Visible,
                                    "full" => webkit2gtk::SnapshotRegion::FullDocument,
                                    _ => unreachable!()
                                };
                                let file_dialog =
                                    gtk::FileDialog::builder()
                                        .accept_label("Save")
                                        .initial_name(format!("{}.png", Utc::now()))
                                        .initial_folder(&gio::File::for_path(glib::user_special_dir(glib::enums::UserDirectory::Pictures).unwrap()))
                                        .title("Select a destination to save the screenshot")
                                        .build();
                                file_dialog.save(
                                    Some(&this),
                                    Some(&gio::Cancellable::new()),
                                    clone!(
                                        #[strong]
                                        snapshot_region,
                                        move |destination| {
                                            match destination {
                                                Ok(destination) => {
                                                    match destination.path() {
                                                        Some(destination_path) => {
                                                            web_view.snapshot(
                                                                snapshot_region,
                                                                webkit2gtk::SnapshotOptions::all(),
                                                                Some(&gio::Cancellable::new()),
                                                                clone!(
                                                                    move |snapshot| {
                                                                        if let Ok(snapshot) = snapshot {
                                                                            match snapshot.save_to_png(destination_path.to_str().unwrap()) {
                                                                                Ok(_) => info!("Saved screenshot to {:?}", destination_path),
                                                                                Err(e) => error!("{}", e)
                                                                            }
                                                                        }
                                                                    },
                                                                )
                                                            );
                                                        },
                                                        None => warn!("No path for {:#?}", destination)
                                                    }
                                                },
                                                Err(e) => error!("{}", e)
                                            };
                                        }
                                    ),
                                );
                            }
                        }
                    ),
                );
                dialog.present(Some(&this));
            }
        ));

        // New Window button clicked
        imp.new_window_button.connect_clicked(clone!(
            #[weak(rename_to = this)]
            self,
            #[weak]
            web_context,
            move |_| {
                self::Window::new(
                    &this.application().unwrap().downcast().unwrap(),
                    &web_context,
                );
            }
        ));

        // Settings button clicked
        imp.settings_button.connect_clicked(clone!(
            #[weak(rename_to = this)]
            self,
            move |_| {
                super::settings::Settings::new(
                    &this.application().unwrap().downcast().unwrap(),
                    &this,
                );
            }
        ));

        // About button clicked
        imp.about_button.connect_clicked(clone!(
            #[weak(rename_to = this)]
            self,
            move |_| this.about_dialog()
        ));

        // Tab dragged off to create new browser window
        imp.tab_view.connect_create_window(clone!(
            #[weak(rename_to = this)]
            self,
            #[weak]
            web_context,
            #[upgrade_or]
            None,
            move |_tab_view| this.create_window_from_drag(&web_context)
        ));
    }

    fn setup_new_view_signals(
        &self,
        web_context: &WebContext,
        style_manager: &libadwaita::StyleManager,
    ) {
        let imp = self.imp();

        imp.tab_view.connect_page_attached(clone!(
            #[weak] web_context,
            #[weak] style_manager,
            #[weak(rename_to = this)]
            self,
            #[weak]
            imp,
            move |_, new_page, _page_position| {
                let new_view = get_view_from_page(&new_page);
                let network_session = new_view.network_session().unwrap();
                let find_controller = new_view.find_controller().unwrap();

                let decide_policy = RefCell::new(Some(new_view.connect_decide_policy(clone!(
                    move |w, policy_decision, decision_type| {
                        match decision_type {
                            PolicyDecisionType::NavigationAction => {
                                let navigation_policy_decision: NavigationPolicyDecision = policy_decision.clone().downcast().unwrap();
                                navigation_policy_decision.use_();
                                if let Some(mut navigation_action) = navigation_policy_decision.navigation_action() {
                                    let navigation_type = navigation_action.navigation_type();
                                    if let Some(request) = navigation_action.request() {
                                        if let Some(back_forward_list) = w.back_forward_list() {
                                            if let Some(current_item) = back_forward_list.current_item() {
                                                if let Some(old_uri) = current_item.original_uri() {
                                                    if let Some(new_uri) = request.uri() {
                                                        match navigation_type {
                                                            NavigationType::LinkClicked => {
                                                                if let Ok(history_manager) = HISTORY_MANAGER.try_lock() {
                                                                    history_manager.add_navigation(old_uri.to_string(), new_uri.to_string());
                                                                    let current_session = history_manager.get_current_session();
                                                                    current_session.update_uri(old_uri.to_string(), current_item.uri().map(|x| x.to_string()), Some(get_title(&w)));
                                                                    current_session.save();
                                                                    drop(current_session);
                                                                    drop(history_manager);
                                                                } else {
                                                                    warn!("Could not lock history manager while clicking link.");
                                                                }
                                                            },
                                                            _ => ()
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                true
                            },
                            _ => false
                        }
                    }
                ))));

                let found_text = RefCell::new(
                    Some(
                        find_controller.connect_found_text(
                            clone!(
                                #[weak]
                                    imp,
                                    move |_find_controller, match_count| {
                                        if match_count > 1 {
                                            imp.total_matches_label.set_text(&format!("{} matches", match_count));
                                        } else {
                                            imp.total_matches_label.set_text("");
                                        }
                                }
                            )
                        )
                    )
                );

                let failed_to_find_text = RefCell::new(
                    Some(
                        find_controller.connect_failed_to_find_text(
                            clone!(
                                #[weak]
                                    imp,
                                    move |_find_controller| {
                                        imp.total_matches_label.set_text("");
                                }
                            )
                        )
                    )
                );

                let create = RefCell::new(
                    Some(
                        new_view.connect_create(
                            clone!(
                                #[weak]
                                web_context,
                                #[weak]
                                this,
                                #[upgrade_or_panic] move |w, navigation_action| {
                                    let mut navigation_action = navigation_action.clone();
                                    let new_related_view = this.new_tab_page(&web_context, Some(w), navigation_action.request().as_ref()).0;
                                    if let Some(back_forward_list) = w.back_forward_list() {
                                        if let Some(current_item) = back_forward_list.current_item() {
                                            if let Some(old_uri) = current_item.original_uri() {
                                                if let Some(new_uri) = new_related_view.uri() {
                                                    if let Ok(history_manager) = HISTORY_MANAGER.try_lock() {
                                                        history_manager.add_navigation(old_uri.to_string(), new_uri.to_string());
                                                        let current_session = history_manager.get_current_session();
                                                        current_session.update_uri(
                                                            old_uri.to_string(),
                                                            current_item.uri().map(|x| x.to_string()),
                                                            Some(get_title(&new_related_view)),
                                                        );
                                                        current_session.save();
                                                        drop(current_session);
                                                        drop(history_manager);
                                                    } else {
                                                        warn!("Could not lock history manager during new tab creation.");
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    new_related_view.into()
                                }
                            )
                        )
                    )
                );

                let status_message = RefCell::new(
                    Some(
                        new_view.connect_mouse_target_changed(
                            clone!(
                                #[weak]
                                imp,
                                move |_w, hit_test_result, _modifier| {
                                    if let Some(link_uri) = hit_test_result.link_uri() {
                                        imp.url_status.set_text(uri_for_display(link_uri.as_str()).unwrap_or_default().as_str());
                                        imp.url_status_box.set_visible(true);
                                    } else {
                                        imp.url_status.set_text("");
                                        imp.url_status_box.set_visible(false);
                                    }
                                }
                            )
                        )
                    )
                );

                let permission_request =
                    RefCell::new(Some(new_view.connect_permission_request(clone!(
                        #[weak]
                        this,
                        #[upgrade_or]
                        false,
                        move |_w, permission_request| {
                            let (title, description) = if permission_request.is::<webkit2gtk::ClipboardPermissionRequest>() {
                                ("Allow access to clipboard?", "This page is requesting permission to read the contents of your clipboard.")
                            } else if permission_request.is::<webkit2gtk::DeviceInfoPermissionRequest>() {
                                ("Allow access to audio & video devices?", "This page is requesting access to information regarding your audio & video devices.")
                            } else if permission_request.is::<webkit2gtk::GeolocationPermissionRequest>() {
                                ("Allow access to location?", "This page is requesting access to your location.")
                            } else if permission_request.is::<webkit2gtk::MediaKeySystemPermissionRequest>() {
                                ("Allow playback of encrypted media?", "This page wishes to play encrypted media.")
                            } else if permission_request.is::<webkit2gtk::NotificationPermissionRequest>() {
                                ("Allow notifications?", "This page is requesting permission to display notifications.")
                            } else if permission_request.is::<webkit2gtk::PointerLockPermissionRequest>() {
                                ("Allow locking the pointer?", "This page is requesting permission to lock your pointer.")
                            } else if permission_request.is::<webkit2gtk::UserMediaPermissionRequest>() {
                                let user_media_permission_request = permission_request.downcast_ref::<webkit2gtk::UserMediaPermissionRequest>().unwrap();
                                if user_media_permission_is_for_audio_device(&user_media_permission_request) {
                                    ("Allow access to audio devices?", "This page is requesting access to your audio source devices.")
                                } else if user_media_permission_is_for_display_device(&user_media_permission_request) {
                                    ("Allow access to display devices?", "This page is requesting access to your display devices.")
                                } else if user_media_permission_is_for_video_device(&user_media_permission_request) {
                                    ("Allow access to video devices?", "This page is requesting access to your video source devices.")
                                } else {
                                    ("Allow access to media devices?", "This page is requesting access to your media devices.")
                                }
                            } else if permission_request.is::<webkit2gtk::WebsiteDataAccessPermissionRequest>() {
                                ("Allow access to third-party cookies?", "This page is requesing permission to read your data from third-party domains.")
                            } else {
                                ("", "")
                            };
                            let dialog = libadwaita::AlertDialog::new(
                                Some(title),
                                Some(description),
                            );
                            dialog.add_responses(&[
                                ("deny", "Deny"),
                                ("allow", "Allow"),
                            ]);
                            dialog.set_response_appearance(
                                "deny",
                                ResponseAppearance::Default,
                            );
                            dialog.set_response_appearance(
                                "allow",
                                ResponseAppearance::Destructive,
                            );
                            dialog.set_default_response(Some("deny"));
                            dialog.set_close_response("deny");
                            dialog.connect_response(
                                None,
                                clone!(
                                    #[strong]
                                    permission_request,
                                    move |_, response| {
                                        match response {
                                            "deny" => permission_request.deny(),
                                            "allow" => {
                                                permission_request.allow()
                                            }
                                            _ => {
                                                unreachable!()
                                            }
                                        }
                                    }
                                ),
                            );
                            dialog.present(Some(&this));
                            true
                        }
                    ))));

                let download_started =
                    RefCell::new(Some(network_session.connect_download_started(clone!(
                        #[weak]
                        this,
                        move |_w, download| {
                            download.connect_decide_destination(clone!(
                                #[weak]
                                this,
                                #[weak]
                                download,
                                #[upgrade_or]
                                false,
                                move |_, suggested_filename| {
                                    let file_uri = download.request().unwrap().uri().unwrap();
                                    let dialog = libadwaita::AlertDialog::new(
                                        Some("Download file?"),
                                        Some(&format!(
                                            "Would you like to download '{}'?",
                                            file_uri
                                        )),
                                    );
                                    dialog.add_responses(&[
                                        ("cancel", "Cancel"),
                                        ("download", "Download"),
                                    ]);
                                    dialog.set_response_appearance(
                                        "cancel",
                                        ResponseAppearance::Default,
                                    );
                                    dialog.set_response_appearance(
                                        "download",
                                        ResponseAppearance::Suggested,
                                    );
                                    dialog.set_default_response(Some("cancel"));
                                    dialog.set_close_response("cancel");
                                    let suggested_filename = suggested_filename.to_string();
                                    dialog.connect_response(
                                        None,
                                        clone!(
                                            #[weak]
                                            this,
                                            #[weak]
                                            download,
                                            move |_, response| {
                                                match response {
                                                    "cancel" => download.cancel(),
                                                    "download" => {
                                                        download.set_allow_overwrite(true);
                                                        let file_dialog =
                                                            gtk::FileDialog::builder()
                                                                .accept_label("Download")
                                                                .initial_name(suggested_filename.clone())
                                                                .initial_folder(&gio::File::for_path(glib::user_special_dir(glib::enums::UserDirectory::Downloads).unwrap()))
                                                                .title(&format!(
                                                                    "Select destination for '{}'",
                                                                    suggested_filename.clone()
                                                                ))
                                                                .build();
                                                        file_dialog.save(
                                                            Some(&this),
                                                            Some(&gio::Cancellable::new()),
                                                            clone!(
                                                                #[weak]
                                                                download,
                                                                move |destination| {
                                                                        if let Ok(destination) = destination {
                                                                            download.set_destination(destination.path()
                                                                            .unwrap()
                                                                            .to_str()
                                                                            .unwrap())
                                                                        } else {
                                                                            download.cancel()
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
                                    dialog.present(Some(&this));
                                    true
                                }
                            ));
                        }
                    ))));
                let title_notify = RefCell::new(Some(new_view.connect_title_notify(clone!(
                    #[weak(rename_to = tab_view)]
                    imp.tab_view,
                    move |w| update_title(tab_view, &w)
                ))));
                let favicon_notify = RefCell::new(Some(new_view.connect_favicon_notify(clone!(
                    #[weak(rename_to = tab_view)]
                    imp.tab_view,
                    move |w| update_favicon(tab_view, &w)
                ))));
                let load_changed = RefCell::new(Some(new_view.connect_load_changed(clone!(
                    #[weak(rename_to = tab_view)]
                    imp.tab_view,
                    #[weak(rename_to = back_button)]
                    imp.back_button,
                    #[weak(rename_to = forward_button)]
                    imp.forward_button,
                    #[weak]
                    imp,
                    #[weak]
                    this,
                    move |w, load_event| {
                        let title = get_title(&w);
                        imp.obj().set_title(Some(
                            &title
                        ));
                        update_favicon(tab_view, &w);
                        if this.get_view() == *w {
                            back_button.set_sensitive(w.can_go_back());
                            forward_button.set_sensitive(w.can_go_forward());
                            match load_event {
                                LoadEvent::Redirected => {
                                    if let Some(back_forward_list) = w.back_forward_list() {
                                        if let Some(current_item) = back_forward_list.current_item() {
                                            if let Some(original_uri) = current_item.original_uri() {
                                                if let Ok(history_manager) = HISTORY_MANAGER.try_lock() {
                                                    let current_session = history_manager.get_current_session();
                                                    current_session.update_uri(original_uri.to_string(), current_item.uri().map(|x| x.to_string()), Some(title));
                                                    current_session.save();
                                                    drop(current_session);
                                                    drop(history_manager);
                                                } else {
                                                    warn!("Could not lock history manager during redirection.");
                                                }
                                            }
                                        }
                                    }
                                }
                                _ => ()
                            }
                        }
                    }
                ))));
                let enter_fullscreen =
                    RefCell::new(Some(new_view.connect_enter_fullscreen(clone!(
                        #[weak]
                        imp,
                        #[upgrade_or]
                        false,
                        move |_w| {
                            imp.obj().set_fullscreened(true);
                            imp.tab_bar_revealer.set_reveal_child(false);
                            true
                        }
                    ))));
                let leave_fullscreen =
                    RefCell::new(Some(new_view.connect_leave_fullscreen(clone!(
                        #[weak]
                        imp,
                        #[upgrade_or]
                        false,
                        move |_w| {
                            imp.obj().set_fullscreened(false);
                            imp.tab_bar_revealer.set_reveal_child(true);
                            true
                        }
                    ))));

                // Indicator appearance
                let is_muted_notify = RefCell::new(Some(new_view.connect_is_muted_notify(clone!(
                    #[weak]
                    new_page,
                    move |the_view| {
                        // Has been muted
                        if the_view.is_muted() {
                            new_page.set_indicator_icon(Some(&gio::ThemedIcon::new(
                                "audio-volume-muted",
                            )));
                            new_page.set_indicator_activatable(true);
                        } else {
                            // Has been unmuted, audio is playing
                            if the_view.is_playing_audio() {
                                new_page.set_indicator_icon(Some(&gio::ThemedIcon::new(
                                    "audio-volume-high",
                                )));
                                new_page.set_indicator_activatable(true);
                            }
                            // Has been unmuted, audio is not playing
                            else {
                                new_page.set_indicator_icon(Some(&gio::ThemedIcon::new(
                                    "view-pin-symbolic",
                                )));
                                new_page.set_indicator_activatable(true);
                            }
                        }
                    }
                ))));
                let is_playing_audio_notify =
                    RefCell::new(Some(new_view.connect_is_playing_audio_notify(clone!(
                        #[weak]
                        new_page,
                        move |the_view| {
                            // Audio has started playing and not muted
                            if the_view.is_playing_audio() && !the_view.is_muted() {
                                new_page.set_indicator_icon(Some(&gio::ThemedIcon::new(
                                    "audio-volume-high",
                                )));
                                new_page.set_indicator_activatable(true);
                            } else if !the_view.is_playing_audio() {
                                // Audio has stopped playing, muted
                                if the_view.is_muted() {
                                    new_page.set_indicator_icon(Some(&gio::ThemedIcon::new(
                                        "audio-volume-muted",
                                    )));
                                    new_page.set_indicator_activatable(true);
                                } else {
                                    // Audio has stopped playing, not muted
                                    new_page.set_indicator_icon(Some(&gio::ThemedIcon::new(
                                        "view-pin-symbolic",
                                    )));
                                    new_page.set_indicator_activatable(true);
                                }
                            }
                        }
                    ))));
                let zoom_level_notify = RefCell::new(Some(new_view.connect_zoom_level_notify(clone!(
                    #[weak(rename_to = zoom_percentage)]
                    imp.zoom_percentage,
                    #[weak]
                    this,
                    move |w| {
                        if this.get_view() == *w {
                            zoom_percentage.set_text(&format!("{:.0}%", w.zoom_level() * 100.0))
                        }
                    }
                ))));
                let uri_notify = RefCell::new(Some(new_view.connect_uri_notify(clone!(
                    #[weak(rename_to = nav_entry)]
                    imp.nav_entry,
                    #[weak(rename_to = back_button)]
                    imp.back_button,
                    #[weak(rename_to = forward_button)]
                    imp.forward_button,
                    #[weak]
                    style_manager,
                    #[weak]
                    this,
                    move |w| {
                        if this.get_view() == *w {
                            update_nav_bar(&nav_entry, &w);
                            back_button.set_sensitive(w.can_go_back());
                            forward_button.set_sensitive(w.can_go_forward());
                            this.update_domain_color(&w, &style_manager);
                        }
                    }
                ))));
                let estimated_load_progress_notify = RefCell::new(Some(
                    new_view.connect_estimated_load_progress_notify(clone!(
                        #[weak(rename_to = tab_view)]
                        imp.tab_view,
                        #[weak(rename_to = refresh_button)]
                        imp.refresh_button,
                        #[weak]
                        this,
                        move |w| {
                            let current_page = tab_view.page(w);
                            current_page.set_loading(w.is_loading());
                            if this.get_view() == *w {
                                this.update_load_progress(&w);
                                if current_page.is_loading() {
                                    refresh_button.set_icon_name("process-stop")
                                } else {
                                    refresh_button.set_icon_name("view-refresh")
                                }
                            }
                        }
                    )),
                ));
                let is_loading_notify =
                    RefCell::new(Some(new_view.connect_is_loading_notify(clone!(
                        #[weak(rename_to = tab_view)]
                        imp.tab_view,
                        #[weak(rename_to = refresh_button)]
                        imp.refresh_button,
                        #[weak(rename_to = progress_bar)]
                        imp.progress_bar,
                        #[weak]
                        this,
                        move |w| {
                            let current_page = tab_view.page(w);
                            current_page.set_loading(w.is_loading());
                            if this.get_view() == *w {
                                if current_page.is_loading() {
                                    refresh_button.set_icon_name("process-stop");
                                    progress_bar.pulse()
                                } else {
                                    refresh_button.set_icon_name("view-refresh")
                                }
                            }
                        }
                    ))));
                imp.tab_view.connect_page_detached(clone!(
                    move |_, old_page, _page_position| {
                        let old_view = get_view_from_page(&old_page);
                        let old_network_session = old_view.network_session().unwrap();
                        let old_find_controller = old_view.find_controller().unwrap();
                        if let Some(id) = decide_policy.take() {
                            old_view.disconnect(id);
                        }
                        if let Some(id) = zoom_level_notify.take() {
                            old_view.disconnect(id);
                        }
                        if let Some(id) = failed_to_find_text.take() {
                            old_find_controller.disconnect(id);
                        }
                        if let Some(id) = found_text.take() {
                            old_find_controller.disconnect(id);
                        }
                        if let Some(id) = create.take() {
                            old_view.disconnect(id);
                        }
                        if let Some(id) = status_message.take() {
                            old_view.disconnect(id);
                        }
                        if let Some(id) = permission_request.take() {
                            old_view.disconnect(id);
                        }
                        if let Some(id) = download_started.take() {
                            old_network_session.disconnect(id);
                        }
                        if let Some(id) = title_notify.take() {
                            old_view.disconnect(id);
                        }
                        if let Some(id) = favicon_notify.take() {
                            old_view.disconnect(id);
                        }
                        if let Some(id) = load_changed.take() {
                            old_view.disconnect(id);
                        }
                        if let Some(id) = enter_fullscreen.take() {
                            old_view.disconnect(id);
                        }
                        if let Some(id) = leave_fullscreen.take() {
                            old_view.disconnect(id);
                        }
                        if let Some(id) = is_muted_notify.take() {
                            old_view.disconnect(id);
                        }
                        if let Some(id) = is_playing_audio_notify.take() {
                            old_view.disconnect(id);
                        }
                        if let Some(id) = uri_notify.take() {
                            old_view.disconnect(id);
                        }
                        if let Some(id) = estimated_load_progress_notify.take() {
                            old_view.disconnect(id);
                        }
                        if let Some(id) = is_loading_notify.take() {
                            old_view.disconnect(id);
                        }
                    }
                ));
            }
        ));
    }

    /// Adapted from Geopard (https://github.com/ranfdev/Geopard)
    pub fn update_domain_color(
        &self,
        web_view: &webkit2gtk::WebView,
        style_manager: &libadwaita::StyleManager,
    ) {
        let imp = self.imp();

        let mut failed_attempts = 0;
        loop {
            match CONFIG.try_lock() {
                Ok(config) => {
                    if !config.colour_per_domain() {
                        imp.style_provider.borrow().load_from_string("");
                        let rgba = gdk::RGBA::new(1.00, 1.00, 1.00, 0.00);
                        web_view.set_background_color(&rgba);
                        return;
                    } else {
                        break;
                    }
                }
                Err(e) => {
                    failed_attempts += 1;
                    error!("{}", e);
                    if failed_attempts == 10 {
                        return;
                    }
                }
            }
        }

        let url = web_view.uri().unwrap_or("about:blank".into());
        let parsed_url = url::Url::parse(&url);
        let domain = parsed_url
            .as_ref()
            .map(|u| u.domain())
            .ok()
            .flatten()
            .unwrap_or(&url)
            .to_string();

        let hash = {
            let mut s = std::collections::hash_map::DefaultHasher::new();
            domain.hash(&mut s);
            s.finish()
        };

        let hue = hash % 360;
        let stylesheet = if style_manager.is_dark() {
            format!(
                "
                    @define-color view_bg_color hsl({hue}, 20%, 8%);
                    @define-color view_fg_color hsl({hue}, 100%, 98%);
                    @define-color window_bg_color hsl({hue}, 20%, 8%);
                    @define-color window_fg_color hsl({hue}, 100%, 98%);
                    @define-color headerbar_bg_color hsl({hue}, 80%, 10%);
                    @define-color headerbar_fg_color hsl({hue}, 100%, 98%);
                "
            )
        } else {
            format!(
                "
                    @define-color view_bg_color hsl({hue}, 100%, 99%);
                    @define-color view_fg_color hsl({hue}, 100%, 12%);
                    @define-color window_bg_color hsl({hue}, 100%, 99%);
                    @define-color window_fg_color hsl({hue}, 100%, 12%);
                    @define-color headerbar_bg_color hsl({hue}, 100%, 96%);
                    @define-color headerbar_fg_color hsl({hue}, 100%, 12%);
                "
            )
        };

        imp.style_provider.borrow().load_from_string(&stylesheet);
        let rgba = {
            let hsl = if style_manager.is_dark() {
                coolor::Hsl::new(hue as f32, 0.20, 0.08)
            } else {
                coolor::Hsl::new(hue as f32, 1.00, 0.99)
            };
            let rgb = hsl.to_rgb();
            gdk::RGBA::new(rgb.r.into(), rgb.g.into(), rgb.b.into(), 0.00)
        };
        web_view.set_background_color(&rgba);
    }
}
