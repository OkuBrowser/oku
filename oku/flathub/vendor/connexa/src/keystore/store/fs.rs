use crate::keystore::{EncryptedEntry, Error, KeyMetadata, Keystore, Result};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

const JOURNAL: &str = "connexa-journal";
const COMMITTED: &str = "connexa-committed";
const STAGING: &str = "connexa-staging";

// The journal stores encrypted entries.
#[derive(serde::Serialize, serde::Deserialize)]
struct Journal {
    version: u32,
    originals: Vec<(String, Option<Vec<u8>>)>,
}

/// Filesystem keystore.
#[derive(Debug, Clone)]
pub struct FilesystemKeystore {
    dir: PathBuf,
}

impl FilesystemKeystore {
    /// Use `dir` for storage. Create it on the first write if needed.
    pub fn new(dir: impl AsRef<Path>) -> Self {
        Self {
            dir: dir.as_ref().to_path_buf(),
        }
    }

    fn entry_path(&self, label: &str) -> Result<PathBuf> {
        crate::keystore::validate_label(label)?;
        Ok(self.dir.join(label))
    }

    fn work_dir(&self) -> PathBuf {
        self.dir.join(".tmp")
    }

    // The job keeps the file lock even if the caller is cancelled.
    async fn access<T: Send + 'static>(
        &self,
        create: bool,
        missing: T,
        operation: impl FnOnce(&Self) -> Result<T> + Send + 'static,
    ) -> Result<T> {
        let store = self.clone();
        tokio::task::spawn_blocking(move || {
            if !create && !store.dir.try_exists().map_err(Error::Backend)? {
                return Ok(missing);
            }
            let _lock = store.lock()?;
            store.recover().map_err(Error::Backend)?;
            operation(&store)
        })
        .await
        .map_err(backend)?
    }

    fn lock(&self) -> Result<File> {
        create_dir(&self.work_dir()).map_err(Error::Backend)?;
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let file = options
            .open(self.work_dir().join("connexa-lock"))
            .map_err(Error::Backend)?;
        file.lock().map_err(Error::Backend)?;
        Ok(file)
    }

    fn prepare(&self, entries: Vec<EncryptedEntry>) -> Result<Vec<String>> {
        // Use the last entry when a label appears more than once.
        let mut encoded = BTreeMap::new();
        for entry in entries {
            let path = self.entry_path(&entry.metadata.label)?;
            if let Ok(metadata) = fs::symlink_metadata(&path)
                && !metadata.is_file()
            {
                return Err(Error::Backend(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "key entry must be a regular file",
                )));
            }
            let bytes = cbor4ii::serde::to_vec(Vec::new(), &entry).map_err(backend)?;
            encoded.insert(entry.metadata.label, bytes);
        }
        let work = self.work_dir();
        let staging = work.join(STAGING);
        create_dir(&staging).map_err(Error::Backend)?;
        let mut originals = Vec::with_capacity(encoded.len());
        let mut labels = Vec::with_capacity(encoded.len());
        for (index, (label, bytes)) in encoded.into_iter().enumerate() {
            let old = match fs::read(self.dir.join(&label)) {
                Ok(bytes) => Some(bytes),
                Err(e) if e.kind() == io::ErrorKind::NotFound => None,
                Err(e) => return Err(Error::Backend(e)),
            };
            write_new(&staging.join(index.to_string()), &bytes).map_err(Error::Backend)?;
            originals.push((label.clone(), old));
            labels.push(label);
        }
        sync_dir(&staging).map_err(Error::Backend)?;
        let bytes = cbor4ii::serde::to_vec(
            Vec::new(),
            &Journal {
                version: 1,
                originals,
            },
        )
        .map_err(backend)?;
        write_new(&work.join("connexa-journal-new"), &bytes).map_err(Error::Backend)?;
        rename_file(work.join("connexa-journal-new"), work.join(JOURNAL))
            .map_err(Error::Backend)?;
        // Flush the journal before replacing any entries.
        sync_dir(&work).map_err(Error::Backend)?;
        Ok(labels)
    }

    fn commit_batch(
        &self,
        entries: Vec<EncryptedEntry>,
        mut after_replace: impl FnMut(usize) -> io::Result<()>,
    ) -> Result<()> {
        let labels = match self.prepare(entries) {
            Ok(labels) => labels,
            Err(error) => return self.rollback_error(error),
        };
        let work = self.work_dir();
        let apply = (|| -> io::Result<()> {
            for (index, label) in labels.iter().enumerate() {
                rename_file(
                    work.join(STAGING).join(index.to_string()),
                    self.dir.join(label),
                )?;
                after_replace(index)?;
            }
            sync_dir(&self.dir)?;
            // Mark the batch committed so recovery keeps the new entries.
            rename_file(work.join(JOURNAL), work.join(COMMITTED))?;
            Ok(())
        })();
        if let Err(error) = apply {
            return self.rollback_error(Error::Backend(error));
        }
        // Leave the commit record in place if flushing fails.
        sync_dir(&work).map_err(Error::Backend)?;
        if let Err(error) = self.recover() {
            // The batch committed. Retry cleanup on the next access.
            tracing::warn!(%error, "committed keystore batch cleanup deferred");
        }
        Ok(())
    }

    fn rollback_error(&self, error: Error) -> Result<()> {
        match self.recover() {
            Ok(()) => Err(error),
            Err(recovery) => Err(Error::Backend(io::Error::other(format!(
                "{error}. Keystore recovery also failed with {recovery}",
            )))),
        }
    }

    fn recover(&self) -> io::Result<()> {
        let work = self.work_dir();
        let journal_path = work.join(JOURNAL);
        let committed = work.join(COMMITTED);
        if committed.try_exists()? && journal_path.try_exists()? {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "conflicting keystore journals",
            ));
        }
        if journal_path.try_exists()? {
            let bytes = fs::read(&journal_path)?;
            let journal: Journal = cbor4ii::serde::from_slice(&bytes).map_err(io::Error::other)?;
            if journal.version != 1 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "unsupported keystore journal",
                ));
            }
            // Check every label before restoring files.
            for (label, _) in &journal.originals {
                crate::keystore::validate_label(label).map_err(io::Error::other)?;
                if label == ".tmp" {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "invalid journal label",
                    ));
                }
            }
            for (label, original) in journal.originals {
                let path = self.dir.join(label);
                if let Some(bytes) = original {
                    let restore = work.join("connexa-restore");
                    remove_file(&restore, &work)?;
                    write_new(&restore, &bytes)?;
                    rename_file(&restore, path)?;
                } else {
                    remove_file(&path, &work)?;
                }
            }
            // Keep the journal until rollback is flushed so recovery can safely retry.
            sync_dir(&self.dir)?;
            remove_file(&journal_path, &work)?;
            sync_dir(&work)?;
        }
        remove_file(&committed, &work)?;
        remove_file(&work.join("connexa-journal-new"), &work)?;
        remove_file(&work.join("connexa-restore"), &work)?;
        match fs::remove_dir_all(work.join(STAGING)) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
        sync_dir(&work)
    }
}

impl Keystore for FilesystemKeystore {
    async fn put(&self, entry: EncryptedEntry) -> Result<()> {
        self.put_many(vec![entry]).await
    }

    async fn put_many(&self, entries: Vec<EncryptedEntry>) -> Result<()> {
        if entries.is_empty() {
            return self.access(false, (), |_| Ok(())).await;
        }
        self.access(true, (), move |store| {
            store.commit_batch(entries, |_| Ok(()))
        })
        .await
    }

    async fn get(&self, label: &str) -> Result<Option<EncryptedEntry>> {
        self.entry_path(label)?;
        let label = label.to_owned();
        self.access(false, None, move |store| {
            match fs::read(store.dir.join(label)) {
                Ok(bytes) => Ok(Some(cbor4ii::serde::from_slice(&bytes).map_err(backend)?)),
                Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
                Err(e) => Err(Error::Backend(e)),
            }
        })
        .await
    }

    async fn list(&self) -> Result<Vec<KeyMetadata>> {
        self.access(false, Vec::new(), |store| {
            let mut metadata = Vec::new();
            for entry in fs::read_dir(&store.dir).map_err(Error::Backend)? {
                let entry = entry.map_err(Error::Backend)?;
                let file_type = entry.file_type().map_err(Error::Backend)?;
                if !file_type.is_file() {
                    if file_type.is_dir() && entry.file_name() == ".tmp" {
                        continue;
                    }
                    return Err(Error::Backend(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "keystore contains an unexpected entry",
                    )));
                }
                let bytes = fs::read(entry.path()).map_err(Error::Backend)?;
                let decoded =
                    cbor4ii::serde::from_slice::<EncryptedEntry>(&bytes).map_err(backend)?;
                if entry.file_name() != std::ffi::OsStr::new(&decoded.metadata.label) {
                    return Err(Error::Backend(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "key entry label does not match its file name",
                    )));
                }
                metadata.push(decoded.metadata);
            }
            Ok(metadata)
        })
        .await
    }

    async fn remove(&self, label: &str) -> Result<bool> {
        self.entry_path(label)?;
        let label = label.to_owned();
        self.access(false, false, move |store| {
            let removed =
                remove_file(&store.dir.join(label), &store.work_dir()).map_err(Error::Backend)?;
            sync_dir(&store.dir).map_err(Error::Backend)?;
            Ok(removed)
        })
        .await
    }
}

fn create_dir(dir: &Path) -> io::Result<()> {
    if dir.is_dir() {
        return Ok(());
    }
    if let Some(parent) = dir.parent().filter(|p| !p.as_os_str().is_empty()) {
        create_dir(parent)?;
    }
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    match builder.create(dir) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists && dir.is_dir() => {}
        Err(e) => return Err(e),
    }
    sync_dir(dir)?;
    sync_dir(
        dir.parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new(".")),
    )
}

fn write_new(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

#[cfg(not(windows))]
fn rename_file(from: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<()> {
    fs::rename(from, to)
}

#[cfg(windows)]
fn rename_file(from: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn MoveFileExW(from: *const u16, to: *const u16, flags: u32) -> i32;
    }
    fn wide(path: &Path) -> io::Result<Vec<u16>> {
        // Resolve the parent to support long Windows paths.
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let name = path
            .file_name()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing filename"))?;
        let absolute = fs::canonicalize(parent)?.join(name);
        let mut value: Vec<u16> = absolute.as_os_str().encode_wide().collect();
        if value.contains(&0) {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "NUL in path"));
        }
        value.push(0);
        Ok(value)
    }
    let from = wide(from.as_ref())?;
    let to = wide(to.as_ref())?;
    // Both paths are null terminated UTF-16 buffers and stay alive through this call.
    let result = unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), 0x1 | 0x8) };
    if result == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn remove_file(path: &Path, _work: &Path) -> io::Result<bool> {
    #[cfg(windows)]
    {
        // Flush the move so removal persists even if deleting the discard file fails.
        let discard = _work.join("connexa-discard");
        match rename_file(path, &discard) {
            Ok(()) => {
                let _ = fs::remove_file(discard);
                Ok(true)
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(e),
        }
    }
    #[cfg(not(windows))]
    match fs::remove_file(path) {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e),
    }
}

#[cfg(not(windows))]
fn sync_dir(path: &Path) -> io::Result<()> {
    File::open(path)?.sync_all()
}

#[cfg(windows)]
fn sync_dir(_path: &Path) -> io::Result<()> {
    // Windows uses write-through moves instead of flushing directories.
    Ok(())
}

fn backend<E: Into<Box<dyn std::error::Error + Send + Sync>>>(err: E) -> Error {
    Error::Backend(io::Error::other(err))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestDir(PathBuf);

    impl TestDir {
        fn new() -> Self {
            Self(
                std::env::temp_dir()
                    .join(format!("connexa-fs-transaction-{}", rand::random::<u64>(),)),
            )
        }
        fn store(&self) -> FilesystemKeystore {
            FilesystemKeystore::new(&self.0)
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn entry(label: &str, version: u32) -> EncryptedEntry {
        EncryptedEntry {
            metadata: KeyMetadata {
                label: label.to_owned(),
                key_type: crate::keystore::KeyType::Ed25519,
                version,
                created_at: web_time::UNIX_EPOCH,
                expires_at: None,
                public_key: Vec::new(),
            },
            ciphertext: vec![version as u8],
        }
    }

    #[tokio::test]
    async fn failed_replacement_restores_originals_and_removes_new_labels() {
        let dir = TestDir::new();
        let store = dir.store();
        store
            .put_many(vec![entry("a", 1), entry("b", 1)])
            .await
            .unwrap();
        {
            let _lock = store.lock().unwrap();
            let result =
                store.commit_batch(vec![entry("0-new", 2), entry("a", 2), entry("b", 2)], |i| {
                    if i == 1 {
                        Err(io::Error::other("injected replacement failure"))
                    } else {
                        Ok(())
                    }
                });
            assert!(result.is_err());
        }
        let reopened = dir.store();
        assert!(reopened.get("0-new").await.unwrap().is_none());
        assert_eq!(
            reopened.get("a").await.unwrap().unwrap().metadata.version,
            1
        );
        assert_eq!(
            reopened.get("b").await.unwrap().unwrap().metadata.version,
            1
        );
        assert!(!store.work_dir().join(JOURNAL).exists());
    }

    #[tokio::test]
    async fn reopening_recovers_interruption_between_replacements() {
        let dir = TestDir::new();
        let store = dir.store();
        store
            .put_many(vec![entry("a", 1), entry("b", 1)])
            .await
            .unwrap();
        {
            let _lock = store.lock().unwrap();
            store.prepare(vec![entry("a", 2), entry("b", 2)]).unwrap();
            rename_file(store.work_dir().join(STAGING).join("0"), dir.0.join("a")).unwrap();
            sync_dir(&dir.0).unwrap();
        }
        let reopened = dir.store();
        assert_eq!(
            reopened.get("a").await.unwrap().unwrap().metadata.version,
            1
        );
        assert_eq!(
            reopened.get("b").await.unwrap().unwrap().metadata.version,
            1
        );
        reopened
            .put_many(vec![entry("a", 3), entry("b", 3)])
            .await
            .unwrap();
        assert_eq!(
            reopened.get("a").await.unwrap().unwrap().metadata.version,
            3
        );
    }

    #[test]
    #[ignore = "subprocess fixture for process_exit_during_batch_recovers_originals"]
    fn interrupted_writer_subprocess() {
        let dir = std::env::var_os("CONNEXA_FS_CRASH_FIXTURE").expect("fixture directory");
        let store = FilesystemKeystore::new(PathBuf::from(dir));
        let _lock = store.lock().unwrap();
        store.recover().unwrap();
        store
            .commit_batch(vec![entry("a", 2), entry("b", 2)], |index| {
                if index == 0 {
                    // Exit without running destructors.
                    std::process::exit(73);
                }
                Ok(())
            })
            .unwrap();
        panic!("crash checkpoint was not reached");
    }

    #[tokio::test]
    async fn process_exit_during_batch_recovers_originals() {
        let dir = TestDir::new();
        let store = dir.store();
        store
            .put_many(vec![entry("a", 1), entry("b", 1)])
            .await
            .unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "keystore::store::fs::tests::interrupted_writer_subprocess",
                "--ignored",
            ])
            .env("CONNEXA_FS_CRASH_FIXTURE", &dir.0)
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(73),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let reopened = dir.store();
        assert_eq!(
            reopened.get("a").await.unwrap().unwrap().metadata.version,
            1
        );
        assert_eq!(
            reopened.get("b").await.unwrap().unwrap().metadata.version,
            1
        );
        assert!(!store.work_dir().join(JOURNAL).exists());
    }

    #[tokio::test]
    async fn reopening_keeps_a_committed_batch_before_cleanup() {
        let dir = TestDir::new();
        let store = dir.store();
        store
            .put_many(vec![entry("a", 1), entry("b", 1)])
            .await
            .unwrap();
        {
            let _lock = store.lock().unwrap();
            let labels = store.prepare(vec![entry("a", 2), entry("b", 2)]).unwrap();
            for (i, label) in labels.iter().enumerate() {
                rename_file(
                    store.work_dir().join(STAGING).join(i.to_string()),
                    dir.0.join(label),
                )
                .unwrap();
            }
            sync_dir(&dir.0).unwrap();
            rename_file(
                store.work_dir().join(JOURNAL),
                store.work_dir().join(COMMITTED),
            )
            .unwrap();
            sync_dir(&store.work_dir()).unwrap();
        }
        let reopened = dir.store();
        assert_eq!(
            reopened.get("a").await.unwrap().unwrap().metadata.version,
            2
        );
        assert_eq!(
            reopened.get("b").await.unwrap().unwrap().metadata.version,
            2
        );
        assert!(!store.work_dir().join(COMMITTED).exists());
    }

    #[tokio::test]
    async fn failed_recovery_blocks_access_until_it_can_finish() {
        let dir = TestDir::new();
        let store = dir.store();
        store
            .put_many(vec![entry("a", 1), entry("b", 1)])
            .await
            .unwrap();
        {
            let _lock = store.lock().unwrap();
            assert!(
                store
                    .commit_batch(vec![entry("a", 2), entry("b", 2)], |i| {
                        if i == 0 {
                            fs::remove_file(dir.0.join("b"))?;
                            fs::create_dir(dir.0.join("b"))?;
                        }
                        Ok(())
                    })
                    .is_err()
            );
        }
        assert!(store.get("a").await.is_err());
        assert!(store.list().await.is_err());
        assert!(store.remove("a").await.is_err());
        assert!(store.work_dir().join(JOURNAL).exists());
        fs::remove_dir(dir.0.join("b")).unwrap();
        assert_eq!(
            dir.store()
                .get("a")
                .await
                .unwrap()
                .unwrap()
                .metadata
                .version,
            1
        );
        assert_eq!(
            dir.store()
                .get("b")
                .await
                .unwrap()
                .unwrap()
                .metadata
                .version,
            1
        );
    }

    #[tokio::test]
    async fn malformed_journal_fails_closed_without_touching_live_files() {
        let dir = TestDir::new();
        let store = dir.store();
        store.put(entry("a", 1)).await.unwrap();
        let original = fs::read(dir.0.join("a")).unwrap();
        write_new(&store.work_dir().join(JOURNAL), b"bad journal").unwrap();
        assert!(store.get("a").await.is_err());
        assert!(store.put(entry("a", 2)).await.is_err());
        assert_eq!(fs::read(dir.0.join("a")).unwrap(), original);
    }

    #[tokio::test]
    async fn abandoned_preparation_is_cleaned_without_changing_entries() {
        let dir = TestDir::new();
        let store = dir.store();
        store.put(entry("a", 1)).await.unwrap();
        create_dir(&store.work_dir().join(STAGING)).unwrap();
        write_new(&store.work_dir().join(STAGING).join("0"), b"staged").unwrap();
        write_new(&store.work_dir().join("connexa-journal-new"), b"incomplete").unwrap();
        assert_eq!(store.get("a").await.unwrap().unwrap().metadata.version, 1);
        assert!(!store.work_dir().join(STAGING).exists());
    }

    #[tokio::test]
    async fn cancelled_batch_keeps_other_instances_locked_until_complete() {
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            let dir = TestDir::new();
            let store = dir.store();
            store
                .put_many(vec![entry("a", 1), entry("b", 1)])
                .await
                .unwrap();
            let started = std::sync::Arc::new(tokio::sync::Notify::new());
            let notify = started.clone();
            let (release, wait) = std::sync::mpsc::channel();
            let task = tokio::spawn(async move {
                store
                    .access(true, (), move |store| {
                        store.commit_batch(vec![entry("a", 2), entry("b", 2)], |i| {
                            if i == 0 {
                                notify.notify_one();
                                wait.recv().map_err(io::Error::other)?;
                            }
                            Ok(())
                        })
                    })
                    .await
            });
            started.notified().await;
            task.abort();
            assert!(task.await.unwrap_err().is_cancelled());
            let reader = dir.store();
            assert!(
                tokio::time::timeout(std::time::Duration::from_millis(25), reader.get("a"))
                    .await
                    .is_err()
            );
            release.send(()).unwrap();
            assert_eq!(reader.get("a").await.unwrap().unwrap().metadata.version, 2);
            assert_eq!(reader.get("b").await.unwrap().unwrap().metadata.version, 2);
        })
        .await
        .expect("filesystem job must finish after the caller is cancelled");
    }

    #[cfg(feature = "ed25519")]
    #[tokio::test]
    async fn cipher_migration_reopens_with_new_key_and_same_identities() {
        use crate::keystore::{Keychain, cipher::xchacha20poly1305::XChaCha20Poly1305Cipher};
        use libp2p::identity::Keypair;
        let dir = TestDir::new();
        let chain = Keychain::new_with_store([1; 32], dir.store());
        chain
            .insert("a", &Keypair::generate_ed25519())
            .await
            .unwrap();
        chain
            .insert("b", &Keypair::generate_ed25519())
            .await
            .unwrap();
        let a = chain.peer_id("a").await.unwrap();
        let b = chain.peer_id("b").await.unwrap();
        let next = chain
            .migrate_cipher(XChaCha20Poly1305Cipher::new([2; 32]))
            .await
            .unwrap();
        assert_eq!(chain.get("a").await.unwrap().public().to_peer_id(), a);
        drop(chain);
        drop(next);
        let reopened = Keychain::new_with_store([2; 32], dir.store());
        assert_eq!(reopened.peer_id("a").await.unwrap(), a);
        assert_eq!(reopened.get("b").await.unwrap().public().to_peer_id(), b);
        assert!(
            Keychain::new_with_store([1; 32], dir.store())
                .get("a")
                .await
                .is_err()
        );
    }

    #[cfg(feature = "ed25519")]
    #[tokio::test]
    async fn persists_across_instances() {
        use crate::keystore::{Keychain, generate_key};
        use libp2p::identity::Keypair;

        let dir = std::env::temp_dir().join(format!("connexa-fskeystore-{}", std::process::id()));
        let _ = tokio::fs::remove_dir_all(&dir).await;
        let key = generate_key();
        let keypair = Keypair::generate_ed25519();

        Keychain::new_with_store(key, FilesystemKeystore::new(&dir))
            .insert("identity", &keypair)
            .await
            .unwrap();

        let reopened = Keychain::new_with_store(key, FilesystemKeystore::new(&dir));
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

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn list_rejects_a_mismatched_file_name() {
        let dir = TestDir::new();
        let store = dir.store();
        store.put(entry("stored", 1)).await.unwrap();
        fs::rename(dir.0.join("stored"), dir.0.join("other")).unwrap();

        assert!(matches!(
            store.list().await,
            Err(Error::Backend(error)) if error.kind() == io::ErrorKind::InvalidData
        ));
    }

    #[tokio::test]
    async fn rejects_unsafe_label() {
        let store = FilesystemKeystore::new(std::env::temp_dir());
        assert!(matches!(
            store.get("../escape").await,
            Err(Error::InvalidLabel(_))
        ));
        assert!(matches!(
            store.remove("a/b").await,
            Err(Error::InvalidLabel(_))
        ));
    }

    #[cfg(all(unix, feature = "ed25519"))]
    #[tokio::test]
    async fn unix_files_are_owner_only() {
        use crate::keystore::{Keychain, generate_key};
        use libp2p::identity::Keypair;
        use std::os::unix::fs::PermissionsExt;

        let dir = std::env::temp_dir().join(format!("connexa-fsperms-{}", std::process::id()));
        let _ = tokio::fs::remove_dir_all(&dir).await;

        Keychain::new_with_store(generate_key(), FilesystemKeystore::new(&dir))
            .insert("identity", &Keypair::generate_ed25519())
            .await
            .unwrap();

        let dir_mode = tokio::fs::metadata(&dir)
            .await
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        let file_mode = tokio::fs::metadata(dir.join("identity"))
            .await
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(dir_mode, 0o700);
        assert_eq!(file_mode, 0o600);

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }
}
