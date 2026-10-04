use crate::keystore::Error;
use crate::keystore::{EncryptedEntry, KeyMetadata, Keystore};
use std::collections::HashMap;
use tokio::sync::RwLock;

/// In-memory [`Keystore`] backend. Entries live only for the lifetime of the process.
#[derive(Debug, Default)]
pub struct MemoryKeystore {
    inner: RwLock<HashMap<String, EncryptedEntry>>,
}

impl Keystore for MemoryKeystore {
    async fn put(&self, entry: EncryptedEntry) -> Result<(), Error> {
        self.inner
            .write()
            .await
            .insert(entry.metadata.label.clone(), entry);
        Ok(())
    }

    async fn put_many(&self, entries: Vec<EncryptedEntry>) -> Result<(), Error> {
        let mut inner = self.inner.write().await;
        for entry in entries {
            inner.insert(entry.metadata.label.clone(), entry);
        }
        Ok(())
    }

    async fn get(&self, label: &str) -> Result<Option<EncryptedEntry>, Error> {
        Ok(self.inner.read().await.get(label).cloned())
    }

    async fn list(&self) -> Result<Vec<KeyMetadata>, Error> {
        Ok(self
            .inner
            .read()
            .await
            .values()
            .map(|entry| entry.metadata.clone())
            .collect())
    }

    async fn remove(&self, label: &str) -> Result<bool, Error> {
        Ok(self.inner.write().await.remove(label).is_some())
    }
}
