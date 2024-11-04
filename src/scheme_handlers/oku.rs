use super::{oku_path::OkuPath, util::SchemeRequest};
use crate::vox_providers::{
    oku_provider::core::OkuProvider, okunet_provider::core::OkuNetProvider,
};
use bytes::Bytes;
use webkit2gtk::functions::uri_for_display;

pub async fn oku_scheme(request: SchemeRequest) {
    let bytes_result = oku_scheme_handler(request.clone()).await;
    request.finish(bytes_result);
}

pub async fn oku_scheme_handler(request: SchemeRequest) -> miette::Result<impl Into<Bytes>> {
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
    let url_path = OkuPath::parse(decoded_url)?;
    match url_path {
        OkuPath::Home => OkuProvider::new()
            .render_and_get("output/home.html")
            .map(|x| x.into_bytes()),
        OkuPath::Tags => OkuNetProvider::new()
            .view_tags()
            .await
            .map(|x| x.into_bytes()),
        OkuPath::Tag(tag) => OkuNetProvider::new()
            .view_tag(tag)
            .await
            .map(|x| x.into_bytes()),
        OkuPath::Me(replica_path) => match replica_path {
            Some(replica_path) => OkuNetProvider::new()
                .view_self_post(replica_path)
                .await
                .map(|x| x.into_bytes()),
            None => OkuNetProvider::new()
                .view_self()
                .await
                .map(|x| x.into_bytes()),
        },
        OkuPath::User(author_id, replica_path) => match replica_path {
            Some(replica_path) => OkuNetProvider::new()
                .view_post(author_id, replica_path)
                .await
                .map(|x| x.into_bytes()),
            None => OkuNetProvider::new()
                .view_user(author_id)
                .await
                .map(|x| x.into_bytes()),
        },
    }
}
