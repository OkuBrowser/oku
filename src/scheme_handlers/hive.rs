use super::{hive_path::HivePath, util::SchemeRequest};
use crate::NODE;
use bytes::Bytes;
use webkit2gtk::functions::uri_for_display;

pub async fn node_scheme(request: SchemeRequest) {
    let bytes_result = node_scheme_handler(request.clone()).await;
    request.finish(bytes_result);
}

pub async fn node_scheme_handler(request: SchemeRequest) -> miette::Result<impl Into<Bytes>> {
    let request_uri = request.uri().ok_or(miette::miette!(
        "Could read request URI ({:?}) … ",
        request.uri()
    ))?;
    let decoded_url = uri_for_display(&request_uri)
        .ok_or(miette::miette!(
            "Could display request URI safely ({}) … ",
            request_uri
        ))?
        .replacen("hive://", "", 1);
    let url_path = HivePath::parse(decoded_url)?;
    let node = NODE
        .get()
        .ok_or(miette::miette!("Oku node has not yet started … "))?;
    match url_path {
        HivePath::ByTicket(ticket, replica_path) => node
            .fetch_file_with_ticket(&ticket, replica_path)
            .await
            .map_err(|e| miette::miette!("{}", e)),
        HivePath::ById(namespace_id, replica_path) => node
            .fetch_file(namespace_id, replica_path)
            .await
            .map_err(|e| miette::miette!("{}", e)),
    }
}
