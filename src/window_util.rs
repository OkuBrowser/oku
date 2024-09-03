use cid::Cid;
use futures::pin_mut;
use glib::object::{Cast, IsA};
use glib_macros::clone;
use gtk::{prelude::EditableExt, prelude::WidgetExt};
use ipfs::Ipfs;
use oku_fs::fs::OkuFs;
use std::{convert::TryFrom, path::PathBuf};
use tokio_stream::StreamExt;
use tracing::error;
use webkit2gtk::{functions::uri_for_display, prelude::WebViewExt, URISchemeRequest};

use crate::HISTORY_MANAGER;

/// Perform the initial connection at startup when passed a URL as a launch argument
///
/// * `initial_url` - The URL passed as a launch argument
///
/// * `web_view` - The WebKit instance for the current tab
pub fn initial_connect(mut initial_url: String, web_view: &webkit2gtk::WebView) {
    let mut parsed_url = url::Url::parse(&initial_url);
    match parsed_url {
        // When URL is completely OK
        Ok(_) => {
            web_view.load_uri(&initial_url);
        }
        // When URL is missing a scheme
        Err(url::ParseError::RelativeUrlWithoutBase) => {
            parsed_url = url::Url::parse(&format!("http://{}", initial_url)); // Try with HTTP first
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
                            parsed_url = url::Url::parse(&format!("ipfs://{}", initial_url));
                            let mut unwrapped_url = parsed_url.unwrap();
                            let cid1_string = &cid1
                                .to_string_of_base(cid::multibase::Base::Base32Lower)
                                .unwrap();
                            unwrapped_url.set_host(Some(cid1_string)).unwrap();
                            initial_url = unwrapped_url.as_str().to_owned();
                            web_view.load_uri(&initial_url);
                        }
                        // It doesn't work as IPFS
                        Err(e) => {
                            error!("{}", e);
                            initial_url = parsed_url.unwrap().as_str().to_owned();
                            web_view.load_uri(&initial_url);
                        }
                    }
                }
                // Still not valid, even with HTTP
                Err(e) => {
                    error!("{}", e);
                    web_view.load_plain_text(&format!("{:#?}", e));
                }
            }
        }
        // URL is malformed beyond missing a scheme
        Err(e) => {
            error!("{}", e);
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
pub fn connect(nav_entry: &gtk::SearchEntry, web_view: &webkit2gtk::WebView) {
    let nav_text = nav_entry.text().to_string();
    let mut parsed_url = url::Url::parse(&nav_text);
    match parsed_url {
        // When URL is completely OK
        Ok(_) => {
            let history_manager = HISTORY_MANAGER.lock().unwrap();
            let current_session = history_manager.get_current_session();
            current_session.add_navigation(
                web_view.uri().unwrap_or("about:blank".into()).to_string(),
                nav_text.to_string(),
            );
            current_session.save();
            drop(history_manager);
            nav_entry.set_text(&nav_text);
            web_view.load_uri(&nav_text);
        }
        // When URL is missing a scheme
        Err(url::ParseError::RelativeUrlWithoutBase) => {
            parsed_url = url::Url::parse(&format!("http://{}", nav_text)); // Try with HTTP first
            match parsed_url {
                // If it's now valid with HTTP
                Ok(nav_url) => {
                    nav_entry.set_text(nav_url.as_str());
                    connect(&nav_entry, &web_view);
                }
                // Still not valid, even with HTTP
                Err(e) => {
                    error!("{}", e);
                }
            }
        }
        // URL is malformed beyond missing a scheme
        Err(e) => {
            error!("{}", e);
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
pub fn update_nav_bar(nav_entry: &gtk::SearchEntry, web_view: &webkit2gtk::WebView) {
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
    nav_entry.set_text(&uri_for_display(&url).unwrap_or_default());
}

/// Provide the default configuration for Oku's WebView
pub fn new_webkit_settings() -> webkit2gtk::Settings {
    let settings = webkit2gtk::Settings::new();
    settings.set_javascript_can_open_windows_automatically(true);
    settings.set_allow_modal_dialogs(true);
    settings.set_allow_file_access_from_file_urls(true);
    settings.set_allow_top_navigation_to_data_urls(true);
    settings.set_allow_universal_access_from_file_urls(true);
    settings.set_auto_load_images(true);
    settings.set_enable_back_forward_navigation_gestures(true);
    settings.set_enable_caret_browsing(false);
    settings.set_enable_developer_extras(true);
    settings.set_enable_dns_prefetching(true);
    settings.set_enable_encrypted_media(true);
    settings.set_enable_fullscreen(true);
    settings.set_enable_html5_database(true);
    settings.set_enable_html5_local_storage(true);
    settings.set_enable_hyperlink_auditing(true);
    settings.set_enable_javascript(true);
    settings.set_enable_javascript_markup(true);
    settings.set_enable_media(true);
    settings.set_enable_media_capabilities(true);
    settings.set_enable_media_stream(true);
    settings.set_enable_mediasource(true);
    settings.set_enable_mock_capture_devices(true);
    settings.set_enable_page_cache(true);
    settings.set_enable_resizable_text_areas(true);
    settings.set_enable_site_specific_quirks(true);
    settings.set_enable_smooth_scrolling(true);
    settings.set_enable_spatial_navigation(true);
    settings.set_enable_tabs_to_links(true);
    settings.set_enable_webaudio(true);
    settings.set_enable_webgl(true);
    settings.set_enable_write_console_messages_to_stdout(true);
    settings.set_hardware_acceleration_policy(webkit2gtk::HardwareAccelerationPolicy::Never);
    settings.set_javascript_can_access_clipboard(true);
    settings.set_media_playback_allows_inline(true);
    settings.set_media_playback_requires_user_gesture(false);
    settings.set_print_backgrounds(true);
    settings.set_zoom_text_only(false);
    settings
}

/// Get the WebKit instance for the current tab
///
/// # Arguments
///
/// * `page` - The TabPage containing the the WebKit instance
pub fn get_view_from_page(page: &libadwaita::TabPage) -> webkit2gtk::WebView {
    page.child().downcast().unwrap()
}

pub fn get_window_from_widget(widget: &impl IsA<gtk::Widget>) -> crate::widgets::window::Window {
    widget
        .ancestor(glib::Type::from_name("OkuWindow").unwrap())
        .unwrap()
        .downcast()
        .unwrap()
}

/// Update a tab's icon
///
/// # Arguments
///
/// * `tab_view` - The tabs of the current browser window
///
/// * `web_view` - The WebKit instance for the tab
pub fn update_favicon(tab_view: libadwaita::TabView, web_view: &webkit2gtk::WebView) {
    let relevant_page = tab_view.page(web_view);
    let web_favicon = &web_view.favicon();
    match &web_favicon {
        Some(favicon_texture) => {
            relevant_page.set_icon(Some(favicon_texture));
        }
        None => {
            relevant_page.set_icon(Some(&gio::ThemedIcon::new("content-loading-symbolic")));
        }
    }
}

pub fn get_title(web_view: &webkit2gtk::WebView) -> String {
    if web_view.uri().unwrap_or_default() == "about:blank" {
        return String::from("Oku");
    }
    match web_view.title() {
        Some(page_title) => {
            if page_title.as_str() == "" {
                String::from("Untitled")
            } else {
                String::from(page_title)
            }
        }
        None => String::from("Untitled"),
    }
}

/// Update a tab's title
///
/// # Arguments
///
/// * `tab_view` - The tabs of the current browser window
///
/// * `web_view` - The WebKit instance for the tab
pub fn update_title(tab_view: libadwaita::TabView, web_view: &webkit2gtk::WebView) {
    let relevant_page = tab_view.page(web_view);
    relevant_page.set_title(&get_title(web_view));
}

pub fn ipfs_scheme_handler<'a>(ipfs: &Ipfs, request: &'a URISchemeRequest) -> &'a URISchemeRequest {
    let ctx = glib::MainContext::default();
    let request_url = request.uri().unwrap().to_string();
    let decoded_url = urlencoding::decode(&request_url).unwrap();
    match decoded_url
        .replacen("ipfs://", "", 1)
        .parse::<ipfs::IpfsPath>()
    {
        Ok(ipfs_path) => {
            let ipfs_stream = ipfs.cat_unixfs(ipfs_path);
            ctx.spawn_local_with_priority(
                glib::source::Priority::HIGH,
                clone!(
                    #[weak]
                    request,
                    async move {
                        let mut file_vec: Vec<u8> = vec![];
                        pin_mut!(ipfs_stream);
                        while let Some(result) = ipfs_stream.next().await {
                            match result {
                                Ok(bytes) => {
                                    file_vec.extend(bytes);
                                }
                                Err(e) => {
                                    error!("{}", e);
                                    if file_vec.len() == 0 {
                                        request.finish_error(&mut glib::error::Error::new(
                                            webkit2gtk::NetworkError::FileDoesNotExist,
                                            &e.to_string(),
                                        ));
                                        return;
                                    } else {
                                        request.finish_error(&mut glib::error::Error::new(
                                            webkit2gtk::NetworkError::Transport,
                                            &e.to_string(),
                                        ));
                                        return;
                                    }
                                }
                            }
                        }
                        let byte_size = file_vec.len();
                        let content_type = tree_magic_mini::from_u8(&file_vec);
                        let mem_stream =
                            gio::MemoryInputStream::from_bytes(&glib::Bytes::from_owned(file_vec));
                        request.finish(
                            &mem_stream,
                            byte_size.try_into().unwrap(),
                            Some(content_type),
                        );
                        return;
                    }
                ),
            );
            return request;
        }
        Err(e) => {
            error!("{}", e);
            request.finish_error(&mut glib::error::Error::new(
                webkit2gtk::NetworkError::Failed,
                &e.to_string(),
            ));
            return request;
        }
    }
}

pub fn node_scheme_handler<'a>(
    node: &OkuFs,
    request: &'a URISchemeRequest,
) -> &'a URISchemeRequest {
    let ctx = glib::MainContext::default();
    let request_url = request.uri().unwrap().to_string();
    let decoded_url = urlencoding::decode(&request_url)
        .unwrap()
        .replacen("hive://", "", 1);
    match oku_fs::fuse::parse_fuse_path(&PathBuf::from(decoded_url.clone())) {
        Ok(parsed_path) => match parsed_path {
            Some((namespace_id, replica_path)) => {
                ctx.spawn_local_with_priority(
                    glib::source::Priority::HIGH,
                    clone!(
                        #[weak]
                        request,
                        #[strong]
                        node,
                        async move {
                            match node
                                .read_local_or_external_file(
                                    namespace_id,
                                    replica_path,
                                    false,
                                    false,
                                )
                                .await
                            {
                                Ok(file_bytes) => {
                                    let file_vec = file_bytes.to_vec();
                                    let byte_size = file_vec.len();
                                    let content_type = tree_magic_mini::from_u8(&file_vec);
                                    let mem_stream = gio::MemoryInputStream::from_bytes(
                                        &glib::Bytes::from_owned(file_vec),
                                    );
                                    request.finish(
                                        &mem_stream,
                                        byte_size.try_into().unwrap(),
                                        Some(content_type),
                                    );
                                    return;
                                }
                                Err(e) => {
                                    error!("{}", e);
                                    request.finish_error(&mut glib::error::Error::new(
                                        webkit2gtk::NetworkError::Failed,
                                        &e.to_string(),
                                    ));
                                    return;
                                }
                            }
                        }
                    ),
                );
                request
            }
            None => {
                request.finish_error(&mut glib::error::Error::new(
                    webkit2gtk::NetworkError::Failed,
                    &format!("{} does not contain a replica ID", decoded_url),
                ));
                request
            }
        },
        Err(e) => {
            error!("{}", e);
            request.finish_error(&mut glib::error::Error::new(
                webkit2gtk::NetworkError::Failed,
                &e.to_string(),
            ));
            request
        }
    }
}

pub fn ipns_scheme_handler<'a>(ipfs: &Ipfs, request: &'a URISchemeRequest) -> &'a URISchemeRequest {
    let ctx = glib::MainContext::default();
    let request_url = request.uri().unwrap().to_string();
    let decoded_url = urlencoding::decode(&request_url).unwrap();
    match format!("/ipns/{}", decoded_url.replacen("ipns://", "", 1)).parse::<ipfs::IpfsPath>() {
        Ok(ipns_path) => {
            ctx.spawn_local_with_priority(
                glib::source::Priority::HIGH,
                clone!(
                    #[weak]
                    request,
                    #[strong]
                    ipfs,
                    async move {
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
                                    error!("{}", e);
                                    if file_vec.len() == 0 {
                                        request.finish_error(&mut glib::error::Error::new(
                                            webkit2gtk::NetworkError::FileDoesNotExist,
                                            &e.to_string(),
                                        ));
                                        return;
                                    } else {
                                        request.finish_error(&mut glib::error::Error::new(
                                            webkit2gtk::NetworkError::Transport,
                                            &e.to_string(),
                                        ));
                                        return;
                                    }
                                }
                            }
                        }
                        let byte_size = file_vec.len();
                        let content_type = tree_magic_mini::from_u8(&file_vec);
                        let mem_stream =
                            gio::MemoryInputStream::from_bytes(&glib::Bytes::from_owned(file_vec));
                        request.finish(
                            &mem_stream,
                            byte_size.try_into().unwrap(),
                            Some(content_type),
                        );
                        return;
                    }
                ),
            );
            request
        }
        Err(e) => {
            error!("{}", e);
            request.finish_error(&mut glib::error::Error::new(
                webkit2gtk::NetworkError::Failed,
                &e.to_string(),
            ));
            request
        }
    }
}
