use std::task::{Context, Poll};

use crate::{
    Block,
    repo::{DefaultStorage, Repo},
};
use bytes::Bytes;
use either::Either;
#[allow(unused_imports)]
use futures::{
    FutureExt, Stream, StreamExt, TryFutureExt,
    future::BoxFuture,
    stream::{BoxStream, FusedStream},
};
use ipld_core::cid::Version;
use multihash_codetable::Code;
use rust_unixfs::Metadata;
use rust_unixfs::file::adder::{Chunker, FileAdderBuilder};
#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};
use std::pin::Pin;
#[cfg(not(target_arch = "wasm32"))]
use tokio_util::io::ReaderStream;
use tracing::{Instrument, Span};

use crate::{Ipfs, IpfsPath};

use super::{TraversalFailed, UnixfsStatus};

pub enum AddOpt {
    #[cfg(not(target_arch = "wasm32"))]
    File(PathBuf),
    Stream {
        name: Option<String>,
        total: Option<usize>,
        stream: BoxStream<'static, Result<Bytes, std::io::Error>>,
    },
}

#[cfg(not(target_arch = "wasm32"))]
impl From<PathBuf> for AddOpt {
    fn from(path: PathBuf) -> Self {
        AddOpt::File(path)
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<&Path> for AddOpt {
    fn from(path: &Path) -> Self {
        AddOpt::File(path.to_path_buf())
    }
}

#[must_use = "does nothing unless you `.await` or poll the stream"]
pub struct UnixfsAdd {
    core: Option<Either<Ipfs, Repo<DefaultStorage>>>,
    opt: Option<AddOpt>,
    span: Span,
    chunk: Chunker,
    cid_version: Version,
    raw_leaves: Option<bool>,
    hasher: Code,
    metadata: Metadata,
    pin: bool,
    provide: bool,
    wrap: bool,
    stream: Option<BoxStream<'static, UnixfsStatus>>,
}

impl UnixfsAdd {
    pub fn with_ipfs(ipfs: &Ipfs, opt: impl Into<AddOpt>) -> Self {
        Self::with_either(Either::Left(ipfs.clone()), opt)
    }

    pub fn with_repo(repo: &Repo<DefaultStorage>, opt: impl Into<AddOpt>) -> Self {
        Self::with_either(Either::Right(repo.clone()), opt)
    }

    fn with_either(core: Either<Ipfs, Repo<DefaultStorage>>, opt: impl Into<AddOpt>) -> Self {
        let opt = opt.into();
        Self {
            core: Some(core),
            opt: Some(opt),
            span: Span::current(),
            chunk: Chunker::Size(256 * 1024),
            cid_version: Version::V1,
            raw_leaves: None,
            hasher: Code::Sha2_256,
            metadata: Metadata::default(),
            pin: true,
            provide: false,
            wrap: false,
            stream: None,
        }
    }

    pub fn span(mut self, span: Span) -> Self {
        self.span = span;
        self
    }

    pub fn chunk(mut self, chunk: Chunker) -> Self {
        self.chunk = chunk;
        self
    }

    pub fn cid_version(mut self, version: Version) -> Self {
        self.cid_version = version;
        self
    }

    pub fn raw_leaves(mut self, raw_leaves: bool) -> Self {
        self.raw_leaves = Some(raw_leaves);
        self
    }

    /// Sets the multihash used for produced blocks. Defaults to sha2-256. A hash other than
    /// sha2-256 implies CIDv1.
    pub fn hasher(mut self, hasher: Code) -> Self {
        self.hasher = hasher;
        self
    }

    pub fn metadata(mut self, metadata: Metadata) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn pin(mut self, pin: bool) -> Self {
        self.pin = pin;
        self
    }

    pub fn provide(mut self) -> Self {
        self.provide = true;
        self
    }

    pub fn wrap(mut self) -> Self {
        self.wrap = true;
        self
    }
}

impl Stream for UnixfsAdd {
    type Item = UnixfsStatus;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.core.is_none() && self.stream.is_none() {
            return Poll::Ready(None);
        }
        loop {
            match &mut self.stream {
                None => {
                    let (ipfs, repo) = match self.core.take().expect("ipfs or repo is used") {
                        Either::Left(ipfs) => {
                            let repo = ipfs.repo().clone();
                            (Some(ipfs), repo)
                        }
                        Either::Right(repo) => (None, repo),
                    };
                    let option = self.opt.take().expect("option already constructed");
                    let chunk = self.chunk;
                    let cid_version = self.cid_version;
                    let raw_leaves = self.raw_leaves;
                    let hasher = self.hasher;
                    let metadata = self.metadata.clone();
                    let pin = self.pin;
                    let provide = self.provide;
                    let wrap = self.wrap;

                    let stream = async_stream::stream! {
                        let guard = repo.gc_guard().await;

                        let mut written = 0;

                        let (name, total_size, mut stream) = match option {
                            #[cfg(not(target_arch = "wasm32"))]
                            AddOpt::File(path) => match tokio::fs::File::open(path.clone())
                                .and_then(|file| async move {
                                    let size = file.metadata().await?.len() as usize;

                                    let stream = ReaderStream::new(file);

                                    let name: Option<String> = path.file_name().map(|f| f.to_string_lossy().to_string());

                                    Ok((name, Some(size), stream.boxed()))
                                }).await {
                                    Ok(s) => s,
                                    Err(e) => {
                                        yield UnixfsStatus::FailedStatus { written, total_size: None, error: e.into() };
                                        return;
                                    }
                                },
                            AddOpt::Stream { name, total, stream } => (name, total, stream),
                        };

                        let mut adder = {
                            let mut builder = FileAdderBuilder::default()
                                .with_chunker(chunk)
                                .with_cid_version(cid_version)
                                .with_hasher(hasher)
                                .with_metadata(metadata);
                            if let Some(raw_leaves) = raw_leaves {
                                builder = builder.with_raw_leaves(raw_leaves);
                            }
                            builder.build()
                        };

                        yield UnixfsStatus::ProgressStatus { written, total_size };

                        while let Some(buffer) = stream.next().await {
                            let buffer = match buffer {
                                Ok(buf) => buf,
                                Err(e) => {
                                    yield UnixfsStatus::FailedStatus { written, total_size, error: e.into() };
                                    return;
                                }
                            };

                            let mut total = 0;
                            while total < buffer.len() {
                                let (blocks, consumed) = adder.push(&buffer[total..]);
                                let mut to_put = Vec::new();
                                for (cid, block) in blocks {
                                    match Block::new(cid, block) {
                                        Ok(block) => to_put.push(block),
                                        Err(e) => {
                                            yield UnixfsStatus::FailedStatus { written, total_size, error: e.into() };
                                            return;
                                        }
                                    }
                                }
                                if let Err(e) = repo.put_blocks(to_put).with_gc_guard(guard.clone()).await {
                                    yield UnixfsStatus::FailedStatus { written, total_size, error: e };
                                    return;
                                }
                                total += consumed;
                                written += consumed;
                            }

                            yield UnixfsStatus::ProgressStatus { written, total_size };
                        }

                        let blocks = adder.finish();
                        let mut last_cid = None;
                        let mut to_put = Vec::new();

                        for (cid, block) in blocks {
                            match Block::new(cid, block) {
                                Ok(block) => to_put.push(block),
                                Err(e) => {
                                    yield UnixfsStatus::FailedStatus { written, total_size, error: e.into() };
                                    return;
                                }
                            }
                            last_cid = Some(cid);
                        }

                        if let Err(e) = repo.put_blocks(to_put).with_gc_guard(guard.clone()).await {
                            yield UnixfsStatus::FailedStatus { written, total_size, error: e };
                            return;
                        }

                        let cid = match last_cid {
                            Some(cid) => cid,
                            None => {
                                yield UnixfsStatus::FailedStatus { written, total_size, error: TraversalFailed::Io(std::io::ErrorKind::InvalidData.into()).into() };
                                return;
                            }
                        };

                        let mut path = IpfsPath::from(cid);

                        if wrap && let Some(name) = name {
                                let result = {
                                    let repo = repo.clone();
                                    let guard = guard.clone();
                                    async move {
                                        let mut opts = rust_unixfs::dir::builder::TreeOptions::default();
                                        opts.wrap_with_directory();
                                        opts.cid_version(cid_version);
                                        opts.hasher(hasher);

                                        let mut tree = rust_unixfs::dir::builder::BufferingTreeBuilder::new(opts);
                                        tree.put_link(&name, cid, written as _)?;

                                        let mut iter = tree.build();
                                        let mut cids = Vec::new();
                                        let mut to_put = Vec::new();

                                        while let Some(node) = iter.next_borrowed() {
                                            //TODO: Determine best course to prevent additional allocation
                                            let node = node?;
                                            let cid = node.cid.to_owned();
                                            to_put.push(Block::new(cid, node.block.to_vec())?);
                                            cids.push(cid);
                                        }

                                        repo.put_blocks(to_put).with_gc_guard(guard).await?;
                                        let cid = cids.last().ok_or(anyhow::anyhow!("no cid available"))?;
                                        let path = IpfsPath::from(*cid).sub_path(&name)?;

                                        Ok::<_, anyhow::Error>(path)
                                    }
                                };

                                path = match result.await {
                                    Ok(path) => path,
                                    Err(e) => {
                                        yield UnixfsStatus::FailedStatus { written, total_size, error: e };
                                        return;
                                    }
                                };
                            }

                        let cid = path.root().cid().copied().expect("Cid is apart of the path");

                        if pin && !repo.is_pinned(&cid).await.unwrap_or_default() && let Err(e) = repo.pin(cid).with_gc_guard(guard.clone()).recursive().await {
                            error!("Unable to pin {cid}: {e}");
                        }

                        if provide &&  let Some(ipfs) = ipfs  && let Err(e) = ipfs.provide(cid).await {
                            error!("Unable to provide {cid}: {e}");
                        }


                        yield UnixfsStatus::CompletedStatus { path, written, total_size }
                    };

                    self.stream = Some(stream.boxed());
                }
                Some(stream) => match futures::ready!(stream.poll_next_unpin(cx)) {
                    Some(item) => {
                        if matches!(
                            item,
                            UnixfsStatus::FailedStatus { .. }
                                | UnixfsStatus::CompletedStatus { .. }
                        ) {
                            self.stream.take();
                        }
                        return Poll::Ready(Some(item));
                    }
                    None => {
                        self.stream.take();
                        return Poll::Ready(None);
                    }
                },
            }
        }
    }
}

impl std::future::IntoFuture for UnixfsAdd {
    type Output = Result<IpfsPath, anyhow::Error>;

    type IntoFuture = BoxFuture<'static, Self::Output>;

    fn into_future(mut self) -> Self::IntoFuture {
        let span = self.span.clone();
        async move {
            while let Some(status) = self.next().await {
                match status {
                    UnixfsStatus::CompletedStatus { path, .. } => return Ok(path),
                    UnixfsStatus::FailedStatus { error, .. } => {
                        return Err(error);
                    }
                    _ => {}
                }
            }
            Err::<_, anyhow::Error>(anyhow::anyhow!("Unable to add file"))
        }
        .instrument(span)
        .boxed()
    }
}

impl FusedStream for UnixfsAdd {
    fn is_terminated(&self) -> bool {
        self.stream.is_none() && self.core.is_none()
    }
}
