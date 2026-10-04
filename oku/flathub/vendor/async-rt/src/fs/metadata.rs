use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::io;
use std::time::SystemTime;

/// Metadata information about a file or directory.
#[derive(Clone)]
pub struct Metadata {
    inner: MetadataInner,
}

#[derive(Clone)]
enum MetadataInner {
    #[cfg(not(target_arch = "wasm32"))]
    Native(std::fs::Metadata),
    #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
    Compio(::compio::fs::Metadata),
    #[cfg(target_arch = "wasm32")]
    Wasm {
        file_type: WasmFileType,
        len: u64,
        modified: Option<SystemTime>,
    },
}

impl Metadata {
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn from_native(metadata: std::fs::Metadata) -> Self {
        Self {
            inner: MetadataInner::Native(metadata),
        }
    }

    #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
    pub(super) fn from_compio(metadata: ::compio::fs::Metadata) -> Self {
        Self {
            inner: MetadataInner::Compio(metadata),
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) fn wasm_file(len: u64, modified: SystemTime) -> Self {
        Self {
            inner: MetadataInner::Wasm {
                file_type: WasmFileType::File,
                len,
                modified: Some(modified),
            },
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) fn wasm_directory() -> Self {
        Self {
            inner: MetadataInner::Wasm {
                file_type: WasmFileType::Directory,
                len: 0,
                modified: None,
            },
        }
    }

    /// Returns the file type for this metadata.
    pub fn file_type(&self) -> FileType {
        let inner = match &self.inner {
            #[cfg(not(target_arch = "wasm32"))]
            MetadataInner::Native(metadata) => FileTypeInner::Native(metadata.file_type()),
            #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
            MetadataInner::Compio(metadata) => FileTypeInner::Compio(metadata.file_type()),
            #[cfg(target_arch = "wasm32")]
            MetadataInner::Wasm { file_type, .. } => FileTypeInner::Wasm(*file_type),
        };
        FileType { inner }
    }

    /// Returns whether this metadata is for a directory.
    pub fn is_dir(&self) -> bool {
        self.file_type().is_dir()
    }

    /// Returns whether this metadata is for a regular file.
    pub fn is_file(&self) -> bool {
        self.file_type().is_file()
    }

    /// Returns whether this metadata is for a symbolic link.
    pub fn is_symlink(&self) -> bool {
        self.file_type().is_symlink()
    }

    /// Returns the file size in bytes.
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> u64 {
        match &self.inner {
            #[cfg(not(target_arch = "wasm32"))]
            MetadataInner::Native(metadata) => metadata.len(),
            #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
            MetadataInner::Compio(metadata) => metadata.len(),
            #[cfg(target_arch = "wasm32")]
            MetadataInner::Wasm { len, .. } => *len,
        }
    }

    /// Returns the permissions for this entry.
    pub fn permissions(&self) -> Permissions {
        let readonly = match &self.inner {
            #[cfg(not(target_arch = "wasm32"))]
            MetadataInner::Native(metadata) => metadata.permissions().readonly(),
            #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
            MetadataInner::Compio(metadata) => metadata.permissions().readonly(),
            #[cfg(target_arch = "wasm32")]
            MetadataInner::Wasm { .. } => false,
        };
        Permissions { readonly }
    }

    /// Returns the last modification time.
    pub fn modified(&self) -> io::Result<SystemTime> {
        match &self.inner {
            #[cfg(not(target_arch = "wasm32"))]
            MetadataInner::Native(metadata) => metadata.modified(),
            #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
            MetadataInner::Compio(metadata) => metadata.modified(),
            #[cfg(target_arch = "wasm32")]
            MetadataInner::Wasm {
                modified: Some(modified),
                ..
            } => Ok(*modified),
            #[cfg(target_arch = "wasm32")]
            MetadataInner::Wasm { modified: None, .. } => Err(unsupported_time()),
        }
    }

    /// Returns the last access time.
    pub fn accessed(&self) -> io::Result<SystemTime> {
        match &self.inner {
            #[cfg(not(target_arch = "wasm32"))]
            MetadataInner::Native(metadata) => metadata.accessed(),
            #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
            MetadataInner::Compio(metadata) => metadata.accessed(),
            #[cfg(target_arch = "wasm32")]
            MetadataInner::Wasm { .. } => Err(unsupported_time()),
        }
    }

    /// Returns the creation time.
    pub fn created(&self) -> io::Result<SystemTime> {
        match &self.inner {
            #[cfg(not(target_arch = "wasm32"))]
            MetadataInner::Native(metadata) => metadata.created(),
            #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
            MetadataInner::Compio(metadata) => metadata.created(),
            #[cfg(target_arch = "wasm32")]
            MetadataInner::Wasm { .. } => Err(unsupported_time()),
        }
    }
}

impl Debug for Metadata {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Metadata")
            .field("file_type", &self.file_type())
            .field("len", &self.len())
            .field("permissions", &self.permissions())
            .field("modified", &self.modified())
            .field("accessed", &self.accessed())
            .field("created", &self.created())
            .finish()
    }
}

/// The type of a filesystem entry.
#[derive(Clone, Copy, Debug)]
pub struct FileType {
    inner: FileTypeInner,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum FileTypeInner {
    #[cfg(not(target_arch = "wasm32"))]
    Native(std::fs::FileType),
    #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
    Compio(::compio::fs::FileType),
    #[cfg(target_arch = "wasm32")]
    Wasm(WasmFileType),
}

impl FileType {
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn from_native(file_type: std::fs::FileType) -> Self {
        Self {
            inner: FileTypeInner::Native(file_type),
        }
    }

    /// Returns whether this is a directory.
    pub fn is_dir(&self) -> bool {
        match &self.inner {
            #[cfg(not(target_arch = "wasm32"))]
            FileTypeInner::Native(file_type) => file_type.is_dir(),
            #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
            FileTypeInner::Compio(file_type) => file_type.is_dir(),
            #[cfg(target_arch = "wasm32")]
            FileTypeInner::Wasm(file_type) => *file_type == WasmFileType::Directory,
        }
    }

    /// Returns whether this is a regular file.
    pub fn is_file(&self) -> bool {
        match &self.inner {
            #[cfg(not(target_arch = "wasm32"))]
            FileTypeInner::Native(file_type) => file_type.is_file(),
            #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
            FileTypeInner::Compio(file_type) => file_type.is_file(),
            #[cfg(target_arch = "wasm32")]
            FileTypeInner::Wasm(file_type) => *file_type == WasmFileType::File,
        }
    }

    /// Returns whether this is a symbolic link.
    pub fn is_symlink(&self) -> bool {
        match &self.inner {
            #[cfg(not(target_arch = "wasm32"))]
            FileTypeInner::Native(file_type) => file_type.is_symlink(),
            #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
            FileTypeInner::Compio(file_type) => file_type.is_symlink(),
            #[cfg(target_arch = "wasm32")]
            FileTypeInner::Wasm(_) => false,
        }
    }
}

impl PartialEq for FileType {
    fn eq(&self, other: &Self) -> bool {
        self.is_dir() == other.is_dir()
            && self.is_file() == other.is_file()
            && self.is_symlink() == other.is_symlink()
    }
}

impl Eq for FileType {}

impl Hash for FileType {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.is_dir().hash(state);
        self.is_file().hash(state);
        self.is_symlink().hash(state);
    }
}

/// Filesystem permissions.
#[derive(Clone, Debug)]
pub struct Permissions {
    readonly: bool,
}

impl Permissions {
    /// Returns whether these permissions are read-only.
    pub fn readonly(&self) -> bool {
        self.readonly
    }

    /// Changes the read-only flag on this value.
    pub fn set_readonly(&mut self, readonly: bool) {
        self.readonly = readonly;
    }
}

impl PartialEq for Permissions {
    fn eq(&self, other: &Self) -> bool {
        self.readonly() == other.readonly()
    }
}

impl Eq for Permissions {}

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum WasmFileType {
    File,
    Directory,
}

#[cfg(target_arch = "wasm32")]
fn unsupported_time() -> io::Error {
    io::Error::new(
        io::ErrorKind::Unsupported,
        "OPFS does not provide this timestamp",
    )
}
