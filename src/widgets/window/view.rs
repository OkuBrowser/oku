use super::*;
use crate::database::policy::PolicySettingRecord;
use crate::database::{HistoryRecord, DATABASE};
use crate::window_util::{
    get_title, get_view_from_page, new_webkit_settings, update_favicon, update_nav_bar,
    update_title,
};
use crate::{DATA_DIR, VERSION};
use glib::clone;
use gtk::prelude::GtkWindowExt;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};
use libadwaita::{prelude::*, ResponseAppearance};
use log::error;
use std::cell::RefCell;
use uuid::Uuid;
use webkit2gtk::functions::{
    uri_for_display, user_media_permission_is_for_audio_device,
    user_media_permission_is_for_display_device, user_media_permission_is_for_video_device,
};
use webkit2gtk::prelude::PermissionRequestExt;
use webkit2gtk::prelude::WebViewExt;
use webkit2gtk::{FaviconDatabase, LoadEvent};
use webkit2gtk::{WebContext, WebView};

impl Window {
    pub fn ask_permission(&self, permission_request: &webkit2gtk::PermissionRequest) {
        let (title, description) = if permission_request
            .is::<webkit2gtk::ClipboardPermissionRequest>()
        {
            (
                "Allow access to clipboard?",
                "This page is requesting permission to read the contents of your clipboard.",
            )
        } else if permission_request.is::<webkit2gtk::DeviceInfoPermissionRequest>() {
            ("Allow access to audio & video devices?", "This page is requesting access to information regarding your audio & video devices.")
        } else if permission_request.is::<webkit2gtk::GeolocationPermissionRequest>() {
            (
                "Allow access to location?",
                "This page is requesting access to your location.",
            )
        } else if permission_request.is::<webkit2gtk::MediaKeySystemPermissionRequest>() {
            (
                "Allow playback of encrypted media?",
                "This page wishes to play encrypted media.",
            )
        } else if permission_request.is::<webkit2gtk::NotificationPermissionRequest>() {
            (
                "Allow notifications?",
                "This page is requesting permission to display notifications.",
            )
        } else if permission_request.is::<webkit2gtk::PointerLockPermissionRequest>() {
            (
                "Allow locking the pointer?",
                "This page is requesting permission to lock your pointer.",
            )
        } else if permission_request.is::<webkit2gtk::UserMediaPermissionRequest>() {
            let user_media_permission_request = permission_request
                .downcast_ref::<webkit2gtk::UserMediaPermissionRequest>()
                .unwrap();
            if user_media_permission_is_for_audio_device(user_media_permission_request) {
                (
                    "Allow access to audio devices?",
                    "This page is requesting access to your audio source devices.",
                )
            } else if user_media_permission_is_for_display_device(user_media_permission_request) {
                (
                    "Allow access to display devices?",
                    "This page is requesting access to your display devices.",
                )
            } else if user_media_permission_is_for_video_device(user_media_permission_request) {
                (
                    "Allow access to video devices?",
                    "This page is requesting access to your video source devices.",
                )
            } else {
                (
                    "Allow access to media devices?",
                    "This page is requesting access to your media devices.",
                )
            }
        } else if permission_request.is::<webkit2gtk::WebsiteDataAccessPermissionRequest>() {
            (
                "Allow access to third-party cookies?",
                "This page is requesing permission to read your data from third-party domains.",
            )
        } else {
            ("", "")
        };
        let dialog = libadwaita::AlertDialog::new(Some(title), Some(description));
        dialog.add_responses(&[("deny", "Deny"), ("allow", "Allow")]);
        dialog.set_response_appearance("deny", ResponseAppearance::Default);
        dialog.set_response_appearance("allow", ResponseAppearance::Destructive);
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
                        "allow" => permission_request.allow(),
                        _ => {
                            unreachable!()
                        }
                    }
                }
            ),
        );
        dialog.present(Some(self));
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
            web_view.load_uri("oku:home");
            web_view
        }
    }

    pub fn favicon_database(&self) -> FaviconDatabase {
        self.get_view()
            .network_session()
            .unwrap()
            .website_data_manager()
            .unwrap()
            .favicon_database()
            .unwrap()
    }

    /// Create a new WebKit instance for the current tab
    ///
    /// # Arguments
    ///  
    /// * `ipfs` - An IPFS client
    pub fn new_view(
        &self,
        web_context: &WebContext,
        related_view: Option<&webkit2gtk::WebView>,
        initial_request: Option<&webkit2gtk::URIRequest>,
    ) -> webkit2gtk::WebView {
        let web_settings: webkit2gtk::Settings = new_webkit_settings();
        let mut web_view_builder = WebView::builder().settings(&web_settings);
        if let Some(related_view) = related_view {
            web_view_builder = web_view_builder.related_view(related_view);
        } else {
            web_view_builder = web_view_builder
                .web_context(web_context)
                .network_session(&self.imp().network_session.borrow())
        };
        let web_view = web_view_builder.build();
        web_view.set_vexpand(true);
        let network_session = web_view.network_session().unwrap();
        let data_manager = network_session.website_data_manager().unwrap();
        let security_manager = web_context.security_manager().unwrap();
        let extensions_path = format!("{}/extensions/", DATA_DIR.to_string_lossy());
        let _ = std::fs::create_dir_all(&extensions_path);

        data_manager.set_favicons_enabled(true);

        security_manager.register_uri_scheme_as_secure("ipfs");
        security_manager.register_uri_scheme_as_secure("ipns");
        security_manager.register_uri_scheme_as_secure("hive");
        security_manager.register_uri_scheme_as_cors_enabled("ipfs");
        security_manager.register_uri_scheme_as_cors_enabled("ipns");
        security_manager.register_uri_scheme_as_cors_enabled("hive");
        security_manager.register_uri_scheme_as_display_isolated("view-source");
        security_manager.register_uri_scheme_as_no_access("view-source");
        security_manager.register_uri_scheme_as_display_isolated("oku");

        web_settings
            .set_user_agent_with_application_details(Some("Oku"), Some(&VERSION.to_string()));
        web_settings.set_enable_write_console_messages_to_stdout(true);
        web_context.set_web_process_extensions_directory(&extensions_path);
        if let Some(initial_request) = initial_request {
            web_view.load_request(initial_request)
        } else {
            web_view.load_uri("oku:home");
        }
        web_view.set_visible(true);

        web_view
    }

    pub fn setup_new_view_signals(
        &self,
        web_context: &WebContext,
        style_manager: &libadwaita::StyleManager,
    ) {
        let imp = self.imp();

        imp.tab_view.connect_page_attached(clone!(
            #[weak]
            web_context,
            #[weak]
            style_manager,
            #[weak(rename_to = this)]
            self,
            #[weak]
            imp,
            move |_, new_page, _page_position| {
                let action_close_tab = gio::ActionEntry::builder("close-tab")
                    .activate(clone!(
                        #[weak(rename_to = tab_view)]
                        imp.tab_view,
                        #[weak]
                        new_page,
                        move |window: &Self, _, _| {
                            if tab_view.n_pages() > 1 {
                                tab_view.close_page(&new_page);
                            } else {
                                window.close();
                            }
                        }
                    ))
                    .build();
                this.add_action_entries([action_close_tab]);

                let new_view = get_view_from_page(new_page);
                let find_controller = new_view.find_controller().unwrap();

                let found_text = RefCell::new(Some(find_controller.connect_found_text(clone!(
                    #[weak]
                    imp,
                    move |_find_controller, match_count| {
                        if match_count > 1 {
                            imp.total_matches_label
                                .set_text(&format!("{} matches", match_count));
                        } else {
                            imp.total_matches_label.set_text("1 match");
                        }
                    }
                ))));

                let failed_to_find_text =
                    RefCell::new(Some(find_controller.connect_failed_to_find_text(clone!(
                        #[weak]
                        imp,
                        move |_find_controller| {
                            imp.total_matches_label.set_text("No matches");
                        }
                    ))));

                let show_notification =
                    RefCell::new(Some(new_view.connect_show_notification(clone!(
                        #[weak]
                        new_page,
                        #[weak]
                        imp,
                        #[upgrade_or]
                        false,
                        move |_w, _notification| {
                            if !matches!(imp.tab_view.selected_page(), Some(x) if x == new_page) {
                                new_page.set_needs_attention(true);
                            }
                            false
                        }
                    ))));

                let close = RefCell::new(Some(new_view.connect_close(clone!(
                    #[weak]
                    new_page,
                    #[weak]
                    imp,
                    #[upgrade_or_panic]
                    move |_w| {
                        imp.tab_view.set_page_pinned(&new_page, false);
                        if imp.tab_view.n_pages() <= 1 {
                            imp.tab_overview.set_open(true);
                        }
                        imp.tab_view.close_page(&new_page);
                    }
                ))));

                let create = RefCell::new(Some(new_view.connect_create(clone!(
                    #[weak]
                    web_context,
                    #[weak]
                    this,
                    #[upgrade_or_panic]
                    move |w, navigation_action| {
                        let mut navigation_action = navigation_action.clone();
                        let new_related_view = this
                            .new_tab_page(
                                &web_context,
                                Some(w),
                                navigation_action.request().as_ref(),
                            )
                            .0;
                        new_related_view.into()
                    }
                ))));

                let status_message =
                    RefCell::new(Some(new_view.connect_mouse_target_changed(clone!(
                        #[weak]
                        imp,
                        move |_w, hit_test_result, _modifier| {
                            if let Some(link_uri) = hit_test_result.link_uri() {
                                imp.url_status.set_text(
                                    uri_for_display(link_uri.as_str())
                                        .unwrap_or_default()
                                        .as_str(),
                                );
                                imp.url_status_box.set_visible(true);
                            } else {
                                imp.url_status.set_text("");
                                imp.url_status_box.set_visible(false);
                            }
                        }
                    ))));

                let permission_request =
                    RefCell::new(Some(new_view.connect_permission_request(clone!(
                        #[weak]
                        this,
                        #[upgrade_or]
                        false,
                        move |w, permission_request| {
                            let policy = PolicySettingRecord::from_uri(
                                w.uri().unwrap_or_default().to_string(),
                            );
                            policy.handle(&this, permission_request);
                            true
                        }
                    ))));

                let title_notify = RefCell::new(Some(new_view.connect_title_notify(clone!(
                    #[weak(rename_to = tab_view)]
                    imp.tab_view,
                    move |w| update_title(tab_view, w)
                ))));
                let favicon_notify = RefCell::new(Some(new_view.connect_favicon_notify(clone!(
                    #[weak(rename_to = tab_view)]
                    imp.tab_view,
                    move |w| update_favicon(tab_view, w)
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
                        let title = get_title(w);
                        match imp.is_private.get() {
                            true => this.set_title(Some(&format!("{} — Private", &title))),
                            false => this.set_title(Some(&title)),
                        }
                        update_favicon(tab_view, w);
                        if this.get_view() == *w {
                            back_button.set_sensitive(w.can_go_back());
                            forward_button.set_sensitive(w.can_go_forward());
                        }
                        if !matches!(w.uri(), Some(x) if x == "oku:home")
                            && !imp.is_private.get()
                            && load_event == LoadEvent::Finished
                        {
                            if let Some(back_forward_list) = w.back_forward_list() {
                                if let Some(current_item) = back_forward_list.current_item() {
                                    if let Err(e) = DATABASE.upsert_history_record(HistoryRecord {
                                        id: Uuid::now_v7(),
                                        original_uri: current_item
                                            .original_uri()
                                            .unwrap_or_default()
                                            .to_string(),
                                        uri: current_item.uri().unwrap_or_default().to_string(),
                                        title: Some(title),
                                        timestamp: chrono::Utc::now(),
                                    }) {
                                        error!("{}", e)
                                    }
                                }
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
                let zoom_level_notify =
                    RefCell::new(Some(new_view.connect_zoom_level_notify(clone!(
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
                            update_nav_bar(&nav_entry, w);
                            back_button.set_sensitive(w.can_go_back());
                            forward_button.set_sensitive(w.can_go_forward());
                            this.update_color(w, &style_manager);
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
                                this.update_load_progress(w);
                                if current_page.is_loading() {
                                    refresh_button.set_icon_name("cross-large-symbolic")
                                } else {
                                    refresh_button
                                        .set_icon_name("arrow-circular-top-right-symbolic")
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
                                    refresh_button.set_icon_name("cross-large-symbolic");
                                    progress_bar.pulse()
                                } else {
                                    refresh_button
                                        .set_icon_name("arrow-circular-top-right-symbolic")
                                }
                            }
                        }
                    ))));
                imp.tab_view.connect_page_detached(clone!(
                    #[weak]
                    new_view,
                    move |_, old_page, _page_position| {
                        let old_view = get_view_from_page(old_page);
                        if old_view != new_view {
                            // When a page is disconnected from a window, the detached handler runs for all pages in the window.
                            // If we don't stop here, the browser will crash as we'll be disconnecting handlers for unrelated pages that weren't detached.
                            return;
                        }
                        let old_find_controller = old_view.find_controller().unwrap();
                        if let Some(id) = zoom_level_notify.take() {
                            old_view.disconnect(id);
                        }
                        if let Some(id) = failed_to_find_text.take() {
                            old_find_controller.disconnect(id);
                        }
                        if let Some(id) = found_text.take() {
                            old_find_controller.disconnect(id);
                        }
                        if let Some(id) = show_notification.take() {
                            old_view.disconnect(id);
                        }
                        if let Some(id) = close.take() {
                            old_view.disconnect(id);
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
}
