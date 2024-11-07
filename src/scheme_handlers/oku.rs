use super::{oku_path::OkuPath, util::SchemeRequest};
use crate::{
    vox_providers::{oku_provider::core::OkuProvider, okunet_provider::core::OkuNetProvider},
    NODE,
};
use bytes::Bytes;
use oku_fs::iroh::docs::AuthorId;
use webkit2gtk::functions::uri_for_display;

pub async fn oku_scheme(request: SchemeRequest) {
    let bytes_result: miette::Result<Bytes> =
        match request.http_method().unwrap_or_default().as_str() {
            "POST" => post_oku_scheme_handler(request.clone()).await,
            _ => get_oku_scheme_handler(request.clone()).await,
        };
    request.finish(bytes_result);
}

pub async fn post_oku_scheme_handler(request: SchemeRequest) -> miette::Result<Bytes> {
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
        OkuPath::ToggleFollow(author_id) => match toggle_follow(author_id).await {
            Ok(_) => Ok("Ok".into()),
            Err(e) => Err(e),
        },
        OkuPath::ToggleBlock(author_id) => match toggle_block(author_id).await {
            Ok(_) => Ok("Ok".into()),
            Err(e) => Err(e),
        },
        _ => Err(miette::miette!(
            "Operation {:?} not supported for POST requests to Oku scheme … ",
            url_path
        )),
    }
}

pub async fn toggle_follow(author_id: AuthorId) -> miette::Result<()> {
    let node = NODE
        .get()
        .ok_or(miette::miette!("No running Oku node … "))?;
    node.toggle_follow(author_id).await?;
    Ok(())
}

pub async fn toggle_block(author_id: AuthorId) -> miette::Result<()> {
    let node = NODE
        .get()
        .ok_or(miette::miette!("No running Oku node … "))?;
    node.toggle_block(author_id).await?;
    Ok(())
}

pub async fn get_oku_scheme_handler(request: SchemeRequest) -> miette::Result<Bytes> {
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
            .map(|x| x.into()),
        OkuPath::Tags => OkuNetProvider::new().view_tags().await.map(|x| x.into()),
        OkuPath::Tag(tag) => OkuNetProvider::new().view_tag(tag).await.map(|x| x.into()),
        OkuPath::Me(replica_path) => match replica_path {
            Some(replica_path) => OkuNetProvider::new()
                .view_self_post(replica_path)
                .await
                .map(|x| x.into()),
            None => OkuNetProvider::new().view_self().await.map(|x| x.into()),
        },
        OkuPath::User(author_id, replica_path) => match replica_path {
            Some(replica_path) => OkuNetProvider::new()
                .view_post(author_id, replica_path)
                .await
                .map(|x| x.into()),
            None => OkuNetProvider::new()
                .view_user(author_id)
                .await
                .map(|x| x.into()),
        },
        _ => Err(miette::miette!(
            "Operation {:?} not supported for GET requests to Oku scheme … ",
            url_path
        )),
    }
}
