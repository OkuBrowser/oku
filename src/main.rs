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
use futures::TryStreamExt;
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
use gtk::IconSize;
use gtk::ImageExt;
use gtk::Inhibit;
use gtk::LabelExt;
use gtk::NotebookExt;
use gtk::Orientation::Horizontal;
use gtk::PopoverExt;
use gtk::WidgetExt;
use ipfs_api::IpfsClient;
use pango::EllipsizeMode;
use std::convert::TryFrom;
use std::env::args;
use std::fs;
use std::path::Path;
use urlencoding::decode;
use webkit2gtk::DownloadExt;
use webkit2gtk::SettingsExt;
use webkit2gtk::URIRequestExt;
use webkit2gtk::URISchemeRequest;
use webkit2gtk::URISchemeRequestExt;
use webkit2gtk::WebContextExt;
use webkit2gtk::WebViewExt;

#[macro_use]
extern crate lazy_static;

lazy_static! {
    /// The platform-specific directories intended for Oku's use
    static ref PROJECT_DIRECTORIES: ProjectDirs =
        ProjectDirs::from("org", "Emil Sayahi", "Oku").unwrap();
    /// The platform-specific directory where Oku caches data
    static ref CACHE_DIR: &'static str = PROJECT_DIRECTORIES.cache_dir().to_str().unwrap();
    /// The platform-specific directory where Oku stores user data
    static ref DATA_DIR: &'static str = PROJECT_DIRECTORIES.data_dir().to_str().unwrap();
}
/// The current release version number of Oku
const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");

struct DownloadItem {
    source: String,
    destination: String,
    requested_time: String,
    successful: bool,
}

/// Connect to a page using the current tab
///
/// # Arguments
///
/// * `nav_entry` - The navigation bar of the browser
///
/// * `web_view` - The WebKit instance for the current tab
fn connect(nav_entry: &gtk::Entry, web_view: &webkit2gtk::WebView) {
    let mut nav_text = nav_entry.get_text().to_string();

    if !nav_text.contains("://") && !nav_text.starts_with("about:") {
        nav_text = format!("http://{}", nav_text);
    }

    web_view.load_uri(&nav_text);
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

/// Comply with a request using the IPFS scheme
///
/// # Arguments
///
/// `request` - The request from the browser for the IPFS resource
fn handle_ipfs_request(request: &URISchemeRequest) {
    let client = IpfsClient::default();
    let request_url = request.get_uri().unwrap().to_string();
    let decoded_url = decode(&request_url).unwrap();
    let ipfs_path = decoded_url.replacen("ipfs://", "", 1);
    let local_path = format!("{}/{}", CACHE_DIR.to_string(), ipfs_path);
    get_from_hash(client, ipfs_path, local_path.to_owned());
    let file = gio::File::new_for_path(&local_path);
    let stream = file.read(gio::NONE_CANCELLABLE);

    match stream {
        Ok(_) => request.finish(&stream.unwrap(), -1, None),
        Err(e) => {
            eprintln!(
                "\nFailed to obtain page: {}\nError: {:#?}\n",
                decoded_url, e
            );
            request.finish(&gio::MemoryInputStream::new(), -1, None);
        }
    }
}

/// Create a new WebKit instance for the current tab
///
/// # Arguments
///
/// * `builder` - The object that contains all graphical widgets of the window
///
/// * `download_dialog` - The dialog box shown when a download is requested
///
/// * `is_private` - Whether the window represents a private session
fn new_view(
    builder: &gtk::Builder,
    download_dialog: &gtk::Dialog,
    is_private: bool,
) -> webkit2gtk::WebView {
    let web_kit = webkit2gtk::WebViewBuilder::new()
        .is_ephemeral(is_private)
        .automation_presentation_type(webkit2gtk::AutomationBrowsingContextPresentation::Tab);
    let web_settings: webkit2gtk::Settings = builder.get_object("webkit_settings").unwrap();
    let web_view = web_kit.build();
    let web_context = web_view.get_context().unwrap();
    let extensions_path = format!("{}/web-extensions/", DATA_DIR.to_string());
    let favicon_database_path = format!("{}/favicon-database/", CACHE_DIR.to_string());
    web_context.register_uri_scheme("ipfs", move |request| handle_ipfs_request(request));
    web_settings.set_user_agent_with_application_details(Some("Oku"), Some(VERSION.unwrap()));
    web_view.set_settings(&web_settings);
    web_context.set_web_extensions_directory(&extensions_path);
    web_context.set_favicon_database_directory(Some(&favicon_database_path));
    web_context.connect_download_started(clone!(@weak download_dialog => move |_, download| {
        let dialog_box_widget = &download_dialog.get_children()[0];
        let dialog_box: gtk::Box = dialog_box_widget.clone().downcast().unwrap();

        let outer_button_box_widget = &dialog_box.get_children()[1];
        let outer_button_box: gtk::Box = outer_button_box_widget.clone().downcast().unwrap();
        let inner_button_box_widget = &outer_button_box.get_children()[0];
        let inner_button_box: gtk::ButtonBox = inner_button_box_widget.clone().downcast().unwrap();
        let cancel_button_widget = &inner_button_box.get_children()[0];
        let save_button_widget = &inner_button_box.get_children()[1];
        let cancel_button: gtk::Button = cancel_button_widget.clone().downcast().unwrap();
        let save_button: gtk::Button = save_button_widget.clone().downcast().unwrap();

        let message_box_widget = &dialog_box.get_children()[0];
        let message_box: gtk::Box = message_box_widget.clone().downcast().unwrap();
        let message_widget = &message_box.get_children()[1];
        let message: gtk::Label = message_widget.clone().downcast().unwrap();

        message.set_text(&download.get_request().unwrap().get_uri().unwrap());

        download_dialog.show_all();
    }));
    web_view.set_visible(true);
    web_view.set_property_width_request(1024);
    web_view.set_property_height_request(640);
    web_view.load_uri("about:blank");
    web_view
}

/// Get an IPFS file asynchronously
///
/// # Arguments
///
/// `client` - The IPFS client running locally
///
/// `hash` - The IPFS identifier of the file
///
/// `local_directory` - Where to save the IPFS file locally
fn get_from_hash(client: IpfsClient, hash: String, local_directory: String) {
    let mut sys = actix_rt::System::new(format!("Oku IPFS System ({})", hash));
    sys.block_on(async move {
        download_ipfs_file(&client, hash.to_owned(), local_directory.to_owned()).await;
        // println!(
        //     "Requesting: {} (local: {}) … \n",
        //     hash.to_owned(),
        //     local_directory.to_owned()
        // );
    });
}

/// Download an IPFS file to the local machine
///
/// # Arguments
///
/// `client` - The IPFS client running locally
///
/// `file_hash` - The CID of the folder the file is in
///
/// `file_path` - The path to the file from the root of the folder
async fn download_ipfs_file(client: &IpfsClient, file_hash: String, file_path: String) {
    match client
        .cat(&file_hash)
        .map_ok(|chunk| chunk.to_vec())
        .try_concat()
        .await
    {
        Ok(res) => {
            fs::create_dir_all(Path::new(&file_path[..]).parent().unwrap()).unwrap();
            fs::write(file_path, &res).unwrap();
        }
        Err(_) => {
            let split_path: Vec<&str> = file_hash.split('/').collect();
            let rest_of_path = file_hash.replacen(split_path[0], "", 1);
            let public_url = format!("https://{}.ipfs.dweb.link{}", split_path[0], rest_of_path);
            let request = reqwest::get(&public_url).await;
            let request_body = request.unwrap().bytes().await;
            fs::create_dir_all(Path::new(&file_path[..]).parent().unwrap()).unwrap();
            fs::write(file_path, request_body.unwrap()).unwrap();
        }
    }
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
    let tab_box = gtk::Box::new(Horizontal, 5);
    let favicon = gtk::Image::from_icon_name(Some("applications-internet"), IconSize::Dnd);
    let tab_label = new_tab_label(&label);
    let close_button = gtk::Button::new();
    let close_icon = gtk::Image::from_icon_name(Some("list-remove"), IconSize::Button);
    favicon.set_visible(true);
    tab_box.set_hexpand(true);
    tab_box.set_vexpand(false);
    tab_box.set_visible(true);
    close_button.set_visible(true);
    close_button.set_image(Some(&close_icon));
    close_icon.set_visible(true);
    tab_box.add(&favicon);
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
///
/// * `is_private` - Whether the window represents a private session
fn new_tab_page(
    builder: &gtk::Builder,
    tabs: &gtk::Notebook,
    download_dialog: &gtk::Dialog,
    new_tab_number: u32,
    is_private: bool,
) -> webkit2gtk::WebView {
    let new_view = new_view(builder, download_dialog, is_private);
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
///
/// * `is_private` - Whether the window represents a private session
fn create_initial_tab(
    builder: &gtk::Builder,
    tabs: &gtk::Notebook,
    download_dialog: &gtk::Dialog,
    is_private: bool,
) {
    let web_view = new_tab_page(&builder, &tabs, &download_dialog, 0, is_private);
    let current_tab_label: gtk::Box = tabs.get_tab_label(&web_view).unwrap().downcast().unwrap();
    let close_button_widget = &current_tab_label.get_children()[2];
    let close_button: gtk::Button = close_button_widget.clone().downcast().unwrap();
    close_button.connect_clicked(clone!(@weak tabs, @weak web_view => move |_| {
        tabs.remove_page(tabs.page_num(&web_view));
    }));
}

/// Update a tab's icon
///
/// # Arguments
///
/// * `web_view` - The WebKit instance for the tab
///
/// * `tabs` - The notebook containing the tabs & pages of the current browser session
fn update_favicon(web_view: &webkit2gtk::WebView, tabs: &gtk::Notebook) {
    let current_tab_label: gtk::Box = tabs.get_tab_label(web_view).unwrap().downcast().unwrap();
    let favicon_widget = &current_tab_label.get_children()[0];
    let favicon: gtk::Image = favicon_widget.clone().downcast().unwrap();
    let web_favicon = &web_view.get_favicon();
    match &web_favicon {
        Some(_) => {
            let favicon_surface =
                cairo::ImageSurface::try_from(web_favicon.to_owned().unwrap()).unwrap();
            let favicon_width = favicon_surface.get_width();
            let favicon_height = favicon_surface.get_height();
            match favicon_width < 32 && favicon_height < 32 {
                true => {
                    favicon.set_from_surface(Some(&favicon_surface));
                }
                false => {
                    let favicon_pixbuf = gdk::pixbuf_get_from_surface(
                        &favicon_surface,
                        0,
                        0,
                        favicon_width,
                        favicon_height,
                    )
                    .unwrap();
                    let scaled_pixbuf = favicon_pixbuf
                        .scale_simple(32, 32, gdk_pixbuf::InterpType::Tiles)
                        .unwrap();
                    favicon.set_from_pixbuf(Some(&scaled_pixbuf));
                }
            }
        }
        None => {
            favicon.set_from_icon_name(Some("applications-internet"), IconSize::Dnd);
        }
    }
}

/// Update a tab's title
///
/// # Arguments
///
/// * `web_view` - The WebKit instance for the tab
///
/// * `tabs` - The notebook containing the tabs & pages of the current browser session
fn update_title(web_view: &webkit2gtk::WebView, tabs: &gtk::Notebook) {
    let current_tab_label: gtk::Box = tabs.get_tab_label(web_view).unwrap().downcast().unwrap();
    let new_label_text = new_tab_label(&web_view.get_title().unwrap());
    current_tab_label.remove(&current_tab_label.get_children()[1]);
    current_tab_label.add(&new_label_text);
    current_tab_label.reorder_child(&new_label_text, 1);
}

/// Update the load progress indicator under the navigation bar
///
/// # Arguments
///
/// * `nav_entry` - The navigation bar of the browser
///
/// * `web_view` - The WebKit instance for the current tab
fn update_load_progress(nav_entry: &gtk::Entry, web_view: &webkit2gtk::WebView) {
    let load_progress = web_view.get_estimated_load_progress();
    if load_progress as i64 == 1 {
        nav_entry.set_progress_fraction(0.00)
    } else {
        nav_entry.set_progress_fraction(load_progress)
    }
}

/// The main function of Oku
fn main() {
    let application = gtk::Application::new(Some("com.github.madebyemil.oku"), Default::default())
        .expect("Initialization failed … ");

    application.connect_activate(|app| {
        new_window(app, false);
    });

    application.run(&args().collect::<Vec<_>>());
}

/// Create a new functional & graphical browser window
///
/// # Arguments
///
/// * `application` - The application data representing Oku
///
/// * `is_private` - Whether the window represents a private session
fn new_window(application: &gtk::Application, is_private: bool) {
    if gtk::init().is_err() {
        println!("Failed to initialize GTK.");
        return;
    }

    let glade_src = include_str!("oku.glade");
    let builder = gtk::Builder::from_string(glade_src);

    let window: gtk::ApplicationWindow = builder.get_object("window").unwrap();
    window.set_title("Oku");
    let downloads_button: gtk::Button = builder.get_object("downloads_button").unwrap();
    let downloads_popover: gtk::Popover = builder.get_object("downloads_popover").unwrap();
    let download_dialog: gtk::Dialog = builder.get_object("download_dialog").unwrap();
    let back_button: gtk::Button = builder.get_object("back_button").unwrap();
    let forward_button: gtk::Button = builder.get_object("forward_button").unwrap();
    let refresh_button: gtk::Button = builder.get_object("refresh_button").unwrap();
    let add_tab: gtk::Button = builder.get_object("add_tab").unwrap();
    let tabs: gtk::Notebook = builder.get_object("tabs").unwrap();
    let nav_entry: gtk::Entry = builder.get_object("nav_entry").unwrap();

    window.set_application(Some(application));

    if tabs.get_n_pages() == 0 {
        create_initial_tab(&builder, &tabs, &download_dialog, is_private)
    }

    tabs.connect_property_page_notify(
        clone!(@weak nav_entry, @weak builder, @weak tabs, @weak window => move |_| {
            let web_view = get_view(&tabs);
            update_nav_bar(&nav_entry, &web_view);
            window.set_title(&web_view.get_title().unwrap_or_else(|| glib::GString::from("Oku")));
        }),
    );

    tabs.connect_page_removed(
        clone!(@weak nav_entry, @weak builder, @weak tabs, @weak download_dialog => move |_, _, _| {
            if tabs.get_n_pages() == 0
            {
                nav_entry.set_text("");
                create_initial_tab(&builder, &tabs, &download_dialog, is_private)
            }
        }),
    );

    nav_entry.connect_activate(clone!(@weak tabs, @weak nav_entry, @weak builder, @weak window => move |_| {
        let web_view = get_view(&tabs);
        connect(&nav_entry, &web_view);
        web_view.connect_property_title_notify(clone!(@weak tabs, @weak web_view => move |_| {
            update_title(&web_view, &tabs)
        }));
        web_view.connect_property_uri_notify(clone!(@weak tabs, @weak web_view, @weak nav_entry => move |_| {
            update_nav_bar(&nav_entry, &web_view)
        }));
        web_view.connect_property_estimated_load_progress_notify(clone!(@weak tabs, @weak web_view, @weak nav_entry => move |_| {
            update_load_progress(&nav_entry, &web_view)
        }));
        web_view.connect_property_favicon_notify(clone!(@weak tabs, @weak web_view => move |_| {
            update_favicon(&web_view, &tabs)
        }));
        web_view.connect_load_changed(clone!(@weak tabs, @weak web_view, @weak nav_entry, @weak window => move |_, _| {
            window.set_title(&web_view.get_title().unwrap_or_else(|| glib::GString::from("Oku")));
        }));
    }));

    add_tab.connect_clicked(clone!(@weak tabs, @weak nav_entry, @weak builder, @weak download_dialog => move |_| {
        let web_view = new_tab_page(&builder, &tabs, &download_dialog, tabs.get_n_pages(), is_private);
        let current_tab_label: gtk::Box = tabs.get_tab_label(&web_view).unwrap().downcast().unwrap();
        let close_button_widget = &current_tab_label.get_children()[2];
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

    downloads_button.connect_clicked(
        clone!(@weak tabs, @weak nav_entry, @weak builder => move |_| {
            downloads_popover.popup();
        }),
    );

    window.show_all();
    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });
    gtk::main();
}
