use connexa::prelude::PeerId;
use either::Either;
use futures::{
    FutureExt, Stream, StreamExt,
    future::BoxFuture,
    stream::{BoxStream, FusedStream},
};
use ipld_core::cid::Cid;
use rust_unixfs::walk::{ContinuedWalk, Walker};
use std::pin::Pin;
use std::task::Context;
use std::{task::Poll, time::Duration};
use tracing::{Instrument, Span};

use crate::{
    Ipfs, IpfsPath,
    dag::IpldDag,
    repo::{DefaultStorage, Repo},
};

#[derive(Debug)]
pub enum Entry {
    Error { error: anyhow::Error },
    RootDirectory { cid: Cid, path: String },
    Directory { cid: Cid, path: String },
    File { cid: Cid, file: String, size: usize },
}

#[must_use = "does nothing unless you `.await` or poll the stream"]
pub struct UnixfsLs {
    core: Option<Either<Ipfs, Repo<DefaultStorage>>>,
    span: Span,
    path: Option<IpfsPath>,
    providers: Vec<PeerId>,
    local_only: bool,
    timeout: Option<Duration>,
    stream: Option<BoxStream<'static, Entry>>,
}

impl UnixfsLs {
    pub fn with_ipfs(ipfs: &Ipfs, path: impl Into<IpfsPath>) -> Self {
        Self::with_either(Either::Left(ipfs.clone()), path)
    }

    pub fn with_repo(repo: &Repo<DefaultStorage>, path: impl Into<IpfsPath>) -> Self {
        Self::with_either(Either::Right(repo.clone()), path)
    }

    fn with_either(core: Either<Ipfs, Repo<DefaultStorage>>, path: impl Into<IpfsPath>) -> Self {
        let path = path.into();
        Self {
            core: Some(core),
            path: Some(path),
            span: Span::current(),
            providers: Vec::new(),
            local_only: false,
            timeout: None,
            stream: None,
        }
    }

    pub fn span(mut self, span: Span) -> Self {
        self.span = span;
        self
    }

    pub fn provider(mut self, peer_id: PeerId) -> Self {
        if !self.providers.contains(&peer_id) {
            self.providers.push(peer_id);
        }
        self
    }

    pub fn providers(mut self, list: &[PeerId]) -> Self {
        self.providers = list.to_vec();
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn local(mut self) -> Self {
        self.local_only = true;
        self
    }

    pub fn set_local(mut self, local: bool) -> Self {
        self.local_only = local;
        self
    }
}

impl Stream for UnixfsLs {
    type Item = Entry;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.core.is_none() && self.stream.is_none() {
            return Poll::Ready(None);
        }
        loop {
            match &mut self.stream {
                Some(stream) => match futures::ready!(stream.poll_next_unpin(cx)) {
                    None => {
                        self.stream.take();
                        return Poll::Ready(None);
                    }
                    task => return Poll::Ready(task),
                },
                None => {
                    let Some(core) = self.core.take() else {
                        return Poll::Ready(None);
                    };

                    let (repo, dag) = match core {
                        Either::Left(ipfs) => (ipfs.repo().clone(), ipfs.dag()),
                        Either::Right(repo) => (repo.clone(), IpldDag::from(repo.clone())),
                    };

                    let path = self.path.take().expect("path exist");
                    let providers = std::mem::take(&mut self.providers);
                    let local_only = self.local_only;
                    let timeout = self.timeout;

                    // using async_stream here at least to get on faster; writing custom streams is not too easy
                    // but this might be easy enough to write open.
                    let stream = async_stream::stream! {

                        let resolved = match dag
                            ._resolve(path, true, &providers, local_only, timeout)
                            .await {
                                Ok((resolved, _)) => resolved,
                                Err(e) => {
                                    yield Entry::Error { error: e.into() };
                                    return;
                                }
                            };

                        let block = match resolved.into_unixfs_block() {
                            Ok(block) => block,
                            Err(e) => {
                                yield Entry::Error { error: e.into() };
                                return;
                            }
                        };

                        let cid = block.cid();
                        let root_name = cid.to_string();

                        let mut walker = Walker::new_shallow(*cid, root_name);
                        let mut cache = None;
                        let mut root_directory = String::new();

                        let mut prefetcher = super::prefetch::BlockPrefetcher::new(
                            repo,
                            providers,
                            local_only,
                            timeout,
                            super::prefetch::WINDOW,
                        );

                        while walker.should_continue() {
                            let next = {
                                let (next, rest) = walker.pending_links();
                                let next = *next;
                                prefetcher.want(std::iter::once(next).chain(rest.copied()));
                                next
                            };
                            let block = match prefetcher.get(next).await {
                                Ok(block) => block,
                                Err(error) => {
                                    yield Entry::Error { error };
                                    return;
                                }
                            };
                            let block_data = block.data();

                            match walker.next(block_data, &mut cache) {
                                Ok(ContinuedWalk::Bucket(..)) => {}
                                Ok(ContinuedWalk::File(_, cid, path, _, size)) => {
                                    let file = path.to_string_lossy().to_string().replace(&format!("{root_directory}/"), "");
                                    yield Entry::File { cid: *cid, file, size: size as _ };
                                },
                                Ok(ContinuedWalk::RootDirectory( cid, path, _)) => {
                                    let path = path.to_string_lossy().to_string();
                                    root_directory.clone_from(&path);
                                    yield Entry::RootDirectory { cid: *cid, path };
                                }
                                Ok(ContinuedWalk::Directory( cid, path, _)) => {
                                    let path = path.to_string_lossy().to_string().replace(&format!("{root_directory}/"), "");
                                    yield Entry::Directory { cid: *cid, path };
                                }
                                Ok(ContinuedWalk::Symlink( .. )) => {},
                                Err(error) => {
                                    yield Entry::Error { error: anyhow::Error::from(error) };
                                    return;
                                }
                            };
                        };

                    }.boxed();

                    self.stream.replace(stream);
                }
            }
        }
    }
}

impl std::future::IntoFuture for UnixfsLs {
    type Output = Result<Vec<Entry>, anyhow::Error>;

    type IntoFuture = BoxFuture<'static, Self::Output>;

    fn into_future(mut self) -> Self::IntoFuture {
        let span = self.span.clone();
        async move {
            let mut items = vec![];
            while let Some(status) = self.next().await {
                match status {
                    Entry::Error { error } => return Err(error),
                    item => items.push(item),
                }
            }
            Ok(items)
        }
        .instrument(span)
        .boxed()
    }
}

impl FusedStream for UnixfsLs {
    fn is_terminated(&self) -> bool {
        self.stream.is_none() && self.core.is_none()
    }
}
