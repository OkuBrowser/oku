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



use gtk::ToggleButtonExt;
use webkit2gtk::FindControllerExt;
use gtk::SearchEntryExt;
use chrono::Utc;
use directories_next::UserDirs;
use std::fs::File;
use gtk::AboutDialogExt;
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
use urlencoding::decode;
use webkit2gtk::SettingsExt;
use webkit2gtk::URISchemeRequest;
use webkit2gtk::URISchemeRequestExt;
use webkit2gtk::WebContextExt;
use webkit2gtk::WebViewExt;
use clap::clap_app;

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
    let ipfs_bytes = get_from_hash(client, ipfs_path);
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
) -> webkit2gtk::WebView {
    let web_kit = webkit2gtk::WebViewBuilder::new()
        .is_ephemeral(is_private)
        .automation_presentation_type(webkit2gtk::AutomationBrowsingContextPresentation::Tab);
    let web_settings: webkit2gtk::Settings = builder.get_object("webkit_settings").unwrap();
    let web_view = web_kit.build();
    let web_context = web_view.get_context().unwrap();
    let extensions_path = format!("{}/web-extensions/", DATA_DIR.to_string());
    let favicon_database_path = format!("{}/favicon-database/", CACHE_DIR.to_string());
    // let allowed_notification_origins: Vec<String> = bincode::deserialize(&fs::read(format!("{}/settings/allowed_notification_origins.bin", DATA_DIR.to_owned())).unwrap()).unwrap();
    // let disallowed_notification_origins: Vec<String> = bincode::deserialize(&fs::read(format!("{}/settings/disallowed_notification_origins.bin", DATA_DIR.to_owned())).unwrap()).unwrap();
    
    // let mut allowed_notification_security_origins: Vec<&webkit2gtk::SecurityOrigin> = vec!();
    // for url in allowed_notification_origins {
    //     let new_origin = webkit2gtk::SecurityOrigin::new_for_uri(&url);
    //     allowed_notification_security_origins.push(&new_origin);
    // }

    // let mut disallowed_notification_security_origins: Vec<&webkit2gtk::SecurityOrigin> = vec!();
    // for url in disallowed_notification_origins {
    //     let new_origin = webkit2gtk::SecurityOrigin::new_for_uri(&url);
    //     disallowed_notification_security_origins.push(&new_origin);
    // }

    web_context.register_uri_scheme("ipfs", move |request| handle_ipfs_request(request));
    // web_context.initialize_notification_permissions(&allowed_notification_security_origins, &disallowed_notification_security_origins);
    web_settings.set_user_agent_with_application_details(Some("Oku"), Some(VERSION.unwrap()));
    web_settings.set_enable_write_console_messages_to_stdout(verbose);
    web_view.set_settings(&web_settings);
    web_context.set_web_extensions_directory(&extensions_path);
    web_context.set_favicon_database_directory(Some(&favicon_database_path));
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
fn get_from_hash(client: IpfsClient, hash: String) -> Vec<u8> {
    let mut sys = actix_rt::System::new(format!("Oku IPFS System ({})", hash));
    sys.block_on(download_ipfs_file(client, hash.to_owned()))
}

/// Download an IPFS file to the local machine
///
/// # Arguments
///
/// `client` - The IPFS client running locally
///
/// `file_hash` - The CID of the folder the file is in
async fn download_ipfs_file(client: IpfsClient, file_hash: String) -> Vec<u8> {
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
/// * `verbose` - Whether browser messages should be printed onto the standard output
///
/// * `is_private` - Whether the window represents a private session
fn new_tab_page(
    builder: &gtk::Builder,
    tabs: &gtk::Notebook,
    new_tab_number: u32,
    verbose: bool,
    is_private: bool,
) -> webkit2gtk::WebView {
    let new_view = new_view(builder, verbose, is_private);
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
/// * `verbose` - Whether browser messages should be printed onto the standard output
///
/// * `is_private` - Whether the window represents a private session
fn create_initial_tab(
    builder: &gtk::Builder,
    tabs: &gtk::Notebook,
    initial_url: String,
    verbose: bool,
    is_private: bool,
) {
    let web_view = new_tab_page(&builder, &tabs, 0, verbose, is_private);
    web_view.load_uri(&initial_url);
    let current_tab_label: gtk::Box = tabs.get_tab_label(&web_view).unwrap().downcast().unwrap();
    let close_button_widget = &current_tab_label.get_children()[2];
    let close_button: gtk::Button = close_button_widget.clone().downcast().unwrap();
    close_button.connect_clicked(clone!(@weak tabs, @weak web_view => move |_| {
        tabs.remove_page(tabs.page_num(&web_view));
    }));
    tabs.set_show_tabs(false)
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

/// Create a dialog box showing information about Oku
fn new_about_dialog()
{
    let about_dialog = gtk::AboutDialog::new();
    about_dialog.set_version(VERSION);
    about_dialog.set_program_name("Oku");
    about_dialog.set_logo_icon_name(Some("oku"));
    about_dialog.set_icon_name(Some("oku"));
    about_dialog.set_license_type(gtk::License::Agpl30);
    about_dialog.set_destroy_with_parent(true);
    about_dialog.set_modal(true);
    about_dialog.set_urgency_hint(true);
    about_dialog.show();
}

/// The main function of Oku
fn main() {
    let matches = clap_app!(myapp =>
        (version: VERSION.unwrap())
        (author: "Emil Sayahi <limesayahi@gmail.com>")
        (about: "A hive browser written in Rust")
        (@arg INPUT: "An optional URL to open in the browser")
        (@arg verbose: -v --verbose "Output browser messages to standard output")
        (@arg private: -p --private "Open a private session")
    ).get_matches();

    let application = gtk::Application::new(Some("com.github.madebyemil.oku"), Default::default())
        .expect("Initialization failed … ");

    application.connect_activate(move |app| {
        new_window(app, matches.to_owned());
    });

    application.run(&args().collect::<Vec<_>>());
}

/// Create a new functional & graphical browser window
///
/// # Arguments
///
/// * `application` - The application data representing Oku
///
/// * `matches` - The launch arguments passed to Oku
fn new_window(application: &gtk::Application, matches: clap::ArgMatches) {
    if gtk::init().is_err() {
        println!("Failed to initialize GTK.");
        return;
    }

    let initial_url;
    match matches.value_of("INPUT")
    {
        Some(url) => {
            initial_url = url.to_owned();
        }
        None => {
            initial_url = "about:blank".to_owned();
        }
    }

    let is_private = matches.is_present("private");
    let verbose = matches.is_present("verbose");

    let glade_src = include_str!("oku.glade");
    let builder = gtk::Builder::from_string(glade_src);

    let window: gtk::ApplicationWindow = builder.get_object("window").unwrap();
    window.set_title("Oku");

    let downloads_button: gtk::Button = builder.get_object("downloads_button").unwrap();
    let downloads_popover: gtk::Popover = builder.get_object("downloads_popover").unwrap();

    let find_button: gtk::Button = builder.get_object("find_button").unwrap();
    let find_popover: gtk::Popover = builder.get_object("find_popover").unwrap();
    let previous_find_button: gtk::Button = builder.get_object("previous_find_button").unwrap();
    let next_find_button: gtk::Button = builder.get_object("next_find_button").unwrap();
    let find_case_insensitive: gtk::ToggleButton = builder.get_object("find_case_insensitive").unwrap();
    let find_at_word_starts: gtk::ToggleButton = builder.get_object("find_at_word_starts").unwrap();
    let find_treat_medial_capital_as_word_start: gtk::ToggleButton = builder.get_object("find_treat_medial_capital_as_word_start").unwrap();
    let find_backwards: gtk::ToggleButton = builder.get_object("find_backwards").unwrap();
    let find_wrap_around: gtk::ToggleButton = builder.get_object("find_wrap_around").unwrap();
    let find_search_entry: gtk::SearchEntry = builder.get_object("find_search_entry").unwrap();
    let current_match_label: gtk::Label = builder.get_object("current_match_label").unwrap();
    let total_matches_label: gtk::Label = builder.get_object("total_matches_label").unwrap();

    let menu_button: gtk::Button = builder.get_object("menu_button").unwrap();
    let menu: gtk::Popover = builder.get_object("menu").unwrap();

    let back_button: gtk::Button = builder.get_object("back_button").unwrap();
    let forward_button: gtk::Button = builder.get_object("forward_button").unwrap();
    let refresh_button: gtk::Button = builder.get_object("refresh_button").unwrap();
    let add_tab: gtk::Button = builder.get_object("add_tab").unwrap();

    let tabs: gtk::Notebook = builder.get_object("tabs").unwrap();
    let nav_entry: gtk::Entry = builder.get_object("nav_entry").unwrap();

    let zoomout_button: gtk::Button = builder.get_object("zoomout_button").unwrap();
    let zoomin_button: gtk::Button = builder.get_object("zoomin_button").unwrap();
    let zoomreset_button: gtk::Button = builder.get_object("zoomreset_button").unwrap();
    let fullscreen_button: gtk::Button = builder.get_object("fullscreen_button").unwrap();
    let screenshot_button: gtk::Button = builder.get_object("screenshot_button").unwrap();
    let new_window_button: gtk::Button = builder.get_object("new_window_button").unwrap();
    let _history_button: gtk::Button = builder.get_object("history_button").unwrap();
    let about_button: gtk::Button = builder.get_object("about_button").unwrap();

    window.set_application(Some(application));

    if tabs.get_n_pages() == 0 {
        create_initial_tab(&builder, &tabs, initial_url.to_owned(), verbose, is_private)
    }

    tabs.connect_property_page_notify(
        clone!(@weak nav_entry, @weak builder, @weak tabs, @weak window => move |_| {
            let web_view = get_view(&tabs);
            update_nav_bar(&nav_entry, &web_view);
            window.set_title(&web_view.get_title().unwrap_or_else(|| glib::GString::from("Oku")));
        }),
    );

    tabs.connect_page_added(
        clone!(@weak nav_entry, @weak builder, @weak tabs => move |_, _, _| {
            match tabs.get_n_pages()
            {
                1 => {
                    tabs.set_show_tabs(false);
                }
                _ => {
                    if !tabs.get_show_tabs()
                    {
                        tabs.set_show_tabs(true);
                    }
                }
            }
        }),
    );

    tabs.connect_page_removed(
        clone!(@weak nav_entry, @weak builder, @weak tabs => move |_, _, _| {
            match tabs.get_n_pages()
            {
                0 => {
                    nav_entry.set_text("");
                    create_initial_tab(&builder, &tabs, initial_url.to_owned(), verbose, is_private)
                }
                1 => {
                    tabs.set_show_tabs(false);
                }
                _ => {
                    if !tabs.get_show_tabs()
                    {
                        tabs.set_show_tabs(true);
                    }
                }
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

    add_tab.connect_clicked(clone!(@weak tabs, @weak nav_entry, @weak builder => move |_| {
        let web_view = new_tab_page(&builder, &tabs, tabs.get_n_pages(), verbose, is_private);
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

    find_button.connect_clicked(
        clone!(@weak tabs, @weak nav_entry, @weak builder, @weak find_popover => move |_| {
            find_popover.popup();
        }),
    );
    find_search_entry.connect_search_changed(
        clone!(@weak tabs, @weak nav_entry, @weak builder, @weak find_search_entry, @weak find_popover => move |_| {
            let web_view = get_view(&tabs);
            let find_controller = web_view.get_find_controller().unwrap();
            let mut find_options = webkit2gtk::FindOptions::empty();
            find_options.set(webkit2gtk::FindOptions::CASE_INSENSITIVE, find_case_insensitive.get_active());
            find_options.set(webkit2gtk::FindOptions::AT_WORD_STARTS, find_at_word_starts.get_active());
            find_options.set(webkit2gtk::FindOptions::TREAT_MEDIAL_CAPITAL_AS_WORD_START, find_treat_medial_capital_as_word_start.get_active());
            find_options.set(webkit2gtk::FindOptions::BACKWARDS, find_backwards.get_active());
            find_options.set(webkit2gtk::FindOptions::WRAP_AROUND, find_wrap_around.get_active());
            let max_match_count = find_controller.get_max_match_count();
            // let current_match = Rc::new(RefCell::new(0));
            // let all_matches = Rc::new(RefCell::new(0));
            find_controller.count_matches(&find_search_entry.get_text(), find_options.bits(), max_match_count);
            find_controller.search(&find_search_entry.get_text(), find_options.bits(), max_match_count);
            find_controller.connect_counted_matches(clone!(@weak web_view, @weak find_controller, @weak find_search_entry, @weak total_matches_label => move |_, total_matches| {
                // *all_matches.borrow_mut() = total_matches;
                if total_matches < u32::MAX
                {
                    total_matches_label.set_text(&total_matches.to_string());
                }
            }));
            find_search_entry.connect_activate(clone!(@weak web_view, @weak find_controller, @weak find_search_entry, @weak current_match_label, @weak total_matches_label => move |_| {
                find_controller.search_next();
                let mut current_match: u32 = current_match_label.get_text().parse().unwrap();
                let total_matches: u32 = total_matches_label.get_text().parse().unwrap();
                current_match += 1;
                if current_match > total_matches
                {
                    current_match = 1;
                }
                current_match_label.set_text(&current_match.to_string());
                find_search_entry.set_progress_fraction(current_match as f64 / total_matches as f64);
                find_search_entry.set_tooltip_text(Some(&format!("{} / {} matches", current_match, total_matches)));
                // println!("{} / {} matches", *current_match.borrow_mut(), *all_matches.borrow_mut());
            }));
            next_find_button.connect_clicked(clone!(@weak web_view, @weak find_controller, @weak find_search_entry, @weak current_match_label, @weak total_matches_label => move |_| {
                find_controller.search_next();
                let mut current_match: u32 = current_match_label.get_text().parse().unwrap();
                let total_matches: u32 = total_matches_label.get_text().parse().unwrap();
                current_match += 1;
                if current_match > total_matches
                {
                    current_match = 1;
                }
                current_match_label.set_text(&current_match.to_string());
                find_search_entry.set_progress_fraction(current_match as f64 / total_matches as f64);
                find_search_entry.set_tooltip_text(Some(&format!("{} / {} matches", current_match, total_matches)));
                // println!("{} / {} matches", *current_match.borrow_mut(), *all_matches.borrow_mut());
            }));
            previous_find_button.connect_clicked(clone!(@weak web_view, @weak find_controller, @weak find_search_entry, @weak current_match_label, @weak total_matches_label => move |_| {
                find_controller.search_previous();
                let mut current_match: u32 = current_match_label.get_text().parse().unwrap();
                let total_matches: u32 = total_matches_label.get_text().parse().unwrap();
                current_match -= 1;
                if current_match < 1
                {
                    current_match = 1;
                }
                current_match_label.set_text(&current_match.to_string());
                find_search_entry.set_progress_fraction(current_match as f64 / total_matches as f64);
                find_search_entry.set_tooltip_text(Some(&format!("{} / {} matches", current_match, total_matches)));
                // println!("{} / {} matches", *current_match.borrow_mut(), *all_matches.borrow_mut());
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
            let web_view = get_view(&tabs);
            let current_zoom_level = web_view.get_zoom_level();
            web_view.set_zoom_level(current_zoom_level + 0.1);
        }),
    );

    zoomout_button.connect_clicked(
        clone!(@weak tabs, @weak nav_entry, @weak builder => move |_| {
            let web_view = get_view(&tabs);
            let current_zoom_level = web_view.get_zoom_level();
            web_view.set_zoom_level(current_zoom_level - 0.1);
        }),
    );

    zoomreset_button.connect_clicked(
        clone!(@weak tabs, @weak nav_entry, @weak builder => move |_| {
            let web_view = get_view(&tabs);
            web_view.set_zoom_level(1.0);
        }),
    );

    fullscreen_button.connect_clicked(
        clone!(@weak tabs, @weak nav_entry, @weak builder => move |_| {
            let web_view = get_view(&tabs);
            web_view.run_javascript("document.documentElement.webkitRequestFullscreen();", gio::NONE_CANCELLABLE, move |_| {
                
            })
        }),
    );

    screenshot_button.connect_clicked(
        clone!(@weak tabs, @weak nav_entry, @weak builder => move |_| {
            let web_view = get_view(&tabs);
            web_view.get_snapshot(webkit2gtk::SnapshotRegion::FullDocument, webkit2gtk::SnapshotOptions::all(), gio::NONE_CANCELLABLE, move |snapshot| {
                let snapshot_surface = cairo::ImageSurface::try_from(snapshot.unwrap()).unwrap();
                let mut writer = File::create(format!("{}/{}.png", PICTURES_DIR.to_owned(), Utc::now())).unwrap();
                snapshot_surface.write_to_png(&mut writer).unwrap();
            });
        }),
    );

    new_window_button.connect_clicked(
        clone!(@weak tabs, @weak nav_entry, @weak builder, @weak window => move |_| {
            new_window(&window.get_application().unwrap(), matches.to_owned())
        }),
    );

    window.show_all();
    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });
    gtk::main();
}
