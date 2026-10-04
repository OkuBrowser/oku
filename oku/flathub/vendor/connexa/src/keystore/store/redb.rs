use crate::keystore::{EncryptedEntry, Error, KeyMetadata, Keystore, Result};
use redb::WriteTransaction;
use redb::{Database, ReadableTable, TableDefinition, TableError, backends::InMemoryBackend};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::OnceCell;

const TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("keys");

/// redb [`Keystore`] backend.
pub struct RedbKeystore {
    path: Option<PathBuf>,
    db: OnceCell<Arc<Database>>,
}

impl RedbKeystore {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: Some(path.as_ref().to_path_buf()),
            db: OnceCell::new(),
        }
    }

    async fn db(&self) -> Result<Arc<Database>> {
        let db = self
            .db
            .get_or_try_init(|| async {
                let path = self.path.clone();
                let database = tokio::task::spawn_blocking(move || match path {
                    Some(path) => Database::create(path),
                    None => Database::builder().create_with_backend(InMemoryBackend::new()),
                })
                .await
                .map_err(backend)?
                .map_err(open_error)?;
                Ok::<_, Error>(Arc::new(database))
            })
            .await?;
        Ok(db.clone())
    }
}

impl Keystore for RedbKeystore {
    async fn put(&self, entry: EncryptedEntry) -> Result<()> {
        let db = self.db().await?;
        let bytes = cbor4ii::serde::to_vec(Vec::new(), &entry).map_err(backend)?;
        let label = entry.metadata.label;
        tokio::task::spawn_blocking(move || -> Result<()> {
            let tx = db.begin_write().map_err(backend)?;
            {
                let mut table = tx.open_table(TABLE).map_err(backend)?;
                table
                    .insert(label.as_str(), bytes.as_slice())
                    .map_err(backend)?;
            }
            tx.commit().map_err(backend)?;
            Ok(())
        })
        .await
        .map_err(backend)?
    }

    async fn put_many(&self, entries: Vec<EncryptedEntry>) -> Result<()> {
        let db = self.db().await?;
        tokio::task::spawn_blocking(move || -> Result<()> {
            let tx = db.begin_write().map_err(backend)?;
            let tx_fn = |tx: &WriteTransaction, entries: Vec<EncryptedEntry>| -> Result<()> {
                let mut table = tx.open_table(TABLE).map_err(backend)?;
                for entry in entries {
                    let bytes = cbor4ii::serde::to_vec(Vec::new(), &entry).map_err(backend)?;
                    table
                        .insert(entry.metadata.label.as_str(), bytes.as_slice())
                        .map_err(backend)?;
                }
                Ok(())
            };

            if let Err(e) = tx_fn(&tx, entries) {
                if let Err(abort) = tx.abort() {
                    return Err(backend(std::io::Error::other(format!(
                        "{e}. Abort also failed with {abort}",
                    ))));
                }
                return Err(e);
            }

            tx.commit().map_err(backend)?;
            Ok(())
        })
        .await
        .map_err(backend)?
    }

    async fn get(&self, label: &str) -> Result<Option<EncryptedEntry>> {
        let db = self.db().await?;
        let label = label.to_owned();
        tokio::task::spawn_blocking(move || -> Result<Option<EncryptedEntry>> {
            let tx = db.begin_read().map_err(backend)?;
            let table = match tx.open_table(TABLE) {
                Ok(table) => table,
                Err(TableError::TableDoesNotExist(_)) => return Ok(None),
                Err(e) => return Err(backend(e)),
            };
            match table.get(label.as_str()).map_err(backend)? {
                Some(value) => Ok(Some(
                    cbor4ii::serde::from_slice(value.value()).map_err(backend)?,
                )),
                None => Ok(None),
            }
        })
        .await
        .map_err(backend)?
    }

    async fn list(&self) -> Result<Vec<KeyMetadata>> {
        let db = self.db().await?;
        tokio::task::spawn_blocking(move || -> Result<Vec<KeyMetadata>> {
            let tx = db.begin_read().map_err(backend)?;
            let table = match tx.open_table(TABLE) {
                Ok(table) => table,
                Err(TableError::TableDoesNotExist(_)) => return Ok(Vec::new()),
                Err(e) => return Err(backend(e)),
            };
            let mut metadata = Vec::new();
            for entry in table.iter().map_err(backend)? {
                let (label, value) = entry.map_err(backend)?;
                let decoded =
                    cbor4ii::serde::from_slice::<EncryptedEntry>(value.value()).map_err(backend)?;
                if decoded.metadata.label != label.value() {
                    return Err(backend(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "key entry label does not match its database key",
                    )));
                }
                metadata.push(decoded.metadata);
            }
            Ok(metadata)
        })
        .await
        .map_err(backend)?
    }

    async fn remove(&self, label: &str) -> Result<bool> {
        let db = self.db().await?;
        let label = label.to_owned();
        tokio::task::spawn_blocking(move || -> Result<bool> {
            let tx = db.begin_write().map_err(backend)?;
            let existed = {
                let mut table = tx.open_table(TABLE).map_err(backend)?;
                table.remove(label.as_str()).map_err(backend)?.is_some()
            };
            tx.commit().map_err(backend)?;
            Ok(existed)
        })
        .await
        .map_err(backend)?
    }
}

fn backend<E: Into<Box<dyn std::error::Error + Send + Sync>>>(err: E) -> Error {
    Error::Backend(std::io::Error::other(err))
}

fn open_error(err: redb::DatabaseError) -> Error {
    let kind = match &err {
        redb::DatabaseError::DatabaseAlreadyOpen => std::io::ErrorKind::ResourceBusy,
        _ => std::io::ErrorKind::Other,
    };
    Error::Backend(std::io::Error::new(kind, err))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "ed25519")]
    #[tokio::test]
    async fn persists_across_instances() {
        use crate::keystore::{Keychain, generate_key};
        use libp2p::identity::Keypair;

        let path = std::env::temp_dir().join(format!("connexa-redb-{}.redb", std::process::id()));
        let _ = tokio::fs::remove_file(&path).await;
        let key = generate_key();
        let keypair = Keypair::generate_ed25519();

        Keychain::new_with_store(key, RedbKeystore::new(&path))
            .insert("identity", &keypair)
            .await
            .unwrap();

        let reopened = Keychain::new_with_store(key, RedbKeystore::new(&path));
        let recovered = reopened.get("identity").await.unwrap();
        assert_eq!(
            recovered.public().to_peer_id(),
            keypair.public().to_peer_id()
        );
        assert_eq!(reopened.list().await.unwrap().len(), 1);
        assert!(reopened.remove("identity").await.unwrap());
        assert!(matches!(
            reopened.get("identity").await,
            Err(Error::NotFound(_))
        ));

        let _ = tokio::fs::remove_file(&path).await;
    }
}
