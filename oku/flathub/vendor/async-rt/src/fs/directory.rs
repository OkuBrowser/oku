use super::{FileType, Metadata};
use std::ffi::OsString;
use std::fmt::{Debug, Formatter};
use std::io;
use std::path::PathBuf;

/// An iterator over entries in a directory.
pub struct ReadDir {
    inner: ReadDirInner,
}

enum ReadDirInner {
    #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
    Tokio(::tokio::fs::ReadDir),
    #[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
    Smol(::smol::fs::ReadDir),
    #[cfg(all(
        any(feature = "compio", feature = "threadpool", feature = "lite"),
        not(target_arch = "wasm32")
    ))]
    Blocking(super::BlockingReadDir<crate::global::BuiltinExecutor>),
    #[cfg(target_arch = "wasm32")]
    Wasm(super::WasmReadDir),
}

impl ReadDir {
    /// Returns the next entry in the directory.
    pub async fn next_entry(&mut self) -> io::Result<Option<DirEntry>> {
        match &mut self.inner {
            #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
            ReadDirInner::Tokio(read_dir) => read_dir
                .next_entry()
                .await
                .map(|entry| entry.map(DirEntry::from_tokio)),
            #[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
            ReadDirInner::Smol(read_dir) => {
                use futures::StreamExt;
                read_dir
                    .next()
                    .await
                    .transpose()
                    .map(|entry| entry.map(DirEntry::from_smol))
            }
            #[cfg(all(
                any(feature = "compio", feature = "threadpool", feature = "lite"),
                not(target_arch = "wasm32")
            ))]
            ReadDirInner::Blocking(read_dir) => read_dir
                .next_entry()
                .await
                .map(|entry| entry.map(DirEntry::from_blocking)),
            #[cfg(target_arch = "wasm32")]
            ReadDirInner::Wasm(read_dir) => read_dir
                .next_entry()
                .await
                .map(|entry| entry.map(DirEntry::from_wasm)),
            #[cfg(all(
                not(target_arch = "wasm32"),
                not(any(
                    feature = "tokio",
                    feature = "smol",
                    feature = "compio",
                    feature = "threadpool",
                    feature = "lite"
                ))
            ))]
            _ => unreachable!(),
        }
    }

    #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
    pub(super) fn from_tokio(read_dir: ::tokio::fs::ReadDir) -> Self {
        Self {
            inner: ReadDirInner::Tokio(read_dir),
        }
    }

    #[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
    pub(super) fn from_smol(read_dir: ::smol::fs::ReadDir) -> Self {
        Self {
            inner: ReadDirInner::Smol(read_dir),
        }
    }

    #[cfg(all(
        any(feature = "compio", feature = "threadpool", feature = "lite"),
        not(target_arch = "wasm32")
    ))]
    pub(super) fn from_blocking(
        read_dir: super::BlockingReadDir<crate::global::BuiltinExecutor>,
    ) -> Self {
        Self {
            inner: ReadDirInner::Blocking(read_dir),
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) fn from_wasm(read_dir: super::WasmReadDir) -> Self {
        Self {
            inner: ReadDirInner::Wasm(read_dir),
        }
    }
}

impl Debug for ReadDir {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReadDir").finish()
    }
}

/// An entry returned by [`ReadDir`].
pub struct DirEntry {
    inner: DirEntryInner,
}

enum DirEntryInner {
    #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
    Tokio(::tokio::fs::DirEntry),
    #[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
    Smol(::smol::fs::DirEntry),
    #[cfg(all(
        any(feature = "compio", feature = "threadpool", feature = "lite"),
        not(target_arch = "wasm32")
    ))]
    Blocking(super::BlockingDirEntry<crate::global::BuiltinExecutor>),
    #[cfg(target_arch = "wasm32")]
    Wasm(super::WasmDirEntry),
}

impl DirEntry {
    /// Returns the full path for this entry.
    pub fn path(&self) -> PathBuf {
        match &self.inner {
            #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
            DirEntryInner::Tokio(entry) => entry.path(),
            #[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
            DirEntryInner::Smol(entry) => entry.path(),
            #[cfg(all(
                any(feature = "compio", feature = "threadpool", feature = "lite"),
                not(target_arch = "wasm32")
            ))]
            DirEntryInner::Blocking(entry) => entry.path(),
            #[cfg(target_arch = "wasm32")]
            DirEntryInner::Wasm(entry) => entry.path(),
            #[cfg(all(
                not(target_arch = "wasm32"),
                not(any(
                    feature = "tokio",
                    feature = "smol",
                    feature = "compio",
                    feature = "threadpool",
                    feature = "lite"
                ))
            ))]
            _ => unreachable!(),
        }
    }

    /// Returns the file name for this entry.
    pub fn file_name(&self) -> OsString {
        match &self.inner {
            #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
            DirEntryInner::Tokio(entry) => entry.file_name(),
            #[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
            DirEntryInner::Smol(entry) => entry.file_name(),
            #[cfg(all(
                any(feature = "compio", feature = "threadpool", feature = "lite"),
                not(target_arch = "wasm32")
            ))]
            DirEntryInner::Blocking(entry) => entry.file_name(),
            #[cfg(target_arch = "wasm32")]
            DirEntryInner::Wasm(entry) => entry.file_name(),
            #[cfg(all(
                not(target_arch = "wasm32"),
                not(any(
                    feature = "tokio",
                    feature = "smol",
                    feature = "compio",
                    feature = "threadpool",
                    feature = "lite"
                ))
            ))]
            _ => unreachable!(),
        }
    }

    /// Reads metadata for this entry.
    pub async fn metadata(&self) -> io::Result<Metadata> {
        match &self.inner {
            #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
            DirEntryInner::Tokio(entry) => entry.metadata().await.map(Metadata::from_native),
            #[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
            DirEntryInner::Smol(entry) => entry.metadata().await.map(Metadata::from_native),
            #[cfg(all(
                any(feature = "compio", feature = "threadpool", feature = "lite"),
                not(target_arch = "wasm32")
            ))]
            DirEntryInner::Blocking(entry) => entry.metadata().await,
            #[cfg(target_arch = "wasm32")]
            DirEntryInner::Wasm(entry) => entry.metadata().await,
            #[cfg(all(
                not(target_arch = "wasm32"),
                not(any(
                    feature = "tokio",
                    feature = "smol",
                    feature = "compio",
                    feature = "threadpool",
                    feature = "lite"
                ))
            ))]
            _ => unreachable!(),
        }
    }

    /// Returns the file type for this entry.
    pub async fn file_type(&self) -> io::Result<FileType> {
        match &self.inner {
            #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
            DirEntryInner::Tokio(entry) => entry.file_type().await.map(FileType::from_native),
            #[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
            DirEntryInner::Smol(entry) => entry.file_type().await.map(FileType::from_native),
            #[cfg(all(
                any(feature = "compio", feature = "threadpool", feature = "lite"),
                not(target_arch = "wasm32")
            ))]
            DirEntryInner::Blocking(entry) => entry.file_type().await,
            #[cfg(target_arch = "wasm32")]
            DirEntryInner::Wasm(entry) => entry.file_type().await,
            #[cfg(all(
                not(target_arch = "wasm32"),
                not(any(
                    feature = "tokio",
                    feature = "smol",
                    feature = "compio",
                    feature = "threadpool",
                    feature = "lite"
                ))
            ))]
            _ => unreachable!(),
        }
    }

    #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
    fn from_tokio(entry: ::tokio::fs::DirEntry) -> Self {
        Self {
            inner: DirEntryInner::Tokio(entry),
        }
    }

    #[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
    fn from_smol(entry: ::smol::fs::DirEntry) -> Self {
        Self {
            inner: DirEntryInner::Smol(entry),
        }
    }

    #[cfg(all(
        any(feature = "compio", feature = "threadpool", feature = "lite"),
        not(target_arch = "wasm32")
    ))]
    fn from_blocking(entry: super::BlockingDirEntry<crate::global::BuiltinExecutor>) -> Self {
        Self {
            inner: DirEntryInner::Blocking(entry),
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn from_wasm(entry: super::WasmDirEntry) -> Self {
        Self {
            inner: DirEntryInner::Wasm(entry),
        }
    }
}

impl Debug for DirEntry {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DirEntry")
            .field("path", &self.path())
            .finish()
    }
}
