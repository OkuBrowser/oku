use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use futures::future::BoxFuture;
use futures::stream::FuturesUnordered;
use indexmap::IndexSet;
use ipld_core::cid::Cid;
use parking_lot::RwLock;
use serde::Deserialize;

use connexa::handle::Connexa;
use connexa::prelude::swarm::dial_opts::DialOpts;
use connexa::prelude::{Multiaddr, MultiaddrExt, PeerId};

use crate::IpfsEvent;
use crate::repo::{DefaultKeystore, DefaultStorage, Repo};

type IpfsConnexa = Connexa<IpfsEvent, DefaultKeystore>;

const DEFAULT_ROUTERS: &[&str] = &["https://delegated-ipfs.dev"];
const MAX_INFLIGHT: usize = 8;
const MAX_PROVIDERS: usize = 8;
const MAX_ADDRS_PER_RECORD: usize = 8;
const MAX_RESPONSE_SIZE: usize = 1024 * 1024;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Clone)]
pub struct RouterList {
    inner: Arc<RwLock<IndexSet<String>>>,
}

impl RouterList {
    pub(crate) fn new<I, S>(routers: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut set: IndexSet<String> = routers.into_iter().map(|r| normalize(&r.into())).collect();
        if set.is_empty() {
            set = DEFAULT_ROUTERS.iter().map(|r| r.to_string()).collect();
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

#[derive(Deserialize)]
struct ProvidersResponse {
    #[serde(rename = "Providers", default)]
    providers: Vec<ProviderRecord>,
}

#[derive(Deserialize)]
struct ProviderRecord {
    #[serde(rename = "ID")]
    id: Option<PeerId>,
    #[serde(rename = "Addrs", default)]
    addrs: Vec<Multiaddr>,
}

pub(crate) async fn run(
    mut rx: futures::channel::mpsc::Receiver<Cid>,
    connexa: IpfsConnexa,
    routers: RouterList,
    repo: Repo<DefaultStorage>,
) {
    let client = match reqwest::Client::builder().build() {
        Ok(client) => client,
        Err(error) => {
            error!(%error, "failed to build delegated routing http client. disabling delegated routing.");
            return;
        }
    };
    let mut inflight: HashSet<Cid> = HashSet::new();
    let mut lookups: FuturesUnordered<BoxFuture<'static, Cid>> = FuturesUnordered::new();

    loop {
        tokio::select! {
            maybe_cid = rx.next(), if lookups.len() < MAX_INFLIGHT => {
                let Some(cid) = maybe_cid else { break };
                if !repo.is_block_wanted(&cid) {
                    continue;
                }
                if !inflight.insert(cid) {
                    continue;
                }
                lookups.push(boxed_lookup(client.clone(), routers.list(), cid, connexa.clone()));
            }
            Some(cid) = lookups.next(), if !lookups.is_empty() => {
                inflight.remove(&cid);
            }
        }
    }
}

fn boxed_lookup(
    client: reqwest::Client,
    routers: Vec<String>,
    cid: Cid,
    connexa: IpfsConnexa,
) -> BoxFuture<'static, Cid> {
    let fut = async move {
        lookup_and_dial(&client, &routers, cid, &connexa).await;
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

async fn lookup_and_dial(
    client: &reqwest::Client,
    routers: &[String],
    cid: Cid,
    connexa: &IpfsConnexa,
) {
    for router in routers {
        let url = format!("{router}/routing/v1/providers/{cid}");
        let Ok(response) = client
            .get(&url)
            .header("Accept", "application/json")
            .timeout(REQUEST_TIMEOUT)
            .send()
            .await
        else {
            continue;
        };
        if !response.status().is_success() {
            continue;
        }
        let Some(bytes) = read_capped(response).await else {
            continue;
        };
        let Ok(body) = serde_json::from_slice::<ProvidersResponse>(&bytes) else {
            continue;
        };

        let mut dialed = 0usize;
        for record in body.providers {
            let Some(peer) = record.id else {
                continue;
            };
            // Note: a router is untrusted so only dial publicly routable addresses (never loopback/private
            // internal endpoints) and cap the count, so a hostile router can't turn a lookup into
            // an internal-network port scan or an unbounded dial fan-out.
            let addrs: Vec<Multiaddr> = record
                .addrs
                .iter()
                .filter(|&addr| addr.is_public())
                .take(MAX_ADDRS_PER_RECORD)
                .cloned()
                .collect();
            if addrs.is_empty() {
                continue;
            }
            let _ = connexa
                .swarm()
                .dial(DialOpts::peer_id(peer).addresses(addrs).build())
                .await;
            dialed += 1;
            if dialed >= MAX_PROVIDERS {
                break;
            }
        }

        if dialed > 0 {
            break;
        }
    }
}

async fn read_capped(response: reqwest::Response) -> Option<bytes::Bytes> {
    if response
        .content_length()
        .is_some_and(|len| len > MAX_RESPONSE_SIZE as u64)
    {
        return None;
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut response = response;
        let mut buffer = Vec::new();
        while let Some(chunk) = response.chunk().await.ok()? {
            if buffer.len() + chunk.len() > MAX_RESPONSE_SIZE {
                return None;
            }
            buffer.extend_from_slice(&chunk);
        }
        Some(bytes::Bytes::from(buffer))
    }

    #[cfg(target_arch = "wasm32")]
    {
        if response.content_length().is_none() {
            return None;
        }
        let bytes = response.bytes().await.ok()?;
        (bytes.len() <= MAX_RESPONSE_SIZE).then_some(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_providers_response() {
        let json = r#"{
            "Providers": [
                { "Schema": "peer", "ID": "12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN", "Addrs": ["/ip4/1.2.3.4/tcp/4001"] },
                { "Schema": "peer", "ID": "12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN" }
            ]
        }"#;
        let body: ProvidersResponse = serde_json::from_str(json).unwrap();
        assert_eq!(body.providers.len(), 2);
        assert_eq!(
            body.providers[0].addrs,
            vec!["/ip4/1.2.3.4/tcp/4001".parse::<Multiaddr>().unwrap()]
        );
        assert!(body.providers[1].addrs.is_empty());
    }

    #[test]
    fn router_list_dedups_and_defaults() {
        let list = RouterList::new(["https://r.example/", "https://r.example"]);
        assert_eq!(list.list(), vec!["https://r.example".to_string()]);
        assert_eq!(
            RouterList::new(Vec::<String>::new()).list(),
            DEFAULT_ROUTERS
                .iter()
                .map(|r| r.to_string())
                .collect::<Vec<_>>()
        );
    }
}
