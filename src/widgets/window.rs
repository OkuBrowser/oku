use crate::window_util::{
    connect, get_title, get_view_from_page, initial_connect, new_webkit_settings, update_favicon,
    update_load_progress, update_nav_bar, update_title,
};
use crate::{DATA_DIR, PICTURES_DIR, VERSION};
use chrono::Utc;
use glib::{clone, Properties};
use gtk::subclass::prelude::*;
use gtk::{gio, glib};
use libadwaita::subclass::application_window::AdwApplicationWindowImpl;
use libadwaita::{prelude::*, ResponseAppearance};
use std::cell::Cell;
use std::cell::RefCell;
use webkit2gtk::functions::{
    user_media_permission_is_for_audio_device, user_media_permission_is_for_display_device,
    user_media_permission_is_for_video_device,
};
use webkit2gtk::prelude::PermissionRequestExt;
use webkit2gtk::prelude::WebViewExt;
use webkit2gtk::{WebContext, WebView};

pub mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::Window)]
    pub struct Window {
        // Window parameters
        pub(crate) _is_private: Cell<bool>,
        pub(crate) initial_url: RefCell<String>,
        // Left header buttons
        pub(crate) nav_entry: gtk::Entry,
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
        pub(crate) tor_button: gtk::Button,
        pub(crate) menu_button: gtk::Button,
        pub(crate) right_header_buttons: gtk::Box,
        // HeaderBar
        pub(crate) headerbar: libadwaita::HeaderBar,
        // Menu popover
        pub(crate) zoomout_button: gtk::Button,
        pub(crate) zoomin_button: gtk::Button,
        pub(crate) zoom_buttons: gtk::Box,
        pub(crate) zoomreset_button: gtk::Button,
        pub(crate) fullscreen_button: gtk::Button,
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
        // Tabs
        pub(crate) tab_bar: libadwaita::TabBar,
        pub(crate) tab_view: libadwaita::TabView,
        // Main content
        pub(crate) main_box: gtk::Box,
        pub(crate) tab_overview: libadwaita::TabOverview,
    }

    impl Window {}

    #[glib::object_subclass]
    impl ObjectSubclass for Window {
        const NAME: &'static str = "OkuWindow";
        type Type = super::Window;
        type ParentType = libadwaita::ApplicationWindow;
    }

    impl ObjectImpl for Window {
        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            Self::derived_set_property(self, id, value, pspec)
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            Self::derived_property(self, id, pspec)
        }
    }
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
    pub fn new(
        app: &libadwaita::Application,
        web_context: &WebContext,
        initial_url: Option<&str>,
    ) -> Self {
        let this: Self = glib::Object::builder::<Self>()
            .property("application", app)
            .build();
        this.set_can_focus(true);
        this.set_title(Some("Oku"));
        this.set_icon_name(Some("com.github.dirout.oku"));

        this.setup_headerbar();
        this.setup_menu_popover();
        this.setup_downloads_popover();
        this.setup_tabs();
        this.setup_main_content();
        this.setup_overview_button_clicked();
        this.setup_downloads_button_clicked();
        this.setup_tab_indicator();
        this.setup_add_tab_button_clicked(&web_context);
        this.setup_tab_signals(&web_context);
        this.setup_navigation_signals();
        this.setup_menu_buttons_clicked(&web_context);
        this.setup_new_view_signals();

        let imp = this.imp();

        imp.initial_url
            .replace(initial_url.unwrap_or("about:blank").to_string());

        if imp.tab_view.n_pages() == 0 && app.windows().len() <= 1 {
            let initial_web_view = this.new_tab_page(&web_context).0;
            initial_connect(imp.initial_url.clone().into_inner(), &initial_web_view);
        }
        this.set_content(Some(&imp.tab_overview));
        this.set_visible(true);

        this
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

    fn setup_navigation_buttons(&self) {
        let imp = self.imp();

        // Navigation bar
        imp.nav_entry.set_can_focus(true);
        imp.nav_entry.set_focusable(true);
        imp.nav_entry.set_focus_on_click(true);
        imp.nav_entry.set_editable(true);
        imp.nav_entry.set_margin_top(4);
        imp.nav_entry.set_margin_bottom(4);
        imp.nav_entry.set_hexpand(true);
        imp.nav_entry.set_truncate_multiline(true);
        imp.nav_entry
            .set_placeholder_text(Some("Enter an address … "));
        imp.nav_entry.set_input_purpose(gtk::InputPurpose::Url);
        imp.nav_entry.set_width_request(8);

        // Back button
        imp.back_button.set_can_focus(true);
        imp.back_button.set_receives_default(true);
        imp.back_button.set_halign(gtk::Align::Start);
        imp.back_button.set_margin_top(4);
        imp.back_button.set_margin_bottom(4);
        imp.back_button.set_icon_name("go-previous");
        imp.back_button.add_css_class("linked");

        // Forward button
        imp.forward_button.set_can_focus(true);
        imp.forward_button.set_receives_default(true);
        imp.forward_button.set_halign(gtk::Align::Start);
        imp.forward_button.set_margin_top(4);
        imp.forward_button.set_margin_bottom(4);
        imp.forward_button.set_icon_name("go-next");
        imp.forward_button.add_css_class("linked");

        // All navigation buttons
        imp.navigation_buttons.set_homogeneous(true);
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
        imp.add_tab_button.set_margin_start(4);
        imp.add_tab_button.set_margin_top(4);
        imp.add_tab_button.set_margin_bottom(4);
        imp.add_tab_button.set_icon_name("list-add");

        // Refresh button
        imp.refresh_button.set_can_focus(true);
        imp.refresh_button.set_receives_default(true);
        imp.refresh_button.set_halign(gtk::Align::Start);
        imp.refresh_button.set_margin_start(4);
        imp.refresh_button.set_margin_end(8);
        imp.refresh_button.set_margin_top(4);
        imp.refresh_button.set_margin_bottom(4);
        imp.refresh_button.set_icon_name("view-refresh");

        // Left header buttons
        imp.left_header_buttons.set_margin_end(4);
        imp.left_header_buttons.append(&imp.navigation_buttons);
        imp.left_header_buttons.append(&imp.add_tab_button);
        imp.left_header_buttons.append(&imp.refresh_button);
    }

    fn setup_right_headerbar(&self) {
        let imp = self.imp();

        // Overview button
        imp.overview_button.set_can_focus(true);
        imp.overview_button.set_receives_default(true);
        imp.overview_button.set_halign(gtk::Align::Start);
        imp.overview_button.set_margin_start(4);
        imp.overview_button.set_margin_bottom(4);
        imp.overview_button.set_view(Some(&imp.tab_view));

        // Downloads button
        imp.downloads_button.set_can_focus(true);
        imp.downloads_button.set_receives_default(true);
        imp.downloads_button.set_halign(gtk::Align::Start);
        imp.downloads_button.set_margin_start(4);
        imp.downloads_button.set_margin_bottom(4);
        imp.downloads_button
            .set_icon_name("folder-download-symbolic");

        // Find button
        imp.find_button.set_can_focus(true);
        imp.find_button.set_receives_default(true);
        imp.find_button.set_halign(gtk::Align::Start);
        imp.find_button.set_margin_start(4);
        imp.find_button.set_margin_bottom(4);
        imp.find_button.set_icon_name("edit-find");

        // Replica menu button
        imp.replicas_button.set_can_focus(true);
        imp.replicas_button.set_receives_default(true);
        imp.replicas_button.set_halign(gtk::Align::Start);
        imp.replicas_button.set_margin_start(4);
        imp.replicas_button.set_margin_bottom(4);
        imp.replicas_button.set_icon_name("emblem-shared");

        // Onion routing button
        imp.tor_button.set_can_focus(true);
        imp.tor_button.set_receives_default(true);
        imp.tor_button.set_halign(gtk::Align::Start);
        imp.tor_button.set_hexpand(false);
        imp.tor_button.set_vexpand(false);
        imp.tor_button.set_overflow(gtk::Overflow::Hidden);
        imp.tor_button.set_margin_start(4);
        imp.tor_button.set_margin_bottom(4);
        imp.tor_button.set_icon_name("security-medium");

        // Menu button
        imp.menu_button.set_can_focus(true);
        imp.menu_button.set_receives_default(true);
        imp.menu_button.set_halign(gtk::Align::Start);
        imp.menu_button.set_margin_start(4);
        imp.menu_button.set_margin_bottom(4);
        imp.menu_button.set_icon_name("document-properties");

        // Right header buttons
        imp.right_header_buttons.set_margin_start(4);
        imp.right_header_buttons.set_spacing(2);
        imp.right_header_buttons.set_homogeneous(true);

        imp.right_header_buttons.append(&imp.overview_button);
        imp.right_header_buttons.append(&imp.downloads_button);
        imp.right_header_buttons.append(&imp.find_button);
        imp.right_header_buttons.append(&imp.replicas_button);
        imp.right_header_buttons.append(&imp.tor_button);
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
        imp.zoomout_button.set_halign(gtk::Align::Start);
        imp.zoomout_button.set_margin_top(4);
        imp.zoomout_button.set_margin_bottom(4);
        imp.zoomout_button.set_icon_name("zoom-out");
        imp.zoomout_button.add_css_class("linked");

        // Zoom in button
        imp.zoomin_button.set_can_focus(true);
        imp.zoomin_button.set_receives_default(true);
        imp.zoomin_button.set_halign(gtk::Align::Start);
        imp.zoomin_button.set_margin_top(4);
        imp.zoomin_button.set_margin_bottom(4);
        imp.zoomin_button.set_icon_name("zoom-in");
        imp.zoomin_button.add_css_class("linked");

        // Both zoom buttons
        imp.zoom_buttons.set_homogeneous(true);
        imp.zoom_buttons.append(&imp.zoomout_button);
        imp.zoom_buttons.append(&imp.zoomin_button);
        imp.zoom_buttons.add_css_class("linked");

        // Zoom reset button
        imp.zoomreset_button.set_can_focus(true);
        imp.zoomreset_button.set_receives_default(true);
        imp.zoomreset_button.set_halign(gtk::Align::Start);
        imp.zoomreset_button.set_margin_top(4);
        imp.zoomreset_button.set_margin_bottom(4);
        imp.zoomreset_button.set_icon_name("zoom-original");

        // Fullscreen button
        imp.fullscreen_button.set_can_focus(true);
        imp.fullscreen_button.set_receives_default(true);
        imp.fullscreen_button.set_halign(gtk::Align::Start);
        imp.fullscreen_button.set_margin_top(4);
        imp.fullscreen_button.set_margin_bottom(4);
        imp.fullscreen_button
            .set_icon_name("video-display-symbolic");

        // Screenshot button
        imp.screenshot_button.set_can_focus(true);
        imp.screenshot_button.set_receives_default(true);
        imp.screenshot_button.set_halign(gtk::Align::Start);
        imp.screenshot_button.set_margin_top(4);
        imp.screenshot_button.set_margin_bottom(4);
        imp.screenshot_button.set_icon_name("camera-photo");

        // New Window button
        imp.new_window_button.set_can_focus(true);
        imp.new_window_button.set_receives_default(true);
        imp.new_window_button.set_halign(gtk::Align::Start);
        imp.new_window_button.set_margin_top(4);
        imp.new_window_button.set_margin_bottom(4);
        imp.new_window_button.set_icon_name("window-new");

        // History button
        imp.history_button.set_can_focus(true);
        imp.history_button.set_receives_default(true);
        imp.history_button.set_halign(gtk::Align::Start);
        imp.history_button.set_margin_top(4);
        imp.history_button.set_margin_bottom(4);
        imp.history_button.set_icon_name("document-open-recent");

        // Settings button
        imp.settings_button.set_can_focus(true);
        imp.settings_button.set_receives_default(true);
        imp.settings_button.set_halign(gtk::Align::Start);
        imp.settings_button.set_margin_top(4);
        imp.settings_button.set_margin_bottom(4);
        imp.settings_button.set_icon_name("preferences-system");

        // About button
        imp.about_button.set_can_focus(true);
        imp.about_button.set_receives_default(true);
        imp.about_button.set_halign(gtk::Align::Start);
        imp.about_button.set_margin_top(4);
        imp.about_button.set_margin_bottom(4);
        imp.about_button.set_icon_name("help-about");

        // Menu popover
        imp.menu_box.set_margin_start(4);
        imp.menu_box.set_margin_end(4);
        imp.menu_box.set_margin_top(4);
        imp.menu_box.set_margin_bottom(4);
        imp.menu_box.set_spacing(8);
        imp.menu_box.set_hexpand(true);
        imp.menu_box.append(&imp.zoom_buttons);
        imp.menu_box.append(&imp.zoomreset_button);
        imp.menu_box.append(&imp.fullscreen_button);
        imp.menu_box.append(&imp.screenshot_button);
        imp.menu_box.append(&imp.new_window_button);
        imp.menu_box.append(&imp.history_button);
        imp.menu_box.append(&imp.settings_button);
        imp.menu_box.append(&imp.about_button);
        imp.menu_box.add_css_class("toolbar");

        imp.menu_popover.set_child(Some(&imp.menu_box));
        imp.menu_popover.set_parent(&imp.menu_button);
    }

    fn setup_downloads_popover(&self) {
        let imp = self.imp();

        // Downloads popover
        imp.downloads_box.set_margin_start(4);
        imp.downloads_box.set_margin_end(4);
        imp.downloads_box.set_margin_top(4);
        imp.downloads_box.set_margin_bottom(4);
        imp.downloads_box.set_spacing(8);

        imp.downloads_popover.set_child(Some(&imp.downloads_box));
        imp.downloads_popover.set_parent(&imp.downloads_button);
    }

    fn setup_tabs(&self) {
        let imp = self.imp();

        imp.tab_view.set_vexpand(true);
        imp.tab_view.set_visible(true);

        imp.tab_bar.set_autohide(true);
        imp.tab_bar.set_expand_tabs(true);
        imp.tab_bar.set_view(Some(&imp.tab_view));
    }

    fn setup_main_content(&self) {
        let imp = self.imp();

        imp.main_box.set_orientation(gtk::Orientation::Vertical);
        imp.main_box.set_vexpand(true);
        imp.main_box.append(&imp.headerbar);
        imp.main_box.append(&imp.tab_bar);
        imp.main_box.append(&imp.tab_view);

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

    /// Create a new WebKit instance for the current tab
    ///
    /// # Arguments
    ///  
    /// * `ipfs` - An IPFS client
    fn new_view(&self, web_context: &WebContext) -> webkit2gtk::WebView {
        let web_settings: webkit2gtk::Settings = new_webkit_settings();
        let web_view = WebView::builder()
            .web_context(web_context)
            .settings(&web_settings)
            .build();
        web_view.set_vexpand(true);
        let network_session = web_view.network_session().unwrap();
        let data_manager = network_session.website_data_manager().unwrap();
        let security_manager = web_context.security_manager().unwrap();
        let extensions_path = format!("{}/web-extensions/", *DATA_DIR);

        data_manager.set_favicons_enabled(true);

        security_manager.register_uri_scheme_as_secure("ipfs");
        security_manager.register_uri_scheme_as_secure("ipns");
        security_manager.register_uri_scheme_as_secure("tor");
        security_manager.register_uri_scheme_as_secure("test");
        security_manager.register_uri_scheme_as_cors_enabled("ipfs");
        security_manager.register_uri_scheme_as_cors_enabled("ipns");
        security_manager.register_uri_scheme_as_cors_enabled("tor");
        security_manager.register_uri_scheme_as_cors_enabled("test");

        web_settings.set_user_agent_with_application_details(Some("Oku"), Some(VERSION.unwrap()));
        web_settings.set_enable_write_console_messages_to_stdout(true);
        web_context.set_web_process_extensions_directory(&extensions_path);
        web_view.set_visible(true);
        web_view.set_width_request(1024);
        web_view.set_height_request(640);
        web_view.load_uri("about:blank");

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
    ) -> (webkit2gtk::WebView, libadwaita::TabPage) {
        let imp = self.imp();

        let new_view = self.new_view(&web_context);
        let new_page = imp.tab_view.append(&new_view);
        new_page.set_title("New Tab");
        new_page.set_icon(Some(&gio::ThemedIcon::new("content-loading-symbolic")));
        new_page.set_live_thumbnail(true);
        imp.tab_view.set_selected_page(&new_page);
        new_page.set_indicator_icon(Some(&gio::ThemedIcon::new("view-pin-symbolic")));
        new_page.set_indicator_activatable(true);

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
                this.new_tab_page(&web_context);
            }
        ));
    }

    pub fn setup_tab_signals(&self, web_context: &WebContext) {
        let imp = self.imp();

        imp.tab_overview.connect_create_tab(clone!(
            #[weak(rename_to = this)]
            self,
            #[weak]
            web_context,
            #[upgrade_or_panic]
            move |_| this.new_tab_page(&web_context).1
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
            #[weak(rename_to = this)]
            self,
            move |_| {
                let web_view = this.get_view();
                update_nav_bar(&nav_entry, &web_view);
                this.set_title(Some(&get_title(&web_view)));
                update_load_progress(&nav_entry, &web_view);
                if web_view.is_loading() {
                    refresh_button.set_icon_name("process-stop")
                } else {
                    refresh_button.set_icon_name("view-refresh")
                }
                back_button.set_sensitive(web_view.can_go_back());
                forward_button.set_sensitive(web_view.can_go_forward());
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
        let new_window = self::Window::new(&application, &web_context, None);
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
            #[weak(rename_to = tab_bar)]
            imp.tab_bar,
            #[weak(rename_to = headerbar)]
            imp.headerbar,
            #[weak(rename_to = this)]
            self,
            move |_| {
                let web_view = this.get_view();
                if !this.is_fullscreen() {
                    this.set_fullscreened(true);
                    tab_bar.set_visible(false);
                    tab_bar.set_opacity(0.0);
                    headerbar.set_visible(false);
                    headerbar.set_opacity(0.0);
                    web_view.evaluate_javascript(
                        "document.documentElement.requestFullscreen();",
                        None,
                        None,
                        gio::Cancellable::NONE,
                        move |_| {},
                    )
                } else {
                    this.set_fullscreened(false);
                    tab_bar.set_visible(true);
                    tab_bar.set_opacity(100.0);
                    headerbar.set_visible(true);
                    headerbar.set_opacity(100.0);
                    web_view.evaluate_javascript(
                        "document.exitFullscreen();",
                        None,
                        None,
                        gio::Cancellable::NONE,
                        move |_| {},
                    )
                }
            }
        ));

        // Screenshot button clicked
        imp.screenshot_button.connect_clicked(clone!(
            #[weak(rename_to = this)]
            self,
            move |_| {
                let web_view = this.get_view();
                web_view.snapshot(
                    webkit2gtk::SnapshotRegion::FullDocument,
                    webkit2gtk::SnapshotOptions::all(),
                    gio::Cancellable::NONE,
                    move |snapshot| {
                        snapshot
                            .unwrap()
                            .save_to_png(format!("{}/{}.png", PICTURES_DIR.to_owned(), Utc::now()))
                            .unwrap();
                    },
                );
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
                    None,
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

    fn setup_new_view_signals(&self) {
        let imp = self.imp();

        imp.tab_view.connect_page_attached(clone!(
            #[weak(rename_to = this)]
            self,
            #[weak]
            imp,
            move |_, new_page, _page_position| {
                let new_view = get_view_from_page(&new_page);
                let network_session = new_view.network_session().unwrap();

                let print_started = RefCell::new(Some(new_view.connect_print(clone!(#[weak]
                this,
                    #[upgrade_or]
                    false, move |_, print_operation| {
                        let dialog = gtk::PrintDialog::builder().title("Print").modal(true).build();
                        let ctx = glib::MainContext::new();
                        ctx.with_thread_default(clone!(#[weak]
                        this, #[weak] print_operation, #[strong] ctx, move || {
                            ctx.block_on(clone!(#[weak]
                                this, #[weak] print_operation, async move {
                                let print_setup = match dialog.setup_future(Some(&this)).await {
                                    Ok(print_setup) => print_setup,
                                    Err(_e) => {
                                        return;
                                    }
                                };
                                print_operation.set_page_setup(&print_setup.page_setup());
                                print_operation.set_print_settings(&print_setup.print_settings());
                                print_operation.print();
                            }));
                        })).unwrap();
                        true
                    })
                )));

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
                                                            gio::Cancellable::NONE,
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
                    move |w, _| {
                        imp.obj().set_title(Some(
                            &get_title(&w),
                        ));
                        update_favicon(tab_view, &w);
                        if this.get_view() == *w {
                            back_button.set_sensitive(w.can_go_back());
                            forward_button.set_sensitive(w.can_go_forward());
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
                            imp.headerbar.set_visible(false);
                            imp.tab_bar.set_visible(false);
                            imp.obj().set_fullscreened(true);
                            imp.tab_bar.set_visible(false);
                            imp.tab_bar.set_opacity(0.0);
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
                            imp.headerbar.set_visible(true);
                            imp.tab_bar.set_visible(true);
                            imp.obj().set_fullscreened(false);
                            imp.tab_bar.set_visible(true);
                            imp.tab_bar.set_opacity(100.0);
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
                let connect_uri_notify = RefCell::new(Some(new_view.connect_uri_notify(clone!(
                    #[weak(rename_to = nav_entry)]
                    imp.nav_entry,
                    #[weak(rename_to = back_button)]
                    imp.back_button,
                    #[weak(rename_to = forward_button)]
                    imp.forward_button,
                    #[weak]
                    this,
                    move |w| {
                        if this.get_view() == *w {
                            update_nav_bar(&nav_entry, &w);
                            back_button.set_sensitive(w.can_go_back());
                            forward_button.set_sensitive(w.can_go_forward());
                        }
                    }
                ))));
                let estimated_load_progress_notify = RefCell::new(Some(
                    new_view.connect_estimated_load_progress_notify(clone!(
                        #[weak(rename_to = tab_view)]
                        imp.tab_view,
                        #[weak(rename_to = nav_entry)]
                        imp.nav_entry,
                        #[weak(rename_to = refresh_button)]
                        imp.refresh_button,
                        #[weak]
                        this,
                        move |w| {
                            let current_page = tab_view.page(w);
                            current_page.set_loading(w.is_loading());
                            if this.get_view() == *w {
                                update_load_progress(&nav_entry, &w);
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
                        #[weak]
                        this,
                        move |w| {
                            let current_page = tab_view.page(w);
                            current_page.set_loading(w.is_loading());
                            if this.get_view() == *w {
                                if current_page.is_loading() {
                                    refresh_button.set_icon_name("process-stop")
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
                        if let Some(id) = print_started.take() {
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
                        if let Some(id) = connect_uri_notify.take() {
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
}
