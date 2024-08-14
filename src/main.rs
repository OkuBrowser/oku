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

// #![cfg_attr(feature = "dox", feature(doc_cfg))]
// #![cfg(feature = "mem-db")]
#![allow(clippy::needless_doctest_main)]
#![doc(
    html_logo_url = "https://github.com/Dirout/oku/raw/master/branding/logo-filled.svg",
    html_favicon_url = "https://github.com/Dirout/oku/raw/master/branding/logo-filled.svg"
)]
// #![feature(async_closure)]
pub mod widgets;
pub mod window_util;

use chrono::Utc;
use cid::Cid;
use directories_next::ProjectDirs;
use directories_next::UserDirs;
use futures::pin_mut;
use gdk::prelude::TextureExt;
use gio::prelude::*;
use glib::prelude::Cast;
use glib_macros::clone;
use gtk::prelude::BoxExt;
use gtk::prelude::ButtonExt;
use gtk::prelude::EditableExt;
use gtk::prelude::EntryExt;
use gtk::prelude::GtkWindowExt;
use gtk::prelude::PopoverExt;
use gtk::prelude::WidgetExt;
use ipfs::Ipfs;
use ipfs::Keypair;
use ipfs::UninitializedIpfsNoop as UninitializedIpfs;
use libadwaita::prelude::AdwApplicationWindowExt;
use libadwaita::TabOverview;
use std::convert::TryFrom;
use tokio::runtime::Handle;
use tokio_stream::StreamExt;
use url::ParseError;
use url::Url;
use urlencoding::decode;
use webkit2gtk::prelude::PolicyDecisionExt;
use webkit2gtk::prelude::WebViewExt;
use webkit2gtk::NavigationPolicyDecision;
use webkit2gtk::PolicyDecisionType;
use webkit2gtk::Settings;
use webkit2gtk::URISchemeRequest;
use webkit2gtk::WebView;

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

/// Provide the default configuration for Oku's WebView
fn new_webkit_settings() -> webkit2gtk::Settings {
    let settings = Settings::new();

    settings.set_javascript_can_open_windows_automatically(true);
    settings.set_allow_modal_dialogs(true);
    settings.set_allow_file_access_from_file_urls(true);
    settings.set_allow_top_navigation_to_data_urls(true);
    settings.set_allow_universal_access_from_file_urls(true);
    settings.set_auto_load_images(true);
    // .draw_compositing_indicators(true)
    // .enable_accelerated_2d_canvas(false)
    settings.set_enable_back_forward_navigation_gestures(true);
    settings.set_enable_caret_browsing(false);
    settings.set_enable_developer_extras(true);
    settings.set_enable_dns_prefetching(true);
    settings.set_enable_encrypted_media(true);
    // settings.set_enable_frame_flattening(true);
    settings.set_enable_fullscreen(true);
    settings.set_enable_html5_database(true);
    settings.set_enable_html5_local_storage(true);
    settings.set_enable_hyperlink_auditing(true);
    // settings.set_enable_java(true);
    settings.set_enable_javascript(true);
    settings.set_enable_javascript_markup(true);
    settings.set_enable_media(true);
    settings.set_enable_media_capabilities(true);
    settings.set_enable_media_stream(true);
    settings.set_enable_mediasource(true);
    settings.set_enable_mock_capture_devices(true);
    settings.set_enable_offline_web_application_cache(true);
    settings.set_enable_page_cache(true);
    // .enable_plugins(true)
    // .enable_private_browsing(true)
    settings.set_enable_resizable_text_areas(true);
    settings.set_enable_site_specific_quirks(true);
    settings.set_enable_smooth_scrolling(true);
    settings.set_enable_spatial_navigation(true);
    settings.set_enable_tabs_to_links(true);
    settings.set_enable_webaudio(true);
    settings.set_enable_webgl(true);
    settings.set_enable_write_console_messages_to_stdout(true);
    // settings.set_enable_xss_auditor(true);
    settings.set_hardware_acceleration_policy(webkit2gtk::HardwareAccelerationPolicy::Never);
    settings.set_javascript_can_access_clipboard(true);
    settings.set_load_icons_ignoring_image_load_setting(true);
    settings.set_media_playback_allows_inline(true);
    settings.set_media_playback_requires_user_gesture(false);
    settings.set_print_backgrounds(true);
    settings.set_zoom_text_only(false);
    settings
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
}

/// Create a new WebKit instance for the current tab
///
/// # Arguments
///  
/// * `verbose` - Whether browser messages should be printed onto the standard output
///
/// * `is_private` - Whether the window represents a private session
///
/// * `ipfs_button` - Button indicating whether the browser is using a built-in (native) IPFS handler, or an external one
///
/// * `headerbar` - The browser's headerbar
fn new_view(
    verbose: bool,
    _is_private: bool,
    ipfs_button: &gtk::ToggleButton,
    tabs: &libadwaita::TabBar,
    _headerbar: &libadwaita::HeaderBar,
    ipfs: &Ipfs,
) -> webkit2gtk::WebView {
    let web_settings: webkit2gtk::Settings = new_webkit_settings();
    let web_view = WebView::new();
    web_view.set_vexpand(true);
    let network_session = web_view.network_session().unwrap();
    let data_manager = network_session.website_data_manager().unwrap();
    let web_context = web_view.context().unwrap();
    let security_manager = web_context.security_manager().unwrap();
    let extensions_path = format!("{}/web-extensions/", *DATA_DIR);

    // match native {
    //     true => {
    //         web_context.register_uri_scheme("ipfs", move |request| {
    //             handle_ipfs_request_natively(request)
    //             // let request_url = request.uri().unwrap().to_string();
    //             // let decoded_url = decode(&request_url).unwrap();
    //             // let ipfs_path = decoded_url.replacen("ipfs://", "", 1);
    //             // let ipfs_bytes = from_hash_natively(ipfs_path);
    //             // //let ipfs_bytes = download_ipfs_file_natively(ipfs_path).await;
    //             // let stream = gio::MemoryInputStream::from_bytes(&glib::Bytes::from(&ipfs_bytes));
    //             // request.finish(&stream, ipfs_bytes.len().try_into().unwrap(), None);
    //         });
    //     }
    //     false => {
    //         web_context.register_uri_scheme("ipfs", move |request| {
    //             handle_ipfs_request_using_api(request)
    //             // let request_url = request.uri().unwrap().to_string();
    //             // let decoded_url = decode(&request_url).unwrap();
    //             // let ipfs_path = decoded_url.replacen("ipfs://", "", 1);
    //             // let ipfs_bytes = from_hash_using_api(ipfs_path);
    //             // //let ipfs_bytes = download_ipfs_file_from_api(ipfs_path).await;
    //             // let stream = gio::MemoryInputStream::from_bytes(&glib::Bytes::from(&ipfs_bytes));
    //             // request.finish(&stream, ipfs_bytes.len().try_into().unwrap(), None);
    //         });
    //     }
    // };
    // web_context.register_uri_scheme(
    //     "ipfs",
    //     clone!(@weak ipfs_button => move |request| {
    //         handle_ipfs_request(&ipfs_button, request);
    //     }),
    // );
    data_manager.set_favicons_enabled(true);
    // web_context.register_uri_scheme(
    //     "ipns",
    //     clone!(@strong ipfs => move |request: &URISchemeRequest| {
    //         let request_url = request.uri().unwrap().to_string();
    //         let decoded_url = decode(&request_url).unwrap();
    //         let ipns_path = decoded_url.replacen("ipns://", "", 1).parse::<ipfs::IpfsPath>().unwrap();
    //         // let ipns = ipfs.ipns();
    //         let mut mem_stream = gio::MemoryInputStream::new();
    //         let mut mem_stream_ref = mem_stream.as_object_ref();
    //         let async_handle = Handle::current();
    //         let _enter_guard = async_handle.enter();
    //         let ipfs_clone = ipfs.clone();
    //         let file_vec: Vec<u8> = futures::executor::block_on(async_handle.spawn(async move {
    //             let mut resolved = ipfs_clone.resolve_ipns(&ipns_path, true).await.unwrap();
    //             let mut ipfs_stream = ipfs_clone.cat_unixfs(resolved);
    //             let mut file_vec: Vec<u8> = vec![];
    //             pin_mut!(ipfs_stream);
    //             while let Some(result) = ipfs_stream.next().await {
    //                 match result {
    //                     Ok(bytes) => {
    //                         file_vec.extend(bytes);
    //                     }
    //                     Err(e) => {
    //                         eprintln!("Error: {}", e);
    //                         break;
    //                     }
    //                 }
    //             }
    //             return file_vec;
    //         })).unwrap();
    //         let byte_size = file_vec.len();
    //         mem_stream.add_bytes(&glib::Bytes::from_owned(file_vec));
    //         request.finish(&mem_stream, byte_size, None);
    //     }),
    // );
    web_context.register_uri_scheme(
        "ipns",
        clone!(@strong ipfs => move |request: &URISchemeRequest| {
            let ctx = glib::MainContext::default();
            let request_url = request.uri().unwrap().to_string();
            let decoded_url = decode(&request_url).unwrap();
            let ipns_path = format!("/ipns/{}", decoded_url.replacen("ipns://", "", 1)).parse::<ipfs::IpfsPath>().unwrap();
            let mem_stream = gio::MemoryInputStream::new();
            let mem_stream_ref = mem_stream.as_object_ref();
            let async_handle = Handle::current();
            let _enter_guard = async_handle.enter();
            ctx.spawn_local_with_priority(glib::source::Priority::HIGH, clone!(@weak request, @strong ipfs => async move {
                let resolved_ipns_path = ipfs.resolve_ipns(&ipns_path, true).await.unwrap();
                let ipfs_stream = ipfs.cat_unixfs(resolved_ipns_path);
                let mut file_vec: Vec<u8> = vec![];
                pin_mut!(ipfs_stream);
                while let Some(result) = ipfs_stream.next().await {
                    match result {
                        Ok(bytes) => {
                            file_vec.extend(bytes);
                        }
                        Err(e) => {
                            eprintln!("Error: {} (streamed {} bytes)", e, file_vec.len());
                            if file_vec.len() == 0 {
                                request.finish_error(&mut glib::error::Error::new(gio::ResolverError::NotFound, &e.to_string()));
                            }
                            break;
                        }
                    }
                }
                let byte_size = file_vec.len();
            mem_stream.add_bytes(&glib::Bytes::from_owned(file_vec));
            request.finish(&mem_stream, byte_size.try_into().unwrap(), None);
            }));
        }),
    );
    web_context.register_uri_scheme(
        "ipfs",
        clone!(@strong ipfs => move |request: &URISchemeRequest| {
            let ctx = glib::MainContext::default();
            let request_url = request.uri().unwrap().to_string();
            let decoded_url = decode(&request_url).unwrap();
            let ipfs_path = decoded_url.replacen("ipfs://", "", 1).parse::<ipfs::IpfsPath>().unwrap();
            let mem_stream = gio::MemoryInputStream::new();
            let mem_stream_ref = mem_stream.as_object_ref();
            let ipfs_stream = ipfs.cat_unixfs(ipfs_path);
            let async_handle = Handle::current();
            let _enter_guard = async_handle.enter();
            ctx.spawn_local_with_priority(glib::source::Priority::HIGH, clone!(@weak request => async move {
                let mut file_vec: Vec<u8> = vec![];
                pin_mut!(ipfs_stream);
                while let Some(result) = ipfs_stream.next().await {
                    match result {
                        Ok(bytes) => {
                            file_vec.extend(bytes);
                        }
                        Err(e) => {
                            eprintln!("Error: {} (streamed {} bytes)", e, file_vec.len());
                            if file_vec.len() == 0 {
                                request.finish_error(&mut glib::error::Error::new(gio::ResolverError::NotFound, &e.to_string()));
                            }
                            break;
                        }
                    }
                }
                let byte_size = file_vec.len();
            mem_stream.add_bytes(&glib::Bytes::from_owned(file_vec));
            request.finish(&mem_stream, byte_size.try_into().unwrap(), None);
            }));
        }),
    );
    security_manager.register_uri_scheme_as_secure("ipfs");
    security_manager.register_uri_scheme_as_secure("ipns");
    web_settings.set_user_agent_with_application_details(Some("Oku"), Some(VERSION.unwrap()));
    web_settings.set_enable_write_console_messages_to_stdout(verbose);
    web_view.set_settings(&web_settings);
    web_context.set_web_process_extensions_directory(&extensions_path);
    // web_context.set_favicon_database_directory(Some(&favicon_database_path));
    //web_context.use_system_appearance_for_scrollbars(true);
    web_view.set_visible(true);
    web_view.set_width_request(1024);
    web_view.set_height_request(640);
    web_view.load_uri("about:blank");

    network_session.connect_download_started(clone!(@weak tabs => move |_w, download| {
        libadwaita::MessageDialog::new(
            gtk::Window::NONE,
            Some("Download file?"),
            Some(&format!(
                "Would you like to download '{}'?",
                download.request().unwrap().uri().unwrap()
            )),
        );
    }));

    web_view.connect_title_notify(clone!(@weak tabs => move |w| update_title(&w)));
    // web_view.connect_uri_notify(clone!(@weak tabs => move |w| {
    //     let window: libadwaita::ApplicationWindow = tabs.parent().unwrap().parent().unwrap().parent().unwrap().downcast().unwrap();
    //     let headerbar: libadwaita::HeaderBar = window.content().unwrap().first_child().unwrap().downcast().unwrap();
    //     let nav_entry: gtk::Entry = headerbar.title_widget().unwrap().downcast().unwrap();
    //     update_nav_bar(&nav_entry, &w)
    // }));
    // web_view.connect_estimated_load_progress_notify(
    //     clone!(@weak tabs => move |w| {
    //         //let window: libadwaita::ApplicationWindow = web_view.parent().unwrap().parent().unwrap().parent().unwrap().parent().unwrap().downcast().unwrap();
    //         let window: libadwaita::ApplicationWindow = tabs.parent().unwrap().parent().unwrap().parent().unwrap().downcast().unwrap();
    //         let headerbar: libadwaita::HeaderBar = window.content().unwrap().first_child().unwrap().downcast().unwrap();
    //         let nav_entry: gtk::Entry = headerbar.title_widget().unwrap().downcast().unwrap();
    //         let tab_view: libadwaita::TabView = tabs.view().unwrap();
    //         let current_page = tab_view.page(w);
    //         current_page.set_loading(true);
    //         update_load_progress(&nav_entry, &w)
    //     }),
    // );
    // web_view.connect_is_loading_notify(clone!(@weak tabs => move |w| {
    //     let tab_view: libadwaita::TabView = tabs.view().unwrap();
    //     let current_page = tab_view.page(w);
    //     current_page.set_loading(w.is_loading())
    // }));
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
    web_view.connect_favicon_notify(clone!(@weak tabs => move |w| {
        update_favicon(&w)
    }));
    web_view.connect_load_changed(clone!(@weak tabs => move |w, _| {
        let window: gtk::ApplicationWindow = tabs.parent().unwrap().parent().unwrap().parent().unwrap().parent().unwrap().parent().unwrap().parent().unwrap().parent().unwrap().downcast().unwrap();
        window.set_title(Some(&w.title().unwrap_or_else(|| glib::GString::from("Oku")).to_string()));
        update_favicon(&w);
    }));
    web_view.connect_enter_fullscreen(
        clone!(@weak tabs => @default-return false, move |w| {
            let window: libadwaita::ApplicationWindow = w.parent().unwrap().parent().unwrap().parent().unwrap().parent().unwrap().parent().unwrap().parent().unwrap().parent().unwrap().parent().unwrap().parent().unwrap().downcast().unwrap();
            let headerbar: libadwaita::HeaderBar = window.content().unwrap().first_child().unwrap().first_child().unwrap().first_child().unwrap().downcast().unwrap();
            headerbar.set_visible(false);
            tabs.set_visible(false);
            window.set_fullscreened(true);
            tabs.hide();
            tabs.set_opacity(0.0);
            true
        }),
    );
    web_view.connect_leave_fullscreen(
        clone!(@weak tabs, @weak web_view => @default-return false, move |w| {
            let window: libadwaita::ApplicationWindow = w.parent().unwrap().parent().unwrap().parent().unwrap().parent().unwrap().parent().unwrap().parent().unwrap().parent().unwrap().parent().unwrap().parent().unwrap().downcast().unwrap();
            let headerbar: libadwaita::HeaderBar = window.content().unwrap().first_child().unwrap().first_child().unwrap().first_child().unwrap().downcast().unwrap();
            headerbar.set_visible(true);
            tabs.set_visible(true);
            window.set_fullscreened(false);
            tabs.show();
            tabs.set_opacity(100.0);
            true
        }),
    );
    web_view.connect_decide_policy(
        clone!(@weak tabs, @strong ipfs => @default-return false, move |_w, policy_decision, decision_type| {
            match decision_type {
                PolicyDecisionType::NewWindowAction => {
                    let window: libadwaita::ApplicationWindow = tabs.parent().unwrap().parent().unwrap().parent().unwrap().parent().unwrap().parent().unwrap().parent().unwrap().parent().unwrap().downcast().unwrap();
                    let navigation_policy_decision: NavigationPolicyDecision = policy_decision.clone().downcast().unwrap();
                    let new_window = new_window_four(&window.application().unwrap().downcast().unwrap(), Some(&navigation_policy_decision.navigation_action().unwrap().request().unwrap().uri().unwrap()), &ipfs);
                    policy_decision.use_();
                    return true;
                }
                PolicyDecisionType::NavigationAction => {
                    policy_decision.use_();
                    return true;
                }
                PolicyDecisionType::Response => {
                    policy_decision.use_();
                    return true;
                }
                _ => {
                    unreachable!()
                }
            }
            true
        }),
    );

    web_view
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
/// * `ipfs_button` - Button indicating whether the browser is using a built-in (native) IPFS handler, or an external one
///
/// * `headerbar` - The browser's headerbar
fn new_tab_page(
    tabs: &libadwaita::TabBar,
    verbose: bool,
    is_private: bool,
    ipfs_button: &gtk::ToggleButton,
    headerbar: &libadwaita::HeaderBar,
    ipfs: &Ipfs,
) -> (webkit2gtk::WebView, libadwaita::TabPage) {
    let tab_view = tabs.view().unwrap();
    let new_view = new_view(verbose, is_private, ipfs_button, tabs, headerbar, &ipfs);
    let new_page = tab_view.append(&new_view);
    new_page.set_title("New Tab");
    new_page.set_icon(Some(&gio::ThemedIcon::new("content-loading-symbolic")));
    new_page.set_live_thumbnail(true);
    tab_view.set_selected_page(&new_page);
    new_page.set_indicator_icon(Some(&gio::ThemedIcon::new("view-pin-symbolic")));
    new_page.set_indicator_activatable(true);
    // Indicator appearance
    new_view.connect_is_muted_notify(
        clone!(@weak new_view, @weak new_page, @weak tab_view => move |the_view| {
            // Has been muted
            if the_view.is_muted() {
                new_page.set_indicator_icon(Some(&gio::ThemedIcon::new("audio-volume-muted")));
                new_page.set_indicator_activatable(true);
            } else {
                // Has been unmuted, audio is playing
                if the_view.is_playing_audio() {
                    new_page.set_indicator_icon(Some(&gio::ThemedIcon::new("audio-volume-high")));
                    new_page.set_indicator_activatable(true);
                }
                // Has been unmuted, audio is not playing
                else {
                    new_page.set_indicator_icon(Some(&gio::ThemedIcon::new("view-pin-symbolic")));
                    new_page.set_indicator_activatable(true);
                }
            }
    }));
    new_view.connect_is_playing_audio_notify(
        clone!(@weak new_view, @weak new_page, @weak tab_view => move |the_view| {
            // Audio has started playing and not muted
            if the_view.is_playing_audio() && !the_view.is_muted() {
                new_page.set_indicator_icon(Some(&gio::ThemedIcon::new("audio-volume-high")));
                new_page.set_indicator_activatable(true);
            } else if !the_view.is_playing_audio() {
                // Audio has stopped playing, muted
                if the_view.is_muted() {
                    new_page.set_indicator_icon(Some(&gio::ThemedIcon::new("audio-volume-muted")));
                    new_page.set_indicator_activatable(true);
                } else {
                    // Audio has stopped playing, not muted
                    new_page.set_indicator_icon(Some(&gio::ThemedIcon::new("view-pin-symbolic")));
                    new_page.set_indicator_activatable(true);
                }
            }
        }),
    );
    new_view.connect_uri_notify(clone!(@weak tabs, @weak new_view => move |w| {
        let window: libadwaita::ApplicationWindow = new_view.parent().unwrap().parent().unwrap().parent().unwrap().parent().unwrap().parent().unwrap().parent().unwrap().parent().unwrap().parent().unwrap().parent().unwrap().downcast().unwrap();
        let headerbar: libadwaita::HeaderBar = window.content().unwrap().first_child().unwrap().first_child().unwrap().first_child().unwrap().downcast().unwrap();
        let nav_entry: gtk::Entry = headerbar.title_widget().unwrap().downcast().unwrap();
        update_nav_bar(&nav_entry, &w)
    }));
    new_view.connect_estimated_load_progress_notify(
        clone!(@weak tabs, @weak new_view => move |w| {
            let window: libadwaita::ApplicationWindow = new_view.parent().unwrap().parent().unwrap().parent().unwrap().parent().unwrap().parent().unwrap().parent().unwrap().parent().unwrap().parent().unwrap().parent().unwrap().downcast().unwrap();
            // let window: libadwaita::ApplicationWindow = tabs.parent().unwrap().parent().unwrap().parent().unwrap().downcast().unwrap();
            let headerbar: libadwaita::HeaderBar = window.content().unwrap().first_child().unwrap().first_child().unwrap().first_child().unwrap().downcast().unwrap();
            let nav_entry: gtk::Entry = headerbar.title_widget().unwrap().downcast().unwrap();
            let tab_view: libadwaita::TabView = tabs.view().unwrap();
            let current_page = tab_view.page(w);
            current_page.set_loading(true);
            update_load_progress(&nav_entry, &w)
        }),
    );
    new_view.connect_is_loading_notify(clone!(@weak tabs => move |w| {
        let tab_view: libadwaita::TabView = tabs.view().unwrap();
        let current_page = tab_view.page(w);
        current_page.set_loading(w.is_loading())
    }));
    // new_view.connect_is_muted_notify(clone!(@weak new_view, @weak new_page, @weak tab_view => move |_| {
    //     if !new_view.is_muted() {
    //         new_page.set_indicator_icon(Some(&gio::ThemedIcon::new("audio-volume-high")));
    //         new_page.set_indicator_activatable(true);
    //         tab_view.connect_indicator_activated(clone!(@weak new_view, @weak new_page => move |_, _| {
    //             new_view.set_is_muted(true);
    //             new_page.set_indicator_icon(Some(&gio::ThemedIcon::new("audio-volume-muted")));
    //             new_page.set_indicator_activatable(true);
    //         }));
    //     } else {
    //         if new_view.is_playing_audio() {
    //             new_page.set_indicator_icon(Some(&gio::ThemedIcon::new("audio-volume-muted")));
    //             new_page.set_indicator_activatable(true);
    //             tab_view.connect_indicator_activated(clone!(@weak new_view, @weak new_page => move |_, _| {
    //                 new_view.set_is_muted(false);
    //                 new_page.set_indicator_icon(Some(&gio::ThemedIcon::new("audio-volume-high")));
    //                 new_page.set_indicator_activatable(true);
    //             }));
    //         } else {
    //             new_page.set_indicator_icon(Some(&gio::ThemedIcon::new("view-pin-symbolic")));
    //             new_page.set_indicator_activatable(true);
    //             tab_view.connect_indicator_activated(clone!(@weak new_view, @weak new_page, @weak tab_view => move |_, _| {
    //                 tab_view.set_page_pinned(&new_page, !new_page.is_pinned());
    //             }));
    //         }
    //     }
    // }));
    (new_view, new_page)
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

/// Get the WebKit instance for the current tab
///
/// # Arguments
///
/// * `page` - The TabPage containing the the WebKit instance
fn get_view_from_page(page: &libadwaita::TabPage) -> webkit2gtk::WebView {
    page.child().downcast().unwrap()
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
/// * `ipfs_button` - Button indicating whether the browser is using a built-in (native) IPFS handler, or an external one
///
/// * `headerbar` - The browser's headerbar
fn create_initial_tab(
    tabs: &libadwaita::TabBar,
    initial_url: String,
    verbose: bool,
    is_private: bool,
    ipfs_button: &gtk::ToggleButton,
    headerbar: &libadwaita::HeaderBar,
    ipfs: &Ipfs,
) {
    let web_view = new_tab_page(tabs, verbose, is_private, ipfs_button, headerbar, ipfs).0;
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
        Some(favicon_texture) => {
            // let favicon_surface =
            //     cairo::ImageSurface::try_from().unwrap();
            // let mut favicon_png_bytes: Vec<u8> = Vec::new();
            // favicon_surface
            //     .write_to_png(&mut favicon_png_bytes)
            //     .unwrap();
            // let icon = gio::BytesIcon::new(&favicon_texture.save_to_png_bytes());
            relevant_page.set_icon(Some(favicon_texture));
        }
        None => {
            relevant_page.set_icon(Some(&gio::ThemedIcon::new("content-loading-symbolic")));
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
        .copyright("Copyright © Emil Sayahi")
        .destroy_with_parent(true)
        .modal(true)
        .build();
    about_dialog.show();
}

/// The main function of Oku
#[tokio::main]
async fn main() {
    // let db = iroh_bytes::store::mem::Store::new();
    // let store = iroh_sync::store::memory::Store::default();
    // let node = Node::builder(db.clone(), store).spawn().await.unwrap();
    // let client = node.client();

    let application = libadwaita::Application::builder()
        .application_id("com.github.dirout.oku")
        .build();
    // let style_manager = application.style_manager();
    // style_manager.set_color_scheme(libadwaita::ColorScheme::ForceDark);

    let keypair = Keypair::generate_ed25519();
    let local_peer_id = keypair.public().to_peer_id();

    // Initialize the repo and start a daemon
    let ipfs: Ipfs = UninitializedIpfs::new()
        .with_default()
        .set_keypair(&keypair)
        .add_listening_addr("/ip4/0.0.0.0/tcp/0".parse().unwrap())
        .with_mdns()
        .with_relay(true)
        .with_relay_server(Default::default())
        .with_upnp()
        .with_rendezvous_server()
        .listen_as_external_addr()
        .fd_limit(ipfs::FDLimit::Max)
        .start()
        .await
        .unwrap();

    ipfs.default_bootstrap().await.unwrap();
    ipfs.bootstrap().await.unwrap();

    application.connect_activate(clone!(
        #[weak]
        application,
        #[strong]
        ipfs,
        move |_| {
            crate::widgets::window::Window::new(&application, None, &ipfs);
        }
    ));
    application.run();

    // Used to wait until the process is terminated instead of creating a loop
    tokio::signal::ctrl_c().await.unwrap();
    ipfs.exit_daemon().await;
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
fn new_window_four(
    application: &libadwaita::Application,
    initial_url_option: Option<&str>,
    ipfs: &Ipfs,
) -> libadwaita::TabView {
    // Options
    let verbose = true;
    let is_private = true;
    // let mut native = true;
    let initial_url = initial_url_option.unwrap_or("about:blank");

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
        .width_request(8)
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
    back_button.add_css_class("linked");

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
    forward_button.add_css_class("linked");

    // All navigation buttons
    //let navigation_buttons_builder = gtk::BoxBuilder::new();
    let navigation_buttons = gtk::Box::builder().homogeneous(true).build();
    navigation_buttons.append(&back_button);
    navigation_buttons.append(&forward_button);
    navigation_buttons.add_css_class("linked");

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

    // Overview button
    let overview_button = gtk::Button::builder()
        .can_focus(true)
        .receives_default(true)
        .halign(gtk::Align::Start)
        .margin_start(4)
        .margin_bottom(4)
        .icon_name("view-grid-symbolic")
        .build();

    // Downloads button
    //let downloads_button_builder = gtk::ButtonBuilder::new();
    let downloads_button = gtk::Button::builder()
        .can_focus(true)
        .receives_default(true)
        .halign(gtk::Align::Start)
        .margin_start(4)
        .margin_bottom(4)
        .icon_name("folder-download-symbolic")
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

    // IPFS menu button
    let ipfs_button = gtk::ToggleButton::builder()
        .can_focus(true)
        .receives_default(true)
        .halign(gtk::Align::Start)
        .margin_start(4)
        .margin_bottom(4)
        .icon_name("emblem-shared")
        .build();

    // Onion routing button
    let tor_button = gtk::ToggleButton::builder()
        .can_focus(true)
        .receives_default(true)
        .halign(gtk::Align::Start)
        .hexpand(false)
        .vexpand(false)
        .overflow(gtk::Overflow::Hidden)
        .margin_start(4)
        .margin_bottom(4)
        .icon_name("security-medium")
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
    right_header_buttons.append(&overview_button);
    right_header_buttons.append(&downloads_button);
    right_header_buttons.append(&find_button);
    right_header_buttons.append(&ipfs_button);
    right_header_buttons.append(&tor_button);
    right_header_buttons.append(&menu_button);

    // HeaderBar
    //let headerbar_builder = gtk::HeaderBarBuilder::new();
    let headerbar = libadwaita::HeaderBar::builder()
        .can_focus(true)
        //.show_title_buttons(true)
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
    zoomout_button.add_css_class("linked");

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
    zoomin_button.add_css_class("linked");

    // Both zoom buttons
    //let zoom_buttons_builder = gtk::BoxBuilder::new();
    let zoom_buttons = gtk::Box::builder().homogeneous(true).build();
    zoom_buttons.append(&zoomout_button);
    zoom_buttons.append(&zoomin_button);
    zoom_buttons.add_css_class("linked");

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
        .icon_name("video-display-symbolic")
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
        .hexpand(true)
        .build();
    menu_box.append(&zoom_buttons);
    menu_box.append(&zoomreset_button);
    menu_box.append(&fullscreen_button);
    menu_box.append(&screenshot_button);
    menu_box.append(&new_window_button);
    menu_box.append(&history_button);
    menu_box.append(&settings_button);
    menu_box.append(&about_button);
    menu_box.add_css_class("toolbar");

    //let menu_builder = gtk::PopoverBuilder::new();
    let menu = gtk::Popover::builder().child(&menu_box).build();
    menu.set_parent(&menu_button);
    // End of menu popover

    // Downloads popover
    let downloads_box = gtk::Box::builder()
        .margin_start(4)
        .margin_end(4)
        .margin_top(4)
        .margin_bottom(4)
        .spacing(8)
        .build();

    let downloads = gtk::Popover::builder().child(&downloads_box).build();
    downloads.set_parent(&downloads_button);
    // End of downloads popover

    // Tabs
    //let tab_view_builder = libadwaita::TabViewBuilder::new();
    let tab_view = libadwaita::TabView::builder().vexpand(true).build();
    tab_view.set_visible(true);
    // Indicator logic
    tab_view.connect_indicator_activated(clone!(@weak tab_view => move |_, current_page| {
        let current_view = get_view_from_page(current_page);
        if !current_view.is_playing_audio() && !current_view.is_muted() {
            tab_view.set_page_pinned(&current_page, !current_page.is_pinned());
        } else {
            current_view.set_is_muted(!current_view.is_muted());
        }
    }));

    //let tabs_builder = libadwaita::TabBarBuilder::new();
    let tabs = libadwaita::TabBar::builder()
        .autohide(true)
        .expand_tabs(true)
        .view(&tab_view)
        .build();

    if tab_view.n_pages() == 0 {
        create_initial_tab(
            &tabs,
            initial_url.to_owned(),
            verbose,
            is_private,
            &ipfs_button,
            &headerbar,
            ipfs,
        )
    }
    // End of Tabs

    // Window
    //let main_box_builder = gtk::BoxBuilder::new();
    let main_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .vexpand(true)
        .build();
    main_box.append(&headerbar);
    main_box.append(&tabs);
    main_box.append(&tab_view);

    let overview = TabOverview::builder()
        .enable_new_tab(true)
        .enable_search(true)
        .view(&tab_view)
        .child(&main_box)
        .build();

    overview.connect_create_tab(
        clone!(@weak tabs, @weak ipfs_button, @weak headerbar, @strong ipfs => @default-panic, move |_| {
            new_tab_page(&tabs, verbose, is_private, &ipfs_button, &headerbar, &ipfs).1
        }),
    );

    overview_button.connect_clicked(clone!(@weak overview => move |_| {
        overview.set_open(!overview.is_open());
    }));

    //let window_builder = gtk::ApplicationWindowBuilder::new();
    let window = libadwaita::ApplicationWindow::builder()
        .application(application)
        .can_focus(true)
        .title("Oku")
        .icon_name("com.github.dirout.oku")
        //.titlebar(&headerbar)
        .content(&overview)
        .build();
    //window.set_titlebar(Some(&headerbar));
    //window.set_child(Some(&main_box));
    // End of Window

    // Signals
    // Add Tab button clicked
    add_tab.connect_clicked(
        clone!(@weak tabs, @weak ipfs_button, @weak headerbar, @strong ipfs => move |_| {
            new_tab_page(&tabs, verbose, is_private, &ipfs_button, &headerbar, &ipfs);
        }),
    );

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

    // Downloads button clicked
    downloads_button.connect_clicked(clone!(@weak downloads => move |_| {
        downloads.popup();
    }));

    // // IPFS button clicked
    // ipfs_button.connect_toggled(clone!(@weak ipfs_button, @weak tabs => move |_| {
    //     let web_view = get_view(&tabs);
    //     let web_context = web_view.context().unwrap();
    //     let mut native;
    //     if (ipfs_button.is_active()) {
    //         native = false;
    //     } else {
    //         native = true;
    //     }
    //     match native {
    //         true => {
    //             web_context.register_uri_scheme("ipfs", move |request| {
    //                 handle_ipfs_request_natively(request)
    //             });
    //         }
    //         false => {
    //             web_context.register_uri_scheme("ipfs", move |request| {
    //                 handle_ipfs_request_using_api(request)
    //             });
    //         }
    //     };
    // }));

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
        clone!(@weak tabs, @weak nav_entry, @weak window => move |_| {
            let web_view = get_view(&tabs);
            if !window.is_fullscreen() {
                window.set_fullscreened(true);
                tabs.hide();
                tabs.set_opacity(0.0);
                web_view.evaluate_javascript("document.documentElement.requestFullscreen();", None, None, gio::Cancellable::NONE, move |_| {
                })
            } else {
                window.set_fullscreened(false);
                tabs.show();
                tabs.set_opacity(100.0);
                web_view.evaluate_javascript("document.exitFullscreen();", None, None, gio::Cancellable::NONE, move |_| {
                })
            }
        }),
    );

    // Screenshot button clicked
    screenshot_button.connect_clicked(
        clone!(@weak tabs, @weak nav_entry => move |_| {
            let web_view = get_view(&tabs);
            web_view.snapshot(webkit2gtk::SnapshotRegion::FullDocument, webkit2gtk::SnapshotOptions::all(), gio::Cancellable::NONE, move |snapshot| {
                snapshot.unwrap().save_to_png(format!("{}/{}.png", PICTURES_DIR.to_owned(), Utc::now())).unwrap();
            });
        }),
    );

    // New Window button clicked
    new_window_button.connect_clicked(
        clone!(@weak tabs, @weak nav_entry, @weak window, @strong ipfs => move |_| {
            new_window_four(&window.application().unwrap().downcast().unwrap(), None, &ipfs);
        }),
    );

    // About button clicked
    about_button.connect_clicked(
        clone!(@weak tabs, @weak nav_entry, @weak window => move |_| {
            new_about_dialog(&window.application().unwrap())
        }),
    );

    // Tab dragged off to create new browser window
    tab_view.connect_create_window(clone!(@strong ipfs => move |tabs| {
        create_window_from_drag(tabs, &ipfs)
    }));
    // End of signals

    // let settings = window.settings();
    // settings.set_gtk_application_prefer_dark_theme(true);
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
    ipfs: &Ipfs,
) -> std::option::Option<libadwaita::TabView> {
    let window: gtk::ApplicationWindow = tab_view
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .downcast()
        .unwrap();
    let application = window.application().unwrap().downcast().unwrap();
    let new_window = new_window_four(&application, None, &ipfs);
    Some(new_window)
}
