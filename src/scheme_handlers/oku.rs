use super::util::SchemeRequest;
use crate::vox_providers::{
    oku_provider::core::OkuProvider, okunet_provider::core::OkuNetProvider,
};
use bytes::Bytes;
use miette::IntoDiagnostic;
use oku_fs::iroh::docs::AuthorId;
use std::{path::PathBuf, str::FromStr};
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
    let url_path = PathBuf::from(decoded_url);
    let url_components: Vec<_> = url_path
        .components()
        .map(|x| PathBuf::from(x.as_os_str()))
        .collect();
    let first_component = url_components
        .first()
        .map(|x| x.to_path_buf())
        .unwrap_or(PathBuf::from("home"));

    match first_component
        .as_os_str()
        .to_string_lossy()
        .to_string()
        .as_str()
    {
        "home" => OkuProvider::new()
            .render_and_get("output/home.html")
            .map(|x| x.into_bytes()),
        "me" => {
            if let Some(_second_component) = url_components.get(1) {
                OkuNetProvider::new()
                    .view_self_post(
                        url_path
                            .strip_prefix(first_component)
                            .into_diagnostic()?
                            .to_path_buf(),
                    )
                    .await
                    .map(|x| x.into_bytes())
            } else {
                OkuNetProvider::new()
                    .view_self()
                    .await
                    .map(|x| x.into_bytes())
            }
        }
        "tag" => {
            if let Some(second_component) = url_components.get(1) {
                OkuNetProvider::new()
                    .view_tag(second_component.to_string_lossy().to_string())
                    .await
                    .map(|x| x.into_bytes())
            } else {
                OkuNetProvider::new()
                    .view_tags()
                    .await
                    .map(|x| x.into_bytes())
            }
        }
        "tags" => OkuNetProvider::new()
            .view_tags()
            .await
            .map(|x| x.into_bytes()),
        _ => {
            let author_id = AuthorId::from_str(
                first_component
                    .as_os_str()
                    .to_string_lossy()
                    .to_string()
                    .as_str(),
            )
            .map_err(|e| miette::miette!("{}", e))?;
            if let Some(_second_component) = url_components.get(1) {
                OkuNetProvider::new()
                    .view_post(
                        author_id,
                        url_path
                            .strip_prefix(first_component)
                            .into_diagnostic()?
                            .to_path_buf(),
                    )
                    .await
                    .map(|x| x.into_bytes())
            } else {
                OkuNetProvider::new()
                    .view_user(author_id)
                    .await
                    .map(|x| x.into_bytes())
            }
        }
    }
}
