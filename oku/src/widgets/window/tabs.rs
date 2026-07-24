use super::*;
use crate::widgets::okunet::net::Net;
use crate::window_util::{get_title, get_view_from_page, update_nav_bar};
use glib::clone;
use gtk::prelude::GtkWindowExt;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};
use libadwaita::prelude::*;
use webkit2gtk::prelude::WebViewExt;
use webkit2gtk::WebContext;

pub struct NewWebTabArguments<'a> {
    pub web_context: &'a WebContext,
    pub related_view: Option<&'a webkit2gtk::WebView>,
    pub initial_request: Option<&'a webkit2gtk::URIRequest>,
}

#[derive(Default)]
pub enum NewTabArguments<'a> {
    #[default]
    OkuNet,
    Web(&'a NewWebTabArguments<'a>),
}

impl<'a> Default for &NewTabArguments<'a> {
    fn default() -> Self {
        &NewTabArguments::OkuNet
    }
}

pub struct NewTabWebReturn {
    pub web_view: webkit2gtk::WebView,
    pub tab_page: libadwaita::TabPage,
}

pub struct NewTabOkuNetReturn {
    pub net: Net,
    pub tab_page: libadwaita::TabPage,
}

pub enum NewTabReturn {
    OkuNet(NewTabOkuNetReturn),
    Web(NewTabWebReturn),
}

impl NewTabReturn {
    pub fn as_okunet(&self) -> miette::Result<&NewTabOkuNetReturn> {
        match self {
            Self::OkuNet(res) => Ok(res),
            Self::Web(_) => Err(miette::miette!("New tab is not an OkuNet tab")),
        }
    }

    pub fn as_web(&self) -> miette::Result<&NewTabWebReturn> {
        match self {
            Self::OkuNet(_) => Err(miette::miette!("New tab is not an OkuNet tab")),
            Self::Web(res) => Ok(res),
        }
    }
}

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
            .set_transition_type(gtk::RevealerTransitionType::SlideDown);
        imp.fullscreen_box
            .property_expression("reveal-top-bars")
            .bind(&imp.tab_bar_revealer, "reveal-child", gtk::Widget::NONE);
    }

    pub fn new_tab(&self, arguments: &Option<&NewTabArguments>) -> NewTabReturn {
        let imp = self.imp();

        let arguments = arguments.unwrap_or_default();

        let unresponsive_spinner = libadwaita::Spinner::builder()
            .valign(gtk::Align::Center)
            .halign(gtk::Align::Center)
            .hexpand(true)
            .vexpand(true)
            .height_request(64)
            .width_request(64)
            .visible(false)
            .build();

        let mut overlay_builder = gtk::Overlay::builder();
        overlay_builder = match arguments {
            NewTabArguments::OkuNet => {
                let okunet = Net::new();
                overlay_builder.child(&okunet)
            }
            NewTabArguments::Web(arguments) => {
                let new_view = self.new_view(
                    arguments.web_context,
                    arguments.related_view,
                    arguments.initial_request,
                );
                overlay_builder.child(&new_view)
            }
        };
        let overlay = overlay_builder.build();
        overlay.add_overlay(&unresponsive_spinner);

        let new_page = imp.tab_view.append(&overlay);
        new_page.set_title("New Tab");
        new_page.set_icon(Some(&gio::ThemedIcon::new("globe-symbolic")));
        new_page.set_live_thumbnail(true);
        imp.tab_view.set_selected_page(&new_page);
        new_page.set_indicator_icon(Some(&gio::ThemedIcon::new("view-pin-symbolic")));
        new_page.set_indicator_activatable(true);
        imp.nav_entry.grab_focus();

        match arguments {
            NewTabArguments::OkuNet => NewTabReturn::OkuNet(NewTabOkuNetReturn {
                tab_page: new_page,
                net: overlay
                    .child()
                    .map(|x| x.downcast().unwrap_or_default())
                    .expect("Overlay should have OkuNet child"),
            }),
            NewTabArguments::Web(_) => NewTabReturn::Web(NewTabWebReturn {
                tab_page: new_page,
                web_view: overlay
                    .child()
                    .map(|x| x.downcast().unwrap_or_default())
                    .expect("Overlay should have WebView child"),
            }),
        }
    }

    pub fn setup_tab_indicator(&self) {
        let imp = self.imp();

        // Indicator logic
        imp.tab_view.connect_indicator_activated(clone!(
            #[weak(rename_to = tab_view)]
            imp.tab_view,
            move |_, current_page| {
                if let Ok((_current_view_overlay, current_view)) = get_view_from_page(current_page)
                {
                    if !current_view.is_playing_audio() && !current_view.is_muted() {
                        tab_view.set_page_pinned(current_page, !current_page.is_pinned());
                    } else {
                        current_view.set_is_muted(!current_view.is_muted());
                    }
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
                this.new_tab(&Some(&NewTabArguments::Web(&NewWebTabArguments {
                    web_context: &web_context,
                    related_view: None,
                    initial_request: None,
                })));
            }
        ));
        let action_new_tab = gio::ActionEntry::builder("new-tab")
            .activate(clone!(
                #[weak]
                web_context,
                move |window: &Self, _, _| {
                    window.new_tab(&Some(&NewTabArguments::Web(&NewWebTabArguments {
                        web_context: &web_context,
                        related_view: None,
                        initial_request: None,
                    })));
                }
            ))
            .build();
        self.add_action_entries([action_new_tab]);
        let action_view_source = gio::ActionEntry::builder("view-source")
            .activate(clone!(move |window: &Self, _, _| {
                let web_view = window.get_view();
                if web_view.is_err() {
                    return;
                }
                let web_view = web_view.expect("Web view exists");
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
            move |_| this
                .new_tab(&Some(&NewTabArguments::Web(&NewWebTabArguments {
                    web_context: &web_context,
                    related_view: None,
                    initial_request: None
                })))
                .as_web()
                .expect("New tab to be Web tab")
                .tab_page
                .clone()
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
                if web_view.is_err() {
                    return;
                }
                let web_view = web_view.expect("Web view exists");
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
