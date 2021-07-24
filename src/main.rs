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



use glib::OptionArg;
use glib::OptionFlags;
use glib::VariantDict;
use glib::VariantTy;
use gtk::prelude::EditableExt;
use ipfs::Types;
use webkit2gtk::URISchemeRequest;
use webkit2gtk::traits::SettingsExt;
use std::path::PathBuf;
use ipfs::Keypair;

use tokio::stream::StreamExt;
use ipfs::IpfsPath;
use ipfs::UninitializedIpfs;

use ipfs::Ipfs;
use ipfs::IpfsOptions;
use cid::Cid;
use url::Url;
use url::ParseError;
use gtk::prelude::ToggleButtonExt;
use webkit2gtk::FindController;
use gtk::SearchEntry;
use chrono::Utc;
use directories_next::UserDirs;
use std::fs::File;
use gtk::AboutDialog;
use directories_next::ProjectDirs;
use futures::TryStreamExt;
use gio::prelude::*;
use glib::clone;
use glib::Cast;
use gtk::Builder;
use gtk::prelude::BoxExt;
use gtk::prelude::ButtonExt;
use gtk::prelude::EntryExt;
use gtk::prelude::GtkWindowExt;
use gtk::IconSize;
use gtk::Image;
use gtk::Inhibit;
use gtk::Label;
use gtk::Orientation::Horizontal;
use gtk::prelude::PopoverExt;
use gtk::prelude::WidgetExt;
use ipfs_api::IpfsClient;
use pango::EllipsizeMode;
use std::convert::TryFrom;
use std::env::args;
use urlencoding::decode;
use webkit2gtk::traits::*;
use webkit2gtk::{traits::{WebContextExt, WebViewExt, URISchemeRequestExt}, WebContext, WebView};
use libadwaita::TabBar;

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
    /// The platform-specific directories containing user files
    static ref USER_DIRECTORIES: UserDirs = UserDirs::new().unwrap();
    /// The platform-specific directory where users store pictures
    static ref PICTURES_DIR: &'static str = USER_DIRECTORIES.picture_dir().unwrap().to_str().unwrap();
}

/// The current release version number of Oku
const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");

struct DownloadItem {
    source: String,
    destination: String,
    requested_time: String,
    successful: bool,
}

/// Perform the initial connection at startup when passed a URL as a launch argument
///
/// * `initial_url` - The URL passed as a launch argument
///
/// * `web_view` - The WebKit instance for the current tab
fn initial_connect(mut initial_url: String, web_view: &webkit2gtk::WebView)
{
    let mut parsed_url = Url::parse(&initial_url);
    match parsed_url
    {
        // When URL is completely OK
        Ok(_) => {
            web_view.load_uri(&initial_url);
        }
        // When URL is missing a scheme
        Err(ParseError::RelativeUrlWithoutBase) => {
            parsed_url = Url::parse(&format!("http://{}", initial_url)); // Try with HTTP first
            match parsed_url
            {
                // If it's now valid with HTTP
                Ok(_) => {
                    let split_url: Vec<&str> = initial_url.split('/').collect();
                    let host = split_url[0];
                    let cid = Cid::try_from(host);
                    // Try seeing if we can swap it with IPFS
                    match cid
                    {
                        // It works as IPFS
                        Ok(_) => {
                            let unwrapped_cid = cid.unwrap();
                            let cid1 = Cid::new_v1(unwrapped_cid.codec(), unwrapped_cid.hash().to_owned());
                            parsed_url = Url::parse(&format!("ipfs://{}", initial_url));
                            let mut unwrapped_url = parsed_url.unwrap();
                            let cid1_string = &cid1.to_string_of_base(cid::multibase::Base::Base32Lower).unwrap();
                            unwrapped_url.set_host(Some(cid1_string)).unwrap();
                            initial_url = unwrapped_url.as_str().to_owned();
                            web_view.load_uri(&initial_url);
                        }
                        // It doesn't work as IPFS
                        Err(_) => {
                            initial_url = parsed_url.unwrap().as_str().to_owned();
                            web_view.load_uri(&initial_url);
                        }
                    }
                }
                // Still not valid, even with HTTP
                Err(e) => {
                    web_view.load_plain_text(&format!("{:#?}", e));
                }
            }
        }
        // URL is malformed beyond missing a scheme
        Err(e) => {
            web_view.load_plain_text(&format!("{:#?}", e));
        }
    }
}

/// Connect to a page using the current tab
///
/// # Arguments
///
/// * `nav_entry` - The navigation bar of the browser
///
/// * `web_view` - The WebKit instance for the current tab
fn connect(nav_entry: &gtk::Entry, web_view: &webkit2gtk::WebView) {
    let mut nav_text = nav_entry.text().to_string();

    let mut parsed_url = Url::parse(&nav_text);
    match parsed_url
    {
        // When URL is completely OK
        Ok(_) => {
            web_view.load_uri(&nav_text);
        }
        // When URL is missing a scheme
        Err(ParseError::RelativeUrlWithoutBase) => {
            parsed_url = Url::parse(&format!("http://{}", nav_text)); // Try with HTTP first
            match parsed_url
            {
                // If it's now valid with HTTP
                Ok(_) => {
                    let split_url: Vec<&str> = nav_text.split('/').collect();
                    let host = split_url[0];
                    let cid = Cid::try_from(host);
                    // Try seeing if we can swap it with IPFS
                    match cid
                    {
                        // It works as IPFS
                        Ok(_) => {
                            let unwrapped_cid = cid.unwrap();
                            let cid1 = Cid::new_v1(unwrapped_cid.codec(), unwrapped_cid.hash().to_owned());
                            parsed_url = Url::parse(&format!("ipfs://{}", nav_text));
                            let mut unwrapped_url = parsed_url.unwrap();
                            let cid1_string = &cid1.to_string_of_base(cid::multibase::Base::Base32Lower).unwrap();
                            unwrapped_url.set_host(Some(cid1_string)).unwrap();
                            nav_text = unwrapped_url.as_str().to_owned();
                            web_view.load_uri(&nav_text);
                            nav_entry.set_text(&nav_text);
                        }
                        // It doesn't work as IPFS
                        Err(_) => {
                            nav_text = parsed_url.unwrap().as_str().to_owned();
                            web_view.load_uri(&nav_text);
                            nav_entry.set_text(&nav_text)
                        }
                    }
                }
                // Still not valid, even with HTTP
                Err(e) => {
                    web_view.load_plain_text(&format!("{:#?}", e));
                }
            }
        }
        // URL is malformed beyond missing a scheme
        Err(e) => {
            web_view.load_plain_text(&format!("{:#?}", e));
        }
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
    let mut url = web_view.uri().unwrap().to_string();
    let cid = url
        .replacen("http://", "", 1)
        .replacen(".ipfs.localhost:8080", "", 1);
    let split_cid: Vec<&str> = cid.split('/').collect();
    if url.starts_with(&format!("http://{}.ipfs.localhost:8080/", split_cid[0])) {
        url = cid;
    }
    nav_entry.set_text(&url);
}

/// Comply with a request using the IPFS scheme
///
/// # Arguments
///
/// `request` - The request from the browser for the IPFS resource
fn handle_ipfs_request_using_api(request: &URISchemeRequest) {
    let request_url = request.uri().unwrap().to_string();
    let decoded_url = decode(&request_url).unwrap();
    let ipfs_path = decoded_url.replacen("ipfs://", "", 1);
    let ipfs_bytes = from_hash_using_api(ipfs_path);
    let stream = gio::MemoryInputStream::from_bytes(&glib::Bytes::from(&ipfs_bytes));
    request.finish(&stream, -1, None);
}

/// Comply with a request using the IPFS scheme natively
///
/// # Arguments
///
/// `request` - The request from the browser for the IPFS resource
fn handle_ipfs_request_natively(request: &URISchemeRequest) {
    let request_url = request.uri().unwrap().to_string();
    let decoded_url = decode(&request_url).unwrap();
    let ipfs_path = decoded_url.replacen("ipfs://", "", 1);
    let ipfs_bytes = from_hash_natively(ipfs_path);
    let stream = gio::MemoryInputStream::from_bytes(&glib::Bytes::from(&ipfs_bytes));
    request.finish(&stream, -1, None);
}

/// Create a new WebKit instance for the current tab
///
/// # Arguments
///
/// * `builder` - The object that contains all graphical widgets of the window
///  
/// * `verbose` - Whether browser messages should be printed onto the standard output
///
/// * `is_private` - Whether the window represents a private session
fn new_view(
    builder: &gtk::Builder,
    verbose: bool,
    is_private: bool,
    native: bool,
) -> webkit2gtk::WebView {
    let web_kit = webkit2gtk::WebViewBuilder::new()
        .is_ephemeral(is_private)
        .automation_presentation_type(webkit2gtk::AutomationBrowsingContextPresentation::Tab);
    let web_settings: webkit2gtk::Settings = builder.object("webkit_settings").unwrap();
    let web_view = web_kit.build();
    let web_context = web_view.context().unwrap();
    let extensions_path = format!("{}/web-extensions/", DATA_DIR.to_string());
    let favicon_database_path = format!("{}/favicon-database/", CACHE_DIR.to_string());

    match native
    {
        true => {
            web_context.register_uri_scheme("ipfs", move |request| handle_ipfs_request_natively(request));
        }
        false => {
            web_context.register_uri_scheme("ipfs", move |request| handle_ipfs_request_using_api(request));
        }
    };
    web_settings.set_user_agent_with_application_details(Some("Oku"), Some(VERSION.unwrap()));
    web_settings.set_enable_write_console_messages_to_stdout(verbose);
    web_view.set_settings(&web_settings);
    web_context.set_web_extensions_directory(&extensions_path);
    web_context.set_favicon_database_directory(Some(&favicon_database_path));
    web_view.set_visible(true);
    web_view.set_width_request(1024);
    web_view.set_height_request(640);
    web_view.load_uri("about:blank");

    web_view
}

/// Setup an IPFS node
async fn setup_native_ipfs() -> Ipfs<Types>
{
    // Initialize an in-memory repo and start a daemon.
    let opts = ipfs_options();
    let (ipfs, fut): (Ipfs<Types>, _) = UninitializedIpfs::new(opts).start().await.unwrap();

    // Spawn the background task
    tokio::task::spawn(fut);

    // Restore the default bootstrappers to enable content discovery
    ipfs.restore_bootstrappers().await.unwrap();

    ipfs
}

/// Get an IPFS file asynchronously using an existing IPFS node
///
/// # Arguments
///
/// `client` - The IPFS client running locally
///
/// `hash` - The IPFS identifier of the file
fn from_hash_using_api(hash: String) -> Vec<u8> {
    let mut sys = actix_rt::System::new(format!("Oku IPFS System ({})", hash));
    sys.block_on(download_ipfs_file_from_api(hash))
}

/// Get an IPFS file asynchronously using an in-memory IPFS node
///
/// # Arguments
///
/// `client` - The IPFS client running locally
///
/// `hash` - The IPFS identifier of the file
fn from_hash_natively(hash: String) -> Vec<u8> {
    let mut sys = actix_rt::System::new(format!("Oku IPFS System ({})", hash));
    sys.block_on(download_ipfs_file_natively(hash))
}

/// Download an IPFS file to the local machine using an existing IPFS node
///
/// # Arguments
///
/// `client` - The IPFS client running locally
///
/// `file_hash` - The CID of the file
async fn download_ipfs_file_from_api(file_hash: String) -> Vec<u8> {
    let client = IpfsClient::default();

    match client
        .cat(&file_hash)
        .map_ok(|chunk| chunk.to_vec())
        .try_concat()
        .await
    {
        Ok(res) => {
            res
        }
        Err(_) => {
            let split_path: Vec<&str> = file_hash.split('/').collect();
            let rest_of_path = file_hash.replacen(split_path[0], "", 1);
            let public_url = format!("https://{}.ipfs.dweb.link{}", split_path[0], rest_of_path);
            let request = reqwest::get(&public_url).await;
            let request_body = request.unwrap().bytes().await;
            request_body.unwrap().to_vec()
        }
    }
}

/// Download an IPFS file to the local machine using an in-memory IPFS node
///
/// # Arguments
///
/// `file_hash` - The CID of the file
async fn download_ipfs_file_natively(file_hash: String) -> Vec<u8> {
    let ipfs = setup_native_ipfs().await;

    // Get the IPFS file
    let path = file_hash
        .parse::<IpfsPath>()
        .unwrap();
    let stream = ipfs.cat_unixfs(path, None).await.unwrap();
    tokio::pin!(stream);
    let mut file_vec: Vec<u8> = vec!();
    loop {
        match stream.next().await {
            Some(Ok(bytes)) => {
                file_vec.extend(bytes);
            }
            Some(Err(e)) => {
                eprintln!("Error: {}", e);
            }
            None => break,
        }
    }
    file_vec
}

fn ipfs_options() -> ipfs::IpfsOptions
{
    IpfsOptions
    {
        ipfs_path: PathBuf::from(CACHE_DIR.to_owned()),
        keypair: Keypair::generate_ed25519(),
        mdns: true,
        bootstrap: Default::default(),
        kad_protocol: None,
        listening_addrs: vec!["/ip4/127.0.0.1/tcp/0".parse().unwrap()],
        span: None,
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

/// Create a tab to be placed in the TabBar
///
/// # Arguments
///
/// * `label` - The text to be displayed on a tab
fn new_tab(label: &str) -> gtk::Box {
    let tab_box = gtk::Box::new(Horizontal, 5);
    let favicon = gtk::Image::from_icon_name(Some("applications-internet"));
    let tab_label = new_tab_label(label);
    let close_button = gtk::Button::new();
    let close_icon = gtk::Image::from_icon_name(Some("list-remove"));
    favicon.set_visible(true);
    tab_box.set_hexpand(true);
    tab_box.set_vexpand(false);
    tab_box.set_visible(true);
    close_button.set_visible(true);
    close_button.set_child(Some(&close_icon));
    close_icon.set_visible(true);
    tab_box.append(&favicon);
    tab_box.append(&tab_label);
    tab_box.append(&close_button);
    tab_box
}

/// Create a new entry in the TabBar
///
/// # Arguments
///
/// * `builder` - The object that contains all graphical widgets of the window
///
/// * `tabs` - The TabBar containing the tabs & pages of the current browser session
///
/// * `new_tab_number` - A number representing the position in the TabBar where this new entry should
///  
/// * `verbose` - Whether browser messages should be printed onto the standard output
///
/// * `is_private` - Whether the window represents a private session
fn new_tab_page(
    builder: &gtk::Builder,
    tabs: &libadwaita::TabBar,
    new_tab_number: i32,
    verbose: bool,
    is_private: bool,
    native: bool,
) -> webkit2gtk::WebView {
    let new_view = new_view(builder, verbose, is_private, native);
    let tab_view = tabs.view().unwrap();
    let new_page = tab_view.add_page(&new_view, None).unwrap();
    new_page.set_title(Some("New Tab"));
    new_page.set_icon(Some(&gio::ThemedIcon::new("applications-internet")));
    tab_view.set_selected_page(&new_page);
    new_view
}

/// Get the WebKit instance for the current tab
///
/// # Arguments
///
/// * `tabs` - The TabBar containing the tabs & pages of the current browser session
fn view(tabs: &libadwaita::TabBar) -> webkit2gtk::WebView {
    let tab_view = tabs.view().unwrap();
    tab_view.selected_page()
        .unwrap().child().unwrap()
        .downcast()
        .unwrap()
}

/// Create an initial tab, for when the TabBar is empty
///
/// # Arguments
///
/// * `builder` - The object that contains all graphical widgets of the window
///
/// * `tabs` - The TabBar containing the tabs & pages of the current browser session
/// 
/// * `verbose` - Whether browser messages should be printed onto the standard output
///
/// * `is_private` - Whether the window represents a private session
fn create_initial_tab(
    builder: &gtk::Builder,
    tabs: &libadwaita::TabBar,
    initial_url: String,
    verbose: bool,
    is_private: bool,
    native: bool,
) {
    let web_view = new_tab_page(builder, tabs, 0, verbose, is_private, native);
    initial_connect(initial_url, &web_view)
}

/// Update a tab's icon
///
/// # Arguments
///
/// * `web_view` - The WebKit instance for the tab
///
/// * `tabs` - The TabBar containing the tabs & pages of the current browser session
fn update_favicon(web_view: &webkit2gtk::WebView, tabs: &libadwaita::TabBar) {
    let tab_view = tabs.view().unwrap();
    let current_page = tab_view.selected_page().unwrap();
    let web_favicon = &web_view.favicon();
    match &web_favicon {
        Some(_) => {
            let favicon_surface =
                cairo::ImageSurface::try_from(web_favicon.to_owned().unwrap()).unwrap();
            let favicon_width = favicon_surface.width();
            let favicon_height = favicon_surface.height();
            match favicon_width < 32 && favicon_height < 32 {
                true => {
                    let favicon_pixbuf = gdk::pixbuf_get_from_surface(
                        &favicon_surface,
                        0,
                        0,
                        favicon_width,
                        favicon_height,
                    )
                    .unwrap();
                    current_page.set_icon(Some(&gio::BytesIcon::new(&favicon_pixbuf.pixel_bytes().unwrap())));
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
                    current_page.set_icon(Some(&gio::BytesIcon::new(&scaled_pixbuf.pixel_bytes().unwrap())));
                }
            }
        }
        None => {
            current_page.set_icon(Some(&gio::ThemedIcon::new("applications-internet")));
        }
    }
}

/// Update a tab's title
///
/// # Arguments
///
/// * `web_view` - The WebKit instance for the tab
///
/// * `tabs` - The TabBar containing the tabs & pages of the current browser session
fn update_title(web_view: &webkit2gtk::WebView, tabs: &libadwaita::TabBar) {
    let tab_view = tabs.view().unwrap();
    let current_page = tab_view.selected_page().unwrap();
    current_page.set_title(Some(&web_view.title().unwrap().to_string()))
}

/// Update the load progress indicator under the navigation bar
///
/// # Arguments
///
/// * `nav_entry` - The navigation bar of the browser
///
/// * `web_view` - The WebKit instance for the current tab
fn update_load_progress(nav_entry: &gtk::Entry, web_view: &webkit2gtk::WebView) {
    let load_progress = web_view.estimated_load_progress();
    if load_progress as i64 == 1 {
        nav_entry.set_progress_fraction(0.00)
    } else {
        nav_entry.set_progress_fraction(load_progress)
    }
}

/// Create a dialog box showing information about Oku
fn new_about_dialog()
{
    let about_dialog = gtk::AboutDialog::new();
    about_dialog.set_version(VERSION);
    about_dialog.set_program_name(Some("Oku"));
    about_dialog.set_logo_icon_name(Some("oku"));
    about_dialog.set_icon_name(Some("oku"));
    about_dialog.set_license_type(gtk::License::Agpl30);
    about_dialog.set_destroy_with_parent(true);
    about_dialog.set_modal(true);
    about_dialog.show();
}

/// The main function of Oku
fn main() {
    let application = gtk::Application::new(Some("com.github.dirout.oku"), Default::default());

    application.add_main_option("url", glib::Char('u' as i8), OptionFlags::NONE, OptionArg::String, "An optional URL to open", Some("Open a URL in the browser"));
    application.add_main_option("verbose", glib::Char('v' as i8), OptionFlags::NONE, OptionArg::None, "Output browser messages to standard output", None);
    application.add_main_option("private", glib::Char('p' as i8), OptionFlags::NONE, OptionArg::None, "Open a private session", None);

    application.connect_activate(move |app| {
        let matches = VariantDict::new(None);
        new_window(app, matches);
    });

    // application.connect_handle_local_options(|app, options| {
    //     let matches = options.to_owned();
    //     app.run_with_args(&args().collect::<Vec<_>>());
    //     new_window(app, matches);
    //     0
    // });

    application.run_with_args(&args().collect::<Vec<_>>());
    // application.run();
}

/// Create a new functional & graphical browser window
///
/// # Arguments
///
/// * `application` - The application data representing Oku
///
/// * `matches` - The launch arguments passed to Oku
fn new_window(application: &gtk::Application, matches: VariantDict) {
    if gtk::init().is_err() {
        println!("Failed to initialize GTK.");
        return;
    }

    let initial_url: String;
    match matches.lookup_value("url", Some(VariantTy::new("s").unwrap()))
    {
        Some(url) => {
            initial_url = url.to_string()[1..url.to_string().len()-1].to_string();
        }
        None => {
            initial_url = "about:blank".to_owned();
        }
    }

    let is_private = matches.contains("private");
    let verbose = matches.contains("verbose");
    let native = true;

    let glade_src = include_str!("oku.glade");
    let builder = gtk::Builder::from_string(glade_src);

    let window: gtk::ApplicationWindow = builder.object("window").unwrap();
    window.set_application(Some(application));
    window.set_title(Some("Oku"));
    window.set_default_size(1920, 1080);

    let downloads_button: gtk::Button = builder.object("downloads_button").unwrap();
    let downloads_popover: gtk::Popover = builder.object("downloads_popover").unwrap();

    let find_button: gtk::Button = builder.object("find_button").unwrap();
    let find_popover: gtk::Popover = builder.object("find_popover").unwrap();
    let previous_find_button: gtk::Button = builder.object("previous_find_button").unwrap();
    let next_find_button: gtk::Button = builder.object("next_find_button").unwrap();
    let find_case_insensitive: gtk::ToggleButton = builder.object("find_case_insensitive").unwrap();
    let find_at_word_starts: gtk::ToggleButton = builder.object("find_at_word_starts").unwrap();
    let find_treat_medial_capital_as_word_start: gtk::ToggleButton = builder.object("find_treat_medial_capital_as_word_start").unwrap();
    let find_backwards: gtk::ToggleButton = builder.object("find_backwards").unwrap();
    let find_wrap_around: gtk::ToggleButton = builder.object("find_wrap_around").unwrap();
    let find_search_entry: gtk::SearchEntry = builder.object("find_search_entry").unwrap();
    let current_match_label: gtk::Label = builder.object("current_match_label").unwrap();
    let total_matches_label: gtk::Label = builder.object("total_matches_label").unwrap();

    let menu_button: gtk::Button = builder.object("menu_button").unwrap();
    let menu: gtk::Popover = builder.object("menu").unwrap();

    let back_button: gtk::Button = builder.object("back_button").unwrap();
    let forward_button: gtk::Button = builder.object("forward_button").unwrap();
    let refresh_button: gtk::Button = builder.object("refresh_button").unwrap();
    let add_tab: gtk::Button = builder.object("add_tab").unwrap();

    let tabs: libadwaita::TabBar = builder.object("tabs").unwrap();
    let tab_view: libadwaita::TabView = libadwaita::TabView::new();
    let nav_entry: gtk::Entry = builder.object("nav_entry").unwrap();

    let zoomout_button: gtk::Button = builder.object("zoomout_button").unwrap();
    let zoomin_button: gtk::Button = builder.object("zoomin_button").unwrap();
    let zoomreset_button: gtk::Button = builder.object("zoomreset_button").unwrap();
    let fullscreen_button: gtk::Button = builder.object("fullscreen_button").unwrap();
    let screenshot_button: gtk::Button = builder.object("screenshot_button").unwrap();
    let new_window_button: gtk::Button = builder.object("new_window_button").unwrap();
    let _history_button: gtk::Button = builder.object("history_button").unwrap();
    let about_button: gtk::Button = builder.object("about_button").unwrap();

    tabs.set_view(Some(&tab_view));

    let tab_view = tabs.view().unwrap();

    if tab_view.n_pages() == 0 {
        create_initial_tab(&builder, &tabs, initial_url.to_owned(), verbose, is_private, native)
    }

    tab_view.connect_pages_notify(
        clone!(@weak nav_entry, @weak builder, @weak tabs, @weak window => move |_| {
            let web_view = view(&tabs);
            update_nav_bar(&nav_entry, &web_view);
            window.set_title(Some(&web_view.title().unwrap_or_else(|| glib::GString::from("Oku")).to_string()));
        }),
    );

    nav_entry.connect_activate(clone!(@weak tabs, @weak nav_entry, @weak builder, @weak window => move |_| {
        let web_view = view(&tabs);
        connect(&nav_entry, &web_view);
        web_view.connect_title_notify(clone!(@weak tabs, @weak web_view => move |_| {
            update_title(&web_view, &tabs)
        }));
        web_view.connect_uri_notify(clone!(@weak tabs, @weak web_view, @weak nav_entry => move |_| {
            update_nav_bar(&nav_entry, &web_view)
        }));
        web_view.connect_estimated_load_progress_notify(clone!(@weak tabs, @weak web_view, @weak nav_entry => move |_| {
            update_load_progress(&nav_entry, &web_view)
        }));
        web_view.connect_favicon_notify(clone!(@weak tabs, @weak web_view => move |_| {
            update_favicon(&web_view, &tabs)
        }));
        web_view.connect_load_changed(clone!(@weak tabs, @weak web_view, @weak nav_entry, @weak window => move |_, _| {
            window.set_title(Some(&web_view.title().unwrap_or_else(|| glib::GString::from("Oku")).to_string()));
        }));
    }));

    add_tab.connect_clicked(clone!(@weak tabs, @weak nav_entry, @weak builder => move |_| {
        let tab_view = tabs.view().unwrap();
        let web_view = new_tab_page(&builder, &tabs, tab_view.n_pages(), verbose, is_private, native);
    }));

    back_button.connect_clicked(
        clone!(@weak tabs, @weak nav_entry, @weak builder => move |_| {
            let web_view = view(&tabs);
            web_view.go_back()
        }),
    );

    forward_button.connect_clicked(
        clone!(@weak tabs, @weak nav_entry, @weak builder => move |_| {
            let web_view = view(&tabs);
            web_view.go_forward()
        }),
    );

    refresh_button.connect_clicked(
        clone!(@weak tabs, @weak nav_entry, @weak builder => move |_| {
            let web_view = view(&tabs);
            web_view.reload_bypass_cache()
        }),
    );

    downloads_button.connect_clicked(
        clone!(@weak tabs, @weak nav_entry, @weak builder => move |_| {
            downloads_popover.popup();
        }),
    );

    find_button.connect_clicked(
        clone!(@weak tabs, @weak nav_entry, @weak builder, @weak find_popover => move |_| {
            find_popover.popup();
        }),
    );
    find_search_entry.connect_search_changed(
        clone!(@weak tabs, @weak nav_entry, @weak builder, @weak find_search_entry, @weak find_popover => move |_| {
            let web_view = view(&tabs);
            let find_controller = web_view.find_controller().unwrap();
            let mut find_options = webkit2gtk::FindOptions::empty();
            find_options.set(webkit2gtk::FindOptions::CASE_INSENSITIVE, find_case_insensitive.is_active());
            find_options.set(webkit2gtk::FindOptions::AT_WORD_STARTS, find_at_word_starts.is_active());
            find_options.set(webkit2gtk::FindOptions::TREAT_MEDIAL_CAPITAL_AS_WORD_START, find_treat_medial_capital_as_word_start.is_active());
            find_options.set(webkit2gtk::FindOptions::BACKWARDS, find_backwards.is_active());
            find_options.set(webkit2gtk::FindOptions::WRAP_AROUND, find_wrap_around.is_active());
            let max_match_count = find_controller.max_match_count();
            find_controller.count_matches(&find_search_entry.text(), find_options.bits(), max_match_count);
            find_controller.search(&find_search_entry.text(), find_options.bits(), max_match_count);
            find_controller.connect_counted_matches(clone!(@weak web_view, @weak find_controller, @weak find_search_entry, @weak total_matches_label => move |_, total_matches| {
                if total_matches < u32::MAX
                {
                    total_matches_label.set_text(&total_matches.to_string());
                }
            }));
            find_search_entry.connect_activate(clone!(@weak web_view, @weak find_controller, @weak find_search_entry, @weak current_match_label, @weak total_matches_label => move |_| {
                find_controller.search_next();
                let mut current_match: u32 = current_match_label.text().parse().unwrap();
                let total_matches: u32 = total_matches_label.text().parse().unwrap();
                current_match += 1;
                if current_match > total_matches
                {
                    current_match = 1;
                }
                current_match_label.set_text(&current_match.to_string());
                find_search_entry.set_tooltip_text(Some(&format!("{} / {} matches", current_match, total_matches)));
            }));
            next_find_button.connect_clicked(clone!(@weak web_view, @weak find_controller, @weak find_search_entry, @weak current_match_label, @weak total_matches_label => move |_| {
                find_controller.search_next();
                let mut current_match: u32 = current_match_label.text().parse().unwrap();
                let total_matches: u32 = total_matches_label.text().parse().unwrap();
                current_match += 1;
                if current_match > total_matches
                {
                    current_match = 1;
                }
                current_match_label.set_text(&current_match.to_string());
                find_search_entry.set_tooltip_text(Some(&format!("{} / {} matches", current_match, total_matches)));
            }));
            previous_find_button.connect_clicked(clone!(@weak web_view, @weak find_controller, @weak find_search_entry, @weak current_match_label, @weak total_matches_label => move |_| {
                find_controller.search_previous();
                let mut current_match: u32 = current_match_label.text().parse().unwrap();
                let total_matches: u32 = total_matches_label.text().parse().unwrap();
                current_match -= 1;
                if current_match < 1
                {
                    current_match = 1;
                }
                current_match_label.set_text(&current_match.to_string());
                find_search_entry.set_tooltip_text(Some(&format!("{} / {} matches", current_match, total_matches)));
            }));
            find_popover.connect_closed(
                clone!(@weak tabs, @weak nav_entry, @weak builder, @weak current_match_label, @weak total_matches_label => move |_| {
                    find_controller.search_finish();
                    current_match_label.set_text("0");
                    total_matches_label.set_text("0");
                }),
            );
        }),
    );

    menu_button.connect_clicked(
        clone!(@weak tabs, @weak nav_entry, @weak builder => move |_| {
            menu.popup();
        }),
    );

    about_button.connect_clicked(
        clone!(@weak tabs, @weak nav_entry, @weak builder => move |_| {
            new_about_dialog()
        }),
    );

    zoomin_button.connect_clicked(
        clone!(@weak tabs, @weak nav_entry, @weak builder => move |_| {
            let web_view = view(&tabs);
            let current_zoom_level = web_view.zoom_level();
            web_view.set_zoom_level(current_zoom_level + 0.1);
        }),
    );

    zoomout_button.connect_clicked(
        clone!(@weak tabs, @weak nav_entry, @weak builder => move |_| {
            let web_view = view(&tabs);
            let current_zoom_level = web_view.zoom_level();
            web_view.set_zoom_level(current_zoom_level - 0.1);
        }),
    );

    zoomreset_button.connect_clicked(
        clone!(@weak tabs, @weak nav_entry, @weak builder => move |_| {
            let web_view = view(&tabs);
            web_view.set_zoom_level(1.0);
        }),
    );

    fullscreen_button.connect_clicked(
        clone!(@weak tabs, @weak nav_entry, @weak builder => move |_| {
            let web_view = view(&tabs);
            web_view.run_javascript("document.documentElement.webkitRequestFullscreen();", gio::NONE_CANCELLABLE, move |_| {
                
            })
        }),
    );

    screenshot_button.connect_clicked(
        clone!(@weak tabs, @weak nav_entry, @weak builder => move |_| {
            let web_view = view(&tabs);
            web_view.snapshot(webkit2gtk::SnapshotRegion::FullDocument, webkit2gtk::SnapshotOptions::all(), gio::NONE_CANCELLABLE, move |snapshot| {
                let snapshot_surface = cairo::ImageSurface::try_from(snapshot.unwrap()).unwrap();
                let mut writer = File::create(format!("{}/{}.png", PICTURES_DIR.to_owned(), Utc::now())).unwrap();
                snapshot_surface.write_to_png(&mut writer).unwrap();
            });
        }),
    );

    new_window_button.connect_clicked(
        clone!(@weak tabs, @weak nav_entry, @weak builder, @weak window => move |_| {
            matches.remove("url");
            new_window(&window.application().unwrap(), matches.to_owned())
        }),
    );

    window.show();
}

fn new_window_four(application: &gtk::Application)
{
    let headerbar_builder = gtk::HeaderBarBuilder::new();
    let headerbar = headerbar_builder.can_focus(false).show_title_buttons(true).build();

    let window_builder = gtk::ApplicationWindowBuilder::new();
    let window = window_builder.application(application).can_focus(true).title("Oku").icon_name("oku").build();
    window.set_titlebar(Some(&headerbar));
}