use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use futures::StreamExt;
use futures::future::BoxFuture;
use futures::stream::FuturesUnordered;
use indexmap::IndexSet;
use ipld_core::cid::Cid;
use parking_lot::RwLock;

use crate::Block;
use crate::repo::{DefaultStorage, Repo};

const DEFAULT_GATEWAYS: &[&str] = &["https://trustless-gateway.link", "https://ipfs.io"];
const MAX_INFLIGHT: usize = 16;
const MAX_BLOCK_SIZE: usize = 2 * 1024 * 1024;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone)]
pub struct GatewayList {
    inner: Arc<RwLock<IndexSet<String>>>,
}

impl GatewayList {
    pub(crate) fn new<I, S>(gateways: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut set: IndexSet<String> =
            gateways.into_iter().map(|g| normalize(&g.into())).collect();
        if set.is_empty() {
            set = DEFAULT_GATEWAYS.iter().map(|g| g.to_string()).collect();
        }
        Self {
            inner: Arc::new(RwLock::new(set)),
        }
    }

    pub(crate) fn add(&self, url: &str) -> bool {
        self.inner.write().insert(normalize(url))
    }

    pub(crate) fn remove(&self, url: &str) -> bool {
        self.inner.write().shift_remove(&normalize(url))
    }

    pub(crate) fn list(&self) -> Vec<String> {
        self.inner.read().iter().cloned().collect()
    }
}

fn normalize(url: &str) -> String {
    url.trim_end_matches('/').to_string()
}

pub(crate) async fn run(
    mut rx: futures::channel::mpsc::Receiver<Cid>,
    repo: Repo<DefaultStorage>,
    gateways: GatewayList,
) {
    let client = match reqwest::Client::builder().build() {
        Ok(client) => client,
        Err(error) => {
            error!(%error, "failed to build gateway http client. disabling gateway retrieval.");
            return;
        }
    };
    let mut inflight: HashSet<Cid> = HashSet::new();
    let mut fetches: FuturesUnordered<BoxFuture<'static, Cid>> = FuturesUnordered::new();

    loop {
        tokio::select! {
            maybe_cid = rx.next(), if fetches.len() < MAX_INFLIGHT => {
                let Some(cid) = maybe_cid else { break };
                if !repo.is_block_wanted(&cid) {
                    continue;
                }
                if !inflight.insert(cid) {
                    continue;
                }
                fetches.push(boxed_fetch(client.clone(), gateways.list(), cid, repo.clone()));
            }
            Some(cid) = fetches.next(), if !fetches.is_empty() => {
                inflight.remove(&cid);
            }
        }
    }
}

fn boxed_fetch(
    client: reqwest::Client,
    gateways: Vec<String>,
    cid: Cid,
    repo: Repo<DefaultStorage>,
) -> BoxFuture<'static, Cid> {
    let fut = async move {
        fetch_and_store(&client, &gateways, cid, &repo).await;
        cid
    };
    #[cfg(target_arch = "wasm32")]
    {
        Box::pin(send_wrapper::SendWrapper::new(fut))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        Box::pin(fut)
    }
}

async fn fetch_and_store(
    client: &reqwest::Client,
    gateways: &[String],
    cid: Cid,
    repo: &Repo<DefaultStorage>,
) {
    if matches!(repo.get_block_now(cid).await, Ok(Some(_))) {
        return;
    }
    let Some(block) = fetch(client, gateways, cid).await else {
        return;
    };
    let _ = repo.put_block(&block).await;
}

async fn fetch(client: &reqwest::Client, gateways: &[String], cid: Cid) -> Option<Block> {
    let mut requests = gateways
        .iter()
        .map(|gateway| {
            let url = format!("{gateway}/ipfs/{cid}");
            let client = client.clone();
            async move {
                let response = client
                    .get(&url)
                    .header("Accept", "application/vnd.ipld.raw")
                    .timeout(REQUEST_TIMEOUT)
                    .send()
                    .await
                    .ok()?;
                if !response.status().is_success() {
                    return None;
                }
                let bytes = read_capped(response).await?;
                Block::new(cid, bytes).ok()
            }
        })
        .collect::<FuturesUnordered<_>>();

    while let Some(result) = requests.next().await {
        if let Some(block) = result {
            return Some(block);
        }
    }
    None
}

async fn read_capped(response: reqwest::Response) -> Option<Bytes> {
    if response
        .content_length()
        .is_some_and(|len| len > MAX_BLOCK_SIZE as u64)
    {
        return None;
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut response = response;
        let mut buffer = Vec::new();
        while let Some(chunk) = response.chunk().await.ok()? {
            if buffer.len() + chunk.len() > MAX_BLOCK_SIZE {
                return None;
            }
            buffer.extend_from_slice(&chunk);
        }
        Some(Bytes::from(buffer))
    }

    #[cfg(target_arch = "wasm32")]
    {
        if response.content_length().is_none() {
            return None;
        }
        let bytes = response.bytes().await.ok()?;
        (bytes.len() <= MAX_BLOCK_SIZE).then_some(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedups_and_normalizes_trailing_slash() {
        let list = GatewayList::new([
            "https://a.example/",
            "https://a.example",
            "https://b.example",
        ]);
        assert_eq!(
            list.list(),
            vec![
                "https://a.example".to_string(),
                "https://b.example".to_string()
            ]
        );
    }

    #[test]
    fn falls_back_to_defaults_when_empty() {
        let list = GatewayList::new(Vec::<String>::new());
        let expected: Vec<String> = DEFAULT_GATEWAYS.iter().map(|g| g.to_string()).collect();
        assert_eq!(list.list(), expected);
    }

    #[test]
    fn add_and_remove() {
        let list = GatewayList::new(["https://a.example"]);
        assert!(list.add("https://b.example/"));
        assert!(!list.add("https://b.example"));
        assert!(list.remove("https://a.example"));
        assert!(!list.remove("https://a.example"));
        assert_eq!(list.list(), vec!["https://b.example".to_string()]);
    }
}
