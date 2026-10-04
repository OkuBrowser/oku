//! This demonstrates using irpc with the derive macro but without the rpc feature
//! for local-only use. Run with:
//! ```
//! cargo run --example local --no-default-features --features derive
//! ```

use std::collections::BTreeMap;

use irpc::{Client, WithChannels, channel::oneshot, rpc_requests};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Get {
    key: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Set {
    key: String,
    value: String,
}

impl From<(String, String)> for Set {
    fn from((key, value): (String, String)) -> Self {
        Self { key, value }
    }
}

#[rpc_requests(message = StorageMessage, no_rpc, no_spans)]
#[derive(Serialize, Deserialize, Debug)]
enum StorageProtocol {
    #[rpc(tx=oneshot::Sender<Option<String>>)]
    Get(Get),
    #[rpc(tx=oneshot::Sender<()>)]
    Set(Set),
}

struct Actor {
    recv: tokio::sync::mpsc::Receiver<StorageMessage>,
    state: BTreeMap<String, String>,
}

impl Actor {
    async fn run(mut self) {
        while let Some(msg) = self.recv.recv().await {
            self.handle(msg).await;
        }
    }

    async fn handle(&mut self, msg: StorageMessage) {
        match msg {
            StorageMessage::Get(get) => {
                let WithChannels { tx, inner, .. } = get;
                tx.send(self.state.get(&inner.key).cloned()).await.ok();
            }
            StorageMessage::Set(set) => {
                let WithChannels { tx, inner, .. } = set;
                self.state.insert(inner.key, inner.value);
                tx.send(()).await.ok();
            }
        }
    }
}

struct StorageApi {
    inner: Client<StorageProtocol>,
}

impl StorageApi {
    pub fn spawn() -> StorageApi {
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let actor = Actor {
            recv: rx,
            state: BTreeMap::new(),
        };
        n0_future::task::spawn(actor.run());
        StorageApi {
            inner: Client::local(tx),
        }
    }

    pub async fn get(&self, key: String) -> irpc::Result<Option<String>> {
        self.inner.rpc(Get { key }).await
    }

    pub async fn set(&self, key: String, value: String) -> irpc::Result<()> {
        self.inner.rpc(Set { key, value }).await
    }
}

#[tokio::main]
async fn main() -> irpc::Result<()> {
    tracing_subscriber::fmt::init();
    let api = StorageApi::spawn();
    api.set("hello".to_string(), "world".to_string()).await?;
    let value = api.get("hello".to_string()).await?;
    println!("get: hello = {value:?}");
    Ok(())
}
