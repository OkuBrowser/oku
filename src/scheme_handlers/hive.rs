use super::util::SchemeRequest;
use crate::NODE;
use bytes::Bytes;
use oku_fs::iroh::docs::{DocTicket, NamespaceId};
use std::{path::PathBuf, str::FromStr};
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
    let path = PathBuf::from(decoded_url.clone());
    let components = &mut path.components();
    let first_component = components.next().ok_or(miette::miette!(
        "URI ({}) does not contain a replica ID or ticket",
        decoded_url
    ))?;
    let first_component_string = first_component.as_os_str().to_str().unwrap_or_default();
    let replica_path = PathBuf::from("/").join(components.as_path()).to_path_buf();
    let node = NODE.get().ok_or(miette::miette!(""))?;
    if let Ok(ticket) = DocTicket::from_str(first_component_string) {
        node.fetch_file_with_ticket(ticket, replica_path)
            .await
            .map_err(|e| miette::miette!("{}", e))
    } else if let Ok(namespace_id) = NamespaceId::from_str(first_component_string) {
        node.fetch_file(namespace_id, replica_path)
            .await
            .map_err(|e| miette::miette!("{}", e))
    } else {
        Err(miette::miette!(
            "URI ({}) does not contain a replica ID or ticket",
            decoded_url
        ))
    }
}
