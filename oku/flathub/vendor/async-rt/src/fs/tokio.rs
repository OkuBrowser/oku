use super::{
    FileRead, FileSeek, FileSystem, FileSystemDirectories, FileSystemMetadata,
    FileSystemOpenOptions, FileSystemPermissions, FileWrite, Metadata, OpenOptions, Permissions,
};
use std::future::Future;
use std::io;
use std::io::SeekFrom;
use std::path::{Path, PathBuf};

/// Tokio's filesystem implementation.
#[derive(Clone, Copy, Debug, Default)]
pub struct TokioFileSystem;

impl FileSystem for TokioFileSystem {
    type File = ::tokio::fs::File;

    fn open<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<Self::File>> {
        ::tokio::fs::File::open(path)
    }

    fn create<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<Self::File>> {
        ::tokio::fs::File::create(path)
    }

    fn read<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<Vec<u8>>> {
        ::tokio::fs::read(path)
    }

    fn read_to_string<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<String>> {
        ::tokio::fs::read_to_string(path)
    }

    fn write<P: AsRef<Path>, C: AsRef<[u8]>>(
        &self,
        path: P,
        contents: C,
    ) -> impl Future<Output = io::Result<()>> {
        ::tokio::fs::write(path, contents)
    }
}

impl FileSystemOpenOptions for TokioFileSystem {
    fn open_with<P: AsRef<Path>>(
        &self,
        options: &OpenOptions,
        path: P,
    ) -> impl Future<Output = io::Result<Self::File>> {
        let path = path.as_ref().to_owned();
        let mut inner = ::tokio::fs::OpenOptions::new();
        inner
            .read(options.read)
            .write(options.write)
            .append(options.append)
            .truncate(options.truncate)
            .create(options.create)
            .create_new(options.create_new);
        async move { inner.open(path).await }
    }
}

impl FileSystemMetadata for TokioFileSystem {
    async fn metadata<P: AsRef<Path>>(&self, path: P) -> io::Result<Metadata> {
        ::tokio::fs::metadata(path).await.map(Metadata::from_native)
    }

    async fn symlink_metadata<P: AsRef<Path>>(&self, path: P) -> io::Result<Metadata> {
        ::tokio::fs::symlink_metadata(path)
            .await
            .map(Metadata::from_native)
    }
}

impl FileSystemPermissions for TokioFileSystem {
    async fn set_permissions<P: AsRef<Path>>(
        &self,
        path: P,
        permissions: Permissions,
    ) -> io::Result<()> {
        let path = path.as_ref().to_owned();
        let mut native = ::tokio::fs::metadata(&path).await?.permissions();
        native.set_readonly(permissions.readonly());
        ::tokio::fs::set_permissions(path, native).await
    }
}

impl FileSystemDirectories for TokioFileSystem {
    type ReadDir = ::tokio::fs::ReadDir;

    fn create_dir<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<()>> {
        ::tokio::fs::create_dir(path)
    }

    fn create_dir_all<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<()>> {
        ::tokio::fs::create_dir_all(path)
    }

    fn read_dir<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<Self::ReadDir>> {
        ::tokio::fs::read_dir(path)
    }

    fn remove_dir<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<()>> {
        ::tokio::fs::remove_dir(path)
    }

    fn remove_dir_all<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<()>> {
        ::tokio::fs::remove_dir_all(path)
    }

    fn remove_file<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<()>> {
        ::tokio::fs::remove_file(path)
    }

    fn rename<P: AsRef<Path>, Q: AsRef<Path>>(
        &self,
        from: P,
        to: Q,
    ) -> impl Future<Output = io::Result<()>> {
        ::tokio::fs::rename(from, to)
    }

    fn copy<P: AsRef<Path>, Q: AsRef<Path>>(
        &self,
        from: P,
        to: Q,
    ) -> impl Future<Output = io::Result<u64>> {
        ::tokio::fs::copy(from, to)
    }

    fn canonicalize<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<PathBuf>> {
        ::tokio::fs::canonicalize(path)
    }

    fn try_exists<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<bool>> {
        ::tokio::fs::try_exists(path)
    }
}

impl FileRead for ::tokio::fs::File {
    fn read<'a>(
        &'a mut self,
        buffer: &'a mut [u8],
    ) -> impl Future<Output = io::Result<usize>> + 'a {
        ::tokio::io::AsyncReadExt::read(self, buffer)
    }
}

impl FileWrite for ::tokio::fs::File {
    fn write<'a>(&'a mut self, buffer: &'a [u8]) -> impl Future<Output = io::Result<usize>> + 'a {
        ::tokio::io::AsyncWriteExt::write(self, buffer)
    }

    fn flush(&mut self) -> impl Future<Output = io::Result<()>> {
        ::tokio::io::AsyncWriteExt::flush(self)
    }
}

impl FileSeek for ::tokio::fs::File {
    fn seek(&mut self, position: SeekFrom) -> impl Future<Output = io::Result<u64>> {
        ::tokio::io::AsyncSeekExt::seek(self, position)
    }
}
