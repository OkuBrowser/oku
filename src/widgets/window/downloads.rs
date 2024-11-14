use super::*;
use crate::window_util::{get_view_stack_page_by_name, get_window_from_widget};
use glib::clone;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};
use libadwaita::{prelude::*, ResponseAppearance};
use std::cell::Ref;
use std::rc::Rc;
use webkit2gtk::Download;

impl Window {
    pub fn downloads_store(&self) -> Ref<gio::ListStore> {
        let downloads_store = self.imp().downloads_store.borrow();

        Ref::map(downloads_store, |downloads_store| {
            let downloads_store = downloads_store.as_deref().unwrap();
            downloads_store
        })
    }

    pub fn setup_downloads_page(&self) {
        let imp = self.imp();

        let downloads_store = gio::ListStore::new::<Download>();
        imp.downloads_store.replace(Some(Rc::new(downloads_store)));

        imp.downloads_model
            .set_model(Some(&self.downloads_store().clone()));
        imp.downloads_model.set_autoselect(false);
        imp.downloads_model.set_can_unselect(true);
        imp.downloads_model
            .connect_selected_item_notify(clone!(move |downloads_model| {
                downloads_model.unselect_all();
            }));

        imp.downloads_factory.connect_setup(clone!(move |_, item| {
            let list_item = item.downcast_ref::<gtk::ListItem>().unwrap();
            let row = crate::widgets::download_row::DownloadRow::default();
            list_item
                .property_expression("item")
                .bind(&row, "download", gtk::Widget::NONE);
            list_item.set_child(Some(&row));
        }));

        imp.downloads_view.set_model(Some(&imp.downloads_model));
        imp.downloads_view.set_factory(Some(&imp.downloads_factory));
        imp.downloads_view.set_enable_rubberband(false);
        imp.downloads_view
            .set_hscroll_policy(gtk::ScrollablePolicy::Minimum);
        imp.downloads_view
            .set_vscroll_policy(gtk::ScrollablePolicy::Natural);
        imp.downloads_view.set_vexpand(true);
        imp.downloads_view.add_css_class("boxed-list-separate");
        imp.downloads_view.add_css_class("navigation-sidebar");

        imp.downloads_scrolled_window
            .set_child(Some(&imp.downloads_view));
        imp.downloads_scrolled_window
            .set_hscrollbar_policy(gtk::PolicyType::Never);
        imp.downloads_scrolled_window
            .set_propagate_natural_height(true);
        imp.downloads_scrolled_window
            .set_propagate_natural_width(true);

        imp.downloads_label.set_label("Downloads");
        imp.downloads_label.set_margin_top(24);
        imp.downloads_label.set_margin_bottom(24);
        imp.downloads_label.add_css_class("title-1");

        imp.downloads_box
            .set_orientation(gtk::Orientation::Vertical);
        imp.downloads_box.set_spacing(4);
        imp.downloads_box.append(&imp.downloads_label);
        imp.downloads_box.append(&imp.downloads_scrolled_window);

        imp.side_view_stack.add_titled_with_icon(
            &imp.downloads_box,
            Some("downloads"),
            "Downloads",
            "arrow-pointing-at-line-down-symbolic",
        );
    }

    pub fn setup_network_session(&self) {
        self.imp()
            .network_session
            .borrow()
            .connect_download_started(clone!(
                #[weak(rename_to = this)]
                self,
                move |_network_session, download| {
                    let window = match download.web_view() {
                        Some(web_view) => get_window_from_widget(&web_view),
                        None => this.clone(),
                    };
                    download.connect_decide_destination(clone!(
                        #[weak]
                        window,
                        #[upgrade_or]
                        false,
                        move |download, suggested_filename| {
                            let file_uri = download.request().unwrap().uri().unwrap();
                            let dialog = libadwaita::AlertDialog::new(
                                Some("Download file?"),
                                Some(&format!("Would you like to download '{}'?", file_uri)),
                            );
                            dialog.add_responses(&[("cancel", "Cancel"), ("download", "Download")]);
                            dialog.set_response_appearance("cancel", ResponseAppearance::Default);
                            dialog
                                .set_response_appearance("download", ResponseAppearance::Suggested);
                            dialog.set_default_response(Some("cancel"));
                            dialog.set_close_response("cancel");
                            let suggested_filename = suggested_filename.to_string();
                            dialog.connect_response(
                                None,
                                clone!(
                                    #[weak]
                                    window,
                                    #[weak]
                                    download,
                                    move |_, response| {
                                        match response {
                                            "cancel" => download.cancel(),
                                            "download" => {
                                                download.set_allow_overwrite(true);
                                                let file_dialog = gtk::FileDialog::builder()
                                                    .accept_label("Download")
                                                    .initial_name(suggested_filename.clone())
                                                    .initial_folder(&gio::File::for_path(
                                                        glib::user_special_dir(
                                                            glib::enums::UserDirectory::Downloads,
                                                        )
                                                        .unwrap(),
                                                    ))
                                                    .title(format!(
                                                        "Select destination for '{}'",
                                                        suggested_filename.clone()
                                                    ))
                                                    .build();
                                                file_dialog.save(
                                                    Some(&window),
                                                    Some(&gio::Cancellable::new()),
                                                    clone!(
                                                        #[weak]
                                                        download,
                                                        move |destination| {
                                                            if let Ok(destination) = destination {
                                                                download.set_destination(
                                                                    destination
                                                                        .path()
                                                                        .unwrap()
                                                                        .to_str()
                                                                        .unwrap(),
                                                                )
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
                            dialog.present(Some(&window));
                            true
                        }
                    ));
                    download.connect_created_destination(clone!(
                        #[weak]
                        this,
                        move |download, _| {
                            let imp = this.imp();
                            this.downloads_store().append(download);
                            if let Some(downloads_page) = get_view_stack_page_by_name(
                                "downloads".to_string(),
                                &imp.side_view_stack,
                            ) {
                                downloads_page.set_needs_attention(true)
                            }
                        }
                    ));
                }
            ));
    }
}
