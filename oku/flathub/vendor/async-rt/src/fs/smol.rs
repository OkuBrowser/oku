use super::{
    FileRead, FileSeek, FileSystem, FileSystemDirectories, FileSystemMetadata,
    FileSystemOpenOptions, FileSystemPermissions, FileWrite, Metadata, OpenOptions, Permissions,
};
use std::future::Future;
use std::io;
use std::io::SeekFrom;
use std::path::{Path, PathBuf};

/// Smol's filesystem implementation.
#[derive(Clone, Copy, Debug, Default)]
pub struct SmolFileSystem;

impl FileSystem for SmolFileSystem {
    type File = ::smol::fs::File;

    fn open<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<Self::File>> {
        ::smol::fs::File::open(path)
    }

    fn create<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<Self::File>> {
        ::smol::fs::File::create(path)
    }

    fn read<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<Vec<u8>>> {
        ::smol::fs::read(path)
    }

    fn read_to_string<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<String>> {
        ::smol::fs::read_to_string(path)
    }

    fn write<P: AsRef<Path>, C: AsRef<[u8]>>(
        &self,
        path: P,
        contents: C,
    ) -> impl Future<Output = io::Result<()>> {
        ::smol::fs::write(path, contents)
    }
}

impl FileSystemOpenOptions for SmolFileSystem {
    fn open_with<P: AsRef<Path>>(
        &self,
        options: &OpenOptions,
        path: P,
    ) -> impl Future<Output = io::Result<Self::File>> {
        let path = path.as_ref().to_owned();
        let mut inner = ::smol::fs::OpenOptions::new();
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

impl FileSystemMetadata for SmolFileSystem {
    async fn metadata<P: AsRef<Path>>(&self, path: P) -> io::Result<Metadata> {
        ::smol::fs::metadata(path).await.map(Metadata::from_native)
    }

    async fn symlink_metadata<P: AsRef<Path>>(&self, path: P) -> io::Result<Metadata> {
        ::smol::fs::symlink_metadata(path)
            .await
            .map(Metadata::from_native)
    }
}

impl FileSystemPermissions for SmolFileSystem {
    async fn set_permissions<P: AsRef<Path>>(
        &self,
        path: P,
        permissions: Permissions,
    ) -> io::Result<()> {
        let path = path.as_ref().to_owned();
        let mut native = ::smol::fs::metadata(&path).await?.permissions();
        native.set_readonly(permissions.readonly());
        ::smol::fs::set_permissions(path, native).await
    }
}

impl FileSystemDirectories for SmolFileSystem {
    type ReadDir = ::smol::fs::ReadDir;

    fn create_dir<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<()>> {
        ::smol::fs::create_dir(path)
    }

    fn create_dir_all<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<()>> {
        ::smol::fs::create_dir_all(path)
    }

    fn read_dir<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<Self::ReadDir>> {
        ::smol::fs::read_dir(path)
    }

    fn remove_dir<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<()>> {
        ::smol::fs::remove_dir(path)
    }

    fn remove_dir_all<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<()>> {
        ::smol::fs::remove_dir_all(path)
    }

    fn remove_file<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<()>> {
        ::smol::fs::remove_file(path)
    }

    fn rename<P: AsRef<Path>, Q: AsRef<Path>>(
        &self,
        from: P,
        to: Q,
    ) -> impl Future<Output = io::Result<()>> {
        ::smol::fs::rename(from, to)
    }

    fn copy<P: AsRef<Path>, Q: AsRef<Path>>(
        &self,
        from: P,
        to: Q,
    ) -> impl Future<Output = io::Result<u64>> {
        ::smol::fs::copy(from, to)
    }

    fn canonicalize<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<PathBuf>> {
        ::smol::fs::canonicalize(path)
    }

    async fn try_exists<P: AsRef<Path>>(&self, path: P) -> io::Result<bool> {
        match ::smol::fs::metadata(path).await {
            Ok(_) => Ok(true),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(error),
        }
    }
}

impl FileRead for ::smol::fs::File {
    fn read<'a>(
        &'a mut self,
        buffer: &'a mut [u8],
    ) -> impl Future<Output = io::Result<usize>> + 'a {
        futures::io::AsyncReadExt::read(self, buffer)
    }
}

impl FileWrite for ::smol::fs::File {
    fn write<'a>(&'a mut self, buffer: &'a [u8]) -> impl Future<Output = io::Result<usize>> + 'a {
        futures::io::AsyncWriteExt::write(self, buffer)
    }

    fn flush(&mut self) -> impl Future<Output = io::Result<()>> {
        futures::io::AsyncWriteExt::flush(self)
    }
}

impl FileSeek for ::smol::fs::File {
    fn seek(&mut self, position: SeekFrom) -> impl Future<Output = io::Result<u64>> {
        futures::io::AsyncSeekExt::seek(self, position)
    }
}
