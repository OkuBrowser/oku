use super::util::SchemeRequest;
use crate::vox_providers::oku_provider::core::OkuProvider;
use bytes::Bytes;
use miette::IntoDiagnostic;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use webkit2gtk::prelude::WebViewExt;

pub struct SendFuture<T>(Pin<Box<dyn Future<Output = T>>>);
unsafe impl<T: Send> Send for SendFuture<T> {}
impl<T> Future for SendFuture<T> {
    type Output = T;
    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        self.0.as_mut().poll(context)
    }
}

#[derive(Clone)]
pub struct Resource(pub webkit2gtk::WebResource);
unsafe impl Send for Resource {}
unsafe impl Sync for Resource {}
impl Resource {
    pub async fn data(&self) -> SendFuture<Result<Vec<u8>, glib::Error>> {
        SendFuture(Box::pin(self.0.data_future()))
    }
}

pub async fn view_source_scheme(request: SchemeRequest) {
    let bytes_result = view_source_scheme_handler(request.clone()).await;
    request.finish(bytes_result);
}

pub async fn view_source_scheme_handler(
    request: SchemeRequest,
) -> miette::Result<impl Into<Bytes>> {
    let web_view = request.0.web_view().ok_or(miette::miette!(""))?;
    let resource = Resource(
        web_view
            .main_resource()
            .ok_or(miette::miette!("No resource loaded to view source of … "))?,
    );
    let data = glib::spawn_future(resource.data().await)
        .await
        .map_err(|e| miette::miette!("{}", e))?
        .map_err(|e| miette::miette!("{}", e))?;
    let html = std::str::from_utf8(&data).into_diagnostic()?.to_string();
    let uri = request.uri().unwrap_or_default();
    OkuProvider::new().view_source(html, uri)
}
