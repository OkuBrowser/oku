//! Asynchronous filesystem operations.

use std::fmt::{Debug, Formatter};
use std::future::Future;
use std::io;
use std::io::SeekFrom;
use std::path::{Path, PathBuf};

mod builtin;
pub use builtin::BuiltinFileSystem;
mod directory;
pub use directory::{DirEntry, ReadDir};
#[path = "fs/io.rs"]
mod file_io;
pub use file_io::{FileRead, FileSeek, FileWrite};
mod metadata;
pub use metadata::{FileType, Metadata, Permissions};
mod open_options;
pub use open_options::OpenOptions;
#[cfg(not(target_arch = "wasm32"))]
pub mod blocking;
#[cfg(not(target_arch = "wasm32"))]
pub use blocking::{BlockingDirEntry, BlockingFile, BlockingFileSystem, BlockingReadDir};
#[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
pub mod compio;
#[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
pub use compio::{CompioFile, CompioFileSystem};
#[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
pub mod smol;
#[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
pub use smol::SmolFileSystem;
#[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
pub mod tokio;
#[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
pub use tokio::TokioFileSystem;
#[cfg(target_arch = "wasm32")]
pub mod wasm;
#[cfg(target_arch = "wasm32")]
pub use wasm::{WasmDirEntry, WasmFile, WasmFileSystem, WasmReadDir};

/// A filesystem implementation.
pub trait FileSystem: Send + Sync {
    /// The file returned by this filesystem.
    type File: 'static;

    /// Opens a file in read-only mode.
    fn open<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<Self::File>>;

    /// Opens a file in write-only mode, creating or truncating it.
    fn create<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<Self::File>>;

    /// Reads an entire file into a byte vector.
    fn read<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<Vec<u8>>>;

    /// Reads an entire file into a string.
    fn read_to_string<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<String>>;

    /// Writes a byte vector as the entire contents of a file.
    fn write<P: AsRef<Path>, C: AsRef<[u8]>>(
        &self,
        path: P,
        contents: C,
    ) -> impl Future<Output = io::Result<()>>;
}

/// A filesystem that supports custom file opening options.
pub trait FileSystemOpenOptions: FileSystem {
    /// Opens a file using the supplied options.
    fn open_with<P: AsRef<Path>>(
        &self,
        options: &OpenOptions,
        path: P,
    ) -> impl Future<Output = io::Result<Self::File>>;
}

/// A filesystem that provides metadata for its entries.
pub trait FileSystemMetadata: FileSystem {
    /// Reads metadata for a path.
    fn metadata<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<Metadata>>;

    /// Reads metadata for a path without following symbolic links.
    fn symlink_metadata<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> impl Future<Output = io::Result<Metadata>>;
}

/// A filesystem that supports changing permissions.
pub trait FileSystemPermissions: FileSystem {
    /// Changes the permissions for a path.
    fn set_permissions<P: AsRef<Path>>(
        &self,
        path: P,
        permissions: Permissions,
    ) -> impl Future<Output = io::Result<()>>;
}

/// A filesystem that supports directory and path operations.
pub trait FileSystemDirectories: FileSystem {
    /// The directory reader returned by this filesystem.
    type ReadDir: 'static;

    /// Creates an empty directory.
    fn create_dir<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<()>>;

    /// Creates a directory and any missing parent directories.
    fn create_dir_all<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<()>>;

    /// Opens a directory for reading.
    fn read_dir<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<Self::ReadDir>>;

    /// Removes an empty directory.
    fn remove_dir<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<()>>;

    /// Removes a directory and everything inside it.
    fn remove_dir_all<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<()>>;

    /// Removes a file.
    fn remove_file<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<()>>;

    /// Renames a file or directory.
    fn rename<P: AsRef<Path>, Q: AsRef<Path>>(
        &self,
        from: P,
        to: Q,
    ) -> impl Future<Output = io::Result<()>>;

    /// Copies a file and returns the number of bytes copied.
    fn copy<P: AsRef<Path>, Q: AsRef<Path>>(
        &self,
        from: P,
        to: Q,
    ) -> impl Future<Output = io::Result<u64>>;

    /// Returns the canonical form of a path.
    fn canonicalize<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<PathBuf>>;

    /// Returns whether a path exists.
    fn try_exists<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<bool>>;
}

/// An open file using one of the built-in filesystem implementations.
pub struct File {
    inner: FileInner,
    io: file_io::FileIo,
}

enum FileInner {
    #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
    Tokio(::tokio::fs::File),
    #[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
    Smol(::smol::fs::File),
    #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
    Compio(CompioFile),
    #[cfg(all(
        any(feature = "threadpool", feature = "lite"),
        not(target_arch = "wasm32")
    ))]
    Blocking(BlockingFile<crate::global::BuiltinExecutor>),
    #[cfg(target_arch = "wasm32")]
    Wasm(WasmFile),
}

impl File {
    #[cfg(any(
        target_arch = "wasm32",
        all(
            not(target_arch = "wasm32"),
            any(
                feature = "tokio",
                feature = "smol",
                feature = "compio",
                feature = "threadpool",
                feature = "lite"
            )
        )
    ))]
    fn from_inner(inner: FileInner) -> Self {
        Self {
            inner,
            io: file_io::FileIo::default(),
        }
    }

    /// Returns a new set of options for opening a file.
    pub fn options() -> OpenOptions {
        OpenOptions::new()
    }

    /// Opens a file in read-only mode.
    pub async fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        current().open(path).await
    }

    /// Opens a file in write-only mode, creating or truncating it.
    pub async fn create(path: impl AsRef<Path>) -> io::Result<Self> {
        current().create(path).await
    }

    /// Creates a new file and fails if it already exists.
    pub async fn create_new(path: impl AsRef<Path>) -> io::Result<Self> {
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .await
    }

    /// Reads bytes into a buffer and returns how many bytes were read.
    pub async fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        FileRead::read(self, buffer).await
    }

    /// Reads enough bytes to fill a buffer.
    pub async fn read_exact(&mut self, buffer: &mut [u8]) -> io::Result<()> {
        FileRead::read_exact(self, buffer).await
    }

    /// Reads all remaining bytes and appends them to a vector.
    pub async fn read_to_end(&mut self, output: &mut Vec<u8>) -> io::Result<usize> {
        FileRead::read_to_end(self, output).await
    }

    /// Reads all remaining bytes and appends them to a string.
    pub async fn read_to_string(&mut self, output: &mut String) -> io::Result<usize> {
        FileRead::read_to_string(self, output).await
    }

    /// Writes bytes and returns how many bytes were written.
    pub async fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        FileWrite::write(self, buffer).await
    }

    /// Writes an entire buffer.
    pub async fn write_all(&mut self, buffer: &[u8]) -> io::Result<()> {
        FileWrite::write_all(self, buffer).await
    }

    /// Flushes buffered data to the file.
    pub async fn flush(&mut self) -> io::Result<()> {
        FileWrite::flush(self).await
    }

    /// Moves the file cursor and returns its new position.
    pub async fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        FileSeek::seek(self, position).await
    }

    /// Moves the file cursor to the start of the file.
    pub async fn rewind(&mut self) -> io::Result<()> {
        FileSeek::rewind(self).await
    }

    /// Returns the current file cursor position.
    pub async fn stream_position(&mut self) -> io::Result<u64> {
        FileSeek::stream_position(self).await
    }

    /// Reads metadata for this file.
    pub async fn metadata(&self) -> io::Result<Metadata> {
        match &self.inner {
            #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
            FileInner::Tokio(file) => file.metadata().await.map(Metadata::from_native),
            #[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
            FileInner::Smol(file) => file.metadata().await.map(Metadata::from_native),
            #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
            FileInner::Compio(file) => file.metadata().await,
            #[cfg(all(
                any(feature = "threadpool", feature = "lite"),
                not(target_arch = "wasm32")
            ))]
            FileInner::Blocking(file) => file.metadata().await,
            #[cfg(target_arch = "wasm32")]
            FileInner::Wasm(file) => file.metadata().await,
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

    /// Changes the size of this file.
    pub async fn set_len(&self, size: u64) -> io::Result<()> {
        let _ = size;
        match &self.inner {
            #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
            FileInner::Tokio(file) => file.set_len(size).await,
            #[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
            FileInner::Smol(file) => file.set_len(size).await,
            #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
            FileInner::Compio(file) => file.set_len(size).await,
            #[cfg(all(
                any(feature = "threadpool", feature = "lite"),
                not(target_arch = "wasm32")
            ))]
            FileInner::Blocking(file) => file.set_len(size).await,
            #[cfg(target_arch = "wasm32")]
            FileInner::Wasm(file) => file.set_len(size).await,
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

    /// Changes the permissions for this file.
    pub async fn set_permissions(&self, permissions: Permissions) -> io::Result<()> {
        let _ = &permissions;
        match &self.inner {
            #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
            FileInner::Tokio(file) => {
                let mut native = file.metadata().await?.permissions();
                native.set_readonly(permissions.readonly());
                file.set_permissions(native).await
            }
            #[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
            FileInner::Smol(file) => {
                let mut native = file.metadata().await?.permissions();
                native.set_readonly(permissions.readonly());
                file.set_permissions(native).await
            }
            #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
            FileInner::Compio(file) => {
                let mut native = file.metadata().await?.permissions();
                native.set_readonly(permissions.readonly());
                file.set_permissions(native).await
            }
            #[cfg(all(
                any(feature = "threadpool", feature = "lite"),
                not(target_arch = "wasm32")
            ))]
            FileInner::Blocking(file) => file.set_permissions(permissions).await,
            #[cfg(target_arch = "wasm32")]
            FileInner::Wasm(file) => file.set_permissions(permissions).await,
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

    /// Synchronizes file contents and metadata to disk.
    pub async fn sync_all(&self) -> io::Result<()> {
        match &self.inner {
            #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
            FileInner::Tokio(file) => file.sync_all().await,
            #[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
            FileInner::Smol(file) => file.sync_all().await,
            #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
            FileInner::Compio(file) => file.sync_all().await,
            #[cfg(all(
                any(feature = "threadpool", feature = "lite"),
                not(target_arch = "wasm32")
            ))]
            FileInner::Blocking(file) => file.sync_all().await,
            #[cfg(target_arch = "wasm32")]
            FileInner::Wasm(file) => file.sync_all().await,
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

    /// Synchronizes file contents to disk.
    pub async fn sync_data(&self) -> io::Result<()> {
        match &self.inner {
            #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
            FileInner::Tokio(file) => file.sync_data().await,
            #[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
            FileInner::Smol(file) => file.sync_data().await,
            #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
            FileInner::Compio(file) => file.sync_data().await,
            #[cfg(all(
                any(feature = "threadpool", feature = "lite"),
                not(target_arch = "wasm32")
            ))]
            FileInner::Blocking(file) => file.sync_data().await,
            #[cfg(target_arch = "wasm32")]
            FileInner::Wasm(file) => file.sync_data().await,
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
}

impl FileRead for File {
    async fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        futures::io::AsyncReadExt::read(self, buffer).await
    }
}

impl FileWrite for File {
    async fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        futures::io::AsyncWriteExt::write(self, buffer).await
    }

    async fn flush(&mut self) -> io::Result<()> {
        futures::io::AsyncWriteExt::flush(self).await
    }
}

impl FileSeek for File {
    async fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        futures::io::AsyncSeekExt::seek(self, position).await
    }
}

impl Debug for File {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("File").finish()
    }
}

/// Reads an entire file into a byte vector.
pub async fn read(path: impl AsRef<Path>) -> io::Result<Vec<u8>> {
    current().read(path).await
}

/// Reads an entire file into a string.
pub async fn read_to_string(path: impl AsRef<Path>) -> io::Result<String> {
    current().read_to_string(path).await
}

/// Reads metadata for a path.
pub async fn metadata(path: impl AsRef<Path>) -> io::Result<Metadata> {
    current().metadata(path).await
}

/// Reads metadata for a path without following symbolic links.
pub async fn symlink_metadata(path: impl AsRef<Path>) -> io::Result<Metadata> {
    current().symlink_metadata(path).await
}

/// Changes the permissions for a path.
pub async fn set_permissions(path: impl AsRef<Path>, permissions: Permissions) -> io::Result<()> {
    current().set_permissions(path, permissions).await
}

/// Writes a byte slice as the entire contents of a file.
pub async fn write(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> io::Result<()> {
    current().write(path, contents).await
}

/// Creates an empty directory.
pub async fn create_dir(path: impl AsRef<Path>) -> io::Result<()> {
    current().create_dir(path).await
}

/// Creates a directory and any missing parent directories.
pub async fn create_dir_all(path: impl AsRef<Path>) -> io::Result<()> {
    current().create_dir_all(path).await
}

/// Opens a directory for reading.
pub async fn read_dir(path: impl AsRef<Path>) -> io::Result<ReadDir> {
    current().read_dir(path).await
}

/// Removes an empty directory.
pub async fn remove_dir(path: impl AsRef<Path>) -> io::Result<()> {
    current().remove_dir(path).await
}

/// Removes a directory and everything inside it.
pub async fn remove_dir_all(path: impl AsRef<Path>) -> io::Result<()> {
    current().remove_dir_all(path).await
}

/// Removes a file.
pub async fn remove_file(path: impl AsRef<Path>) -> io::Result<()> {
    current().remove_file(path).await
}

/// Renames a file or directory.
pub async fn rename(from: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<()> {
    current().rename(from, to).await
}

/// Copies a file and returns the number of bytes copied.
pub async fn copy(from: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<u64> {
    current().copy(from, to).await
}

/// Returns the canonical form of a path.
pub async fn canonicalize(path: impl AsRef<Path>) -> io::Result<PathBuf> {
    current().canonicalize(path).await
}

/// Returns whether a path exists.
pub async fn try_exists(path: impl AsRef<Path>) -> io::Result<bool> {
    current().try_exists(path).await
}

fn current() -> BuiltinFileSystem {
    crate::task::executor().into()
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    #[cfg(not(any(
        feature = "tokio",
        feature = "smol",
        feature = "compio",
        feature = "threadpool",
        feature = "lite"
    )))]
    use super::{File, read_dir};

    #[cfg(any(
        feature = "tokio",
        feature = "smol",
        feature = "compio",
        feature = "threadpool",
        feature = "lite"
    ))]
    mod available {
        use super::super::{
            File, canonicalize, copy, create_dir, create_dir_all, metadata, read, read_dir,
            read_to_string, remove_dir, remove_dir_all, remove_file, rename, set_permissions,
            symlink_metadata, try_exists, write,
        };
        use crate::ExecutorBlockOn;
        use crate::global::BuiltinExecutor;
        use std::io::SeekFrom;
        use std::path::PathBuf;
        use std::sync::atomic::{AtomicU64, Ordering};

        static NEXT_PATH: AtomicU64 = AtomicU64::new(0);

        struct TestPath(PathBuf);

        impl TestPath {
            fn new(backend: &str) -> Self {
                let id = NEXT_PATH.fetch_add(1, Ordering::Relaxed);
                let path = std::env::temp_dir()
                    .join(format!("async-rt-fs-{backend}-{}-{id}", std::process::id()));
                Self(path)
            }
        }

        impl Drop for TestPath {
            fn drop(&mut self) {
                let _ = std::fs::remove_file(&self.0);
            }
        }

        struct TestDir(PathBuf);

        impl TestDir {
            fn new(backend: &str) -> Self {
                let id = NEXT_PATH.fetch_add(1, Ordering::Relaxed);
                let path = std::env::temp_dir().join(format!(
                    "async-rt-fs-dir-{backend}-{}-{id}",
                    std::process::id()
                ));
                Self(path)
            }
        }

        impl Drop for TestDir {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }

        async fn file_facade_round_trip(backend: &str) {
            let path = TestPath::new(backend);

            write(&path.0, b"hello").await.unwrap();
            assert_eq!(read(&path.0).await.unwrap(), b"hello");
            assert_eq!(read_to_string(&path.0).await.unwrap(), "hello");

            let mut file = File::open(&path.0).await.unwrap();
            let mut first = [0; 2];
            assert_eq!(file.read(&mut first).await.unwrap(), 2);
            assert_eq!(&first, b"he");
            assert_eq!(file.stream_position().await.unwrap(), 2);
            assert_eq!(file.seek(SeekFrom::End(-2)).await.unwrap(), 3);
            let mut rest = [0; 2];
            file.read_exact(&mut rest).await.unwrap();
            assert_eq!(&rest, b"lo");
            assert_eq!(file.read(&mut first).await.unwrap(), 0);

            file.rewind().await.unwrap();
            let mut bytes = vec![b'>'];
            assert_eq!(file.read_to_end(&mut bytes).await.unwrap(), 5);
            assert_eq!(&bytes, b">hello");

            file.rewind().await.unwrap();
            let mut string = String::from(">");
            assert_eq!(file.read_to_string(&mut string).await.unwrap(), 5);
            assert_eq!(string, ">hello");
            drop(file);

            let mut file = File::options()
                .read(true)
                .write(true)
                .open(&path.0)
                .await
                .unwrap();
            assert_eq!(file.seek(SeekFrom::Start(1)).await.unwrap(), 1);
            file.write_all(b"XY").await.unwrap();
            file.flush().await.unwrap();
            assert_eq!(file.stream_position().await.unwrap(), 3);
            file.rewind().await.unwrap();
            let mut changed = String::new();
            file.read_to_string(&mut changed).await.unwrap();
            assert_eq!(changed, "hXYlo");
            file.rewind().await.unwrap();
            file.write_all(b"hello").await.unwrap();
            file.flush().await.unwrap();
            drop(file);

            let file_metadata = metadata(&path.0).await.unwrap();
            assert!(file_metadata.is_file());
            assert!(!file_metadata.is_dir());
            assert!(!file_metadata.is_symlink());
            assert_eq!(file_metadata.len(), 5);
            assert!(file_metadata.file_type().is_file());
            let permissions = file_metadata.permissions();
            set_permissions(&path.0, permissions.clone()).await.unwrap();
            assert_eq!(symlink_metadata(&path.0).await.unwrap().len(), 5);
            assert!(metadata(std::env::temp_dir()).await.unwrap().is_dir());

            let file = File::open(&path.0).await.unwrap();
            assert_eq!(file.metadata().await.unwrap().len(), 5);
            file.set_permissions(permissions).await.unwrap();
            file.sync_all().await.unwrap();
            file.sync_data().await.unwrap();
            drop(file);

            let file = File::options()
                .read(true)
                .write(true)
                .open(&path.0)
                .await
                .unwrap();
            file.set_len(2).await.unwrap();
            assert_eq!(file.metadata().await.unwrap().len(), 2);
            drop(file);
            assert_eq!(read(&path.0).await.unwrap(), b"he");

            let file = File::create(&path.0).await.unwrap();
            file.sync_all().await.unwrap();
            drop(file);

            assert!(read(&path.0).await.unwrap().is_empty());

            let new_path = TestPath::new(backend);
            File::create_new(&new_path.0).await.unwrap();
            let error = File::create_new(&new_path.0).await.unwrap_err();
            assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
        }

        async fn file_facade_edge_cases(backend: &str, supports_append: bool) {
            let short = TestPath::new(backend);
            write(&short.0, b"ab").await.unwrap();
            let mut file = File::open(&short.0).await.unwrap();
            let mut buffer = [0; 3];
            let error = file.read_exact(&mut buffer).await.unwrap_err();
            assert_eq!(error.kind(), std::io::ErrorKind::UnexpectedEof);
            assert_eq!(&buffer[..2], b"ab");

            file.rewind().await.unwrap();
            let error = file.seek(SeekFrom::Current(-1)).await.unwrap_err();
            assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
            drop(file);

            let sparse = TestPath::new(backend);
            write(&sparse.0, b"abc").await.unwrap();
            let mut file = File::options()
                .read(true)
                .write(true)
                .open(&sparse.0)
                .await
                .unwrap();
            file.seek(SeekFrom::Start(5)).await.unwrap();
            file.write_all(b"z").await.unwrap();
            file.flush().await.unwrap();
            drop(file);
            assert_eq!(read(&sparse.0).await.unwrap(), b"abc\0\0z");

            let invalid_utf8 = TestPath::new(backend);
            write(&invalid_utf8.0, [0xff]).await.unwrap();
            let error = read_to_string(&invalid_utf8.0).await.unwrap_err();
            assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);

            let mut file = File::open(&invalid_utf8.0).await.unwrap();
            let mut output = String::from("unchanged");
            let error = file.read_to_string(&mut output).await.unwrap_err();
            assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
            assert_eq!(output, "unchanged");

            let options = TestPath::new(backend);
            let error = File::options().open(&options.0).await.unwrap_err();
            assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);

            let error = File::options()
                .create(true)
                .open(&options.0)
                .await
                .unwrap_err();
            assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);

            File::options()
                .write(true)
                .create(true)
                .open(&options.0)
                .await
                .unwrap();
            write(&options.0, b"contents").await.unwrap();

            File::options()
                .write(true)
                .create(true)
                .open(&options.0)
                .await
                .unwrap();
            assert_eq!(read(&options.0).await.unwrap(), b"contents");

            File::options()
                .write(true)
                .truncate(true)
                .open(&options.0)
                .await
                .unwrap();
            assert!(read(&options.0).await.unwrap().is_empty());

            let missing = TestPath::new(backend);
            let error = File::options()
                .write(true)
                .truncate(true)
                .open(&missing.0)
                .await
                .unwrap_err();
            assert_eq!(error.kind(), std::io::ErrorKind::NotFound);

            let appended = TestPath::new(backend);
            write(&appended.0, b"first").await.unwrap();
            let result = File::options().append(true).open(&appended.0).await;
            if supports_append {
                let mut file = result.unwrap();
                file.seek(SeekFrom::Start(0)).await.unwrap();
                file.write_all(b" second").await.unwrap();
                file.flush().await.unwrap();
                drop(file);
                assert_eq!(read(&appended.0).await.unwrap(), b"first second");
            } else {
                assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::Unsupported);
            }
        }

        async fn futures_io_traits_round_trip(backend: &str) {
            let path = TestPath::new(backend);
            let mut file = File::options()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(&path.0)
                .await
                .unwrap();

            futures::io::AsyncWriteExt::write_all(&mut file, b"futures")
                .await
                .unwrap();
            futures::io::AsyncWriteExt::flush(&mut file).await.unwrap();
            assert_eq!(
                futures::io::AsyncSeekExt::seek(&mut file, SeekFrom::Start(0))
                    .await
                    .unwrap(),
                0
            );

            let mut contents = String::new();
            futures::io::AsyncReadExt::read_to_string(&mut file, &mut contents)
                .await
                .unwrap();
            assert_eq!(contents, "futures");
            futures::io::AsyncWriteExt::close(&mut file).await.unwrap();
        }

        #[cfg(feature = "tokio")]
        async fn tokio_io_traits_round_trip(backend: &str) {
            let path = TestPath::new(backend);
            let mut file = File::options()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(&path.0)
                .await
                .unwrap();

            ::tokio::io::AsyncWriteExt::write_all(&mut file, b"tokio")
                .await
                .unwrap();
            ::tokio::io::AsyncWriteExt::flush(&mut file).await.unwrap();
            assert_eq!(
                ::tokio::io::AsyncSeekExt::seek(&mut file, SeekFrom::Start(0))
                    .await
                    .unwrap(),
                0
            );

            let mut contents = String::new();
            ::tokio::io::AsyncReadExt::read_to_string(&mut file, &mut contents)
                .await
                .unwrap();
            assert_eq!(contents, "tokio");
            ::tokio::io::AsyncWriteExt::shutdown(&mut file)
                .await
                .unwrap();
        }

        async fn directory_facade_round_trip(backend: &str) {
            let root = TestDir::new(backend);
            assert!(!try_exists(&root.0).await.unwrap());

            create_dir(&root.0).await.unwrap();
            assert!(try_exists(&root.0).await.unwrap());
            assert!(metadata(&root.0).await.unwrap().is_dir());

            let empty = root.0.join("empty");
            create_dir(&empty).await.unwrap();

            let nested = root.0.join("nested/child");
            create_dir_all(&nested).await.unwrap();

            let source = root.0.join("source.txt");
            let copied = root.0.join("copied.txt");
            let moved = root.0.join("moved.txt");
            write(&source, b"hello").await.unwrap();
            assert_eq!(copy(&source, &copied).await.unwrap(), 5);
            rename(&copied, &moved).await.unwrap();
            assert!(!try_exists(&copied).await.unwrap());
            assert_eq!(read(&moved).await.unwrap(), b"hello");

            let mut entries = read_dir(&root.0).await.unwrap();
            let mut names = Vec::new();
            while let Some(entry) = entries.next_entry().await.unwrap() {
                assert_eq!(entry.path().file_name(), Some(entry.file_name().as_ref()));
                let file_type = entry.file_type().await.unwrap();
                assert_eq!(file_type, entry.metadata().await.unwrap().file_type());
                names.push(entry.file_name());
            }
            assert!(names.iter().any(|name| name == "empty"));
            assert!(names.iter().any(|name| name == "nested"));
            assert!(names.iter().any(|name| name == "source.txt"));
            assert!(names.iter().any(|name| name == "moved.txt"));

            assert!(canonicalize(&root.0).await.unwrap().is_absolute());
            remove_file(&source).await.unwrap();
            remove_file(&moved).await.unwrap();
            remove_dir(&empty).await.unwrap();
            remove_dir_all(root.0.join("nested")).await.unwrap();
            remove_dir(&root.0).await.unwrap();
            assert!(!try_exists(&root.0).await.unwrap());
        }

        async fn directory_facade_edge_cases(backend: &str) {
            let root = TestDir::new(backend);
            create_dir(&root.0).await.unwrap();

            let non_empty = root.0.join("non-empty");
            create_dir(&non_empty).await.unwrap();
            write(non_empty.join("file.txt"), b"contents")
                .await
                .unwrap();
            assert!(remove_dir(&non_empty).await.is_err());
            assert!(try_exists(non_empty.join("file.txt")).await.unwrap());
            remove_dir_all(&non_empty).await.unwrap();

            let copy_source = root.0.join("copy-source.txt");
            let destination = root.0.join("destination.txt");
            write(&copy_source, b"copied").await.unwrap();
            write(&destination, b"old").await.unwrap();
            assert_eq!(copy(&copy_source, &destination).await.unwrap(), 6);
            assert_eq!(read(&destination).await.unwrap(), b"copied");

            let rename_source = root.0.join("rename-source.txt");
            write(&rename_source, b"renamed").await.unwrap();
            rename(&rename_source, &destination).await.unwrap();
            assert!(!try_exists(&rename_source).await.unwrap());
            assert_eq!(read(&destination).await.unwrap(), b"renamed");
        }

        #[cfg(feature = "tokio")]
        #[test]
        fn tokio_file_facade() {
            let _guard = crate::task::set_executor(BuiltinExecutor::Tokio);
            let executor = crate::rt::tokio::TokioRuntimeExecutor::with_single_thread().unwrap();
            executor.block_on(async {
                file_facade_round_trip("tokio").await;
                file_facade_edge_cases("tokio", true).await;
                futures_io_traits_round_trip("tokio").await;
                tokio_io_traits_round_trip("tokio").await;
                directory_facade_round_trip("tokio").await;
                directory_facade_edge_cases("tokio").await;
            });
        }

        #[cfg(feature = "smol")]
        #[test]
        fn smol_file_facade() {
            let _guard = crate::task::set_executor(BuiltinExecutor::Smol);
            crate::rt::smol::SmolExecutor.block_on(async {
                file_facade_round_trip("smol").await;
                file_facade_edge_cases("smol", true).await;
                futures_io_traits_round_trip("smol").await;
                #[cfg(feature = "tokio")]
                tokio_io_traits_round_trip("smol").await;
                directory_facade_round_trip("smol").await;
                directory_facade_edge_cases("smol").await;
            });
        }

        #[cfg(feature = "compio")]
        #[test]
        fn compio_file_facade() {
            let _guard = crate::task::set_executor(BuiltinExecutor::Compio);
            let executor = crate::rt::compio::CompioRuntimeExecutor::new().unwrap();
            executor.block_on(async {
                file_facade_round_trip("compio").await;
                file_facade_edge_cases("compio", false).await;
                futures_io_traits_round_trip("compio").await;
                #[cfg(feature = "tokio")]
                tokio_io_traits_round_trip("compio").await;
                directory_facade_round_trip("compio").await;
                directory_facade_edge_cases("compio").await;
            });
        }

        #[cfg(feature = "threadpool")]
        #[test]
        fn threadpool_file_facade() {
            let _guard = crate::task::set_executor(BuiltinExecutor::ThreadPool);
            crate::rt::threadpool::ThreadPoolExecutor.block_on(async {
                file_facade_round_trip("threadpool").await;
                file_facade_edge_cases("threadpool", true).await;
                futures_io_traits_round_trip("threadpool").await;
                #[cfg(feature = "tokio")]
                tokio_io_traits_round_trip("threadpool").await;
                directory_facade_round_trip("threadpool").await;
                directory_facade_edge_cases("threadpool").await;
            });
        }

        #[cfg(feature = "lite")]
        #[test]
        fn lite_file_facade() {
            let _guard = crate::task::set_executor(BuiltinExecutor::Lite);
            crate::rt::lite::LiteExecutor.block_on(async {
                file_facade_round_trip("lite").await;
                file_facade_edge_cases("lite", true).await;
                futures_io_traits_round_trip("lite").await;
                #[cfg(feature = "tokio")]
                tokio_io_traits_round_trip("lite").await;
                directory_facade_round_trip("lite").await;
                directory_facade_edge_cases("lite").await;
            });
        }
    }

    #[cfg(not(any(
        feature = "tokio",
        feature = "smol",
        feature = "compio",
        feature = "threadpool",
        feature = "lite"
    )))]
    #[test]
    fn unavailable_file_facade() {
        let error = futures::executor::block_on(File::open("unavailable")).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::Unsupported);

        let error = futures::executor::block_on(read_dir("unavailable")).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::Unsupported);
    }
}
