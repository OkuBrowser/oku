//! An [AsyncSliceReader] implementation for HTTP resources, using range requests.
//!
//! Uses the [reqwest](https://docs.rs/reqwest) crate. Somewhat inspired by
//! <https://github.com/fasterthanlime/ubio/blob/main/src/http/mod.rs>
use std::{pin::Pin, str::FromStr, sync::Arc};

use futures_lite::{Stream, StreamExt};
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Method, StatusCode, Url,
};

use self::http_adapter::Opts;
use super::*;

/// A struct that implements [AsyncSliceReader] using HTTP range requests
#[derive(Debug)]
pub struct HttpAdapter {
    opts: Arc<http_adapter::Opts>,
    size: Option<u64>,
}

impl HttpAdapter {
    /// Creates a new [`HttpAdapter`] from a URL
    pub fn new(url: Url) -> Self {
        Self::with_opts(Arc::new(Opts {
            url,
            client: reqwest::Client::new(),
            headers: None,
        }))
    }

    /// Creates a new [`HttpAdapter`] from a URL and options
    pub fn with_opts(opts: Arc<http_adapter::Opts>) -> Self {
        Self { opts, size: None }
    }

    /// Returns the client used for requests
    pub fn client(&self) -> &reqwest::Client {
        &self.opts.client
    }

    /// Returns the URL of the resource
    pub fn url(&self) -> &Url {
        &self.opts.url
    }

    async fn head_request(&self) -> Result<reqwest::Response, reqwest::Error> {
        let mut req_builder = self.client().request(Method::HEAD, self.url().clone());
        if let Some(headers) = self.opts.headers.as_ref() {
            for (k, v) in headers.iter() {
                req_builder = req_builder.header(k, v);
            }
        }
        let req = req_builder.build()?;
        let res = self.client().execute(req).await?;
        Ok(res)
    }

    async fn range_request(
        &self,
        from: u64,
        to: Option<u64>,
    ) -> Result<reqwest::Response, reqwest::Error> {
        // to is inclusive, commented out because warp is non spec compliant
        let to = to.and_then(|x| x.checked_add(1));
        let range = match to {
            Some(to) => format!("bytes={from}-{to}"),
            None => format!("bytes={from}-"),
        };
        let mut req_builder = self.client().request(Method::GET, self.url().clone());
        if let Some(headers) = self.opts.headers.as_ref() {
            for (k, v) in headers.iter() {
                req_builder = req_builder.header(k, v);
            }
        }
        req_builder = req_builder.header("range", range);

        let req = req_builder.build()?;
        let res = self.client().execute(req).await?;
        Ok(res)
    }

    async fn get_stream_at(
        &self,
        offset: u64,
        len: usize,
    ) -> io::Result<Pin<Box<dyn Stream<Item = io::Result<Bytes>>>>> {
        if let Some(size) = self.size {
            if offset >= size {
                return Ok(Box::pin(futures_lite::stream::empty()));
            }
        }
        let from = offset;
        let to = offset.checked_add(len as u64);
        // if we have a size, clamp the range
        let from = self.size.map(|size| from.min(size)).unwrap_or(from);
        let to = self
            .size
            .map(|size| to.map(|to| to.min(size)))
            .unwrap_or(to);
        let res = self.range_request(from, to).await.map_err(make_io_error)?;
        if res.status().is_success() {
            Ok(Box::pin(
                res.bytes_stream().map(|r| r.map_err(make_io_error)),
            ))
        } else if res.status() == StatusCode::RANGE_NOT_SATISFIABLE {
            // https://developer.mozilla.org/en-US/docs/Web/HTTP/Status/416
            // we requested a range that is out of bounds, just return nothing
            Ok(Box::pin(futures_lite::stream::empty()))
        } else {
            Err(make_io_error(format!("http error {}", res.status())))
        }
    }
}

/// Support for the [HttpAdapter]
pub mod http_adapter {
    use super::*;

    /// Options for [HttpAdapter]
    #[derive(Debug, Clone)]
    pub struct Opts {
        /// The URL of the resource
        pub url: Url,
        /// Additional headers to send with the requests
        pub headers: Option<HeaderMap<HeaderValue>>,
        /// The client to use for requests. If not set, a new client will be created
        /// for each reader, which is wasteful.
        pub client: reqwest::Client,
    }

    impl AsyncSliceReader for HttpAdapter {
        async fn read_at(&mut self, offset: u64, len: usize) -> io::Result<Bytes> {
            let mut stream = self.get_stream_at(offset, len).await?;
            let mut res = BytesMut::with_capacity(len.min(1024));
            while let Some(chunk) = stream.next().await {
                let chunk = chunk?;
                res.extend_from_slice(&chunk);
                if BytesMut::len(&res) >= len {
                    break;
                }
            }
            // we do not want to rely on the server sending the exact amount of bytes
            res.truncate(len);
            Ok(res.freeze())
        }

        async fn size(&mut self) -> io::Result<u64> {
            let io_err = |text: &str| io::Error::other(text);
            let head_response = self
                .head_request()
                .await
                .map_err(|_| io_err("head request failed"))?;
            if !head_response.status().is_success() {
                return Err(io_err("head request failed"));
            }
            let size = head_response
                .headers()
                .get("content-length")
                .ok_or_else(|| io_err("content-length header missing"))?;
            let text = size
                .to_str()
                .map_err(|_| io_err("content-length malformed"))?;
            let size = u64::from_str(text).map_err(|_| io_err("content-length malformed"))?;
            self.size = Some(size);
            Ok(size)
        }
    }
}
