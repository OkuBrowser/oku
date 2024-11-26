use glib::object::{Cast, IsA};
use gtk::{prelude::EditableExt, prelude::WidgetExt};
use log::error;
use oku_fs::iroh::docs::{DocTicket, NamespaceId};
use std::{path::PathBuf, str::FromStr};
use webkit2gtk::{functions::uri_for_display, prelude::WebViewExt};

/// Connect to a page using the current tab
///
/// # Arguments
///
/// * `nav_entry` - The navigation bar of the browser
///
/// * `web_view` - The WebKit instance for the current tab
pub fn connect(nav_entry: &impl IsA<gtk::Editable>, web_view: &webkit2gtk::WebView) {
    let nav_text = nav_entry.text().to_string();
    let mut parsed_url = url::Url::parse(&nav_text);
    match parsed_url {
        // When URL is completely OK
        Ok(_) => {
            nav_entry.set_text(&nav_text);
            web_view.load_uri(&nav_text);
        }
        // When URL is missing a scheme
        Err(url::ParseError::RelativeUrlWithoutBase) => {
            let nav_text_with_base = if is_hive_uri(&nav_text) {
                format!("hive://{}", nav_text)
            } else if is_ipfs_uri(&nav_text) {
                format!("ipfs://{}", nav_text)
            } else {
                nav_text.clone()
            };
            parsed_url = url::Url::parse(&nav_text_with_base); // Try with protocol first
            match parsed_url {
                // If it's now valid with protocol
                Ok(nav_url) => {
                    nav_entry.set_text(nav_url.as_str());
                    web_view.load_uri(nav_url.as_str());
                }
                // Still not valid, even with protocol
                Err(e) => {
                    error!("{}", e);
                    web_view.load_uri(&format!("oku:search/{}", nav_text));
                }
            }
        }
        // URL is malformed beyond missing a scheme
        Err(e) => {
            error!("{}", e);
            web_view.load_uri(&format!("oku:search/{}", nav_text));
        }
    }
}

pub fn is_hive_uri(nav_text: &str) -> bool {
    let path = PathBuf::from(nav_text);
    let components = &mut path.components();
    if let Some(first_component) = components.next() {
        let first_component_string = first_component.as_os_str().to_str().unwrap_or_default();
        DocTicket::from_str(first_component_string).is_ok()
            || NamespaceId::from_str(first_component_string).is_ok()
    } else {
        false
    }
}

pub fn is_ipfs_uri(nav_text: &str) -> bool {
    nav_text.parse::<ipfs::IpfsPath>().is_ok()
}

/// Update the contents of the navigation bar
///
/// # Arguments
///
/// * `nav_entry` - The navigation bar of the browser
///
/// * `web_view` - The WebKit instance for the current tab
pub fn update_nav_bar(nav_entry: &impl IsA<gtk::Editable>, web_view: &webkit2gtk::WebView) {
    let mut url = web_view.uri().unwrap().to_string();
    if url.starts_with("oku:home") || url.starts_with("about:") || url.starts_with("view-source:") {
        url = "".to_string();
    }
    if let Some(search_stripped) = url.strip_prefix("oku:search/") {
        url = search_stripped.to_owned();
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

pub fn get_view_stack_page_by_name(
    name: String,
    view_stack: &libadwaita::ViewStack,
) -> Option<libadwaita::ViewStackPage> {
    view_stack.child_by_name(&name).map(|x| view_stack.page(&x))
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
            relevant_page.set_icon(Some(&gio::ThemedIcon::new("globe-symbolic")));
        }
    }
}

pub fn get_title(web_view: &webkit2gtk::WebView) -> String {
    let uri = web_view.uri().unwrap_or("about:blank".into());
    match web_view.title() {
        Some(page_title) => {
            if page_title.trim().is_empty() {
                if uri == "about:blank" || uri.starts_with("oku:") {
                    String::from("Oku")
                } else {
                    String::from("Untitled")
                }
            } else {
                String::from(page_title)
            }
        }
        None => {
            if uri == "about:blank" || uri.starts_with("oku:") {
                String::from("Oku")
            } else {
                String::from("Untitled")
            }
        }
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
    let title = get_title(web_view);
    relevant_page.set_title(&title);
}
