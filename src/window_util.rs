use arti_client::TorClient;
use cid::Cid;
use futures::pin_mut;
use glib::object::{Cast, IsA};
use glib_macros::clone;
use gtk::{
    prelude::WidgetExt,
    prelude::{EditableExt, EntryExt},
};
use http_body_util::{BodyExt, Empty};
use hyper::StatusCode;
use hyper_util::rt::TokioIo;
use ipfs::Ipfs;
use std::convert::TryFrom;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_stream::StreamExt;
use tor_rtcompat::PreferredRuntime;
use webkit2gtk::{
    functions::uri_for_display, prelude::WebViewExt, soup::MessageHeaders, URISchemeRequest,
    URISchemeResponse,
};

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
pub fn connect(nav_entry: &gtk::Entry, web_view: &webkit2gtk::WebView) {
    let mut nav_text = nav_entry.text().to_string();
    let mut parsed_url = url::Url::parse(&nav_text);
    match parsed_url {
        // When URL is completely OK
        Ok(_) => {
            web_view.load_uri(&nav_text);
        }
        // When URL is missing a scheme
        Err(url::ParseError::RelativeUrlWithoutBase) => {
            parsed_url = url::Url::parse(&format!("http://{}", nav_text)); // Try with HTTP first
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
                            parsed_url = url::Url::parse(&format!("ipfs://{}", nav_text));
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
pub fn update_nav_bar(nav_entry: &gtk::Entry, web_view: &webkit2gtk::WebView) {
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

/// Update the load progress indicator under the navigation bar
///
/// # Arguments
///
/// * `nav_entry` - The navigation bar of the browser
///
/// * `web_view` - The WebKit instance for the current tab
pub fn update_load_progress(nav_entry: &gtk::Entry, web_view: &webkit2gtk::WebView) {
    let load_progress = web_view.estimated_load_progress();
    if load_progress as i64 == 1 {
        nav_entry.set_progress_fraction(0.00)
    } else {
        nav_entry.set_progress_fraction(load_progress)
    }
}

pub async fn make_tor_request(
    request_http_method: &str,
    request_pair: (String, String),
    tor_stream: impl AsyncRead + AsyncWrite + Unpin + Send + 'static,
) -> (Vec<u8>, StatusCode, MessageHeaders, String) {
    let mut file_vec: Vec<u8> = vec![];
    let (mut request_sender, connection) =
        hyper::client::conn::http1::handshake(TokioIo::new(tor_stream))
            .await
            .unwrap();
    let connection_handle = tokio::spawn(async move {
        connection.await.unwrap();
    });
    let mut resp = request_sender
        .send_request(
            hyper::Request::builder()
                .header("Host", request_pair.clone().0)
                .method(request_http_method)
                .uri(request_pair.1)
                .body(Empty::<hyper::body::Bytes>::new())
                .unwrap(),
        )
        .await
        .unwrap();
    let message_headers = MessageHeaders::new(webkit2gtk::soup::MessageHeadersType::Response);
    for (header_name, header_value) in resp.headers() {
        message_headers.append(header_name.as_str(), header_value.to_str().unwrap())
    }
    while let Some(frame) = resp.body_mut().frame().await {
        let bytes = frame.unwrap().into_data().unwrap();
        file_vec.extend(bytes);
    }
    let content_type = match resp.headers().get(hyper::header::CONTENT_TYPE) {
        Some(h) => {
            if let Ok(h_str) = h.to_str() {
                h_str
            } else {
                tree_magic_mini::from_u8(&file_vec)
            }
        }
        None => tree_magic_mini::from_u8(&file_vec),
    }
    .to_string();
    message_headers.set_content_type(Some(content_type.clone()), None);
    message_headers.set_content_length(file_vec.len().try_into().unwrap_or_default());
    connection_handle.abort();
    (file_vec, resp.status(), message_headers, content_type)
}

pub fn tor_scheme_handler<'a>(
    tor_client: &TorClient<PreferredRuntime>,
    request: &'a URISchemeRequest,
) -> &'a URISchemeRequest {
    let ctx = glib::MainContext::default();
    let request_url = url::Url::parse(
        &request
            .uri()
            .unwrap()
            .to_string()
            .replacen("tor://", "http://", 1),
    )
    .unwrap();
    let request_port = request_url.port_or_known_default().unwrap_or(80);
    let request_scheme = if request_port == 443 {
        String::from("https")
    } else {
        String::from("http")
    };
    let request_pair = (
        request_url.host_str().unwrap_or_default().to_string(),
        format!(
            "{}://{}{}",
            request_scheme,
            request_url.host_str().unwrap_or_default(),
            request_url.path()
        ),
    );
    ctx.spawn_local_with_priority(
        glib::source::Priority::HIGH,
        clone!(
            #[weak]
            request,
            #[strong]
            tor_client,
            async move {
                match tor_client
                    .connect((request_pair.clone().0, request_port))
                    .await
                {
                    Ok(tor_stream) => {
                        if request_port == 443 {
                            let cx = tokio_native_tls::native_tls::TlsConnector::builder()
                                .build()
                                .unwrap();
                            let cx = tokio_native_tls::TlsConnector::from(cx);
                            match cx.connect(&request_pair.clone().0, tor_stream).await {
                                Ok(tor_stream) => {
                                    let (file_vec, status, message_headers, content_type) =
                                        make_tor_request(
                                            request.http_method().unwrap_or("GET".into()).as_str(),
                                            request_pair,
                                            tor_stream,
                                        )
                                        .await;
                                    let byte_size = file_vec.len();
                                    let input = gio::MemoryInputStream::from_bytes(
                                        &glib::Bytes::from_owned(file_vec),
                                    );
                                    let response = URISchemeResponse::new(
                                        &input,
                                        byte_size.try_into().unwrap(),
                                    );
                                    response.set_status(
                                        status.as_u16().into(),
                                        status.canonical_reason(),
                                    );
                                    response.set_http_headers(message_headers);
                                    response.set_content_type(&content_type);
                                    request.finish_with_response(&response);
                                    return;
                                }
                                Err(e) => {
                                    request.finish_error(&mut glib::error::Error::new(
                                        webkit2gtk::NetworkError::Transport,
                                        &e.to_string(),
                                    ));
                                    return;
                                }
                            }
                        } else {
                            let (file_vec, status, message_headers, content_type) =
                                make_tor_request(
                                    request.http_method().unwrap_or("GET".into()).as_str(),
                                    request_pair,
                                    tor_stream,
                                )
                                .await;
                            let byte_size = file_vec.len();
                            let input = gio::MemoryInputStream::from_bytes(
                                &glib::Bytes::from_owned(file_vec),
                            );
                            let response =
                                URISchemeResponse::new(&input, byte_size.try_into().unwrap());
                            response.set_status(status.as_u16().into(), status.canonical_reason());
                            response.set_http_headers(message_headers);
                            response.set_content_type(&content_type);
                            request.finish_with_response(&response);
                            return;
                        }
                    }
                    Err(e) => {
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
            request.finish_error(&mut glib::error::Error::new(
                webkit2gtk::NetworkError::Failed,
                &e.to_string(),
            ));
            return request;
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
            request.finish_error(&mut glib::error::Error::new(
                webkit2gtk::NetworkError::Failed,
                &e.to_string(),
            ));
            request
        }
    }
}
