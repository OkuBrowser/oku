use super::{oku_path::OkuPath, util::SchemeRequest};
use crate::{
    vox_providers::{oku_provider::core::OkuProvider, okunet_provider::core::OkuNetProvider},
    window_util::get_window_from_widget,
    HOME_REPLICA_SET, NODE,
};
use bytes::Bytes;
use glib::clone;
use libadwaita::{
    prelude::{AdwDialogExt, AlertDialogExt, AlertDialogExtManual},
    ResponseAppearance,
};
use log::error;
use oku_fs::iroh_docs::AuthorId;
use std::{path::PathBuf, sync::atomic::Ordering};
use webkit2gtk::{functions::uri_for_display, prelude::WebViewExt};

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
        OkuPath::Delete(replica_path) => match glib::MainContext::default()
            .spawn(async move { delete(request, replica_path) })
            .await
        {
            Ok(_) => Ok("Ok".into()),
            Err(e) => Err(miette::miette!("{}", e)),
        },
        _ => Err(miette::miette!(
            "Operation {:?} not supported for POST requests to Oku scheme … ",
            url_path
        )),
    }
}

pub fn delete(request: SchemeRequest, replica_path: PathBuf) -> miette::Result<()> {
    let window = request.0.web_view().map(|x| get_window_from_widget(&x));
    let ctx = glib::MainContext::default();
    let node = NODE
        .get()
        .ok_or(miette::miette!("No running Oku node … "))?;
    let dialog = libadwaita::AlertDialog::new(
        Some("Delete post?"),
        Some(&format!(
            "You are trying to delete the post {:?}. This cannot be undone.",
            replica_path
        )),
    );
    dialog.add_responses(&[("cancel", "Cancel"), ("delete", "Delete")]);
    dialog.set_response_appearance("cancel", ResponseAppearance::Default);
    dialog.set_response_appearance("delete", ResponseAppearance::Destructive);
    dialog.set_default_response(Some("cancel"));
    dialog.set_close_response("cancel");
    dialog.connect_response(
        None,
        clone!(
            #[strong]
            ctx,
            #[strong]
            node,
            #[strong]
            replica_path,
            #[strong]
            request,
            move |_, response| {
                match response {
                    "cancel" => (),
                    "delete" => {
                        ctx.spawn_local(clone!(
                            #[strong]
                            node,
                            #[strong]
                            replica_path,
                            #[strong]
                            request,
                            async move {
                                if let Err(e) = node.delete_post(&replica_path).await {
                                    error!("{}", e);
                                } else {
                                    let web_view = request.0.web_view().unwrap();
                                    web_view.reload_bypass_cache();
                                }
                            }
                        ));
                    }
                    _ => {
                        unreachable!()
                    }
                }
            }
        ),
    );
    dialog.present(window.as_ref());
    Ok(())
}

pub async fn toggle_follow(author_id: AuthorId) -> miette::Result<()> {
    let node = NODE
        .get()
        .ok_or(miette::miette!("No running Oku node … "))?;
    node.toggle_follow(&author_id).await?;
    Ok(())
}

pub async fn toggle_block(author_id: AuthorId) -> miette::Result<()> {
    let node = NODE
        .get()
        .ok_or(miette::miette!("No running Oku node … "))?;
    node.toggle_block(&author_id).await?;
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
        OkuPath::Home => home().await,
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
        OkuPath::Search(query) => OkuNetProvider::new().search(query).await.map(|x| x.into()),
        _ => Err(miette::miette!(
            "Operation {:?} not supported for GET requests to Oku scheme … ",
            url_path
        )),
    }
}

pub async fn home() -> miette::Result<Bytes> {
    match HOME_REPLICA_SET.load(Ordering::Relaxed) {
        false => OkuProvider::new()
            .render_and_get("output/home.html")
            .map(|x| x.into()),
        true => OkuNetProvider::new().view_home().await.map(|x| x.into()),
    }
}
