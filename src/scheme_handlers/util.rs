use super::{
    hive::node_scheme,
    ipfs::{ipfs_scheme, ipns_scheme},
    oku::oku_scheme,
    view_source::view_source_scheme,
};
use bytes::Bytes;
use ipfs::Ipfs;
use log::error;
use tokio::runtime::Handle;
use webkit2gtk::URISchemeRequest;

pub enum RequestScheme {
    Oku,
    Hive,
    Ipfs,
    Ipns,
    ViewSource,
}

#[derive(Clone)]
pub struct SchemeRequest(pub URISchemeRequest);
unsafe impl Send for SchemeRequest {}
unsafe impl Sync for SchemeRequest {}
impl SchemeRequest {
    pub fn uri(&self) -> Option<String> {
        self.0.uri().map(|uri| uri.to_string())
    }
    pub fn http_method(&self) -> Option<String> {
        self.0
            .http_method()
            .map(|http_method| http_method.to_string())
    }
    pub fn finish(&self, bytes_result: miette::Result<impl Into<Bytes>>) {
        match bytes_result {
            Ok(bytes) => {
                let byte_vec = bytes.into().to_vec();
                let bytes_size = byte_vec.len();
                let content_type = tree_magic_mini::from_u8(&byte_vec);
                let mem_stream =
                    gio::MemoryInputStream::from_bytes(&glib::Bytes::from_owned(byte_vec));
                self.0.finish(
                    &mem_stream,
                    bytes_size.try_into().unwrap_or(-1),
                    Some(content_type),
                );
            }
            Err(e) => {
                error!("{}", e);
                self.0.finish_error(&mut glib::error::Error::new(
                    webkit2gtk::NetworkError::Failed,
                    &e.to_string(),
                ));
            }
        }
    }
}

pub fn handle_request(ipfs: Ipfs, request: SchemeRequest, request_scheme: RequestScheme) {
    let handle = Handle::current();
    std::thread::spawn(move || {
        handle.block_on(handle_request_tokio(&ipfs, request.clone(), request_scheme));
    });
}

pub async fn handle_request_tokio(
    ipfs: &Ipfs,
    request: SchemeRequest,
    request_scheme: RequestScheme,
) {
    match request_scheme {
        RequestScheme::Hive => node_scheme(request).await,
        RequestScheme::Oku => oku_scheme(request).await,
        RequestScheme::Ipfs => ipfs_scheme(ipfs, request).await,
        RequestScheme::Ipns => ipns_scheme(ipfs, request).await,
        RequestScheme::ViewSource => view_source_scheme(request).await,
    }
}
