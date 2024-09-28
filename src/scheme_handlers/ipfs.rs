use super::util::SchemeRequest;
use bytes::Bytes;
use futures::pin_mut;
use ipfs::Ipfs;
use miette::IntoDiagnostic;
use tokio_stream::StreamExt;
use webkit2gtk::functions::uri_for_display;

pub async fn ipfs_scheme(ipfs: &Ipfs, request: SchemeRequest) {
    let bytes_result = ipfs_scheme_handler(ipfs, request.clone()).await;
    request.finish(bytes_result);
}

pub async fn ipns_scheme(ipfs: &Ipfs, request: SchemeRequest) {
    let bytes_result = ipns_scheme_handler(ipfs, request.clone()).await;
    request.finish(bytes_result);
}

pub async fn ipfs_scheme_handler(
    ipfs: &Ipfs,
    request: SchemeRequest,
) -> miette::Result<impl Into<Bytes>> {
    let request_uri = request.uri().ok_or(miette::miette!(
        "Could read request URI ({:?}) … ",
        request.uri()
    ))?;
    let decoded_url = uri_for_display(&request_uri)
        .ok_or(miette::miette!(
            "Could display request URI safely ({}) … ",
            request_uri
        ))?
        .replacen("ipfs://", "", 1)
        .parse::<ipfs::IpfsPath>()
        .map_err(|e| miette::miette!("{}", e))?;
    let ipfs_stream = ipfs.cat_unixfs(decoded_url);
    let mut bytes_vec: Vec<u8> = vec![];
    pin_mut!(ipfs_stream);
    while let Some(bytes_result) = ipfs_stream.next().await {
        bytes_vec.extend(bytes_result.into_diagnostic()?)
    }
    Ok(bytes_vec)
}

pub async fn ipns_scheme_handler(
    ipfs: &Ipfs,
    request: SchemeRequest,
) -> miette::Result<impl Into<Bytes>> {
    let request_uri = request.uri().ok_or(miette::miette!(
        "Could read request URI ({:?}) … ",
        request.uri()
    ))?;
    let uri_for_display = uri_for_display(&request_uri).ok_or(miette::miette!(
        "Could display request URI safely ({}) … ",
        request_uri
    ))?;
    let decoded_url = format!("/ipns/{}", uri_for_display.replacen("ipns://", "", 1))
        .parse::<ipfs::IpfsPath>()
        .map_err(|e| miette::miette!("{}", e))?;
    let ipfs_stream = ipfs.cat_unixfs(decoded_url);
    let mut bytes_vec: Vec<u8> = vec![];
    pin_mut!(ipfs_stream);
    while let Some(bytes_result) = ipfs_stream.next().await {
        bytes_vec.extend(bytes_result.into_diagnostic()?)
    }
    Ok(bytes_vec)
}
