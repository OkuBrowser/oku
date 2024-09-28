use super::util::SchemeRequest;
use bytes::Bytes;
use webkit2gtk::functions::uri_for_display;

pub fn oku_scheme(request: SchemeRequest) {
    let bytes_result = oku_scheme_handler(request.clone());
    request.finish(bytes_result);
}

pub fn oku_scheme_handler(request: SchemeRequest) -> miette::Result<impl Into<Bytes>> {
    let request_uri = request.uri().ok_or(miette::miette!(
        "Could read request URI ({:?}) … ",
        request.uri()
    ))?;
    let decoded_url = uri_for_display(&request_uri)
        .ok_or(miette::miette!(
            "Could display request URI safely ({}) … ",
            request_uri
        ))?
        .replacen("oku:", "", 1);
    match decoded_url.as_str() {
        "home" => Ok(include_bytes!("../browser_pages/output/home.html").to_vec()),
        _ => Err(miette::miette!("Unknown browser page ({}) … ", decoded_url)),
    }
}
