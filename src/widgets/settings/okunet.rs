use super::core::Settings;
use crate::window_util::get_window_from_widget;
use crate::{HOME_REPLICA_SET, NODE};
use glib::clone;
use gtk::glib;
use gtk::subclass::prelude::*;
use libadwaita::{prelude::*, ResponseAppearance};
use log::error;
use std::sync::atomic::Ordering;

impl Settings {
    pub fn setup_okunet_group(&self) {
        let imp = self.imp();

        imp.display_name_row.set_title("Display name");
        imp.display_name_row.set_show_apply_button(true);

        imp.import_author_button
            .set_icon_name("system-switch-user-symbolic");
        imp.import_author_button.add_css_class("circular");
        imp.import_author_button.add_css_class("linked");
        imp.import_author_button
            .set_tooltip_text(Some("Import user credentials"));
        imp.import_author_button.set_valign(gtk::Align::Center);
        imp.import_author_button.connect_clicked(clone!(
            #[weak(rename_to = this)]
            self,
            move |_| {
                glib::spawn_future_local(async move {
                    if let Err(e) = this.import_user().await {
                        error!("{}", e);
                    }
                    this.initialise_okunet_information();
                });
            }
        ));

        imp.export_author_button.set_icon_name("user-info-symbolic");
        imp.export_author_button.add_css_class("circular");
        imp.export_author_button.add_css_class("linked");
        imp.export_author_button
            .set_tooltip_text(Some("Export user credentials"));
        imp.export_author_button.set_valign(gtk::Align::Center);
        imp.export_author_button.connect_clicked(clone!(
            #[weak(rename_to = this)]
            self,
            move |_| {
                glib::spawn_future_local(async move {
                    if let Err(e) = this.save_exported_user().await {
                        error!("{}", e);
                    }
                });
            }
        ));

        imp.copy_author_button.set_icon_name("copy-symbolic");
        imp.copy_author_button.add_css_class("circular");
        imp.copy_author_button.set_valign(gtk::Align::Center);
        imp.copy_author_button.connect_clicked(clone!(
            #[weak(rename_to = this)]
            self,
            #[weak]
            imp,
            move |_| {
                let clipboard = gdk::Display::default().unwrap().clipboard();
                let author_id = imp.author_row.subtitle().unwrap_or_default();
                clipboard.set_text(&author_id);
                let window = get_window_from_widget(&this);
                let app = window.application().unwrap();
                let notification = gio::Notification::new("Author ID copied");
                notification.set_body(Some(&format!(
                    "Author ID ({}) has been copied to the clipboard.",
                    author_id
                )));
                app.send_notification(None, &notification);
            }
        ));

        imp.import_export_buttons.append(&imp.import_author_button);
        imp.import_export_buttons.append(&imp.export_author_button);
        imp.import_export_buttons.add_css_class("linked");

        imp.author_buttons.set_spacing(4);
        imp.author_buttons.append(&imp.copy_author_button);
        imp.author_buttons.append(&imp.import_export_buttons);

        imp.author_row.set_title("Author ID");
        imp.author_row.add_css_class("property");
        imp.author_row.add_css_class("monospace");
        imp.author_row.set_subtitle_lines(1);
        imp.author_row.add_suffix(&imp.author_buttons);

        self.initialise_okunet_information();

        imp.okunet_group.set_title("OkuNet");
        imp.okunet_group
            .set_description(Some("Settings affecting the use of OkuNet"));
        imp.okunet_group.add(&imp.author_row);
        imp.okunet_group.add(&imp.display_name_row);
    }

    pub fn initialise_okunet_information(&self) {
        let imp = self.imp();

        if let Some(node) = NODE.get() {
            imp.author_row
                .set_subtitle(&oku_fs::fs::util::fmt(node.default_author()));
            let home_replica_set = HOME_REPLICA_SET.load(Ordering::Relaxed);
            match home_replica_set {
                true => {
                    let ctx = glib::MainContext::default();
                    ctx.spawn_local(clone!(
                        #[weak]
                        imp,
                        async move {
                            if let Some(current_identity) = node.identity().await {
                                imp.display_name_row.set_text(&current_identity.name);
                            }
                        }
                    ));
                    imp.display_name_row
                        .connect_apply(clone!(move |display_name_row| {
                            ctx.spawn_local(clone!(
                                #[weak]
                                display_name_row,
                                async move {
                                    if let Err(e) =
                                        node.set_display_name(&display_name_row.text().into()).await
                                    {
                                        error!("{}", e);
                                    }
                                }
                            ));
                        }));
                }
                false => imp.display_name_row.set_text(""),
            }
            imp.display_name_row.set_sensitive(home_replica_set);
        }
    }

    pub async fn save_exported_user(&self) -> miette::Result<()> {
        let node = NODE
            .get()
            .ok_or(miette::miette!("Oku node has not yet started … "))?;
        let exported_user_toml = node.export_user_toml().await?;
        let dialog = libadwaita::AlertDialog::new(
            Some("Export user credentials?"),
            Some("Do not share your user credentials with anyone."),
        );
        dialog.add_responses(&[("cancel", "Cancel"), ("export", "Export")]);
        dialog.set_response_appearance("cancel", ResponseAppearance::Default);
        dialog.set_response_appearance("export", ResponseAppearance::Destructive);
        dialog.set_default_response(Some("cancel"));
        dialog.set_close_response("cancel");
        dialog.connect_response(
            None,
            clone!(
                #[weak(rename_to = this)]
                self,
                move |_, response| {
                    match response {
                        "cancel" => (),
                        "export" => {
                            let toml_filter = gtk::FileFilter::new();
                            toml_filter.add_pattern("*.toml");
                            let filter_store = gio::ListStore::new::<gtk::FileFilter>();
                            filter_store.append(&toml_filter);
                            let file_dialog = gtk::FileDialog::builder()
                                .accept_label("Export")
                                .initial_name("user.toml")
                                .filters(&filter_store)
                                .title("Select destination for exported user credentials.")
                                .build();
                            file_dialog.save(
                                Some(&get_window_from_widget(&this)),
                                Some(&gio::Cancellable::new()),
                                clone!(
                                    #[strong]
                                    exported_user_toml,
                                    move |destination| {
                                        let path = destination.ok().and_then(|x| x.path());
                                        if let Some(path) = path {
                                            if let Err(e) = std::fs::write(path, exported_user_toml)
                                            {
                                                error!("{}", e);
                                            }
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
        dialog.present(Some(self));
        Ok(())
    }

    pub async fn import_user(&self) -> miette::Result<()> {
        let node = NODE
            .get()
            .ok_or(miette::miette!("Oku node has not yet started … "))?;
        let dialog = libadwaita::AlertDialog::new(
            Some("Import user credentials?"),
            Some("This will overwrite your existing user credentials. This cannot be undone."),
        );
        dialog.add_responses(&[("cancel", "Cancel"), ("import", "Import")]);
        dialog.set_response_appearance("cancel", ResponseAppearance::Default);
        dialog.set_response_appearance("import", ResponseAppearance::Destructive);
        dialog.set_default_response(Some("cancel"));
        dialog.set_close_response("cancel");
        dialog.connect_response(
            None,
            clone!(
                #[weak(rename_to = this)]
                self,
                move |_, response| {
                    match response {
                        "cancel" => (),
                        "import" => {
                            let toml_filter = gtk::FileFilter::new();
                            toml_filter.add_pattern("*.toml");
                            let filter_store = gio::ListStore::new::<gtk::FileFilter>();
                            filter_store.append(&toml_filter);
                            let file_dialog = gtk::FileDialog::builder()
                                .accept_label("Import")
                                .initial_name("user.toml")
                                .filters(&filter_store)
                                .title("Select destination for exported user credentials.")
                                .build();
                            file_dialog.open(
                                Some(&get_window_from_widget(&this)),
                                Some(&gio::Cancellable::new()),
                                clone!(
                                    #[strong]
                                    node,
                                    #[weak]
                                    this,
                                    move |destination| {
                                        let exported_user_toml = destination
                                            .ok()
                                            .and_then(|x| x.path())
                                            .and_then(|x| std::fs::read_to_string(x).ok());
                                        this.imp().okunet_group.set_sensitive(false);
                                        if let Some(exported_user_toml) = exported_user_toml {
                                            glib::spawn_future_local(clone!(async move {
                                                if let Err(e) =
                                                    node.import_user_toml(&exported_user_toml).await
                                                {
                                                    error!("{}", e);
                                                }
                                            }));
                                        }
                                        this.imp().okunet_group.set_sensitive(true);
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
        dialog.present(Some(self));
        Ok(())
    }
}
