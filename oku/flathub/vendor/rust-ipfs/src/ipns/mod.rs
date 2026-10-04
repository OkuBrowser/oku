//! IPNS functionality around [`Ipfs`].

use futures_timeout::TimeoutExt;
use std::borrow::Borrow;

use crate::path::{IpfsPath, PathRoot};
use crate::repo::DataStore;
use crate::{Ipfs, PeerId, RecordKey};

#[cfg(feature = "dns")]
mod dnslink;

#[cfg(feature = "dns")]
use connexa::prelude::transport::dns::DnsResolver;

/// IPNS facade around [`Ipns`].
#[derive(Clone, Debug)]
pub struct Ipns {
    ipfs: Ipfs,
    #[cfg(feature = "dns")]
    resolver: DnsResolver,
}

#[derive(Clone, Copy, Debug, Default)]
pub enum IpnsOption {
    Local,
    #[default]
    DHT,
}

fn ipns_dht_key(peer: &PeerId) -> RecordKey {
    let mut key = b"/ipns/".to_vec();
    key.extend_from_slice(&peer.to_bytes());
    RecordKey::from(key)
}

impl Ipns {
    pub fn new(ipfs: Ipfs) -> Self {
        Ipns {
            ipfs,
            #[cfg(feature = "dns")]
            resolver: DnsResolver::default(),
        }
    }

    /// Set dns resolver
    #[cfg(feature = "dns")]
    pub fn set_resolver(&mut self, resolver: DnsResolver) {
        self.resolver = resolver;
    }

    /// Resolves a ipns path to an ipld path.
    // TODO: Implement ipns pubsub
    // TODO: Maybe implement a check to the dht store itself too?
    pub async fn resolve(&self, path: impl Borrow<IpfsPath>) -> Result<IpfsPath, IpnsError> {
        let path = path.borrow();
        match path.root() {
            PathRoot::Ipld(_) => Ok(path.clone()),
            PathRoot::Ipns(peer) => {
                use std::str::FromStr;
                use std::time::Duration;

                use futures::StreamExt;
                use ipld_core::cid::Cid;
                use multihash::Multihash;

                let mut path_iter = path.iter();

                let hash = Multihash::from_bytes(&peer.to_bytes()).map_err(anyhow::Error::from)?;

                let cid = Cid::new_v1(0x72, hash);

                let mb = format!(
                    "/ipns/{}",
                    cid.to_string_of_base(multibase::Base::Base36Lower)
                        .map_err(anyhow::Error::from)?
                );

                let repo = self.ipfs.repo();
                let datastore = repo.data_store();

                if let Ok(Some(data)) = datastore.get(mb.as_bytes()).await
                    && let Ok(path) = rust_ipns::Record::decode(data)
                        .map_err(std::io::Error::from)
                        .and_then(|record| {
                            //Although stored locally, we should verify the record anyway
                            record.verify(*peer)?;
                            let data = record.data()?;
                            let path = String::from_utf8_lossy(data.value());
                            IpfsPath::from_str(&path)
                                .and_then(|mut internal_path| {
                                    internal_path.path.push_split(path_iter.by_ref()).map_err(
                                        |_| {
                                            crate::path::IpfsPathError::InvalidPath(
                                                path.to_string(),
                                            )
                                        },
                                    )?;
                                    Ok(internal_path)
                                })
                                .map_err(|e| {
                                    std::io::Error::new(std::io::ErrorKind::InvalidData, e)
                                })
                        })
                {
                    return Ok(path);
                }

                let key = ipns_dht_key(peer);
                let stream = self.ipfs.dht_get(key.clone()).await?;

                //TODO: Implement configurable timeout
                let records = stream
                    .filter(|record| futures::future::ready(record.key == key))
                    .filter_map(|record| async move {
                        let record = rust_ipns::Record::decode(&record.value).ok()?;
                        record.verify(*peer).ok()?;
                        Some(record)
                    })
                    .collect::<Vec<_>>()
                    .timeout(Duration::from_secs(60 * 2))
                    .await
                    .unwrap_or_default();

                let record = records
                    .iter()
                    .max_by(|a, b| a.compare(b).unwrap_or(std::cmp::Ordering::Equal))
                    .ok_or_else(|| anyhow::anyhow!("No records found"))?;

                let data = record.data()?;

                let path = String::from_utf8_lossy(data.value()).to_string();

                IpfsPath::from_str(&path)
                    .map_err(IpnsError::from)
                    .and_then(|mut internal_path| {
                        internal_path
                            .path
                            .push_split(path_iter)
                            .map_err(|_| crate::path::IpfsPathError::InvalidPath(path))?;
                        Ok(internal_path)
                    })
            }
            #[cfg(feature = "dns")]
            PathRoot::Dns(domain) => {
                let path_iter = path.iter();
                dnslink::resolve(self.resolver, domain, path_iter)
                    .await
                    .map_err(IpnsError::from)
            }
        }
    }

    pub async fn publish(
        &self,
        key: Option<&str>,
        path: impl Borrow<IpfsPath>,
        option: IpnsOption,
    ) -> Result<IpfsPath, IpnsError> {
        use connexa::prelude::dht::Quorum;
        use ipld_core::cid::Cid;
        use multihash::Multihash;
        use std::str::FromStr;

        let path = path.borrow();

        let keypair = match key {
            Some(key) => self
                .ipfs
                .keychain()
                .get(key)
                .await
                .map_err(anyhow::Error::from)?,
            None => self.ipfs.keypair().clone(),
        };

        let peer_id = keypair.public().to_peer_id();

        let hash = Multihash::from_bytes(&peer_id.to_bytes()).map_err(anyhow::Error::from)?;

        let cid = Cid::new_v1(0x72, hash);

        let mb = format!(
            "/ipns/{}",
            cid.to_string_of_base(multibase::Base::Base36Lower)
                .map_err(anyhow::Error::from)?
        );

        let repo = self.ipfs.repo();
        let _publish_guard = repo.ipns_publish_guard(&peer_id).await;
        let datastore = repo.data_store();

        let record_data = datastore.get(mb.as_bytes()).await?;
        let path_bytes = path.to_string();
        let mut seq = 0;
        let mut validity = chrono::Utc::now() + chrono::Duration::hours(48);

        if let Some(record) = record_data.as_ref() {
            let record = rust_ipns::Record::decode(record)?;
            // Verify the previous record even when it has expired.
            record.verify_signature(peer_id)?;
            validity = validity.max(record.validity()?.with_timezone(&chrono::Utc));
            seq = record.sequence();

            if record.value() != path_bytes.as_bytes() {
                seq = seq
                    .checked_add(1)
                    .ok_or_else(|| anyhow::anyhow!("IPNS sequence exhausted"))?;
            }
        }

        let record = rust_ipns::Record::new(
            &keypair,
            path_bytes.as_bytes(),
            validity,
            seq,
            // IPNS TTL is a caching hint of 60 seconds.
            std::time::Duration::from_secs(60),
        )?;

        let bytes = record.encode()?;

        datastore.put(mb.as_bytes(), &bytes).await?;

        match option {
            IpnsOption::DHT => {
                self.ipfs
                    .dht_put_record(ipns_dht_key(&peer_id), bytes, Quorum::One)
                    .await?;
            }
            IpnsOption::Local => {}
        };

        IpfsPath::from_str(&mb).map_err(IpnsError::from)
    }
}

#[non_exhaustive]
#[derive(thiserror::Error, Debug)]
pub enum IpnsError {
    #[error(transparent)]
    IpfsPath(#[from] crate::path::IpfsPathError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Ipns(#[from] rust_ipns::Error),
    #[error(transparent)]
    Any(#[from] anyhow::Error),
}
