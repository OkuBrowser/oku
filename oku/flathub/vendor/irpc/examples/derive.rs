use std::{
    collections::BTreeMap,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
};

use anyhow::{Context, Result};
use irpc::{
    Client, WithChannels,
    channel::{mpsc, oneshot},
    rpc::RemoteService,
    rpc_requests,
    util::{make_client_endpoint, make_server_endpoint},
};
// Import the macro
use n0_future::task::{self, AbortOnDropHandle};
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Serialize, Deserialize)]
struct Get {
    key: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct List;

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

#[derive(Debug, Serialize, Deserialize)]
struct SetMany;

// Use the macro to generate both the StorageProtocol and StorageMessage enums
// plus implement Channels for each type
#[rpc_requests(message = StorageMessage)]
#[derive(Serialize, Deserialize, Debug)]
enum StorageProtocol {
    #[rpc(tx=oneshot::Sender<Option<String>>)]
    Get(Get),
    #[rpc(tx=oneshot::Sender<()>)]
    Set(Set),
    #[rpc(tx=oneshot::Sender<u64>, rx=mpsc::Receiver<(String, String)>)]
    SetMany(SetMany),
    #[rpc(tx=mpsc::Sender<String>)]
    List(List),
}

struct StorageActor {
    recv: tokio::sync::mpsc::Receiver<StorageMessage>,
    state: BTreeMap<String, String>,
}

impl StorageActor {
    pub fn spawn() -> StorageApi {
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let actor = Self {
            recv: rx,
            state: BTreeMap::new(),
        };
        n0_future::task::spawn(actor.run());
        StorageApi {
            inner: Client::local(tx),
        }
    }

    async fn run(mut self) {
        while let Some(msg) = self.recv.recv().await {
            self.handle(msg).await;
        }
    }

    async fn handle(&mut self, msg: StorageMessage) {
        match msg {
            StorageMessage::Get(get) => {
                info!("get {:?}", get);
                let WithChannels { tx, inner, .. } = get;
                tx.send(self.state.get(&inner.key).cloned()).await.ok();
            }
            StorageMessage::Set(set) => {
                info!("set {:?}", set);
                let WithChannels { tx, inner, .. } = set;
                self.state.insert(inner.key, inner.value);
                tx.send(()).await.ok();
            }
            StorageMessage::SetMany(set) => {
                info!("set-many {:?}", set);
                let WithChannels { mut rx, tx, .. } = set;
                let mut count = 0;
                while let Ok(Some((key, value))) = rx.recv().await {
                    self.state.insert(key, value);
                    count += 1;
                }
                tx.send(count).await.ok();
            }
            StorageMessage::List(list) => {
                info!("list {:?}", list);
                let WithChannels { tx, .. } = list;
                for (key, value) in &self.state {
                    if tx.send(format!("{key}={value}")).await.is_err() {
                        break;
                    }
                }
            }
        }
    }
}

struct StorageApi {
    inner: Client<StorageProtocol>,
}

impl StorageApi {
    pub fn connect(endpoint: noq::Endpoint, addr: SocketAddr) -> Result<StorageApi> {
        Ok(StorageApi {
            inner: Client::noq(endpoint, addr),
        })
    }

    pub fn listen(&self, endpoint: noq::Endpoint) -> Result<AbortOnDropHandle<()>> {
        let local = self
            .inner
            .as_local()
            .context("cannot listen on remote API")?;
        let join_handle = task::spawn(irpc::rpc::listen(
            endpoint,
            StorageProtocol::remote_handler(local),
        ));
        Ok(AbortOnDropHandle::new(join_handle))
    }

    pub async fn get(&self, key: String) -> irpc::Result<Option<String>> {
        self.inner.rpc(Get { key }).await
    }

    pub async fn list(&self) -> irpc::Result<mpsc::Receiver<String>> {
        self.inner.server_streaming(List, 16).await
    }

    pub async fn set(&self, key: String, value: String) -> irpc::Result<()> {
        self.inner.rpc(Set { key, value }).await
    }

    pub async fn set_many(
        &self,
    ) -> irpc::Result<(mpsc::Sender<(String, String)>, oneshot::Receiver<u64>)> {
        self.inner.client_streaming(SetMany, 4).await
    }
}

async fn client_demo(api: StorageApi) -> Result<()> {
    api.set("hello".to_string(), "world".to_string()).await?;
    let value = api.get("hello".to_string()).await?;
    println!("get: hello = {value:?}");

    let (tx, rx) = api.set_many().await?;
    for i in 0..3 {
        tx.send((format!("key{i}"), format!("value{i}"))).await?;
    }
    drop(tx);
    let count = rx.await?;
    println!("set-many: {count} values set");

    let mut list = api.list().await?;
    while let Some(value) = list.recv().await? {
        println!("list value = {value:?}");
    }
    Ok(())
}

async fn local() -> Result<()> {
    let api = StorageActor::spawn();
    client_demo(api).await?;
    Ok(())
}

async fn remote() -> Result<()> {
    let port = 10113;
    let addr: SocketAddr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, port).into();

    let (server_handle, cert) = {
        let (endpoint, cert) = make_server_endpoint(addr)?;
        let api = StorageActor::spawn();
        let handle = api.listen(endpoint)?;
        (handle, cert)
    };

    let endpoint =
        make_client_endpoint(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0).into(), &[&cert])?;
    let api = StorageApi::connect(endpoint, addr)?;
    client_demo(api).await?;

    drop(server_handle);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    println!("Local use");
    local().await?;
    println!("Remote use");
    remote().await.unwrap();
    Ok(())
}
