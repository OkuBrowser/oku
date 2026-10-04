#[cfg(any(
    target_arch = "wasm32",
    feature = "tokio",
    feature = "smol",
    feature = "compio",
    feature = "threadpool",
    feature = "lite"
))]
use super::FileInner;
use super::{
    File, FileSystem, FileSystemDirectories, FileSystemMetadata, FileSystemOpenOptions,
    FileSystemPermissions, Metadata, OpenOptions, Permissions, ReadDir,
};
use crate::global::BuiltinExecutor;
use std::future::Future;
use std::io;
use std::path::{Path, PathBuf};

/// A built-in filesystem implementation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuiltinFileSystem {
    /// Tokio's filesystem.
    #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
    Tokio,
    /// Smol's filesystem.
    #[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
    Smol,
    /// Compio's filesystem.
    #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
    Compio,
    /// A filesystem backed by the thread-pool executor.
    #[cfg(all(feature = "threadpool", not(target_arch = "wasm32")))]
    ThreadPool,
    /// A filesystem backed by the Lite executor.
    #[cfg(all(feature = "lite", not(target_arch = "wasm32")))]
    Lite,
    /// Browser filesystem backed by OPFS.
    #[cfg(target_arch = "wasm32")]
    Wasm,
    /// No filesystem backend is available.
    Unavailable,
}

impl Default for BuiltinFileSystem {
    fn default() -> Self {
        Self::from(BuiltinExecutor::default())
    }
}

impl From<BuiltinExecutor> for BuiltinFileSystem {
    fn from(executor: BuiltinExecutor) -> Self {
        match executor {
            #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
            BuiltinExecutor::Tokio => Self::Tokio,
            #[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
            BuiltinExecutor::Smol => Self::Smol,
            #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
            BuiltinExecutor::Compio => Self::Compio,
            #[cfg(all(feature = "threadpool", not(target_arch = "wasm32")))]
            BuiltinExecutor::ThreadPool => Self::ThreadPool,
            #[cfg(all(feature = "lite", not(target_arch = "wasm32")))]
            BuiltinExecutor::Lite => Self::Lite,
            #[cfg(target_arch = "wasm32")]
            BuiltinExecutor::Wasm => Self::Wasm,
            BuiltinExecutor::Dummy => Self::Unavailable,
        }
    }
}

impl FileSystem for BuiltinFileSystem {
    type File = File;

    fn open<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<Self::File>> {
        let filesystem = *self;
        async move {
            match filesystem {
                #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
                Self::Tokio => Ok(File::from_inner(FileInner::Tokio(
                    super::TokioFileSystem.open(path).await?,
                ))),
                #[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
                Self::Smol => Ok(File::from_inner(FileInner::Smol(
                    super::SmolFileSystem.open(path).await?,
                ))),
                #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
                Self::Compio => Ok(File::from_inner(FileInner::Compio(
                    super::CompioFileSystem.open(path).await?,
                ))),
                #[cfg(all(feature = "threadpool", not(target_arch = "wasm32")))]
                Self::ThreadPool => Ok(File::from_inner(FileInner::Blocking(
                    super::BlockingFileSystem::new(BuiltinExecutor::ThreadPool)
                        .open(path)
                        .await?,
                ))),
                #[cfg(all(feature = "lite", not(target_arch = "wasm32")))]
                Self::Lite => Ok(File::from_inner(FileInner::Blocking(
                    super::BlockingFileSystem::new(BuiltinExecutor::Lite)
                        .open(path)
                        .await?,
                ))),
                #[cfg(target_arch = "wasm32")]
                Self::Wasm => Ok(File::from_inner(FileInner::Wasm(
                    super::WasmFileSystem.open(path).await?,
                ))),
                Self::Unavailable => {
                    drop(path);
                    Err(unavailable())
                }
            }
        }
    }

    fn create<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<Self::File>> {
        let filesystem = *self;
        async move {
            match filesystem {
                #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
                Self::Tokio => Ok(File::from_inner(FileInner::Tokio(
                    super::TokioFileSystem.create(path).await?,
                ))),
                #[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
                Self::Smol => Ok(File::from_inner(FileInner::Smol(
                    super::SmolFileSystem.create(path).await?,
                ))),
                #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
                Self::Compio => Ok(File::from_inner(FileInner::Compio(
                    super::CompioFileSystem.create(path).await?,
                ))),
                #[cfg(all(feature = "threadpool", not(target_arch = "wasm32")))]
                Self::ThreadPool => Ok(File::from_inner(FileInner::Blocking(
                    super::BlockingFileSystem::new(BuiltinExecutor::ThreadPool)
                        .create(path)
                        .await?,
                ))),
                #[cfg(all(feature = "lite", not(target_arch = "wasm32")))]
                Self::Lite => Ok(File::from_inner(FileInner::Blocking(
                    super::BlockingFileSystem::new(BuiltinExecutor::Lite)
                        .create(path)
                        .await?,
                ))),
                #[cfg(target_arch = "wasm32")]
                Self::Wasm => Ok(File::from_inner(FileInner::Wasm(
                    super::WasmFileSystem.create(path).await?,
                ))),
                Self::Unavailable => {
                    drop(path);
                    Err(unavailable())
                }
            }
        }
    }

    fn read<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<Vec<u8>>> {
        let filesystem = *self;
        async move {
            match filesystem {
                #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
                Self::Tokio => super::TokioFileSystem.read(path).await,
                #[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
                Self::Smol => super::SmolFileSystem.read(path).await,
                #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
                Self::Compio => super::CompioFileSystem.read(path).await,
                #[cfg(all(feature = "threadpool", not(target_arch = "wasm32")))]
                Self::ThreadPool => {
                    super::BlockingFileSystem::new(BuiltinExecutor::ThreadPool)
                        .read(path)
                        .await
                }
                #[cfg(all(feature = "lite", not(target_arch = "wasm32")))]
                Self::Lite => {
                    super::BlockingFileSystem::new(BuiltinExecutor::Lite)
                        .read(path)
                        .await
                }
                #[cfg(target_arch = "wasm32")]
                Self::Wasm => super::WasmFileSystem.read(path).await,
                Self::Unavailable => {
                    drop(path);
                    Err(unavailable())
                }
            }
        }
    }

    fn read_to_string<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<String>> {
        let filesystem = *self;
        async move {
            match filesystem {
                #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
                Self::Tokio => super::TokioFileSystem.read_to_string(path).await,
                #[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
                Self::Smol => super::SmolFileSystem.read_to_string(path).await,
                #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
                Self::Compio => super::CompioFileSystem.read_to_string(path).await,
                #[cfg(all(feature = "threadpool", not(target_arch = "wasm32")))]
                Self::ThreadPool => {
                    super::BlockingFileSystem::new(BuiltinExecutor::ThreadPool)
                        .read_to_string(path)
                        .await
                }
                #[cfg(all(feature = "lite", not(target_arch = "wasm32")))]
                Self::Lite => {
                    super::BlockingFileSystem::new(BuiltinExecutor::Lite)
                        .read_to_string(path)
                        .await
                }
                #[cfg(target_arch = "wasm32")]
                Self::Wasm => super::WasmFileSystem.read_to_string(path).await,
                Self::Unavailable => {
                    drop(path);
                    Err(unavailable())
                }
            }
        }
    }

    fn write<P: AsRef<Path>, C: AsRef<[u8]>>(
        &self,
        path: P,
        contents: C,
    ) -> impl Future<Output = io::Result<()>> {
        let filesystem = *self;
        async move {
            match filesystem {
                #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
                Self::Tokio => super::TokioFileSystem.write(path, contents).await,
                #[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
                Self::Smol => super::SmolFileSystem.write(path, contents).await,
                #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
                Self::Compio => super::CompioFileSystem.write(path, contents).await,
                #[cfg(all(feature = "threadpool", not(target_arch = "wasm32")))]
                Self::ThreadPool => {
                    super::BlockingFileSystem::new(BuiltinExecutor::ThreadPool)
                        .write(path, contents)
                        .await
                }
                #[cfg(all(feature = "lite", not(target_arch = "wasm32")))]
                Self::Lite => {
                    super::BlockingFileSystem::new(BuiltinExecutor::Lite)
                        .write(path, contents)
                        .await
                }
                #[cfg(target_arch = "wasm32")]
                Self::Wasm => super::WasmFileSystem.write(path, contents).await,
                Self::Unavailable => {
                    drop(path);
                    drop(contents);
                    Err(unavailable())
                }
            }
        }
    }
}

impl FileSystemOpenOptions for BuiltinFileSystem {
    fn open_with<P: AsRef<Path>>(
        &self,
        options: &OpenOptions,
        path: P,
    ) -> impl Future<Output = io::Result<Self::File>> {
        let filesystem = *self;
        async move {
            let _ = options;
            match filesystem {
                #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
                Self::Tokio => Ok(File::from_inner(FileInner::Tokio(
                    super::TokioFileSystem.open_with(options, path).await?,
                ))),
                #[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
                Self::Smol => Ok(File::from_inner(FileInner::Smol(
                    super::SmolFileSystem.open_with(options, path).await?,
                ))),
                #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
                Self::Compio => Ok(File::from_inner(FileInner::Compio(
                    super::CompioFileSystem.open_with(options, path).await?,
                ))),
                #[cfg(all(feature = "threadpool", not(target_arch = "wasm32")))]
                Self::ThreadPool => Ok(File::from_inner(FileInner::Blocking(
                    super::BlockingFileSystem::new(BuiltinExecutor::ThreadPool)
                        .open_with(options, path)
                        .await?,
                ))),
                #[cfg(all(feature = "lite", not(target_arch = "wasm32")))]
                Self::Lite => Ok(File::from_inner(FileInner::Blocking(
                    super::BlockingFileSystem::new(BuiltinExecutor::Lite)
                        .open_with(options, path)
                        .await?,
                ))),
                #[cfg(target_arch = "wasm32")]
                Self::Wasm => Ok(File::from_inner(FileInner::Wasm(
                    super::WasmFileSystem.open_with(options, path).await?,
                ))),
                Self::Unavailable => {
                    drop(path);
                    Err(unavailable())
                }
            }
        }
    }
}

impl FileSystemMetadata for BuiltinFileSystem {
    fn metadata<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<Metadata>> {
        let filesystem = *self;
        async move {
            match filesystem {
                #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
                Self::Tokio => super::TokioFileSystem.metadata(path).await,
                #[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
                Self::Smol => super::SmolFileSystem.metadata(path).await,
                #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
                Self::Compio => super::CompioFileSystem.metadata(path).await,
                #[cfg(all(feature = "threadpool", not(target_arch = "wasm32")))]
                Self::ThreadPool => {
                    super::BlockingFileSystem::new(BuiltinExecutor::ThreadPool)
                        .metadata(path)
                        .await
                }
                #[cfg(all(feature = "lite", not(target_arch = "wasm32")))]
                Self::Lite => {
                    super::BlockingFileSystem::new(BuiltinExecutor::Lite)
                        .metadata(path)
                        .await
                }
                #[cfg(target_arch = "wasm32")]
                Self::Wasm => super::WasmFileSystem.metadata(path).await,
                Self::Unavailable => {
                    drop(path);
                    Err(unavailable())
                }
            }
        }
    }

    fn symlink_metadata<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> impl Future<Output = io::Result<Metadata>> {
        let filesystem = *self;
        async move {
            match filesystem {
                #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
                Self::Tokio => super::TokioFileSystem.symlink_metadata(path).await,
                #[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
                Self::Smol => super::SmolFileSystem.symlink_metadata(path).await,
                #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
                Self::Compio => super::CompioFileSystem.symlink_metadata(path).await,
                #[cfg(all(feature = "threadpool", not(target_arch = "wasm32")))]
                Self::ThreadPool => {
                    super::BlockingFileSystem::new(BuiltinExecutor::ThreadPool)
                        .symlink_metadata(path)
                        .await
                }
                #[cfg(all(feature = "lite", not(target_arch = "wasm32")))]
                Self::Lite => {
                    super::BlockingFileSystem::new(BuiltinExecutor::Lite)
                        .symlink_metadata(path)
                        .await
                }
                #[cfg(target_arch = "wasm32")]
                Self::Wasm => super::WasmFileSystem.symlink_metadata(path).await,
                Self::Unavailable => {
                    drop(path);
                    Err(unavailable())
                }
            }
        }
    }
}

impl FileSystemPermissions for BuiltinFileSystem {
    fn set_permissions<P: AsRef<Path>>(
        &self,
        path: P,
        permissions: Permissions,
    ) -> impl Future<Output = io::Result<()>> {
        let filesystem = *self;
        async move {
            match filesystem {
                #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
                Self::Tokio => {
                    super::TokioFileSystem
                        .set_permissions(path, permissions)
                        .await
                }
                #[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
                Self::Smol => {
                    super::SmolFileSystem
                        .set_permissions(path, permissions)
                        .await
                }
                #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
                Self::Compio => {
                    super::CompioFileSystem
                        .set_permissions(path, permissions)
                        .await
                }
                #[cfg(all(feature = "threadpool", not(target_arch = "wasm32")))]
                Self::ThreadPool => {
                    super::BlockingFileSystem::new(BuiltinExecutor::ThreadPool)
                        .set_permissions(path, permissions)
                        .await
                }
                #[cfg(all(feature = "lite", not(target_arch = "wasm32")))]
                Self::Lite => {
                    super::BlockingFileSystem::new(BuiltinExecutor::Lite)
                        .set_permissions(path, permissions)
                        .await
                }
                #[cfg(target_arch = "wasm32")]
                Self::Wasm => {
                    super::WasmFileSystem
                        .set_permissions(path, permissions)
                        .await
                }
                Self::Unavailable => {
                    drop(path);
                    let _ = permissions;
                    Err(unavailable())
                }
            }
        }
    }
}

impl FileSystemDirectories for BuiltinFileSystem {
    type ReadDir = ReadDir;

    async fn create_dir<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        match *self {
            #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
            Self::Tokio => super::TokioFileSystem.create_dir(path).await,
            #[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
            Self::Smol => super::SmolFileSystem.create_dir(path).await,
            #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
            Self::Compio => super::CompioFileSystem.create_dir(path).await,
            #[cfg(all(feature = "threadpool", not(target_arch = "wasm32")))]
            Self::ThreadPool => {
                super::BlockingFileSystem::new(BuiltinExecutor::ThreadPool)
                    .create_dir(path)
                    .await
            }
            #[cfg(all(feature = "lite", not(target_arch = "wasm32")))]
            Self::Lite => {
                super::BlockingFileSystem::new(BuiltinExecutor::Lite)
                    .create_dir(path)
                    .await
            }
            #[cfg(target_arch = "wasm32")]
            Self::Wasm => super::WasmFileSystem.create_dir(path).await,
            Self::Unavailable => {
                drop(path);
                Err(unavailable())
            }
        }
    }

    async fn create_dir_all<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        match *self {
            #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
            Self::Tokio => super::TokioFileSystem.create_dir_all(path).await,
            #[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
            Self::Smol => super::SmolFileSystem.create_dir_all(path).await,
            #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
            Self::Compio => super::CompioFileSystem.create_dir_all(path).await,
            #[cfg(all(feature = "threadpool", not(target_arch = "wasm32")))]
            Self::ThreadPool => {
                super::BlockingFileSystem::new(BuiltinExecutor::ThreadPool)
                    .create_dir_all(path)
                    .await
            }
            #[cfg(all(feature = "lite", not(target_arch = "wasm32")))]
            Self::Lite => {
                super::BlockingFileSystem::new(BuiltinExecutor::Lite)
                    .create_dir_all(path)
                    .await
            }
            #[cfg(target_arch = "wasm32")]
            Self::Wasm => super::WasmFileSystem.create_dir_all(path).await,
            Self::Unavailable => {
                drop(path);
                Err(unavailable())
            }
        }
    }

    async fn read_dir<P: AsRef<Path>>(&self, path: P) -> io::Result<Self::ReadDir> {
        match *self {
            #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
            Self::Tokio => Ok(ReadDir::from_tokio(
                super::TokioFileSystem.read_dir(path).await?,
            )),
            #[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
            Self::Smol => Ok(ReadDir::from_smol(
                super::SmolFileSystem.read_dir(path).await?,
            )),
            #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
            Self::Compio => Ok(ReadDir::from_blocking(
                super::CompioFileSystem.read_dir(path).await?,
            )),
            #[cfg(all(feature = "threadpool", not(target_arch = "wasm32")))]
            Self::ThreadPool => Ok(ReadDir::from_blocking(
                super::BlockingFileSystem::new(BuiltinExecutor::ThreadPool)
                    .read_dir(path)
                    .await?,
            )),
            #[cfg(all(feature = "lite", not(target_arch = "wasm32")))]
            Self::Lite => Ok(ReadDir::from_blocking(
                super::BlockingFileSystem::new(BuiltinExecutor::Lite)
                    .read_dir(path)
                    .await?,
            )),
            #[cfg(target_arch = "wasm32")]
            Self::Wasm => Ok(ReadDir::from_wasm(
                super::WasmFileSystem.read_dir(path).await?,
            )),
            Self::Unavailable => {
                drop(path);
                Err(unavailable())
            }
        }
    }

    async fn remove_dir<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        match *self {
            #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
            Self::Tokio => super::TokioFileSystem.remove_dir(path).await,
            #[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
            Self::Smol => super::SmolFileSystem.remove_dir(path).await,
            #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
            Self::Compio => super::CompioFileSystem.remove_dir(path).await,
            #[cfg(all(feature = "threadpool", not(target_arch = "wasm32")))]
            Self::ThreadPool => {
                super::BlockingFileSystem::new(BuiltinExecutor::ThreadPool)
                    .remove_dir(path)
                    .await
            }
            #[cfg(all(feature = "lite", not(target_arch = "wasm32")))]
            Self::Lite => {
                super::BlockingFileSystem::new(BuiltinExecutor::Lite)
                    .remove_dir(path)
                    .await
            }
            #[cfg(target_arch = "wasm32")]
            Self::Wasm => super::WasmFileSystem.remove_dir(path).await,
            Self::Unavailable => {
                drop(path);
                Err(unavailable())
            }
        }
    }

    async fn remove_dir_all<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        match *self {
            #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
            Self::Tokio => super::TokioFileSystem.remove_dir_all(path).await,
            #[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
            Self::Smol => super::SmolFileSystem.remove_dir_all(path).await,
            #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
            Self::Compio => super::CompioFileSystem.remove_dir_all(path).await,
            #[cfg(all(feature = "threadpool", not(target_arch = "wasm32")))]
            Self::ThreadPool => {
                super::BlockingFileSystem::new(BuiltinExecutor::ThreadPool)
                    .remove_dir_all(path)
                    .await
            }
            #[cfg(all(feature = "lite", not(target_arch = "wasm32")))]
            Self::Lite => {
                super::BlockingFileSystem::new(BuiltinExecutor::Lite)
                    .remove_dir_all(path)
                    .await
            }
            #[cfg(target_arch = "wasm32")]
            Self::Wasm => super::WasmFileSystem.remove_dir_all(path).await,
            Self::Unavailable => {
                drop(path);
                Err(unavailable())
            }
        }
    }

    async fn remove_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        match *self {
            #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
            Self::Tokio => super::TokioFileSystem.remove_file(path).await,
            #[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
            Self::Smol => super::SmolFileSystem.remove_file(path).await,
            #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
            Self::Compio => super::CompioFileSystem.remove_file(path).await,
            #[cfg(all(feature = "threadpool", not(target_arch = "wasm32")))]
            Self::ThreadPool => {
                super::BlockingFileSystem::new(BuiltinExecutor::ThreadPool)
                    .remove_file(path)
                    .await
            }
            #[cfg(all(feature = "lite", not(target_arch = "wasm32")))]
            Self::Lite => {
                super::BlockingFileSystem::new(BuiltinExecutor::Lite)
                    .remove_file(path)
                    .await
            }
            #[cfg(target_arch = "wasm32")]
            Self::Wasm => super::WasmFileSystem.remove_file(path).await,
            Self::Unavailable => {
                drop(path);
                Err(unavailable())
            }
        }
    }

    async fn rename<P: AsRef<Path>, Q: AsRef<Path>>(&self, from: P, to: Q) -> io::Result<()> {
        match *self {
            #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
            Self::Tokio => super::TokioFileSystem.rename(from, to).await,
            #[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
            Self::Smol => super::SmolFileSystem.rename(from, to).await,
            #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
            Self::Compio => super::CompioFileSystem.rename(from, to).await,
            #[cfg(all(feature = "threadpool", not(target_arch = "wasm32")))]
            Self::ThreadPool => {
                super::BlockingFileSystem::new(BuiltinExecutor::ThreadPool)
                    .rename(from, to)
                    .await
            }
            #[cfg(all(feature = "lite", not(target_arch = "wasm32")))]
            Self::Lite => {
                super::BlockingFileSystem::new(BuiltinExecutor::Lite)
                    .rename(from, to)
                    .await
            }
            #[cfg(target_arch = "wasm32")]
            Self::Wasm => super::WasmFileSystem.rename(from, to).await,
            Self::Unavailable => {
                drop(from);
                drop(to);
                Err(unavailable())
            }
        }
    }

    async fn copy<P: AsRef<Path>, Q: AsRef<Path>>(&self, from: P, to: Q) -> io::Result<u64> {
        match *self {
            #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
            Self::Tokio => super::TokioFileSystem.copy(from, to).await,
            #[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
            Self::Smol => super::SmolFileSystem.copy(from, to).await,
            #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
            Self::Compio => super::CompioFileSystem.copy(from, to).await,
            #[cfg(all(feature = "threadpool", not(target_arch = "wasm32")))]
            Self::ThreadPool => {
                super::BlockingFileSystem::new(BuiltinExecutor::ThreadPool)
                    .copy(from, to)
                    .await
            }
            #[cfg(all(feature = "lite", not(target_arch = "wasm32")))]
            Self::Lite => {
                super::BlockingFileSystem::new(BuiltinExecutor::Lite)
                    .copy(from, to)
                    .await
            }
            #[cfg(target_arch = "wasm32")]
            Self::Wasm => super::WasmFileSystem.copy(from, to).await,
            Self::Unavailable => {
                drop(from);
                drop(to);
                Err(unavailable())
            }
        }
    }

    async fn canonicalize<P: AsRef<Path>>(&self, path: P) -> io::Result<PathBuf> {
        match *self {
            #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
            Self::Tokio => super::TokioFileSystem.canonicalize(path).await,
            #[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
            Self::Smol => super::SmolFileSystem.canonicalize(path).await,
            #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
            Self::Compio => super::CompioFileSystem.canonicalize(path).await,
            #[cfg(all(feature = "threadpool", not(target_arch = "wasm32")))]
            Self::ThreadPool => {
                super::BlockingFileSystem::new(BuiltinExecutor::ThreadPool)
                    .canonicalize(path)
                    .await
            }
            #[cfg(all(feature = "lite", not(target_arch = "wasm32")))]
            Self::Lite => {
                super::BlockingFileSystem::new(BuiltinExecutor::Lite)
                    .canonicalize(path)
                    .await
            }
            #[cfg(target_arch = "wasm32")]
            Self::Wasm => super::WasmFileSystem.canonicalize(path).await,
            Self::Unavailable => {
                drop(path);
                Err(unavailable())
            }
        }
    }

    async fn try_exists<P: AsRef<Path>>(&self, path: P) -> io::Result<bool> {
        match *self {
            #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
            Self::Tokio => super::TokioFileSystem.try_exists(path).await,
            #[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
            Self::Smol => super::SmolFileSystem.try_exists(path).await,
            #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
            Self::Compio => super::CompioFileSystem.try_exists(path).await,
            #[cfg(all(feature = "threadpool", not(target_arch = "wasm32")))]
            Self::ThreadPool => {
                super::BlockingFileSystem::new(BuiltinExecutor::ThreadPool)
                    .try_exists(path)
                    .await
            }
            #[cfg(all(feature = "lite", not(target_arch = "wasm32")))]
            Self::Lite => {
                super::BlockingFileSystem::new(BuiltinExecutor::Lite)
                    .try_exists(path)
                    .await
            }
            #[cfg(target_arch = "wasm32")]
            Self::Wasm => super::WasmFileSystem.try_exists(path).await,
            Self::Unavailable => {
                drop(path);
                Err(unavailable())
            }
        }
    }
}

fn unavailable() -> io::Error {
    io::Error::new(
        io::ErrorKind::Unsupported,
        "no filesystem backend is available",
    )
}
