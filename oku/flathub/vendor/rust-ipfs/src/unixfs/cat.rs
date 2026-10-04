use crate::repo::DefaultStorage;
use crate::{Block, Ipfs, dag::IpldDag, repo::Repo};
use async_stream::try_stream;
use bytes::Bytes;
use connexa::prelude::PeerId;
use either::Either;
use futures::future::BoxFuture;
use futures::stream::{BoxStream, FusedStream, Stream};
use futures::{FutureExt, StreamExt, TryStreamExt};
use rust_unixfs::file::visit::IdleFileVisit;
use std::ops::Range;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use tracing::{Instrument, Span};

use super::TraversalFailed;

/// IPFS cat operation, producing a stream of file bytes. This is generic over the different kinds
/// of ways to own an `Ipfs` value in order to support both operating with borrowed `Ipfs` value
/// and an owned value. Passing an owned value allows the return value to be `'static`, which can
/// be helpful in some contexts, like the http.
///
/// Returns a stream of bytes on the file pointed with the Cid.
#[must_use = "does nothing unless you `.await` or poll the stream"]
pub struct UnixfsCat {
    core: Option<Either<Ipfs, Repo<DefaultStorage>>>,
    span: Span,
    length: Option<usize>,
    starting_point: Option<StartingPoint>,
    range: Option<Range<u64>>,
    providers: Vec<PeerId>,
    local_only: bool,
    timeout: Option<Duration>,
    stream: Option<BoxStream<'static, Result<Bytes, TraversalFailed>>>,
}

impl UnixfsCat {
    pub fn with_ipfs(ipfs: &Ipfs, starting_point: impl Into<StartingPoint>) -> Self {
        Self::with_either(Either::Left(ipfs.clone()), starting_point)
    }

    pub fn with_repo(
        repo: &Repo<DefaultStorage>,
        starting_point: impl Into<StartingPoint>,
    ) -> Self {
        Self::with_either(Either::Right(repo.clone()), starting_point)
    }

    fn with_either(
        core: Either<Ipfs, Repo<DefaultStorage>>,
        starting_point: impl Into<StartingPoint>,
    ) -> Self {
        let starting_point = starting_point.into();
        Self {
            core: Some(core),
            starting_point: Some(starting_point),
            span: Span::current(),
            range: None,
            length: None,
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

    pub fn max_length(mut self, length: usize) -> Self {
        self.length = Some(length);
        self
    }

    pub fn set_max_length(mut self, length: impl Into<Option<usize>>) -> Self {
        self.length = length.into();
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

/// The starting point for unixfs walks. Can be converted from IpfsPath and Blocks, and Cids can be
/// converted to IpfsPath.
pub enum StartingPoint {
    Left(crate::IpfsPath),
    Right(Block),
}

impl<T: Into<crate::IpfsPath>> From<T> for StartingPoint {
    fn from(a: T) -> Self {
        Self::Left(a.into())
    }
}

impl From<Block> for StartingPoint {
    fn from(b: Block) -> Self {
        Self::Right(b)
    }
}

impl Stream for UnixfsCat {
    type Item = Result<Bytes, TraversalFailed>;
    #[allow(clippy::result_large_err)]
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

                    let mut visit = IdleFileVisit::default();

                    if let Some(range) = self.range.clone() {
                        visit = visit.with_target_range(range);
                    }

                    let starting_point = self.starting_point.take().expect("starting point exist");
                    let providers = std::mem::take(&mut self.providers);
                    let local_only = self.local_only;
                    let timeout = self.timeout;

                    let length = self.length;

                    // using async_stream here at least to get on faster; writing custom streams is not too easy
                    // but this might be easy enough to write open.
                    let stream = try_stream! {

                        // Get the root block to start the traversal. The stream does not expose any of the file
                        // metadata. To get to it the user needs to create a Visitor over the first block.
                        let block = match starting_point {
                            StartingPoint::Left(path) => dag
                                ._resolve(path.clone(), true, &providers, local_only, timeout)
                                .await
                                .map_err(TraversalFailed::Resolving)
                                .and_then(|(resolved, _)| {
                                    resolved.into_unixfs_block().map_err(TraversalFailed::Path)
                                })?,
                            StartingPoint::Right(block) => block,
                        };

                        if block.cid().codec() == 0x55 {
                            let content = visit.start_from_raw(block.data()).0;
                            if !content.is_empty() {
                                if let Some(length) = length && content.len() > length {
                                    Err::<(), TraversalFailed>(TraversalFailed::MaxLengthExceeded { size: content.len(), length })?;
                                }
                                yield Bytes::copy_from_slice(content);
                            }
                            return;
                        }

                        let mut cache = None;
                        let mut size = 0;

                        // Start the visit from the root block. We need to move the both components as Options into the
                        // stream as we can't yet return them from this Future context.
                        let (visit, bytes) = visit.start(block.data()).map(|(bytes, _, _, visit)| {
                            let bytes = (!bytes.is_empty()).then(|| Bytes::copy_from_slice(bytes));
                            (visit, bytes)
                        }).map_err(|e| {
                            TraversalFailed::Walking(*block.cid(), e)
                        }).and_then(|(visit, bytes)| {
                            if let Some(bytes) = &bytes {
                                size += bytes.len();
                                if let Some(length) = length && size > length {
                                    return Err::<_, TraversalFailed>(TraversalFailed::MaxLengthExceeded { size, length });
                                }
                            }
                            Ok::<_, TraversalFailed>((visit, bytes))
                        })?;


                        if let Some(bytes) = bytes {
                            yield bytes;
                        }

                        let mut visit = match visit {
                            Some(visit) => visit,
                            None => return,
                        };

                        let mut prefetcher = super::prefetch::BlockPrefetcher::new(
                            repo,
                            providers,
                            local_only,
                            timeout,
                            super::prefetch::WINDOW,
                        );

                        loop {
                            let next = {
                                let (next, rest) = visit.pending_links();
                                let next = *next;
                                prefetcher.want(std::iter::once(next).chain(rest.copied()));
                                next
                            };

                            let block = prefetcher
                                .get(next)
                                .await
                                .map_err(|e| TraversalFailed::Loading(next, e))?;

                            let (bytes, next_visit) = visit.continue_walk(block.data(), &mut cache).map_err(|e| TraversalFailed::Walking(*block.cid(), e))?;

                            size += bytes.len();

                            if let Some(length) = length && size > length {
                                    let fn_err = || Err::<_, TraversalFailed>(TraversalFailed::MaxLengthExceeded { size, length });
                                    fn_err()?;
                                    return;
                            }

                            if !bytes.is_empty() {
                                yield Bytes::copy_from_slice(bytes);
                            }

                            match next_visit {
                                Some(v) => visit = v,
                                None => return,
                            }

                        }
                    }.boxed();

                    self.stream.replace(stream);
                }
            }
        }
    }
}

impl std::future::IntoFuture for UnixfsCat {
    type Output = Result<Bytes, TraversalFailed>;

    type IntoFuture = BoxFuture<'static, Self::Output>;

    fn into_future(mut self) -> Self::IntoFuture {
        let span = self.span.clone();
        async move {
            let mut data = vec![];
            while let Some(bytes) = self.try_next().await? {
                data.extend(bytes);
            }
            Ok(data.into())
        }
        .instrument(span)
        .boxed()
    }
}

impl FusedStream for UnixfsCat {
    fn is_terminated(&self) -> bool {
        self.stream.is_none() && self.core.is_none()
    }
}
