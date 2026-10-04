use super::{
    FileRead, FileSeek, FileSystem, FileSystemDirectories, FileSystemMetadata,
    FileSystemOpenOptions, FileSystemPermissions, FileType, FileWrite, Metadata, OpenOptions,
    Permissions,
};
use crate::{ExecutorBlocking, JoinError};
use parking_lot::Mutex;
use std::ffi::OsString;
use std::fmt::{Debug, Formatter};
use std::future::Future;
use std::io;
use std::io::SeekFrom;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// A filesystem that performs operations through an executor's blocking pool.
#[derive(Clone, Copy, Debug, Default)]
pub struct BlockingFileSystem<E> {
    executor: E,
}

impl<E> BlockingFileSystem<E> {
    /// Creates a filesystem using the supplied executor.
    pub fn new(executor: E) -> Self {
        Self { executor }
    }

    /// Returns the executor used for filesystem operations.
    pub fn executor(&self) -> &E {
        &self.executor
    }
}

impl<E> FileSystem for BlockingFileSystem<E>
where
    E: ExecutorBlocking + Clone + Send + Sync + 'static,
{
    type File = BlockingFile<E>;

    fn open<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<Self::File>> {
        let path = path.as_ref().to_owned();
        let executor = self.executor.clone();
        async move {
            let file = executor
                .spawn_blocking(move || std::fs::File::open(path))
                .await
                .map_err(join_error)??;
            Ok(BlockingFile::new(file, executor))
        }
    }

    fn create<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<Self::File>> {
        let path = path.as_ref().to_owned();
        let executor = self.executor.clone();
        async move {
            let file = executor
                .spawn_blocking(move || std::fs::File::create(path))
                .await
                .map_err(join_error)??;
            Ok(BlockingFile::new(file, executor))
        }
    }

    fn read<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<Vec<u8>>> {
        let path = path.as_ref().to_owned();
        let executor = self.executor.clone();
        async move {
            executor
                .spawn_blocking(move || std::fs::read(path))
                .await
                .map_err(join_error)?
        }
    }

    fn read_to_string<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<String>> {
        let path = path.as_ref().to_owned();
        let executor = self.executor.clone();
        async move {
            executor
                .spawn_blocking(move || std::fs::read_to_string(path))
                .await
                .map_err(join_error)?
        }
    }

    fn write<P: AsRef<Path>, C: AsRef<[u8]>>(
        &self,
        path: P,
        contents: C,
    ) -> impl Future<Output = io::Result<()>> {
        let path = path.as_ref().to_owned();
        let contents = contents.as_ref().to_owned();
        let executor = self.executor.clone();
        async move {
            executor
                .spawn_blocking(move || std::fs::write(path, contents))
                .await
                .map_err(join_error)?
        }
    }
}

impl<E> FileSystemOpenOptions for BlockingFileSystem<E>
where
    E: ExecutorBlocking + Clone + Send + Sync + 'static,
{
    fn open_with<P: AsRef<Path>>(
        &self,
        options: &OpenOptions,
        path: P,
    ) -> impl Future<Output = io::Result<Self::File>> {
        let path = path.as_ref().to_owned();
        let options = options.clone();
        let executor = self.executor.clone();
        async move {
            let file = executor
                .spawn_blocking(move || {
                    let mut inner = std::fs::OpenOptions::new();
                    inner
                        .read(options.read)
                        .write(options.write)
                        .append(options.append)
                        .truncate(options.truncate)
                        .create(options.create)
                        .create_new(options.create_new);
                    inner.open(path)
                })
                .await
                .map_err(join_error)??;
            Ok(BlockingFile::new(file, executor))
        }
    }
}

impl<E> FileSystemMetadata for BlockingFileSystem<E>
where
    E: ExecutorBlocking + Clone + Send + Sync + 'static,
{
    fn metadata<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<Metadata>> {
        let path = path.as_ref().to_owned();
        let executor = self.executor.clone();
        async move {
            executor
                .spawn_blocking(move || std::fs::metadata(path).map(Metadata::from_native))
                .await
                .map_err(join_error)?
        }
    }

    fn symlink_metadata<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> impl Future<Output = io::Result<Metadata>> {
        let path = path.as_ref().to_owned();
        let executor = self.executor.clone();
        async move {
            executor
                .spawn_blocking(move || std::fs::symlink_metadata(path).map(Metadata::from_native))
                .await
                .map_err(join_error)?
        }
    }
}

impl<E> FileSystemPermissions for BlockingFileSystem<E>
where
    E: ExecutorBlocking + Clone + Send + Sync + 'static,
{
    fn set_permissions<P: AsRef<Path>>(
        &self,
        path: P,
        permissions: Permissions,
    ) -> impl Future<Output = io::Result<()>> {
        let path = path.as_ref().to_owned();
        let executor = self.executor.clone();
        async move {
            executor
                .spawn_blocking(move || {
                    let mut native = std::fs::metadata(&path)?.permissions();
                    native.set_readonly(permissions.readonly());
                    std::fs::set_permissions(path, native)
                })
                .await
                .map_err(join_error)?
        }
    }
}

impl<E> FileSystemDirectories for BlockingFileSystem<E>
where
    E: ExecutorBlocking + Clone + Send + Sync + 'static,
{
    type ReadDir = BlockingReadDir<E>;

    fn create_dir<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<()>> {
        self.run_path(path, std::fs::create_dir)
    }

    fn create_dir_all<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<()>> {
        self.run_path(path, std::fs::create_dir_all)
    }

    fn read_dir<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<Self::ReadDir>> {
        let path = path.as_ref().to_owned();
        let executor = self.executor.clone();
        async move {
            let read_dir = executor
                .spawn_blocking(move || std::fs::read_dir(path))
                .await
                .map_err(join_error)??;
            Ok(BlockingReadDir::new(read_dir, executor))
        }
    }

    fn remove_dir<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<()>> {
        self.run_path(path, std::fs::remove_dir)
    }

    fn remove_dir_all<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<()>> {
        self.run_path(path, std::fs::remove_dir_all)
    }

    fn remove_file<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<()>> {
        self.run_path(path, std::fs::remove_file)
    }

    fn rename<P: AsRef<Path>, Q: AsRef<Path>>(
        &self,
        from: P,
        to: Q,
    ) -> impl Future<Output = io::Result<()>> {
        let from = from.as_ref().to_owned();
        let to = to.as_ref().to_owned();
        let executor = self.executor.clone();
        async move {
            executor
                .spawn_blocking(move || std::fs::rename(from, to))
                .await
                .map_err(join_error)?
        }
    }

    fn copy<P: AsRef<Path>, Q: AsRef<Path>>(
        &self,
        from: P,
        to: Q,
    ) -> impl Future<Output = io::Result<u64>> {
        let from = from.as_ref().to_owned();
        let to = to.as_ref().to_owned();
        let executor = self.executor.clone();
        async move {
            executor
                .spawn_blocking(move || std::fs::copy(from, to))
                .await
                .map_err(join_error)?
        }
    }

    fn canonicalize<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<PathBuf>> {
        let path = path.as_ref().to_owned();
        let executor = self.executor.clone();
        async move {
            executor
                .spawn_blocking(move || std::fs::canonicalize(path))
                .await
                .map_err(join_error)?
        }
    }

    fn try_exists<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<bool>> {
        let path = path.as_ref().to_owned();
        let executor = self.executor.clone();
        async move {
            executor
                .spawn_blocking(move || path.try_exists())
                .await
                .map_err(join_error)?
        }
    }
}

impl<E> BlockingFileSystem<E>
where
    E: ExecutorBlocking + Clone + Send + Sync + 'static,
{
    async fn run_path<P, F>(&self, path: P, operation: F) -> io::Result<()>
    where
        P: AsRef<Path>,
        F: FnOnce(PathBuf) -> io::Result<()> + Send + 'static,
    {
        let path = path.as_ref().to_owned();
        self.executor
            .spawn_blocking(move || operation(path))
            .await
            .map_err(join_error)?
    }
}

/// An iterator over directory entries read through a blocking pool.
pub struct BlockingReadDir<E> {
    inner: Arc<Mutex<std::fs::ReadDir>>,
    executor: E,
}

impl<E> BlockingReadDir<E> {
    fn new(read_dir: std::fs::ReadDir, executor: E) -> Self {
        Self {
            inner: Arc::new(Mutex::new(read_dir)),
            executor,
        }
    }
}

impl<E> BlockingReadDir<E>
where
    E: ExecutorBlocking + Clone + Send + Sync + 'static,
{
    /// Returns the next entry in the directory.
    pub async fn next_entry(&mut self) -> io::Result<Option<BlockingDirEntry<E>>> {
        let read_dir = self.inner.clone();
        let executor = self.executor.clone();
        let entry = executor
            .spawn_blocking(move || read_dir.lock().next().transpose())
            .await
            .map_err(join_error)??;
        Ok(entry.map(|entry| BlockingDirEntry::new(entry, executor)))
    }
}

impl<E> Debug for BlockingReadDir<E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BlockingReadDir").finish()
    }
}

/// A directory entry read through a blocking pool.
pub struct BlockingDirEntry<E> {
    inner: Arc<std::fs::DirEntry>,
    executor: E,
}

impl<E> BlockingDirEntry<E> {
    fn new(entry: std::fs::DirEntry, executor: E) -> Self {
        Self {
            inner: Arc::new(entry),
            executor,
        }
    }

    /// Returns the full path for this entry.
    pub fn path(&self) -> PathBuf {
        self.inner.path()
    }

    /// Returns the file name for this entry.
    pub fn file_name(&self) -> OsString {
        self.inner.file_name()
    }
}

impl<E> BlockingDirEntry<E>
where
    E: ExecutorBlocking + Clone + Send + Sync + 'static,
{
    /// Reads metadata for this entry.
    pub async fn metadata(&self) -> io::Result<Metadata> {
        let entry = self.inner.clone();
        self.executor
            .spawn_blocking(move || entry.metadata().map(Metadata::from_native))
            .await
            .map_err(join_error)?
    }

    /// Returns the file type for this entry.
    pub async fn file_type(&self) -> io::Result<FileType> {
        let entry = self.inner.clone();
        self.executor
            .spawn_blocking(move || entry.file_type().map(FileType::from_native))
            .await
            .map_err(join_error)?
    }
}

impl<E> Debug for BlockingDirEntry<E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BlockingDirEntry")
            .field("path", &self.path())
            .finish()
    }
}

/// A file opened through [`BlockingFileSystem`].
pub struct BlockingFile<E> {
    pub(super) inner: Arc<std::fs::File>,
    pub(super) executor: E,
}

impl<E> BlockingFile<E> {
    fn new(file: std::fs::File, executor: E) -> Self {
        Self {
            inner: Arc::new(file),
            executor,
        }
    }
}

impl<E> BlockingFile<E>
where
    E: ExecutorBlocking + Clone + Send + Sync + 'static,
{
    /// Reads metadata for this file.
    pub async fn metadata(&self) -> io::Result<Metadata> {
        let file = self.inner.clone();
        self.executor
            .spawn_blocking(move || file.metadata().map(Metadata::from_native))
            .await
            .map_err(join_error)?
    }

    /// Changes the size of this file.
    pub async fn set_len(&self, size: u64) -> io::Result<()> {
        let file = self.inner.clone();
        self.executor
            .spawn_blocking(move || file.set_len(size))
            .await
            .map_err(join_error)?
    }

    /// Changes the permissions for this file.
    pub async fn set_permissions(&self, permissions: Permissions) -> io::Result<()> {
        let file = self.inner.clone();
        self.executor
            .spawn_blocking(move || {
                let mut native = file.metadata()?.permissions();
                native.set_readonly(permissions.readonly());
                file.set_permissions(native)
            })
            .await
            .map_err(join_error)?
    }

    /// Synchronizes file contents and metadata to disk.
    pub async fn sync_all(&self) -> io::Result<()> {
        let file = self.inner.clone();
        self.executor
            .spawn_blocking(move || file.sync_all())
            .await
            .map_err(join_error)?
    }

    /// Synchronizes file contents to disk.
    pub async fn sync_data(&self) -> io::Result<()> {
        let file = self.inner.clone();
        self.executor
            .spawn_blocking(move || file.sync_data())
            .await
            .map_err(join_error)?
    }
}

impl<E> FileRead for BlockingFile<E>
where
    E: ExecutorBlocking + Clone + Send + Sync + 'static,
{
    async fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let file = self.inner.clone();
        let scratch = vec![0; buffer.len()];
        let (result, scratch) = self
            .executor
            .spawn_blocking(move || {
                let mut file = &*file;
                let mut scratch = scratch;
                let result = std::io::Read::read(&mut file, &mut scratch);
                (result, scratch)
            })
            .await
            .map_err(join_error)?;
        let read = result?;
        buffer[..read].copy_from_slice(&scratch[..read]);
        Ok(read)
    }
}

impl<E> FileWrite for BlockingFile<E>
where
    E: ExecutorBlocking + Clone + Send + Sync + 'static,
{
    async fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let file = self.inner.clone();
        let buffer = buffer.to_owned();
        self.executor
            .spawn_blocking(move || {
                let mut file = &*file;
                std::io::Write::write(&mut file, &buffer)
            })
            .await
            .map_err(join_error)?
    }

    async fn flush(&mut self) -> io::Result<()> {
        let file = self.inner.clone();
        self.executor
            .spawn_blocking(move || {
                let mut file = &*file;
                std::io::Write::flush(&mut file)
            })
            .await
            .map_err(join_error)?
    }
}

impl<E> FileSeek for BlockingFile<E>
where
    E: ExecutorBlocking + Clone + Send + Sync + 'static,
{
    async fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        let file = self.inner.clone();
        self.executor
            .spawn_blocking(move || {
                let mut file = &*file;
                std::io::Seek::seek(&mut file, position)
            })
            .await
            .map_err(join_error)?
    }
}

impl<E> Debug for BlockingFile<E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BlockingFile").finish()
    }
}

fn join_error(error: JoinError) -> io::Error {
    io::Error::other(error)
}
