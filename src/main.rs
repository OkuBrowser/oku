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

#![cfg_attr(feature = "dox", feature(doc_cfg))]
#![allow(clippy::needless_doctest_main)]
#![doc(
    html_logo_url = "https://github.com/Dirout/oku/raw/master/branding/logo-filled.svg",
    html_favicon_url = "https://github.com/Dirout/oku/raw/master/branding/logo-filled.svg"
)]
#![feature(async_closure)]

use chrono::Utc;
use cid::Cid;
use directories_next::ProjectDirs;
use directories_next::UserDirs;
use futures::TryStreamExt;
use gio::prelude::*;
use glib::clone;
use glib::Cast;
use gtk::prelude::BoxExt;
use gtk::prelude::ButtonExt;
use gtk::prelude::EditableExt;
use gtk::prelude::EntryExt;
use gtk::prelude::GtkWindowExt;
use gtk::prelude::PopoverExt;
use gtk::prelude::StyleContextExt;
use gtk::prelude::WidgetExt;
use ipfs::Ipfs;
use ipfs::IpfsOptions;
use ipfs::IpfsPath;
use ipfs::Keypair;
use ipfs::Types;
use ipfs::UninitializedIpfs;
use ipfs_api_backend_hyper::{IpfsApi, IpfsClient};
use std::convert::TryFrom;
use std::fs::File;
use std::path::PathBuf;
use tokio_stream::StreamExt;
use url::ParseError;
use url::Url;
use urlencoding::decode;
use webkit2gtk::{
    traits::{SettingsExt, URISchemeRequestExt, WebContextExt, WebViewExt},
    URISchemeRequest,
};

#[macro_use]
extern crate lazy_static;

lazy_static! {
    /// The platform-specific directories intended for Oku's use
    static ref PROJECT_DIRECTORIES: ProjectDirs =
        ProjectDirs::from("com", "github.dirout", "oku").unwrap();
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

/// Perform the initial connection at startup when passed a URL as a launch argument
///
/// * `initial_url` - The URL passed as a launch argument
///
/// * `web_view` - The WebKit instance for the current tab
fn initial_connect(mut initial_url: String, web_view: &webkit2gtk::WebView) {
    let mut parsed_url = Url::parse(&initial_url);
    match parsed_url {
        // When URL is completely OK
        Ok(_) => {
            web_view.load_uri(&initial_url);
        }
        // When URL is missing a scheme
        Err(ParseError::RelativeUrlWithoutBase) => {
            parsed_url = Url::parse(&format!("http://{}", initial_url)); // Try with HTTP first
            match parsed_url {
                // If it's now valid with HTTP
                Ok(_) => {
                    let split_url: Vec<&str> = initial_url.split('/').collect();
                    let host = split_url[0];
                    let cid = Cid::try_from(host);
                    // Try seeing if we can swap it with IPFS
                    match cid {
                        // It works as IPFS
                        Ok(_) => {
                            let unwrapped_cid = cid.unwrap();
                            let cid1 =
                                Cid::new_v1(unwrapped_cid.codec(), unwrapped_cid.hash().to_owned());
                            parsed_url = Url::parse(&format!("ipfs://{}", initial_url));
                            let mut unwrapped_url = parsed_url.unwrap();
                            let cid1_string = &cid1
                                .to_string_of_base(cid::multibase::Base::Base32Lower)
                                .unwrap();
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
    match parsed_url {
        // When URL is completely OK
        Ok(_) => {
            web_view.load_uri(&nav_text);
        }
        // When URL is missing a scheme
        Err(ParseError::RelativeUrlWithoutBase) => {
            parsed_url = Url::parse(&format!("http://{}", nav_text)); // Try with HTTP first
            match parsed_url {
                // If it's now valid with HTTP
                Ok(_) => {
                    let split_url: Vec<&str> = nav_text.split('/').collect();
                    let host = split_url[0];
                    let cid = Cid::try_from(host);
                    // Try seeing if we can swap it with IPFS
                    match cid {
                        // It works as IPFS
                        Ok(_) => {
                            let unwrapped_cid = cid.unwrap();
                            let cid1 =
                                Cid::new_v1(unwrapped_cid.codec(), unwrapped_cid.hash().to_owned());
                            parsed_url = Url::parse(&format!("ipfs://{}", nav_text));
                            let mut unwrapped_url = parsed_url.unwrap();
                            let cid1_string = &cid1
                                .to_string_of_base(cid::multibase::Base::Base32Lower)
                                .unwrap();
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
    if url == "about:blank" {
        url = "".to_string();
    }
    nav_entry.set_text(&url);
}

/// Comply with a request using the IPFS scheme
///
/// # Arguments
///
/// * `request` - The request from the browser for the IPFS resource
fn handle_ipfs_request_using_api(request: &URISchemeRequest) {
    let request_url = request.uri().unwrap().to_string();
    let decoded_url = decode(&request_url).unwrap();
    let ipfs_path = decoded_url.replacen("ipfs://", "", 1);
    let ipfs_bytes = from_hash_using_api(ipfs_path);
    //let ipfs_bytes = download_ipfs_file_from_api(ipfs_path).await;
    let stream = gio::MemoryInputStream::from_bytes(&glib::Bytes::from(&ipfs_bytes));
    request.finish(&stream, -1, None);
}

/// Comply with a request using the IPFS scheme natively
///
/// # Arguments
///
/// * `request` - The request from the browser for the IPFS resource
fn handle_ipfs_request_natively(request: &URISchemeRequest) {
    let request_url = request.uri().unwrap().to_string();
    let decoded_url = decode(&request_url).unwrap();
    let ipfs_path = decoded_url.replacen("ipfs://", "", 1);
    let ipfs_bytes = from_hash_natively(ipfs_path);
    //let ipfs_bytes = download_ipfs_file_natively(ipfs_path).await;
    let stream = gio::MemoryInputStream::from_bytes(&glib::Bytes::from(&ipfs_bytes));
    request.finish(&stream, -1, None);
}

/// Provide the default configuration for Oku's WebView
fn new_webkit_settings() -> webkit2gtk::Settings {
    let settings_builder = webkit2gtk::SettingsBuilder::new();

    settings_builder
        .load_icons_ignoring_image_load_setting(true)
        .javascript_can_open_windows_automatically(true)
        .allow_file_access_from_file_urls(true)
        .allow_modal_dialogs(true)
        .allow_top_navigation_to_data_urls(true)
        .allow_universal_access_from_file_urls(true)
        .auto_load_images(true)
        .draw_compositing_indicators(true)
        .enable_accelerated_2d_canvas(false)
        .enable_back_forward_navigation_gestures(true)
        .enable_caret_browsing(true)
        .enable_developer_extras(true)
        .enable_dns_prefetching(true)
        .enable_encrypted_media(true)
        .enable_frame_flattening(true)
        .enable_fullscreen(true)
        .enable_html5_database(true)
        .enable_html5_local_storage(true)
        .enable_hyperlink_auditing(true)
        .enable_java(true)
        .enable_javascript(true)
        .enable_javascript_markup(true)
        .enable_media(true)
        .enable_media_capabilities(true)
        .enable_media_stream(true)
        .enable_mediasource(true)
        .enable_mock_capture_devices(true)
        .enable_offline_web_application_cache(true)
        .enable_page_cache(true)
        .enable_plugins(true)
        .enable_private_browsing(true)
        .enable_resizable_text_areas(true)
        .enable_site_specific_quirks(true)
        .enable_smooth_scrolling(true)
        .enable_spatial_navigation(true)
        .enable_tabs_to_links(true)
        .enable_webaudio(true)
        .enable_webgl(true)
        .enable_write_console_messages_to_stdout(true)
        .enable_xss_auditor(true)
        .hardware_acceleration_policy(webkit2gtk::HardwareAccelerationPolicy::Never)
        .javascript_can_access_clipboard(true)
        .load_icons_ignoring_image_load_setting(false)
        .media_playback_allows_inline(true)
        .media_playback_requires_user_gesture(false)
        .print_backgrounds(true)
        .zoom_text_only(false)
        // .enable_developer_extras(true)
        // .enable_dns_prefetching(true)
        // .enable_caret_browsing(true)
        // .allow_modal_dialogs(true)
        // .javascript_can_access_clipboard(true)
        // .media_playback_requires_user_gesture(true)
        // .enable_smooth_scrolling(true)
        // .enable_media_stream(true)
        // .enable_spatial_navigation(true)
        // .enable_encrypted_media(true)
        // .enable_media_capabilities(true)
        // .allow_file_access_from_file_urls(true)
        // .allow_universal_access_from_file_urls(true)
        // .allow_top_navigation_to_data_urls(true)
        // .enable_back_forward_navigation_gestures(true)
        .build()
}

/// Create a new WebKit instance for the current tab
///
/// # Arguments
///  
/// * `verbose` - Whether browser messages should be printed onto the standard output
///
/// * `is_private` - Whether the window represents a private session
///
/// * `native` - Whether the browser is using a built-in (native) IPFS handler, or an external one
fn new_view(
    verbose: bool,
    is_private: bool,
    native: bool,
    tabs: &libadwaita::TabBar,
) -> webkit2gtk::WebView {
    let web_kit = webkit2gtk::WebViewBuilder::new()
        .vexpand(true)
        .is_ephemeral(is_private);
    let web_settings: webkit2gtk::Settings = new_webkit_settings();
    let web_view = web_kit.build();
    let web_context = web_view.context().unwrap();
    let extensions_path = format!("{}/web-extensions/", *DATA_DIR);
    let favicon_database_path = format!("{}/favicon-database/", *CACHE_DIR);

    match native {
        true => {
            web_context.register_uri_scheme("ipfs", move |request| {
                handle_ipfs_request_natively(request)
                // let request_url = request.uri().unwrap().to_string();
                // let decoded_url = decode(&request_url).unwrap();
                // let ipfs_path = decoded_url.replacen("ipfs://", "", 1);
                // let ipfs_bytes = from_hash_natively(ipfs_path);
                // //let ipfs_bytes = download_ipfs_file_natively(ipfs_path).await;
                // let stream = gio::MemoryInputStream::from_bytes(&glib::Bytes::from(&ipfs_bytes));
                // request.finish(&stream, ipfs_bytes.len().try_into().unwrap(), None);
            });
        }
        false => {
            web_context.register_uri_scheme("ipfs", move |request| {
                handle_ipfs_request_using_api(request)
                // let request_url = request.uri().unwrap().to_string();
                // let decoded_url = decode(&request_url).unwrap();
                // let ipfs_path = decoded_url.replacen("ipfs://", "", 1);
                // let ipfs_bytes = from_hash_using_api(ipfs_path);
                // //let ipfs_bytes = download_ipfs_file_from_api(ipfs_path).await;
                // let stream = gio::MemoryInputStream::from_bytes(&glib::Bytes::from(&ipfs_bytes));
                // request.finish(&stream, ipfs_bytes.len().try_into().unwrap(), None);
            });
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

    web_view.connect_title_notify(clone!(@weak web_view => move |_| {
        update_title(&web_view)
    }));
    web_view.connect_uri_notify(clone!(@weak web_view => move |_| {
        let window: gtk::ApplicationWindow = web_view.parent().unwrap().parent().unwrap().parent().unwrap().parent().unwrap().downcast().unwrap();
        let headerbar: gtk::HeaderBar = window.titlebar().unwrap().downcast().unwrap();
        let nav_entry: gtk::Entry = headerbar.title_widget().unwrap().downcast().unwrap();
        update_nav_bar(&nav_entry, &web_view)
    }));
    web_view.connect_estimated_load_progress_notify(
        clone!(@weak tabs, @weak web_view => move |_| {
            let window: gtk::ApplicationWindow = web_view.parent().unwrap().parent().unwrap().parent().unwrap().parent().unwrap().downcast().unwrap();
            let headerbar: gtk::HeaderBar = window.titlebar().unwrap().downcast().unwrap();
            let nav_entry: gtk::Entry = headerbar.title_widget().unwrap().downcast().unwrap();
            let tab_view: libadwaita::TabView = web_view.parent().unwrap().parent().unwrap().downcast().unwrap();
            let current_page = tab_view.page(&web_view);
            current_page.set_loading(true);
            update_load_progress(&nav_entry, &web_view)
        }),
    );
    web_view.connect_is_loading_notify(clone!(@weak tabs, @weak web_view => move |_| {
        let tab_view: libadwaita::TabView = web_view.parent().unwrap().parent().unwrap().downcast().unwrap();
        let current_page = tab_view.page(&web_view);
        current_page.set_loading(web_view.is_loading())
    }));
    // web_view.connect_is_playing_audio_notify(clone!(@weak web_view => move |_| {
    //     let tab_view: libadwaita::TabView = web_view.parent().unwrap().parent().unwrap().downcast().unwrap();
    //     let current_page = tab_view.page(&web_view).unwrap();
    //     match web_view.is_playing_audio()
    //     {
    //         true => {
    //             current_page.set_indicator_icon(Some(&gio::ThemedIcon::new("notification-audio-volume-high")));
    //             current_page.set_indicator_activatable(true);
    //             tab_view.connect_indicator_activated(clone!(@weak web_view => move |_, _| {
    //                 if !web_view.is_muted() {
    //                     web_view.set_is_muted(true);
    //                     current_page.set_indicator_icon(Some(&gio::ThemedIcon::new("notification-audio-volume-muted")));
    //                 } else {
    //                     web_view.set_is_muted(false);
    //                     current_page.set_indicator_icon(Some(&gio::ThemedIcon::new("notification-audio-volume-high")));
    //                 }
    //             }));
    //         },
    //         false => {
    //             if !web_view.is_muted()
    //             {

    //             }
    //         }
    //     }
    // }));
    web_view.connect_favicon_notify(clone!(@weak tabs, @weak web_view => move |_| {
        update_favicon(&web_view)
    }));
    web_view.connect_load_changed(clone!(@weak tabs, @weak web_view => move |_, _| {
        let window: gtk::ApplicationWindow = tabs.parent().unwrap().parent().unwrap().downcast().unwrap();
        window.set_title(Some(&web_view.title().unwrap_or_else(|| glib::GString::from("Oku")).to_string()));
    }));

    web_view
}

/// Setup an IPFS node
async fn setup_native_ipfs() -> Ipfs<Types> {
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
/// * `hash` - The IPFS identifier of the file
fn from_hash_using_api(hash: String) -> Vec<u8> {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(download_ipfs_file_from_api(hash))
}

/// Get an IPFS file asynchronously using an in-memory IPFS node
///
/// # Arguments
///
/// * `hash` - The IPFS identifier of the file
fn from_hash_natively(hash: String) -> Vec<u8> {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(download_ipfs_file_natively(hash))
}

/// Download an IPFS file to the local machine using an existing IPFS node
///
/// # Arguments
///
/// * `file_hash` - The CID of the file
async fn download_ipfs_file_from_api(file_hash: String) -> Vec<u8> {
    let client: IpfsClient = IpfsClient::default();

    match client
        .cat(&file_hash)
        .map_ok(|chunk| chunk.to_vec())
        .try_concat()
        .await
    {
        Ok(res) => res,
        Err(e) => {
            // let split_path: Vec<&str> = file_hash.split('/').collect();
            // let rest_of_path = file_hash.replacen(split_path[0], "", 1);
            // let public_url = format!("https://{}.ipfs.dweb.link{}", split_path[0], rest_of_path);
            // let request = reqwest::get(&public_url).await;
            // let request_body = request.unwrap().bytes().await;
            // request_body.unwrap().to_vec()
            e.to_string().as_bytes().to_vec()
        }
    }
}

/// Download an IPFS file to the local machine using an in-memory IPFS node
///
/// # Arguments
///
/// * `file_hash` - The CID of the file
async fn download_ipfs_file_natively(file_hash: String) -> Vec<u8> {
    let ipfs = setup_native_ipfs().await;

    // Get the IPFS file
    let path = file_hash.parse::<IpfsPath>().unwrap();
    let stream = ipfs.cat_unixfs(path, None).await.unwrap();
    tokio::pin!(stream);
    let mut file_vec: Vec<u8> = vec![];
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

/// Get the default IPFS options for Oku's native IPFS instance
fn ipfs_options() -> ipfs::IpfsOptions {
    IpfsOptions {
        ipfs_path: PathBuf::from(CACHE_DIR.to_owned()),
        keypair: Keypair::generate_ed25519(),
        mdns: true,
        bootstrap: Default::default(),
        kad_protocol: None,
        listening_addrs: vec!["/ip4/127.0.0.1/tcp/0".parse().unwrap()],
        span: None,
    }
}

/// Create a new entry in the TabBar
///
/// # Arguments
///
/// * `tabs` - The TabBar containing the tabs of the current browser session
///  
/// * `verbose` - Whether browser messages should be printed onto the standard output
///
/// * `is_private` - Whether the window represents a private session
///
/// * `native` - Whether the browser is using a built-in (native) IPFS handler, or an external one
fn new_tab_page(
    tabs: &libadwaita::TabBar,
    verbose: bool,
    is_private: bool,
    native: bool,
) -> webkit2gtk::WebView {
    let tab_view = tabs.view().unwrap();
    let new_view = new_view(verbose, is_private, native, tabs);
    let new_page = tab_view.append(&new_view);
    new_page.set_title("New Tab");
    new_page.set_icon(Some(&gio::ThemedIcon::new("applications-internet")));
    tab_view.set_selected_page(&new_page);
    tab_view.connect_indicator_activated(clone!(@weak new_view, @weak new_page => move |_, _| {
        new_view.connect_is_playing_audio_notify(clone!(@weak new_view, @weak new_page => move |_| {
            if new_view.is_playing_audio() {
                if !new_view.is_muted() {
                    new_view.set_is_muted(true);
                    new_page.set_indicator_icon(Some(&gio::ThemedIcon::new("notification-audio-volume-muted")));    
                    new_page.set_indicator_activatable(true);
                } else {
                    new_view.set_is_muted(false);
                    new_page.set_indicator_icon(Some(&gio::ThemedIcon::new("notification-audio-volume-high")));
                    new_page.set_indicator_activatable(true);
                }
            } else {
                new_page.set_indicator_icon(gio::Icon::NONE);
                new_page.set_indicator_activatable(false);
            }
        }));
    }));
    new_view
}

/// Get the WebKit instance for the current tab
///
/// # Arguments
///
/// * `tabs` - The TabBar containing the tabs of the current browser session
fn get_view(tabs: &libadwaita::TabBar) -> webkit2gtk::WebView {
    let tab_view = tabs.view().unwrap();
    let current_page = tab_view.selected_page().unwrap();
    let current_page_number = tab_view.page_position(&current_page);
    let specific_page = tab_view.nth_page(current_page_number);
    specific_page.child().downcast().unwrap()
}

/// Create an initial tab, for when the TabBar is empty
///
/// # Arguments
///
/// * `tabs` - The TabBar containing the tabs of the current browser session
///
/// * `verbose` - Whether browser messages should be printed onto the standard output
///
/// * `is_private` - Whether the window represents a private session
///
/// * `native` - Whether the browser is using a built-in (native) IPFS handler, or an external one
fn create_initial_tab(
    tabs: &libadwaita::TabBar,
    initial_url: String,
    verbose: bool,
    is_private: bool,
    native: bool,
) {
    let web_view = new_tab_page(tabs, verbose, is_private, native);
    initial_connect(initial_url, &web_view)
}

/// Update a tab's icon
///
/// # Arguments
///
/// * `web_view` - The WebKit instance for the tab
fn update_favicon(web_view: &webkit2gtk::WebView) {
    let tab_view: libadwaita::TabView = web_view
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .downcast()
        .unwrap();
    let relevant_page = tab_view.page(web_view);
    let web_favicon = &web_view.favicon();
    match &web_favicon {
        Some(_) => {
            let favicon_surface =
                cairo::ImageSurface::try_from(web_favicon.to_owned().unwrap()).unwrap();
            let mut favicon_png_bytes: Vec<u8> = Vec::new();
            favicon_surface
                .write_to_png(&mut favicon_png_bytes)
                .unwrap();
            let icon = gio::BytesIcon::new(&glib::Bytes::from(&favicon_png_bytes));
            relevant_page.set_icon(Some(&icon));
        }
        None => {
            relevant_page.set_icon(Some(&gio::ThemedIcon::new("applications-internet")));
        }
    }
}

/// Update a tab's title
///
/// # Arguments
///
/// * `web_view` - The WebKit instance for the tab
fn update_title(web_view: &webkit2gtk::WebView) {
    let tab_view: libadwaita::TabView = web_view
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .downcast()
        .unwrap();
    let relevant_page = tab_view.page(web_view);
    let web_page_title = &web_view.title();
    match web_page_title {
        Some(page_title) => {
            if page_title.as_str() == "" {
                relevant_page.set_title("Untitled");
            } else {
                relevant_page.set_title(page_title)
            }
        }
        None => {
            relevant_page.set_title("Untitled");
        }
    }
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
///
/// # Arguments
///
/// * `application` - The application data representing Oku
fn new_about_dialog(application: &gtk::Application) {
    //let about_dialog_builder = gtk::AboutDialogBuilder::new();
    let about_dialog = gtk::AboutDialog::builder()
        .version(VERSION.unwrap())
        .program_name("Oku")
        .logo_icon_name("com.github.dirout.oku")
        .title("About Oku")
        .application(application)
        .icon_name("com.github.dirout.oku")
        .license_type(gtk::License::Agpl30)
        .copyright("Copyright © 2020, 2021, 2022 Emil Sayahi")
        .destroy_with_parent(true)
        .modal(true)
        .build();
    about_dialog.show();
}

/// The main function of Oku
fn main() {
    let application = gtk::Application::new(Some("com.github.dirout.oku"), Default::default());

    // application.add_main_option("url", glib::Char('u' as i8), OptionFlags::NONE, OptionArg::String, "An optional URL to open", Some("Open a URL in the browser"));
    // application.add_main_option("verbose", glib::Char('v' as i8), OptionFlags::NONE, OptionArg::None, "Output browser messages to standard output", None);
    // application.add_main_option("private", glib::Char('p' as i8), OptionFlags::NONE, OptionArg::None, "Open a private session", None);

    // application.connect_activate(move |app| {
    //     let matches = VariantDict::new(None);
    //     new_window(app, matches);
    // });
    application.connect_activate(clone!(@weak application => move |_| {
        new_window_four(&application);
    }));

    // application.connect_handle_local_options(|app, options| {
    //     let matches = options.to_owned();
    //     app.run_with_args(&args().collect::<Vec<_>>());
    //     new_window(app, matches);
    //     0
    // });

    // application.run_with_args(&args().collect::<Vec<_>>());
    application.run();
}

/// Create a new functional & graphical browser window
///
/// # Arguments
///
/// * `application` - The application data representing Oku
///
/// * `matches` - The launch arguments passed to Oku
/*
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
        create_initial_tab(&tabs, &nav_entry,initial_url.to_owned(), verbose, is_private, native)
    }

    tab_view.connect_pages_notify(
        clone!(@weak nav_entry, @weak builder, @weak tabs, @weak window => move |_| {
            let web_view = get_view(&tabs);
            update_nav_bar(&nav_entry, &web_view);
            window.set_title(Some(&web_view.title().unwrap_or_else(|| glib::GString::from("Oku")).to_string()));
        }),
    );

    nav_entry.connect_activate(clone!(@weak tabs, @weak nav_entry, @weak builder, @weak window => move |_| {
        let web_view = get_view(&tabs);
        connect(&nav_entry, &web_view);
    }));

    add_tab.connect_clicked(clone!(@weak tabs, @weak nav_entry, @weak builder => move |_| {
        let _tab_view = tabs.view().unwrap();
        let _web_view = new_tab_page(&tabs, &nav_entry, verbose, is_private, native);
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
        clone!(@weak tabs, @weak nav_entry, @weak window => move |_| {
            new_about_dialog(&window.application().unwrap())
        }),
    );

    zoomin_button.connect_clicked(
        clone!(@weak tabs, @weak nav_entry, @weak builder => move |_| {
            let web_view = get_view(&tabs);
            let current_zoom_level = web_view.zoom_level();
            web_view.set_zoom_level(current_zoom_level + 0.1);
        }),
    );

    zoomout_button.connect_clicked(
        clone!(@weak tabs, @weak nav_entry, @weak builder => move |_| {
            let web_view = get_view(&tabs);
            let current_zoom_level = web_view.zoom_level();
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
*/

/// Create a new functional & graphical browser window
///
/// # Arguments
///
/// * `application` - The application data representing Oku
fn new_window_four(application: &gtk::Application) -> libadwaita::TabView {
    // Options
    let verbose = true;
    let is_private = true;
    let native = true;
    let initial_url = "about:blank";

    // Browser header
    // Navigation bar
    //let nav_entry_builder = gtk::EntryBuilder::new();
    let nav_entry = gtk::Entry::builder()
        .can_focus(true)
        .focusable(true)
        .focus_on_click(true)
        .editable(true)
        .margin_top(4)
        .margin_bottom(4)
        .hexpand(true)
        .truncate_multiline(true)
        .placeholder_text("Enter an address … ")
        .input_purpose(gtk::InputPurpose::Url)
        .build();

    // Back button
    //let back_button_builder = gtk::ButtonBuilder::new();
    let back_button = gtk::Button::builder()
        .can_focus(true)
        .receives_default(true)
        .halign(gtk::Align::Start)
        .margin_top(4)
        .margin_bottom(4)
        .icon_name("go-previous")
        .build();
    back_button.style_context().add_class("linked");

    // Forward button
    //let forward_button_builder = gtk::ButtonBuilder::new();
    let forward_button = gtk::Button::builder()
        .can_focus(true)
        .receives_default(true)
        .halign(gtk::Align::Start)
        .margin_top(4)
        .margin_bottom(4)
        .icon_name("go-next")
        .build();
    forward_button.style_context().add_class("linked");

    // All navigation buttons
    //let navigation_buttons_builder = gtk::BoxBuilder::new();
    let navigation_buttons = gtk::Box::builder().homogeneous(true).build();
    navigation_buttons.append(&back_button);
    navigation_buttons.append(&forward_button);
    navigation_buttons.style_context().add_class("linked");

    // Add Tab button
    //let add_tab_builder = gtk::ButtonBuilder::new();
    let add_tab = gtk::Button::builder()
        .can_focus(true)
        .receives_default(true)
        .margin_start(4)
        .margin_top(4)
        .margin_bottom(4)
        .icon_name("list-add")
        .build();

    // Refresh button
    //let refresh_button_builder = gtk::ButtonBuilder::new();
    let refresh_button = gtk::Button::builder()
        .can_focus(true)
        .receives_default(true)
        .halign(gtk::Align::Start)
        .margin_start(4)
        .margin_end(8)
        .margin_top(4)
        .margin_bottom(4)
        .icon_name("view-refresh")
        .build();

    // Left header buttons
    //let left_header_buttons_builder = gtk::BoxBuilder::new();
    let left_header_buttons = gtk::Box::builder().margin_end(4).build();
    left_header_buttons.append(&navigation_buttons);
    left_header_buttons.append(&add_tab);
    left_header_buttons.append(&refresh_button);

    // Downloads button
    //let downloads_button_builder = gtk::ButtonBuilder::new();
    let downloads_button = gtk::Button::builder()
        .can_focus(true)
        .receives_default(true)
        .halign(gtk::Align::Start)
        .margin_start(4)
        .margin_bottom(4)
        .icon_name("emblem-downloads")
        .build();

    // Find button
    //let find_button_builder = gtk::ButtonBuilder::new();
    let find_button = gtk::Button::builder()
        .can_focus(true)
        .receives_default(true)
        .halign(gtk::Align::Start)
        .margin_start(4)
        .margin_bottom(4)
        .icon_name("edit-find")
        .build();
    
    // IPFS button
    let ipfs_button = gtk::Button::builder()
        .can_focus(true)
        .receives_default(true)
        .halign(gtk::Align::Start)
        .margin_start(4)
        .margin_bottom(4)
        .icon_name("emblem-shared")
        .build();

    // Onion routing button
    let tor_button = gtk::Button::builder()
        .can_focus(true)
        .receives_default(true)
        .halign(gtk::Align::Start)
        .hexpand(false)
        .vexpand(false)
        .overflow(gtk::Overflow::Hidden)
        .margin_start(4)
        .margin_bottom(4)
        .label("🧅")
        .build();

    // Menu button
    //let menu_button_builder = gtk::ButtonBuilder::new();
    let menu_button = gtk::Button::builder()
        .can_focus(true)
        .receives_default(true)
        .halign(gtk::Align::Start)
        .margin_start(4)
        .margin_bottom(4)
        .icon_name("document-properties")
        .build();

    // Right header buttons
    //let right_header_buttons_builder = gtk::BoxBuilder::new();
    let right_header_buttons = gtk::Box::builder()
        .margin_start(4)
        .spacing(2)
        .homogeneous(true)
        .build();
    right_header_buttons.append(&downloads_button);
    right_header_buttons.append(&find_button);
    right_header_buttons.append(&ipfs_button);
    //right_header_buttons.append(&tor_button);
    right_header_buttons.append(&menu_button);

    // HeaderBar
    //let headerbar_builder = gtk::HeaderBarBuilder::new();
    let headerbar = gtk::HeaderBar::builder()
        .can_focus(true)
        .show_title_buttons(true)
        .title_widget(&nav_entry)
        .build();
    headerbar.pack_start(&left_header_buttons);
    headerbar.pack_end(&right_header_buttons);
    // End of browser header

    // Zoom out button
    //let zoomout_button_builder = gtk::ButtonBuilder::new();
    let zoomout_button = gtk::Button::builder()
        .can_focus(true)
        .receives_default(true)
        .halign(gtk::Align::Start)
        .margin_top(4)
        .margin_bottom(4)
        .icon_name("zoom-out")
        .build();
    zoomout_button.style_context().add_class("linked");

    // Zoom in button
    //let zoomin_button_builder = gtk::ButtonBuilder::new();
    let zoomin_button = gtk::Button::builder()
        .can_focus(true)
        .receives_default(true)
        .halign(gtk::Align::Start)
        .margin_top(4)
        .margin_bottom(4)
        .icon_name("zoom-in")
        .build();
    zoomin_button.style_context().add_class("linked");

    // Both zoom buttons
    //let zoom_buttons_builder = gtk::BoxBuilder::new();
    let zoom_buttons = gtk::Box::builder().homogeneous(true).build();
    zoom_buttons.append(&zoomout_button);
    zoom_buttons.append(&zoomin_button);
    zoom_buttons.style_context().add_class("linked");

    // Zoom reset button
    //let zoomreset_button_builder = gtk::ButtonBuilder::new();
    let zoomreset_button = gtk::Button::builder()
        .can_focus(true)
        .receives_default(true)
        .halign(gtk::Align::Start)
        .margin_top(4)
        .margin_bottom(4)
        .icon_name("zoom-original")
        .build();

    // Fullscreen button
    //let fullscreen_button_builder = gtk::ButtonBuilder::new();
    let fullscreen_button = gtk::Button::builder()
        .can_focus(true)
        .receives_default(true)
        .halign(gtk::Align::Start)
        .margin_top(4)
        .margin_bottom(4)
        .icon_name("video-display")
        .build();

    // Screenshot button
    //let screenshot_button_builder = gtk::ButtonBuilder::new();
    let screenshot_button = gtk::Button::builder()
        .can_focus(true)
        .receives_default(true)
        .halign(gtk::Align::Start)
        .margin_top(4)
        .margin_bottom(4)
        .icon_name("camera-photo")
        .build();

    // New Window button
    //let new_window_button_builder = gtk::ButtonBuilder::new();
    let new_window_button = gtk::Button::builder()
        .can_focus(true)
        .receives_default(true)
        .halign(gtk::Align::Start)
        .margin_top(4)
        .margin_bottom(4)
        .icon_name("window-new")
        .build();

    // History button
    //let history_button_builder = gtk::ButtonBuilder::new();
    let history_button = gtk::Button::builder()
        .can_focus(true)
        .receives_default(true)
        .halign(gtk::Align::Start)
        .margin_top(4)
        .margin_bottom(4)
        .icon_name("document-open-recent")
        .build();

    // Settings button
    //let settings_button_builder = gtk::ButtonBuilder::new();
    let settings_button = gtk::Button::builder()
        .can_focus(true)
        .receives_default(true)
        .halign(gtk::Align::Start)
        .margin_top(4)
        .margin_bottom(4)
        .icon_name("preferences-system")
        .build();

    // About button
    //let about_button_builder = gtk::ButtonBuilder::new();
    let about_button = gtk::Button::builder()
        .can_focus(true)
        .receives_default(true)
        .halign(gtk::Align::Start)
        .margin_top(4)
        .margin_bottom(4)
        .icon_name("help-about")
        .build();

    // Menu popover
    //let menu_box_builder = gtk::BoxBuilder::new();
    let menu_box = gtk::Box::builder()
        .margin_start(4)
        .margin_end(4)
        .margin_top(4)
        .margin_bottom(4)
        .spacing(8)
        .build();
    menu_box.append(&zoom_buttons);
    menu_box.append(&zoomreset_button);
    menu_box.append(&fullscreen_button);
    menu_box.append(&screenshot_button);
    menu_box.append(&new_window_button);
    menu_box.append(&history_button);
    menu_box.append(&settings_button);
    menu_box.append(&about_button);

    //let menu_builder = gtk::PopoverBuilder::new();
    let menu = gtk::Popover::builder().child(&menu_box).build();
    menu.set_parent(&menu_button);
    // End of menu popover

    // Tabs
    //let tab_view_builder = libadwaita::TabViewBuilder::new();
    let tab_view = libadwaita::TabView::builder().vexpand(true).build();

    //let tabs_builder = libadwaita::TabBarBuilder::new();
    let tabs = libadwaita::TabBar::builder()
        .autohide(true)
        .expand_tabs(true)
        .view(&tab_view)
        .build();

    if tab_view.n_pages() == 0 {
        create_initial_tab(&tabs, initial_url.to_owned(), verbose, is_private, native)
    }
    // End of Tabs

    // Window
    //let main_box_builder = gtk::BoxBuilder::new();
    let main_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .vexpand(true)
        .build();
    main_box.append(&tabs);
    main_box.append(&tab_view);

    //let window_builder = gtk::ApplicationWindowBuilder::new();
    let window = gtk::ApplicationWindow::builder()
        .application(application)
        .can_focus(true)
        .title("Oku")
        .icon_name("com.github.dirout.oku")
        .build();
    window.set_titlebar(Some(&headerbar));
    window.set_child(Some(&main_box));
    // End of Window

    // Signals
    // Add Tab button clicked
    add_tab.connect_clicked(clone!(@weak tabs => move |_| {
        new_tab_page(&tabs, verbose, is_private, native);
    }));

    // Back button clicked
    back_button.connect_clicked(clone!(@weak tabs, @weak nav_entry => move |_| {
        let web_view = get_view(&tabs);
        web_view.go_back()
    }));

    // Forward button clicked
    forward_button.connect_clicked(clone!(@weak tabs, @weak nav_entry => move |_| {
        let web_view = get_view(&tabs);
        web_view.go_forward()
    }));

    // Refresh button clicked
    refresh_button.connect_clicked(clone!(@weak tabs, @weak nav_entry => move |_| {
        let web_view = get_view(&tabs);
        web_view.reload_bypass_cache()
    }));

    // Selected tab changed
    tab_view.connect_selected_page_notify(
        clone!(@weak nav_entry, @weak tabs, @weak window => move |_| {
            let web_view = get_view(&tabs);
            update_nav_bar(&nav_entry, &web_view);
            window.set_title(Some(&web_view.title().unwrap_or_else(|| glib::GString::from("Oku")).to_string()));
        }),
    );

    // User hit return key in navbar, prompting navigation
    nav_entry.connect_activate(
        clone!(@weak tabs, @weak nav_entry, @weak window => move |_| {
            let web_view = get_view(&tabs);
            connect(&nav_entry, &web_view);
        }),
    );

    // Menu button clicked
    menu_button.connect_clicked(clone!(@weak menu => move |_| {
        menu.popup();
    }));

    // Zoom-in button clicked
    zoomin_button.connect_clicked(clone!(@weak tabs, @weak nav_entry => move |_| {
        let web_view = get_view(&tabs);
        let current_zoom_level = web_view.zoom_level();
        web_view.set_zoom_level(current_zoom_level + 0.1);
    }));

    // Zoom-out button clicked
    zoomout_button.connect_clicked(clone!(@weak tabs, @weak nav_entry => move |_| {
        let web_view = get_view(&tabs);
        let current_zoom_level = web_view.zoom_level();
        web_view.set_zoom_level(current_zoom_level - 0.1);
    }));

    // Reset Zoom button clicked
    zoomreset_button.connect_clicked(clone!(@weak tabs, @weak nav_entry => move |_| {
        let web_view = get_view(&tabs);
        web_view.set_zoom_level(1.0);
    }));

    // Enter Fullscreen button clicked
    fullscreen_button.connect_clicked(
        clone!(@weak tabs, @weak nav_entry => move |_| {
            let web_view = get_view(&tabs);
            web_view.run_javascript("document.documentElement.webkitRequestFullscreen();", gio::Cancellable::NONE, move |_| {
            })
        }),
    );

    // Screenshot button clicked
    screenshot_button.connect_clicked(
        clone!(@weak tabs, @weak nav_entry => move |_| {
            let web_view = get_view(&tabs);
            web_view.snapshot(webkit2gtk::SnapshotRegion::FullDocument, webkit2gtk::SnapshotOptions::all(), gio::Cancellable::NONE, move |snapshot| {
                let snapshot_surface = cairo::ImageSurface::try_from(snapshot.unwrap()).unwrap();
                let mut writer = File::create(format!("{}/{}.png", PICTURES_DIR.to_owned(), Utc::now())).unwrap();
                snapshot_surface.write_to_png(&mut writer).unwrap();
            });
        }),
    );

    // New Window button clicked
    new_window_button.connect_clicked(
        clone!(@weak tabs, @weak nav_entry, @weak window => move |_| {
            new_window_four(&window.application().unwrap());
        }),
    );

    // About button clicked
    about_button.connect_clicked(
        clone!(@weak tabs, @weak nav_entry, @weak window => move |_| {
            new_about_dialog(&window.application().unwrap())
        }),
    );

    // Tab dragged off to create new browser window
    tab_view.connect_create_window(create_window_from_drag);
    // End of signals

    window.show();
    tab_view
}

/// Create new browser window when a tab is dragged off
///
/// # Arguments
///
/// * `tab_view` - The AdwTabView object containing each tab's WebView
fn create_window_from_drag(
    tab_view: &libadwaita::TabView,
) -> std::option::Option<libadwaita::TabView> {
    let window: gtk::ApplicationWindow = tab_view
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .downcast()
        .unwrap();
    let application = window.application().unwrap();
    let new_window = new_window_four(&application);
    Some(new_window)
}
