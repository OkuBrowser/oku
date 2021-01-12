/*
    This file is part of Oku.

    Oku is free software: you can redistribute it and/or modify
    it under the terms of the GNU Affero General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    Oku is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU Affero General Public License for more details.

    You should have received a copy of the GNU Affero General Public License
    along with Oku.  If not, see <https://www.gnu.org/licenses/>.
*/

use directories_next::ProjectDirs;
use gio::prelude::*;
use glib::clone;
use glib::Cast;
use gtk::prelude::BuilderExtManual;
use gtk::prelude::NotebookExtManual;
use gtk::BoxExt;
use gtk::ButtonExt;
use gtk::ContainerExt;
use gtk::EntryExt;
use gtk::GtkWindowExt;
use gtk::IconSize::Button;
use gtk::Inhibit;
use gtk::LabelExt;
use gtk::NotebookExt;
use gtk::Orientation::Horizontal;
use gtk::WidgetExt;
use pango::EllipsizeMode;
use percent_encoding::percent_decode_str;
use std::env::args;
use webkit2gtk::SettingsExt;
use webkit2gtk::WebContextExt;
use webkit2gtk::WebViewExt;

#[macro_use]
extern crate lazy_static;

lazy_static! {
    static ref PROJECT_DIRECTORIES: ProjectDirs =
        ProjectDirs::from("org", "Emil Sayahi", "Oku").unwrap();
}
const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");

/// Connect to a page using the current tab
///
/// # Arguments
///
/// * `nav_entry` - The navigation bar of the browser
///
/// * `web_view` - The WebKit instance for the current tab
fn connect(nav_entry: &gtk::Entry, web_view: &webkit2gtk::WebView) {
    let mut nav_text = nav_entry.get_text().to_string();

    if !nav_text.contains("://") {
        nav_text = format!("http://{}", nav_text);
    }

    if nav_text.starts_with("ipfs://") {
        let hash = nav_text.replacen("ipfs://", "", 1);
        let decoded_hash = percent_decode_str(&hash).decode_utf8().unwrap().to_string();
        let split_hash: Vec<&str> = decoded_hash.split('/').collect();
        let path = &decoded_hash
            .replacen(split_hash[0], "", 1)
            .replacen('/', "", 1);
        let gateway_url = format!("http://{}.ipfs.localhost:8080/{}", split_hash[0], path);
        web_view.load_uri(&gateway_url);
    } else {
        web_view.load_uri(&nav_text);
    }
}

/// Update the contents of the navigation bar
///
/// # Arguments
///
/// * `nav_entry` - The navigation bar of the browser
///
/// * `web_view` - The WebKit instance for the current tab
fn update_nav_bar(nav_entry: &gtk::Entry, web_view: &webkit2gtk::WebView) {
    let mut url = web_view.get_uri().unwrap().to_string();
    let cid = url
        .replacen("http://", "", 1)
        .replacen(".ipfs.localhost:8080", "", 1);
    let split_cid: Vec<&str> = cid.split('/').collect();
    if url.starts_with(&format!("http://{}.ipfs.localhost:8080/", split_cid[0])) {
        url = url
            .replacen("http://", "ipfs://", 1)
            .replacen(".ipfs.localhost:8080", "", 1);
    }
    nav_entry.set_text(&url);
}

/// Create a new WebKit instance for the current tab
///
/// # Arguments
///
/// * `builder` - The object that contains all graphical widgets of the window
fn new_view(builder: &gtk::Builder) -> webkit2gtk::WebView {
    let web_kit = webkit2gtk::WebViewBuilder::new();
    let web_settings: webkit2gtk::Settings = builder.get_object("webkit_settings").unwrap();
    let web_view = web_kit.build();
    web_settings.set_user_agent_with_application_details(Some("Oku"), Some(VERSION.unwrap()));
    web_view.set_settings(&web_settings);
    let extensions_path = format!(
        "{}/web-extensions/",
        PROJECT_DIRECTORIES.data_dir().to_str().unwrap()
    );
    web_view
        .get_context()
        .unwrap()
        .set_web_extensions_directory(&extensions_path);
    web_view.set_visible(true);
    web_view.set_property_width_request(1024);
    web_view.set_property_height_request(640);
    web_view.load_uri("about:blank");
    web_view
}

/// Create the text to be displayed on a tab
///
/// # Arguments
///
/// * `label` - The text to be displayed on a tab
fn new_tab_label(label: &str) -> gtk::Label {
    let tab_label = gtk::Label::new(Some(label));
    tab_label.set_hexpand(true);
    tab_label.set_ellipsize(EllipsizeMode::End);
    tab_label.set_visible(true);
    tab_label
}

/// Create a tab to be placed in the notebook
///
/// # Arguments
///
/// * `label` - The text to be displayed on a tab
fn new_tab(label: &str) -> gtk::Box {
    let tab_box = gtk::Box::new(Horizontal, 4);
    tab_box.set_hexpand(true);
    tab_box.set_visible(true);
    let tab_label = new_tab_label(&label);
    let close_button = gtk::Button::new();
    close_button.set_visible(true);
    let close_icon = gtk::Image::from_icon_name(Some("list-remove"), Button);
    close_button.set_image(Some(&close_icon));
    close_icon.set_visible(true);
    tab_box.add(&tab_label);
    tab_box.add(&close_button);
    tab_box
}

/// Create a new entry in the notebook
///
/// # Arguments
///
/// * `builder` - The object that contains all graphical widgets of the window
///
/// * `tabs` - The notebook containing the tabs & pages of the current browser session
///
/// * `new_tab_number` - A number representing the position in the notebook where this new entry should
fn new_tab_page(
    builder: &gtk::Builder,
    tabs: &gtk::Notebook,
    new_tab_number: u32,
) -> webkit2gtk::WebView {
    let new_view = new_view(&builder);
    tabs.insert_page(&new_view, Some(&new_tab("New Tab")), Some(new_tab_number));
    tabs.set_tab_reorderable(&new_view, true);
    tabs.set_tab_detachable(&new_view, true);
    tabs.set_current_page(Some(new_tab_number));
    new_view
}

/// Get the WebKit instance for the current tab
///
/// # Arguments
///
/// * `tabs` - The notebook containing the tabs & pages of the current browser session
fn get_view(tabs: &gtk::Notebook) -> webkit2gtk::WebView {
    tabs.get_nth_page(Some(tabs.get_current_page().unwrap()))
        .unwrap()
        .downcast()
        .unwrap()
}

/// Create an initial tab, for when the notebook is empty
///
/// # Arguments
///
/// * `builder` - The object that contains all graphical widgets of the window
///
/// * `tabs` - The notebook containing the tabs & pages of the current browser session
fn initial_tab(builder: &gtk::Builder, tabs: &gtk::Notebook) {
    let web_view = new_tab_page(&builder, &tabs, 0);
    let current_tab_label: gtk::Box = tabs.get_tab_label(&web_view).unwrap().downcast().unwrap();
    let close_button_widget = &current_tab_label.get_children()[1];
    let close_button: gtk::Button = close_button_widget.clone().downcast().unwrap();
    close_button.connect_clicked(clone!(@weak tabs, @weak web_view => move |_| {
        tabs.remove_page(tabs.page_num(&web_view));
    }));
}

/// The main function of Oku.
fn main() {
    let application = gtk::Application::new(Some("com.github.madebyemil.oku"), Default::default())
        .expect("Initialization failed … ");

    application.connect_activate(|app| {
        new_window(app);
    });

    application.run(&args().collect::<Vec<_>>());
}

/// Creates a new functional & graphical browser window.
fn new_window(application: &gtk::Application) {
    if gtk::init().is_err() {
        println!("Failed to initialize GTK.");
        return;
    }

    let glade_src = include_str!("window.glade");
    let builder = gtk::Builder::from_string(glade_src);

    let window: gtk::ApplicationWindow = builder.get_object("window").unwrap();
    let _downloads_button: gtk::Button = builder.get_object("downloads_button").unwrap();
    let back_button: gtk::Button = builder.get_object("back_button").unwrap();
    let forward_button: gtk::Button = builder.get_object("forward_button").unwrap();
    let refresh_button: gtk::Button = builder.get_object("refresh_button").unwrap();
    let add_tab: gtk::Button = builder.get_object("add_tab").unwrap();
    let tabs: gtk::Notebook = builder.get_object("tabs").unwrap();
    let nav_entry: gtk::Entry = builder.get_object("nav_entry").unwrap();

    window.set_application(Some(application));

    if tabs.get_n_pages() == 0 {
        initial_tab(&builder, &tabs)
    }

    tabs.connect_property_page_notify(
        clone!(@weak nav_entry, @weak builder, @weak tabs => move |_| {
            let web_view = get_view(&tabs);
            update_nav_bar(&nav_entry, &web_view);
        }),
    );

    tabs.connect_page_removed(
        clone!(@weak nav_entry, @weak builder, @weak tabs => move |_, _, _| {
            if tabs.get_n_pages() == 0
            {
                nav_entry.set_text("");
                initial_tab(&builder, &tabs)
            }
        }),
    );

    nav_entry.connect_activate(clone!(@weak tabs, @weak nav_entry, @weak builder => move |_| {
        let web_view = get_view(&tabs);
        connect(&nav_entry, &web_view);
        web_view.connect_property_title_notify(clone!(@weak tabs => move |_| {
            let web_view = get_view(&tabs);
            let current_tab_label: gtk::Box = tabs.get_tab_label(&web_view).unwrap().downcast().unwrap();
            let new_label_text = new_tab_label(&web_view.get_title().unwrap());
            current_tab_label.remove(&current_tab_label.get_children()[0]);
            current_tab_label.add(&new_label_text);
            current_tab_label.reorder_child(&new_label_text, 0)
        }));
        web_view.connect_property_uri_notify(clone!(@weak tabs, @weak nav_entry => move |_| {
            let web_view = get_view(&tabs);
            update_nav_bar(&nav_entry, &web_view)
        }));
        web_view.connect_load_changed(clone!(@weak tabs, @weak nav_entry => move |_, _| {
            let web_view = get_view(&tabs);

            let load_progress = web_view.get_estimated_load_progress();
            if load_progress == 1.00
            {
                nav_entry.set_progress_fraction(0.00)
            } else {
                nav_entry.set_progress_fraction(load_progress)
            }

            update_nav_bar(&nav_entry, &web_view)
        }));
    }));

    add_tab.connect_clicked(clone!(@weak tabs, @weak nav_entry, @weak builder => move |_| {
        let web_view = new_tab_page(&builder, &tabs, tabs.get_n_pages());
        let current_tab_label: gtk::Box = tabs.get_tab_label(&web_view).unwrap().downcast().unwrap();
        let close_button_widget = &current_tab_label.get_children()[1];
        let close_button: gtk::Button = close_button_widget.clone().downcast().unwrap();
        close_button.connect_clicked(clone!(@weak tabs => move |_| {
            tabs.remove_page(tabs.page_num(&web_view));
        }));
    }));

    back_button.connect_clicked(
        clone!(@weak tabs, @weak nav_entry, @weak builder => move |_| {
            let web_view = get_view(&tabs);
            web_view.go_back()
        }),
    );

    forward_button.connect_clicked(
        clone!(@weak tabs, @weak nav_entry, @weak builder => move |_| {
            let web_view = get_view(&tabs);
            web_view.go_forward()
        }),
    );

    refresh_button.connect_clicked(
        clone!(@weak tabs, @weak nav_entry, @weak builder => move |_| {
            let web_view = get_view(&tabs);
            web_view.reload_bypass_cache()
        }),
    );

    window.show_all();
    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });
    gtk::main();
}
