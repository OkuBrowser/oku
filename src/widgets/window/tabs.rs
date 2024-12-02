use super::*;
use crate::window_util::{get_title, get_view_from_page, update_nav_bar};
use glib::clone;
use gtk::prelude::GtkWindowExt;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};
use libadwaita::prelude::*;
use webkit2gtk::prelude::WebViewExt;
use webkit2gtk::WebContext;

impl Window {
    pub fn setup_tabs(&self) {
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

        let new_view = self.new_view(web_context, related_view, initial_request);
        let new_page = imp.tab_view.append(&new_view);
        new_page.set_title("New Tab");
        new_page.set_icon(Some(&gio::ThemedIcon::new("globe-symbolic")));
        new_page.set_live_thumbnail(true);
        imp.tab_view.set_selected_page(&new_page);
        new_page.set_indicator_icon(Some(&gio::ThemedIcon::new("view-pin-symbolic")));
        new_page.set_indicator_activatable(true);
        imp.nav_entry.grab_focus();

        (new_view, new_page)
    }

    pub fn setup_tab_indicator(&self) {
        let imp = self.imp();

        // Indicator logic
        imp.tab_view.connect_indicator_activated(clone!(
            #[weak(rename_to = tab_view)]
            imp.tab_view,
            move |_, current_page| {
                let current_view = get_view_from_page(current_page);
                if !current_view.is_playing_audio() && !current_view.is_muted() {
                    tab_view.set_page_pinned(current_page, !current_page.is_pinned());
                } else {
                    current_view.set_is_muted(!current_view.is_muted());
                }
            }
        ));
    }

    pub fn setup_add_tab_button_clicked(&self, web_context: &WebContext) {
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
        let action_new_tab = gio::ActionEntry::builder("new-tab")
            .activate(clone!(
                #[weak]
                web_context,
                move |window: &Self, _, _| {
                    window.new_tab_page(&web_context, None, None);
                }
            ))
            .build();
        self.add_action_entries([action_new_tab]);
        let action_view_source = gio::ActionEntry::builder("view-source")
            .activate(clone!(move |window: &Self, _, _| {
                let web_view = window.get_view();
                web_view.load_uri("view-source:");
            }))
            .build();
        self.add_action_entries([action_view_source]);
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
            #[weak]
            style_manager,
            #[weak]
            imp,
            #[weak(rename_to = this)]
            self,
            move |tab_view| {
                if let Some(selected_page) = tab_view.selected_page() {
                    selected_page.set_needs_attention(false);
                }
                let web_view = this.get_view();
                update_nav_bar(&imp.nav_entry, &web_view);
                match imp.is_private.get() {
                    true => this.set_title(Some(&format!("{} — Private", get_title(&web_view)))),
                    false => this.set_title(Some(&get_title(&web_view))),
                }
                this.update_load_progress(&web_view);
                if web_view.is_loading() {
                    imp.refresh_button.set_icon_name("cross-large-symbolic")
                } else {
                    imp.refresh_button
                        .set_icon_name("arrow-circular-top-right-symbolic")
                }
                imp.back_button.set_sensitive(web_view.can_go_back());
                imp.forward_button.set_sensitive(web_view.can_go_forward());
                this.update_color(&web_view, &style_manager);
                imp.zoom_percentage
                    .set_text(&format!("{:.0}%", web_view.zoom_level() * 100.0))
            }
        ));
    }

    /// Create new browser window when a tab is dragged off
    ///
    /// # Arguments
    ///
    /// * `ipfs` - An IPFS client
    pub fn create_window_from_drag(
        &self,
        web_context: &WebContext,
    ) -> std::option::Option<libadwaita::TabView> {
        let application = self.application().unwrap().downcast().unwrap();
        let new_window = self::Window::new(
            &application,
            &self.imp().style_provider.borrow(),
            web_context,
            self.imp().is_private.get(),
        );
        Some(new_window.imp().tab_view.to_owned())
    }
}
