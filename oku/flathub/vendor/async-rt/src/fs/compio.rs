use super::{
    BlockingFileSystem, BlockingReadDir, FileRead, FileSeek, FileSystem, FileSystemDirectories,
    FileSystemMetadata, FileSystemOpenOptions, FileSystemPermissions, FileWrite, Metadata,
    OpenOptions, Permissions,
};
use crate::global::BuiltinExecutor;
use ::compio::io::{AsyncReadAt, AsyncWriteAt};
use std::fmt::{Debug, Formatter};
use std::future::Future;
use std::io;
use std::io::SeekFrom;
use std::path::{Path, PathBuf};

/// Compio's filesystem implementation.
#[derive(Clone, Copy, Debug, Default)]
pub struct CompioFileSystem;

impl FileSystem for CompioFileSystem {
    type File = CompioFile;

    fn open<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<Self::File>> {
        let path = path.as_ref().to_owned();
        async move {
            let file = ::compio::fs::File::open(path).await?;
            Ok(CompioFile::new(file))
        }
    }

    fn create<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<Self::File>> {
        let path = path.as_ref().to_owned();
        async move {
            let file = ::compio::fs::File::create(path).await?;
            Ok(CompioFile::new(file))
        }
    }

    fn read<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<Vec<u8>>> {
        ::compio::fs::read(path)
    }

    async fn read_to_string<P: AsRef<Path>>(&self, path: P) -> io::Result<String> {
        let contents = ::compio::fs::read(path).await?;
        String::from_utf8(contents)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.utf8_error()))
    }

    async fn write<P: AsRef<Path>, C: AsRef<[u8]>>(&self, path: P, contents: C) -> io::Result<()> {
        ::compio::fs::write(path, contents.as_ref().to_owned())
            .await
            .0
    }
}

impl FileSystemOpenOptions for CompioFileSystem {
    async fn open_with<P: AsRef<Path>>(
        &self,
        options: &OpenOptions,
        path: P,
    ) -> io::Result<Self::File> {
        if options.append {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Compio does not support append mode",
            ));
        }

        let mut inner = ::compio::fs::OpenOptions::new();
        inner
            .read(options.read)
            .write(options.write)
            .truncate(options.truncate)
            .create(options.create)
            .create_new(options.create_new);
        let file = inner.open(path).await?;
        Ok(CompioFile::new(file))
    }
}

/// A Compio file with a cursor for the common file interface.
pub struct CompioFile {
    pub(super) inner: ::compio::fs::File,
    pub(super) position: u64,
}

impl CompioFile {
    fn new(inner: ::compio::fs::File) -> Self {
        Self { inner, position: 0 }
    }

    /// Returns the underlying Compio file.
    pub fn get_ref(&self) -> &::compio::fs::File {
        &self.inner
    }

    /// Returns a mutable reference to the underlying Compio file.
    pub fn get_mut(&mut self) -> &mut ::compio::fs::File {
        &mut self.inner
    }

    /// Returns the underlying Compio file.
    pub fn into_inner(self) -> ::compio::fs::File {
        self.inner
    }

    /// Reads metadata for this file.
    pub async fn metadata(&self) -> io::Result<Metadata> {
        self.inner.metadata().await.map(Metadata::from_compio)
    }

    /// Changes the size of this file.
    pub async fn set_len(&self, size: u64) -> io::Result<()> {
        self.inner.set_len(size).await
    }

    /// Changes the permissions for this file.
    pub async fn set_permissions(&self, permissions: Permissions) -> io::Result<()> {
        let mut native = self.inner.metadata().await?.permissions();
        native.set_readonly(permissions.readonly());
        self.inner.set_permissions(native).await
    }

    /// Synchronizes file contents and metadata to disk.
    pub async fn sync_all(&self) -> io::Result<()> {
        self.inner.sync_all().await
    }

    /// Synchronizes file contents to disk.
    pub async fn sync_data(&self) -> io::Result<()> {
        self.inner.sync_data().await
    }
}

impl FileRead for CompioFile {
    async fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let scratch = vec![0; buffer.len()];
        let ::compio::BufResult(result, scratch) = self.inner.read_at(scratch, self.position).await;
        let read = result?;
        buffer[..read].copy_from_slice(&scratch[..read]);
        self.position = self
            .position
            .checked_add(read as u64)
            .ok_or_else(|| io::Error::other("file position overflowed"))?;
        Ok(read)
    }
}

impl FileWrite for CompioFile {
    async fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let ::compio::BufResult(result, _) =
            self.inner.write_at(buffer.to_vec(), self.position).await;
        let written = result?;
        self.position = self
            .position
            .checked_add(written as u64)
            .ok_or_else(|| io::Error::other("file position overflowed"))?;
        Ok(written)
    }

    async fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl FileSeek for CompioFile {
    async fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        let end = match position {
            SeekFrom::End(_) => self.inner.metadata().await?.len(),
            _ => 0,
        };
        self.position = super::file_io::seek_position(self.position, end, position)?;
        Ok(self.position)
    }
}

impl Debug for CompioFile {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompioFile").finish()
    }
}

impl FileSystemMetadata for CompioFileSystem {
    async fn metadata<P: AsRef<Path>>(&self, path: P) -> io::Result<Metadata> {
        ::compio::fs::metadata(path)
            .await
            .map(Metadata::from_compio)
    }

    async fn symlink_metadata<P: AsRef<Path>>(&self, path: P) -> io::Result<Metadata> {
        ::compio::fs::symlink_metadata(path)
            .await
            .map(Metadata::from_compio)
    }
}

impl FileSystemPermissions for CompioFileSystem {
    async fn set_permissions<P: AsRef<Path>>(
        &self,
        path: P,
        permissions: Permissions,
    ) -> io::Result<()> {
        let path = path.as_ref().to_owned();
        let mut native = ::compio::fs::metadata(&path).await?.permissions();
        native.set_readonly(permissions.readonly());
        ::compio::fs::set_permissions(path, native).await
    }
}

impl FileSystemDirectories for CompioFileSystem {
    type ReadDir = BlockingReadDir<BuiltinExecutor>;

    fn create_dir<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<()>> {
        ::compio::fs::create_dir(path)
    }

    fn create_dir_all<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<()>> {
        ::compio::fs::create_dir_all(path)
    }

    fn read_dir<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<Self::ReadDir>> {
        let path = path.as_ref().to_owned();
        async move {
            BlockingFileSystem::new(BuiltinExecutor::Compio)
                .read_dir(path)
                .await
        }
    }

    fn remove_dir<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<()>> {
        ::compio::fs::remove_dir(path)
    }

    fn remove_dir_all<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<()>> {
        let path = path.as_ref().to_owned();
        async move {
            BlockingFileSystem::new(BuiltinExecutor::Compio)
                .remove_dir_all(path)
                .await
        }
    }

    fn remove_file<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<()>> {
        ::compio::fs::remove_file(path)
    }

    fn rename<P: AsRef<Path>, Q: AsRef<Path>>(
        &self,
        from: P,
        to: Q,
    ) -> impl Future<Output = io::Result<()>> {
        ::compio::fs::rename(from, to)
    }

    async fn copy<P: AsRef<Path>, Q: AsRef<Path>>(&self, from: P, to: Q) -> io::Result<u64> {
        let contents = ::compio::fs::read(from).await?;
        let len = contents.len() as u64;
        ::compio::fs::write(to, contents).await.0?;
        Ok(len)
    }

    fn canonicalize<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<PathBuf>> {
        let path = path.as_ref().to_owned();
        async move {
            BlockingFileSystem::new(BuiltinExecutor::Compio)
                .canonicalize(path)
                .await
        }
    }

    async fn try_exists<P: AsRef<Path>>(&self, path: P) -> io::Result<bool> {
        match ::compio::fs::metadata(path).await {
            Ok(_) => Ok(true),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(error),
        }
    }
}
