//! Bounded-window block prefetcher shared by the unixfs cat/get DAG walks.

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use anyhow::Error;
use connexa::prelude::PeerId;
use futures::future::BoxFuture;
use futures::stream::FuturesUnordered;
use futures::{FutureExt, StreamExt};
use ipld_core::cid::Cid;

use crate::Block;
use crate::repo::{DefaultStorage, Repo};

pub(crate) const WINDOW: usize = 16;

type FetchResult = (Cid, Result<Block, Error>);

pub(crate) struct BlockPrefetcher {
    repo: Repo<DefaultStorage>,
    providers: Vec<PeerId>,
    local: bool,
    timeout: Option<Duration>,
    window: usize,
    inflight: FuturesUnordered<BoxFuture<'static, FetchResult>>,
    ready: HashMap<Cid, Result<Block, Error>>,
    requested: HashSet<Cid>,
}

impl BlockPrefetcher {
    pub(crate) fn new(
        repo: Repo<DefaultStorage>,
        providers: Vec<PeerId>,
        local: bool,
        timeout: Option<Duration>,
        window: usize,
    ) -> Self {
        Self {
            repo,
            providers,
            local,
            timeout,
            window: window.max(1),
            inflight: FuturesUnordered::new(),
            ready: HashMap::new(),
            requested: HashSet::new(),
        }
    }

    fn request(&self, cid: Cid) -> BoxFuture<'static, FetchResult> {
        self.repo
            .get_block(cid)
            .providers(self.providers.iter().copied())
            .set_local(self.local)
            .timeout(self.timeout)
            .map(move |r| (cid, r))
            .boxed()
    }

    fn fetch(&mut self, cid: Cid) {
        if !self.requested.insert(cid) {
            return;
        }
        let fut = self.request(cid);
        self.inflight.push(fut);
    }

    /// Top up the in-flight set from the prefetch frontier (`next` first, then the upcoming links in
    /// pop order) until a window's worth is outstanding. Fully drains `links`, so any visitor borrow
    /// it carries ends when this returns.
    pub(crate) fn want(&mut self, links: impl IntoIterator<Item = Cid>) {
        for cid in links {
            if self.requested.len() >= self.window && !self.requested.contains(&cid) {
                break;
            }
            self.fetch(cid);
        }
    }

    pub(crate) async fn get(&mut self, next: Cid) -> Result<Block, Error> {
        self.fetch(next);
        while !self.ready.contains_key(&next) {
            let (cid, res) = self
                .inflight
                .next()
                .await
                .expect("inflight is non-empty while next is unresolved");
            self.ready.insert(cid, res);
        }
        self.requested.remove(&next);
        match self
            .ready
            .remove(&next)
            .expect("present per the loop guard")
        {
            Ok(block) => Ok(block),
            Err(_) => self.request(next).await.1,
        }
    }
}
