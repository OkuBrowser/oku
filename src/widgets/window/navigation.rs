use super::*;
use crate::database::DATABASE;
use crate::window_util::connect;
use glib::clone;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};
use libadwaita::prelude::*;
use webkit2gtk::prelude::WebViewExt;

impl Window {
    /// Adapted from Geopard (https://github.com/ranfdev/Geopard)
    pub fn set_progress_animated(&self, progress: f64) {
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

    pub fn setup_progress_bar(&self) {
        let imp = self.imp();

        imp.progress_bar.add_css_class("osd");
        imp.progress_bar.set_valign(gtk::Align::Start);
    }

    pub fn setup_url_status(&self) {
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

    pub fn setup_navigation_buttons(&self) {
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

    pub fn setup_navigation_signals(&self) {
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
        let action_previous = gio::ActionEntry::builder("previous")
            .activate(|window: &Self, _, _| {
                let web_view = window.get_view();
                web_view.go_back();
            })
            .build();
        self.add_action_entries([action_previous]);

        // Forward button clicked
        imp.forward_button.connect_clicked(clone!(
            #[weak(rename_to = this)]
            self,
            move |_| {
                let web_view = this.get_view();
                web_view.go_forward()
            }
        ));
        let action_next = gio::ActionEntry::builder("next")
            .activate(|window: &Self, _, _| {
                let web_view = window.get_view();
                web_view.go_forward();
            })
            .build();
        self.add_action_entries([action_next]);

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
        let action_reload = gio::ActionEntry::builder("reload")
            .activate(|window: &Self, _, _| {
                let web_view = window.get_view();
                web_view.reload();
            })
            .build();
        self.add_action_entries([action_reload]);
        let action_reload_bypass = gio::ActionEntry::builder("reload-bypass")
            .activate(|window: &Self, _, _| {
                let web_view = window.get_view();
                web_view.reload_bypass_cache();
            })
            .build();
        self.add_action_entries([action_reload_bypass]);
        let action_stop_loading = gio::ActionEntry::builder("stop-loading")
            .activate(|window: &Self, _, _| {
                let web_view = window.get_view();
                web_view.stop_loading()
            })
            .build();
        self.add_action_entries([action_stop_loading]);

        let action_go_home = gio::ActionEntry::builder("go-home")
            .activate(|window: &Self, _, _| {
                let web_view = window.get_view();
                web_view.load_uri("oku:home")
            })
            .build();
        self.add_action_entries([action_go_home]);

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

                    let suggestion_items = DATABASE
                        .search(imp.nav_entry.text().to_string(), &favicon_database)
                        .unwrap_or_default();
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
}
