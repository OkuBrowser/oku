#![allow(unused)]

use std::{
    ops::Deref,
    path::{Path, PathBuf},
};

use iroh::{
    endpoint::{presets, BindError},
    test_utils::DnsPkarrServer,
    tls::CaTlsConfig,
    Endpoint, EndpointId, RelayMap, RelayMode, SecretKey,
};
use iroh_blobs::store::GcConfig;
use iroh_docs::{engine::ProtectCallbackHandler, protocol::Docs};
use iroh_gossip::net::Gossip;
use n0_error::Result;

pub async fn empty_endpoint() -> Result<Endpoint, BindError> {
    Endpoint::bind(presets::Minimal).await
}

pub async fn endpoint(
    secret_key: SecretKey,
    relay_map: RelayMap,
    dns_pkarr_server: Option<&DnsPkarrServer>,
) -> Result<Endpoint, BindError> {
    let mut builder = Endpoint::builder(presets::Minimal);
    if let Some(dns_pkarr_server) = dns_pkarr_server {
        builder = builder.preset(dns_pkarr_server.preset());
    }

    builder
        .secret_key(secret_key)
        .relay_mode(RelayMode::Custom(relay_map))
        .ca_tls_config(CaTlsConfig::insecure_skip_verify())
        .bind()
        .await
}

/// An iroh node that just has the blobs transport
#[derive(Debug)]
pub struct Node {
    router: iroh::protocol::Router,
    client: Client,
}

impl Deref for Node {
    type Target = Client;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

#[derive(Debug, Clone)]
pub struct Client {
    blobs: iroh_blobs::api::Store,
    docs: iroh_docs::api::DocsApi,
}

impl Client {
    fn new(blobs: iroh_blobs::api::Store, docs: iroh_docs::api::DocsApi) -> Self {
        Self { blobs, docs }
    }

    pub fn blobs(&self) -> &iroh_blobs::api::Store {
        &self.blobs
    }

    pub fn docs(&self) -> &iroh_docs::api::DocsApi {
        &self.docs
    }
}

/// An iroh node builder
#[derive(derive_more::Debug)]
pub struct Builder {
    endpoint: iroh::Endpoint,
    storage: Storage,
    gc_interval: Option<n0_future::time::Duration>,
    #[debug(skip)]
    register_gc_done_cb: Option<Box<dyn Fn() + Send + 'static>>,
}

impl Builder {
    /// Spawns the node
    async fn spawn0(
        self,
        blobs: iroh_blobs::api::Store,
        protect_cb: Option<ProtectCallbackHandler>,
    ) -> anyhow::Result<Node> {
        let mut router = iroh::protocol::Router::builder(self.endpoint.clone());
        let gossip = Gossip::builder().spawn(self.endpoint.clone());
        let mut docs_builder = match self.storage {
            Storage::Memory => Docs::memory(),
            #[cfg(feature = "fs-store")]
            Storage::Persistent(ref path) => Docs::persistent(path.to_path_buf()),
        };
        if let Some(protect_cb) = protect_cb {
            docs_builder = docs_builder.protect_handler(protect_cb);
        }
        let docs = match docs_builder
            .spawn(self.endpoint.clone(), blobs.clone(), gossip.clone())
            .await
        {
            Ok(docs) => docs,
            Err(err) => {
                blobs.shutdown().await.ok();
                return Err(err);
            }
        };
        router = router.accept(
            iroh_blobs::ALPN,
            iroh_blobs::BlobsProtocol::new(&blobs, None),
        );
        router = router.accept(iroh_docs::ALPN, docs.clone());
        router = router.accept(iroh_gossip::ALPN, gossip.clone());

        // Build the router
        let router = router.spawn();

        let client = Client::new(blobs.clone(), docs.api().clone());
        Ok(Node { router, client })
    }

    pub fn gc_interval(mut self, value: Option<n0_future::time::Duration>) -> Self {
        self.gc_interval = value;
        self
    }

    pub fn register_gc_done_cb(mut self, value: Box<dyn Fn() + Send + Sync>) -> Self {
        self.register_gc_done_cb = Some(value);
        self
    }

    fn new(storage: Storage, endpoint: Endpoint) -> Self {
        Self {
            endpoint,
            storage,
            gc_interval: None,
            register_gc_done_cb: None,
        }
    }
}

#[derive(Debug)]
enum Storage {
    Memory,
    #[cfg(feature = "fs-store")]
    Persistent(PathBuf),
}

impl Node {
    /// Creates a new node with memory storage
    pub fn memory(endpoint: Endpoint) -> Builder {
        Builder::new(Storage::Memory, endpoint)
    }

    /// Creates a new node with persistent storage
    #[cfg(feature = "fs-store")]
    pub fn persistent(path: impl AsRef<Path>, endpoint: Endpoint) -> Builder {
        Builder::new(Storage::Persistent(path.as_ref().to_owned()), endpoint)
    }
}

impl Builder {
    /// Spawns the node
    pub async fn spawn(self) -> anyhow::Result<Node> {
        let (store, protect_handler) = match self.storage {
            Storage::Memory => {
                let store = iroh_blobs::store::mem::MemStore::new();
                ((*store).clone(), None)
            }
            #[cfg(feature = "fs-store")]
            Storage::Persistent(ref path) => {
                let db_path = path.join("blobs.db");
                let mut opts = iroh_blobs::store::fs::options::Options::new(path);
                let protect_handler = if let Some(interval) = self.gc_interval {
                    let (handler, cb) = ProtectCallbackHandler::new();
                    opts.gc = Some(GcConfig {
                        interval,
                        add_protected: Some(cb),
                    });
                    Some(handler)
                } else {
                    None
                };
                let store = iroh_blobs::store::fs::FsStore::load_with_opts(db_path, opts).await?;
                ((*store).clone(), protect_handler)
            }
        };
        self.spawn0(store, protect_handler).await
    }
}

impl Node {
    /// Returns the node id
    pub fn id(&self) -> EndpointId {
        self.router.endpoint().id()
    }

    /// Ensure the node is "online", aka, is connected to a relay and
    /// has a direct addresses
    pub async fn online(&self) {
        self.router.endpoint().online().await
    }

    /// Shuts down the node
    pub async fn shutdown(self) -> anyhow::Result<()> {
        self.router.shutdown().await?;
        Ok(())
    }

    /// Returns the client
    pub fn client(&self) -> &Client {
        &self.client
    }
}

pub mod path {
    use std::path::{Component, Path, PathBuf};

    use anyhow::Context;
    use bytes::Bytes;

    /// Helper function that translates a key that was derived from the [`path_to_key`] function back
    /// into a path.
    ///
    /// If `prefix` exists, it will be stripped before converting back to a path
    /// If `root` exists, will add the root as a parent to the created path
    /// Removes any null byte that has been appended to the key
    pub fn key_to_path(
        key: impl AsRef<[u8]>,
        prefix: Option<String>,
        root: Option<PathBuf>,
    ) -> anyhow::Result<PathBuf> {
        let mut key = key.as_ref();
        if key.is_empty() {
            return Ok(PathBuf::new());
        }
        // if the last element is the null byte, remove it
        if b'\0' == key[key.len() - 1] {
            key = &key[..key.len() - 1]
        }

        let key = if let Some(prefix) = prefix {
            let prefix = prefix.into_bytes();
            if prefix[..] == key[..prefix.len()] {
                &key[prefix.len()..]
            } else {
                anyhow::bail!("key {:?} does not begin with prefix {:?}", key, prefix);
            }
        } else {
            key
        };

        let mut path = if key[0] == b'/' {
            PathBuf::from("/")
        } else {
            PathBuf::new()
        };
        for component in key
            .split(|c| c == &b'/')
            .map(|c| String::from_utf8(c.into()).context("key contains invalid data"))
        {
            let component = component?;
            path = path.join(component);
        }

        // add root if it exists
        let path = if let Some(root) = root {
            root.join(path)
        } else {
            path
        };

        Ok(path)
    }

    /// Helper function that creates a document key from a canonicalized path, removing the `root` and adding the `prefix`, if they exist
    ///
    /// Appends the null byte to the end of the key.
    pub fn path_to_key(
        path: impl AsRef<Path>,
        prefix: Option<String>,
        root: Option<PathBuf>,
    ) -> anyhow::Result<Bytes> {
        let path = path.as_ref();
        let path = if let Some(root) = root {
            path.strip_prefix(root)?
        } else {
            path
        };
        let suffix = canonicalized_path_to_string(path, false)?.into_bytes();
        let mut key = if let Some(prefix) = prefix {
            prefix.into_bytes().to_vec()
        } else {
            Vec::new()
        };
        key.extend(suffix);
        key.push(b'\0');
        Ok(key.into())
    }

    /// This function converts an already canonicalized path to a string.
    ///
    /// If `must_be_relative` is true, the function will fail if any component of the path is
    /// `Component::RootDir`
    ///
    /// This function will also fail if the path is non canonical, i.e. contains
    /// `..` or `.`, or if the path components contain any windows or unix path
    /// separators.
    pub fn canonicalized_path_to_string(
        path: impl AsRef<Path>,
        must_be_relative: bool,
    ) -> anyhow::Result<String> {
        let mut path_str = String::new();
        let parts = path
            .as_ref()
            .components()
            .filter_map(|c| match c {
                Component::Normal(x) => {
                    let c = match x.to_str() {
                        Some(c) => c,
                        None => return Some(Err(anyhow::anyhow!("invalid character in path"))),
                    };

                    if !c.contains('/') && !c.contains('\\') {
                        Some(Ok(c))
                    } else {
                        Some(Err(anyhow::anyhow!("invalid path component {:?}", c)))
                    }
                }
                Component::RootDir => {
                    if must_be_relative {
                        Some(Err(anyhow::anyhow!("invalid path component {:?}", c)))
                    } else {
                        path_str.push('/');
                        None
                    }
                }
                _ => Some(Err(anyhow::anyhow!("invalid path component {:?}", c))),
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        let parts = parts.join("/");
        path_str.push_str(&parts);
        Ok(path_str)
    }
}
