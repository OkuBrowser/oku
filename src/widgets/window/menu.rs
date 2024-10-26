use super::*;
use crate::widgets;
use chrono::Utc;
use glib::clone;
use gtk::prelude::GtkWindowExt;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};
use libadwaita::{prelude::*, ResponseAppearance};
use log::{error, info, warn};
use webkit2gtk::prelude::WebViewExt;
use webkit2gtk::WebContext;

impl Window {
    pub fn setup_menu_popover(&self) {
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

        // New Private Window button
        imp.new_private_window_button.set_can_focus(true);
        imp.new_private_window_button.set_receives_default(true);
        imp.new_private_window_button
            .set_icon_name("screen-privacy7-symbolic");

        // Settings button
        imp.settings_button.set_can_focus(true);
        imp.settings_button.set_receives_default(true);
        imp.settings_button.set_icon_name("preferences-system");

        // About button
        imp.about_button.set_can_focus(true);
        imp.about_button.set_receives_default(true);
        imp.about_button.set_icon_name("help-about");

        // Shortcuts button
        imp.shortcuts_button.set_can_focus(true);
        imp.shortcuts_button.set_receives_default(true);
        imp.shortcuts_button
            .set_icon_name("keyboard-shortcuts-symbolic");

        // Menu popover
        imp.menu_box.set_hexpand(true);
        imp.menu_box.append(&imp.zoom_buttons);
        imp.menu_box.append(&imp.zoom_percentage);
        imp.menu_box.append(&imp.zoomreset_button);
        imp.menu_box.append(&imp.fullscreen_button);
        imp.menu_box.append(&imp.print_button);
        imp.menu_box.append(&imp.screenshot_button);
        imp.menu_box.append(&imp.new_window_button);
        imp.menu_box.append(&imp.new_private_window_button);
        imp.menu_box.append(&imp.shortcuts_button);
        imp.menu_box.append(&imp.settings_button);
        imp.menu_box.append(&imp.about_button);
        imp.menu_box.add_css_class("toolbar");

        imp.menu_popover.set_child(Some(&imp.menu_box));
        imp.menu_popover.set_parent(&imp.menu_button);
        imp.menu_popover.set_autohide(true);
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
        let action_zoom_in = gio::ActionEntry::builder("zoom-in")
            .activate(move |window: &Self, _, _| {
                let web_view = window.get_view();
                let current_zoom_level = web_view.zoom_level();
                web_view.set_zoom_level(current_zoom_level + 0.1);
            })
            .build();
        self.add_action_entries([action_zoom_in]);

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
        let action_zoom_out = gio::ActionEntry::builder("zoom-out")
            .activate(move |window: &Self, _, _| {
                let web_view = window.get_view();
                let current_zoom_level = web_view.zoom_level();
                web_view.set_zoom_level(current_zoom_level - 0.1);
            })
            .build();
        self.add_action_entries([action_zoom_out]);

        // Reset Zoom button clicked
        imp.zoomreset_button.connect_clicked(clone!(
            #[weak(rename_to = this)]
            self,
            move |_| {
                let web_view = this.get_view();
                web_view.set_zoom_level(1.0);
            }
        ));
        let action_reset_zoom = gio::ActionEntry::builder("reset-zoom")
            .activate(move |window: &Self, _, _| {
                let web_view = window.get_view();
                web_view.set_zoom_level(1.0);
            })
            .build();
        self.add_action_entries([action_reset_zoom]);
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
        let action_fullscreen = gio::ActionEntry::builder("fullscreen")
            .activate(clone!(
                #[weak(rename_to = fullscreen_button)]
                imp.fullscreen_button,
                move |window: &Self, _, _| {
                    let web_view = window.get_view();
                    if !window.is_fullscreen() {
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
            ))
            .build();
        self.add_action_entries([action_fullscreen]);

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
        let action_print = gio::ActionEntry::builder("print")
            .activate(clone!(move |window: &Self, _, _| {
                if window.imp().tab_view.n_pages() == 0 {
                    return;
                }
                let web_view = window.get_view();
                web_view.evaluate_javascript(
                    "window.print();",
                    None,
                    None,
                    Some(&gio::Cancellable::new()),
                    move |_| {},
                )
            }))
            .build();
        self.add_action_entries([action_print]);

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
        let action_screenshot = gio::ActionEntry::builder("screenshot")
            .activate(
                |window: &Self, _, _| {
                    if window.imp().tab_view.n_pages() == 0 {
                        return;
                    }
                    let web_view = window.get_view();
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
                            window,
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
                                        Some(&window),
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
                    dialog.present(Some(window));
                }
            )
            .build();
        self.add_action_entries([action_screenshot]);

        // New Window button clicked
        imp.new_window_button.connect_clicked(clone!(
            #[weak(rename_to = this)]
            self,
            #[weak]
            web_context,
            move |_| {
                self::Window::new(
                    &this.application().unwrap().downcast().unwrap(),
                    &*this.imp().style_provider.borrow(),
                    &web_context,
                    false,
                );
            }
        ));
        let action_new = gio::ActionEntry::builder("new")
            .activate(clone!(
                #[weak]
                web_context,
                move |window: &Self, _, _| {
                    self::Window::new(
                        &window.application().unwrap().downcast().unwrap(),
                        &*window.imp().style_provider.borrow(),
                        &web_context,
                        false,
                    );
                }
            ))
            .build();
        self.add_action_entries([action_new]);

        // New Private Window button clicked
        imp.new_private_window_button.connect_clicked(clone!(
            #[weak(rename_to = this)]
            self,
            #[weak]
            web_context,
            move |_| {
                self::Window::new(
                    &this.application().unwrap().downcast().unwrap(),
                    &*this.imp().style_provider.borrow(),
                    &web_context,
                    true,
                );
            }
        ));
        let action_new_private = gio::ActionEntry::builder("new-private")
            .activate(clone!(
                #[weak]
                web_context,
                move |window: &Self, _, _| {
                    self::Window::new(
                        &window.application().unwrap().downcast().unwrap(),
                        &*window.imp().style_provider.borrow(),
                        &web_context,
                        true,
                    );
                }
            ))
            .build();
        self.add_action_entries([action_new_private]);

        // Settings button clicked
        imp.settings_button.connect_clicked(clone!(
            #[weak(rename_to = this)]
            self,
            move |_| {
                widgets::settings::Settings::new(
                    &this.application().unwrap().downcast().unwrap(),
                    &this,
                );
            }
        ));
        let action_settings = gio::ActionEntry::builder("settings")
            .activate(|window: &Self, _, _| {
                widgets::settings::Settings::new(
                    &window.application().unwrap().downcast().unwrap(),
                    &window,
                );
            })
            .build();
        self.add_action_entries([action_settings]);

        // About button clicked
        imp.about_button.connect_clicked(clone!(
            #[weak(rename_to = this)]
            self,
            move |_| this.about_dialog()
        ));

        // Shortcuts button clicked
        imp.shortcuts_button.connect_clicked(clone!(
            #[weak(rename_to = this)]
            self,
            move |_| this.shortcuts_window()
        ));
        let action_shortcuts = gio::ActionEntry::builder("shortcuts")
            .activate(|window: &Self, _, _| window.shortcuts_window())
            .build();
        self.add_action_entries([action_shortcuts]);

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
}
