use super::{
    FileRead, FileSeek, FileSystem, FileSystemDirectories, FileSystemMetadata,
    FileSystemOpenOptions, FileSystemPermissions, FileType, FileWrite, Metadata, OpenOptions,
    Permissions,
};
use js_sys::{Array, AsyncIterator, Reflect, Uint8Array, global};
use std::cell::RefCell;
use std::ffi::OsString;
use std::fmt::{Debug, Formatter};
use std::future::Future;
use std::io;
use std::io::SeekFrom;
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, UNIX_EPOCH};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    FileSystemCreateWritableOptions, FileSystemDirectoryHandle, FileSystemFileHandle,
    FileSystemGetDirectoryOptions, FileSystemGetFileOptions, FileSystemRemoveOptions,
    FileSystemWritableFileStream, StorageManager,
};

thread_local! {
    static ROOT: RefCell<Option<FileSystemDirectoryHandle>> = const { RefCell::new(None) };
}

/// Browser filesystem implementation backed by OPFS.
#[derive(Clone, Copy, Debug, Default)]
pub struct WasmFileSystem;

/// A file opened in the browser's OPFS storage.
pub struct WasmFile {
    pub(super) handle: FileSystemFileHandle,
    pub(super) readable: bool,
    pub(super) writable: bool,
    pub(super) append: bool,
    pub(super) position: u64,
}

impl WasmFile {
    /// Reads metadata for this file.
    pub async fn metadata(&self) -> io::Result<Metadata> {
        file_metadata(&self.handle).await
    }

    /// Changes the size of this file.
    pub async fn set_len(&self, size: u64) -> io::Result<()> {
        if !self.writable {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "the file was not opened for writing",
            ));
        }
        truncate(&self.handle, size).await
    }

    /// Changes the permissions for this file.
    pub async fn set_permissions(&self, permissions: Permissions) -> io::Result<()> {
        let _ = permissions;
        Err(unsupported_permissions())
    }

    /// Synchronizes file contents and metadata.
    pub async fn sync_all(&self) -> io::Result<()> {
        Ok(())
    }

    /// Synchronizes file contents.
    pub async fn sync_data(&self) -> io::Result<()> {
        Ok(())
    }
}

impl Debug for WasmFile {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasmFile").finish()
    }
}

impl FileSystem for WasmFileSystem {
    type File = WasmFile;

    fn open<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<Self::File>> {
        let path = path.as_ref().to_owned();
        async move {
            Ok(WasmFile {
                handle: file_at(&path, false).await?,
                readable: true,
                writable: false,
                append: false,
                position: 0,
            })
        }
    }

    fn create<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<Self::File>> {
        let path = path.as_ref().to_owned();
        async move {
            let handle = file_at(&path, true).await?;
            overwrite(&handle, &[]).await?;
            Ok(WasmFile {
                handle,
                readable: false,
                writable: true,
                append: false,
                position: 0,
            })
        }
    }

    fn read<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<Vec<u8>>> {
        let path = path.as_ref().to_owned();
        async move {
            let file = file_at(&path, false).await?;
            read_all(&file).await
        }
    }

    fn read_to_string<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<String>> {
        let path = path.as_ref().to_owned();
        async move {
            let file = file_at(&path, false).await?;
            let contents = read_all(&file).await?;
            String::from_utf8(contents)
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "file is not UTF-8"))
        }
    }

    fn write<P: AsRef<Path>, C: AsRef<[u8]>>(
        &self,
        path: P,
        contents: C,
    ) -> impl Future<Output = io::Result<()>> {
        let path = path.as_ref().to_owned();
        let contents = contents.as_ref().to_owned();
        async move {
            let file = file_at(&path, true).await?;
            overwrite(&file, &contents).await
        }
    }
}

impl FileRead for WasmFile {
    async fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if !self.readable {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "the file was not opened for reading",
            ));
        }
        let contents = read_all(&self.handle).await?;
        let start = usize::try_from(self.position).unwrap_or(usize::MAX);
        if start >= contents.len() || buffer.is_empty() {
            return Ok(0);
        }
        let read = buffer.len().min(contents.len() - start);
        buffer[..read].copy_from_slice(&contents[start..start + read]);
        self.position += read as u64;
        Ok(read)
    }
}

impl FileWrite for WasmFile {
    async fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        if !self.writable {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "the file was not opened for writing",
            ));
        }
        let position = if self.append {
            file_metadata(&self.handle).await?.len()
        } else {
            self.position
        };
        write_at(&self.handle, position, buffer).await?;
        self.position = position + buffer.len() as u64;
        Ok(buffer.len())
    }

    async fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl FileSeek for WasmFile {
    async fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        let end = match position {
            SeekFrom::End(_) => file_metadata(&self.handle).await?.len(),
            _ => 0,
        };
        let position = super::file_io::seek_position(self.position, end, position)?;
        if position > 9_007_199_254_740_991 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "the file position exceeds JavaScript's integer range",
            ));
        }
        self.position = position;
        Ok(position)
    }
}

impl FileSystemOpenOptions for WasmFileSystem {
    fn open_with<P: AsRef<Path>>(
        &self,
        options: &OpenOptions,
        path: P,
    ) -> impl Future<Output = io::Result<Self::File>> {
        let path = path.as_ref().to_owned();
        let options = options.clone();
        async move { open_file(&path, &options).await }
    }
}

impl FileSystemMetadata for WasmFileSystem {
    fn metadata<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<Metadata>> {
        let path = path.as_ref().to_owned();
        async move { path_metadata(&path).await }
    }

    fn symlink_metadata<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> impl Future<Output = io::Result<Metadata>> {
        let path = path.as_ref().to_owned();
        async move { path_metadata(&path).await }
    }
}

impl FileSystemPermissions for WasmFileSystem {
    fn set_permissions<P: AsRef<Path>>(
        &self,
        path: P,
        permissions: Permissions,
    ) -> impl Future<Output = io::Result<()>> {
        async move {
            drop(path);
            let _ = permissions;
            Err(unsupported_permissions())
        }
    }
}

impl FileSystemDirectories for WasmFileSystem {
    type ReadDir = WasmReadDir;

    async fn create_dir<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let mut names = split_path(path.as_ref())?;
        let Some(name) = names.pop() else {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "the OPFS root already exists",
            ));
        };
        let parent = directory_at(&names, false).await?;
        let options = FileSystemGetDirectoryOptions::new();
        options.set_create(false);
        match await_promise(
            parent.get_directory_handle_with_options(&name, &options),
            EntryKind::Directory,
        )
        .await
        {
            Ok(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "the directory already exists",
                ));
            }
            Err(error) if error.kind() == io::ErrorKind::NotADirectory => {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "the path is already in use",
                ));
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }

        let options = FileSystemGetDirectoryOptions::new();
        options.set_create(true);
        await_promise(
            parent.get_directory_handle_with_options(&name, &options),
            EntryKind::Directory,
        )
        .await?;
        Ok(())
    }

    async fn create_dir_all<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let names = split_path(path.as_ref())?;
        directory_at(&names, true).await?;
        Ok(())
    }

    async fn read_dir<P: AsRef<Path>>(&self, path: P) -> io::Result<Self::ReadDir> {
        let names = split_path(path.as_ref())?;
        let directory = directory_at(&names, false).await?;
        Ok(WasmReadDir {
            iterator: directory.entries(),
            path: joined_path(&names),
        })
    }

    async fn remove_dir<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        remove_entry(path.as_ref(), EntryKind::Directory, false).await
    }

    async fn remove_dir_all<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        remove_entry(path.as_ref(), EntryKind::Directory, true).await
    }

    async fn remove_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        remove_entry(path.as_ref(), EntryKind::File, false).await
    }

    async fn rename<P: AsRef<Path>, Q: AsRef<Path>>(&self, from: P, to: Q) -> io::Result<()> {
        let from = joined_path(&split_path(from.as_ref())?);
        let to = joined_path(&split_path(to.as_ref())?);
        if from == to {
            path_metadata(&from).await?;
            return Ok(());
        }

        let metadata = path_metadata(&from).await?;
        if metadata.is_file() {
            self.copy(&from, &to).await?;
            self.remove_file(&from).await
        } else {
            let from_names = split_path(&from)?;
            let to_names = split_path(&to)?;
            if to_names.starts_with(&from_names) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "a directory cannot be moved into itself",
                ));
            }
            copy_directory(&from, &to).await?;
            self.remove_dir_all(&from).await
        }
    }

    async fn copy<P: AsRef<Path>, Q: AsRef<Path>>(&self, from: P, to: Q) -> io::Result<u64> {
        let contents = self.read(from).await?;
        let len = contents.len() as u64;
        self.write(to, contents).await?;
        Ok(len)
    }

    async fn canonicalize<P: AsRef<Path>>(&self, path: P) -> io::Result<PathBuf> {
        let names = split_path(path.as_ref())?;
        let path = joined_path(&names);
        path_metadata(&path).await?;
        Ok(path)
    }

    async fn try_exists<P: AsRef<Path>>(&self, path: P) -> io::Result<bool> {
        match path_metadata(path.as_ref()).await {
            Ok(_) => Ok(true),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(error),
        }
    }
}

/// An iterator over entries in an OPFS directory.
pub struct WasmReadDir {
    iterator: AsyncIterator,
    path: PathBuf,
}

impl WasmReadDir {
    /// Returns the next entry in the directory.
    pub async fn next_entry(&mut self) -> io::Result<Option<WasmDirEntry>> {
        let promise = self
            .iterator
            .next()
            .map_err(|error| js_error(error, EntryKind::Either))?;
        let result = await_promise(promise, EntryKind::Either).await?;
        if Reflect::get(&result, &JsValue::from_str("done"))
            .ok()
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
        {
            return Ok(None);
        }

        let value = Reflect::get(&result, &JsValue::from_str("value"))
            .map_err(|error| js_error(error, EntryKind::Either))?;
        let pair = Array::from(&value);
        let name = pair.get(0).as_string().ok_or_else(|| {
            io::Error::other("OPFS returned a directory entry without a file name")
        })?;
        let handle = pair.get(1);
        let kind = property(&handle, "kind").unwrap_or_default();
        let kind = match kind.as_str() {
            "file" => WasmDirEntryKind::File(cast(handle)?),
            "directory" => {
                let _: FileSystemDirectoryHandle = cast(handle)?;
                WasmDirEntryKind::Directory
            }
            _ => return Err(io::Error::other("OPFS returned an unknown entry type")),
        };
        Ok(Some(WasmDirEntry {
            path: self.path.join(&name),
            name: OsString::from(name),
            kind,
        }))
    }
}

impl Debug for WasmReadDir {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasmReadDir").finish()
    }
}

/// An entry returned by [`WasmReadDir`].
pub struct WasmDirEntry {
    path: PathBuf,
    name: OsString,
    kind: WasmDirEntryKind,
}

enum WasmDirEntryKind {
    File(FileSystemFileHandle),
    Directory,
}

impl WasmDirEntry {
    /// Returns the full path for this entry.
    pub fn path(&self) -> PathBuf {
        self.path.clone()
    }

    /// Returns the file name for this entry.
    pub fn file_name(&self) -> OsString {
        self.name.clone()
    }

    /// Reads metadata for this entry.
    pub async fn metadata(&self) -> io::Result<Metadata> {
        match &self.kind {
            WasmDirEntryKind::File(handle) => file_metadata(handle).await,
            WasmDirEntryKind::Directory => Ok(Metadata::wasm_directory()),
        }
    }

    /// Returns the file type for this entry.
    pub async fn file_type(&self) -> io::Result<FileType> {
        Ok(self.metadata().await?.file_type())
    }
}

impl Debug for WasmDirEntry {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasmDirEntry")
            .field("path", &self.path)
            .finish()
    }
}

async fn remove_entry(path: &Path, kind: EntryKind, recursive: bool) -> io::Result<()> {
    let (directories, name) = split_parent(path)?;
    let directory = directory_at(&directories, false).await?;

    match kind {
        EntryKind::File => {
            let options = FileSystemGetFileOptions::new();
            options.set_create(false);
            await_promise(
                directory.get_file_handle_with_options(&name, &options),
                EntryKind::File,
            )
            .await?;
        }
        EntryKind::Directory => {
            let options = FileSystemGetDirectoryOptions::new();
            options.set_create(false);
            await_promise(
                directory.get_directory_handle_with_options(&name, &options),
                EntryKind::Directory,
            )
            .await?;
        }
        EntryKind::Either => {}
    }

    let options = FileSystemRemoveOptions::new();
    options.set_recursive(recursive);
    await_promise(directory.remove_entry_with_options(&name, &options), kind).await?;
    Ok(())
}

fn copy_directory<'a>(
    from: &'a Path,
    to: &'a Path,
) -> futures::future::LocalBoxFuture<'a, io::Result<()>> {
    Box::pin(async move {
        WasmFileSystem.create_dir_all(to).await?;
        let mut entries = WasmFileSystem.read_dir(from).await?;
        while let Some(entry) = entries.next_entry().await? {
            let source = entry.path();
            let destination = to.join(entry.file_name());
            if entry.file_type().await?.is_dir() {
                copy_directory(&source, &destination).await?;
            } else {
                WasmFileSystem.copy(&source, &destination).await?;
            }
        }
        Ok(())
    })
}

fn joined_path(names: &[String]) -> PathBuf {
    let mut path = PathBuf::from("/");
    for name in names {
        path.push(name);
    }
    path
}

async fn root() -> io::Result<FileSystemDirectoryHandle> {
    if let Some(root) = ROOT.with_borrow(Clone::clone) {
        return Ok(root);
    }

    let navigator = Reflect::get(&global(), &JsValue::from_str("navigator"))
        .map_err(|error| js_error(error, EntryKind::Either))?;
    let storage = Reflect::get(&navigator, &JsValue::from_str("storage"))
        .map_err(|error| js_error(error, EntryKind::Either))?;
    if storage.is_null() || storage.is_undefined() {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "OPFS is unavailable outside a secure browser context",
        ));
    }

    let storage = cast::<StorageManager>(storage)?;
    let root = cast::<FileSystemDirectoryHandle>(
        await_promise(storage.get_directory(), EntryKind::Directory).await?,
    )?;
    ROOT.with_borrow_mut(|cached| *cached = Some(root.clone()));
    Ok(root)
}

async fn file_at(path: &Path, create: bool) -> io::Result<FileSystemFileHandle> {
    let (directories, name) = split_parent(path)?;
    let directory = directory_at(&directories, false).await?;

    let options = FileSystemGetFileOptions::new();
    options.set_create(create);
    let handle = await_promise(
        directory.get_file_handle_with_options(&name, &options),
        EntryKind::File,
    )
    .await?;
    cast(handle)
}

async fn open_file(path: &Path, options: &OpenOptions) -> io::Result<WasmFile> {
    let writable = options.write || options.append;
    if !options.read && !writable {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "a file must be opened for reading or writing",
        ));
    }
    if (options.truncate || options.create || options.create_new) && !writable {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "creating or truncating a file requires write access",
        ));
    }
    if options.truncate && options.append {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "a file cannot be truncated and appended at the same time",
        ));
    }

    if options.create_new {
        match file_at(path, false).await {
            Ok(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "the file already exists",
                ));
            }
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::NotFound | io::ErrorKind::IsADirectory
                ) =>
            {
                if error.kind() == io::ErrorKind::IsADirectory {
                    return Err(io::Error::new(
                        io::ErrorKind::AlreadyExists,
                        "the path is already in use",
                    ));
                }
            }
            Err(error) => return Err(error),
        }
    }

    let handle = file_at(path, options.create || options.create_new).await?;
    if options.truncate {
        overwrite(&handle, &[]).await?;
    }
    Ok(WasmFile {
        handle,
        readable: options.read,
        writable,
        append: options.append,
        position: 0,
    })
}

async fn directory_at(names: &[String], create: bool) -> io::Result<FileSystemDirectoryHandle> {
    let mut directory = root().await?;
    let options = FileSystemGetDirectoryOptions::new();
    options.set_create(create);

    for name in names {
        let handle = await_promise(
            directory.get_directory_handle_with_options(name, &options),
            EntryKind::Directory,
        )
        .await?;
        directory = cast(handle)?;
    }
    Ok(directory)
}

async fn path_metadata(path: &Path) -> io::Result<Metadata> {
    let mut names = split_path(path)?;
    let Some(name) = names.pop() else {
        directory_at(&names, false).await?;
        return Ok(Metadata::wasm_directory());
    };
    let directory = directory_at(&names, false).await?;
    let options = FileSystemGetFileOptions::new();
    options.set_create(false);
    match await_promise(
        directory.get_file_handle_with_options(&name, &options),
        EntryKind::File,
    )
    .await
    {
        Ok(handle) => file_metadata(&cast(handle)?).await,
        Err(error) if error.kind() == io::ErrorKind::IsADirectory => {
            let options = FileSystemGetDirectoryOptions::new();
            options.set_create(false);
            await_promise(
                directory.get_directory_handle_with_options(&name, &options),
                EntryKind::Directory,
            )
            .await?;
            Ok(Metadata::wasm_directory())
        }
        Err(error) => Err(error),
    }
}

async fn file_metadata(handle: &FileSystemFileHandle) -> io::Result<Metadata> {
    let file = get_file(handle).await?;
    let millis = file.last_modified();
    if !millis.is_finite() || millis.is_sign_negative() {
        return Err(io::Error::other("OPFS returned an invalid timestamp"));
    }
    let modified = UNIX_EPOCH
        .checked_add(Duration::from_millis(millis as u64))
        .ok_or_else(|| io::Error::other("OPFS returned an invalid timestamp"))?;
    Ok(Metadata::wasm_file(file.size() as u64, modified))
}

async fn get_file(handle: &FileSystemFileHandle) -> io::Result<web_sys::File> {
    cast(await_promise(handle.get_file(), EntryKind::File).await?)
}

async fn read_all(handle: &FileSystemFileHandle) -> io::Result<Vec<u8>> {
    let file = get_file(handle).await?;
    let buffer = await_promise(file.array_buffer(), EntryKind::File).await?;
    Ok(Uint8Array::new(&buffer).to_vec())
}

async fn overwrite(handle: &FileSystemFileHandle, contents: &[u8]) -> io::Result<()> {
    let stream = open_writer(handle, false).await?;
    let bytes = Uint8Array::new_with_length(contents.len() as u32);
    bytes.copy_from(contents);
    let operation = stream
        .write_with_js_u8_array(&bytes)
        .map_err(|error| js_error(error, EntryKind::File));
    finish_write(stream, operation).await
}

async fn write_at(handle: &FileSystemFileHandle, position: u64, contents: &[u8]) -> io::Result<()> {
    if contents.is_empty() {
        return Ok(());
    }
    if position > 9_007_199_254_740_991 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "the file position exceeds JavaScript's integer range",
        ));
    }

    let stream = open_writer(handle, true).await?;
    let seek = stream
        .seek_with_f64(position as f64)
        .map_err(|error| js_error(error, EntryKind::File));
    match seek {
        Ok(seek) => {
            if let Err(error) = await_promise(seek, EntryKind::File).await {
                let _ = await_promise(stream.abort(), EntryKind::File).await;
                return Err(error);
            }
        }
        Err(error) => {
            let _ = await_promise(stream.abort(), EntryKind::File).await;
            return Err(error);
        }
    }

    let bytes = Uint8Array::new_with_length(contents.len() as u32);
    bytes.copy_from(contents);
    let operation = stream
        .write_with_js_u8_array(&bytes)
        .map_err(|error| js_error(error, EntryKind::File));
    finish_write(stream, operation).await
}

async fn truncate(handle: &FileSystemFileHandle, size: u64) -> io::Result<()> {
    if size > 9_007_199_254_740_991 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "the file size exceeds JavaScript's integer range",
        ));
    }
    let stream = open_writer(handle, true).await?;
    let operation = stream
        .truncate_with_f64(size as f64)
        .map_err(|error| js_error(error, EntryKind::File));
    finish_write(stream, operation).await
}

async fn open_writer(
    handle: &FileSystemFileHandle,
    keep_existing_data: bool,
) -> io::Result<FileSystemWritableFileStream> {
    let options = FileSystemCreateWritableOptions::new();
    options.set_keep_existing_data(keep_existing_data);
    cast(
        await_promise(
            handle.create_writable_with_options(&options),
            EntryKind::File,
        )
        .await?,
    )
}

async fn finish_write(
    stream: FileSystemWritableFileStream,
    operation: io::Result<js_sys::Promise>,
) -> io::Result<()> {
    let operation = match operation {
        Ok(operation) => await_promise(operation, EntryKind::File).await,
        Err(error) => Err(error),
    };
    if let Err(error) = operation {
        let _ = await_promise(stream.abort(), EntryKind::File).await;
        return Err(error);
    }

    await_promise(stream.close(), EntryKind::File).await?;
    Ok(())
}

fn split_parent(path: &Path) -> io::Result<(Vec<String>, String)> {
    let mut names = split_path(path)?;
    let name = names.pop().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "path names the OPFS root instead of a file",
        )
    })?;
    Ok((names, name))
}

fn split_path(path: &Path) -> io::Result<Vec<String>> {
    let mut names = Vec::new();
    for component in path.components() {
        match component {
            Component::RootDir | Component::CurDir => {}
            Component::ParentDir => {
                if names.pop().is_none() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "path leads outside OPFS",
                    ));
                }
            }
            Component::Normal(name) => names.push(
                name.to_str()
                    .ok_or_else(|| {
                        io::Error::new(io::ErrorKind::InvalidInput, "path is not valid UTF-8")
                    })?
                    .to_owned(),
            ),
            Component::Prefix(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "path prefixes are not supported by OPFS",
                ));
            }
        }
    }

    Ok(names)
}

#[derive(Clone, Copy)]
enum EntryKind {
    File,
    Directory,
    Either,
}

async fn await_promise(promise: js_sys::Promise, entry: EntryKind) -> io::Result<JsValue> {
    JsFuture::from(promise)
        .await
        .map_err(|error| js_error(error, entry))
}

fn cast<T: JsCast>(value: JsValue) -> io::Result<T> {
    value
        .dyn_into::<T>()
        .map_err(|value| io::Error::other(format!("OPFS returned an unexpected value: {value:?}")))
}

fn js_error(value: JsValue, entry: EntryKind) -> io::Error {
    let name = property(&value, "name").unwrap_or_default();
    let kind = match name.as_str() {
        "NotFoundError" => io::ErrorKind::NotFound,
        "NotAllowedError" | "SecurityError" => io::ErrorKind::PermissionDenied,
        "TypeMismatchError" => match entry {
            EntryKind::File => io::ErrorKind::IsADirectory,
            EntryKind::Directory => io::ErrorKind::NotADirectory,
            EntryKind::Either => io::ErrorKind::InvalidInput,
        },
        "InvalidModificationError" => io::ErrorKind::DirectoryNotEmpty,
        "NoModificationAllowedError" => io::ErrorKind::ResourceBusy,
        "QuotaExceededError" => io::ErrorKind::QuotaExceeded,
        "AbortError" => io::ErrorKind::Interrupted,
        _ => io::ErrorKind::Other,
    };
    let message = property(&value, "message").unwrap_or_else(|| format!("{value:?}"));
    if name.is_empty() {
        io::Error::new(kind, message)
    } else {
        io::Error::new(kind, format!("{name}: {message}"))
    }
}

fn property(value: &JsValue, name: &str) -> Option<String> {
    Reflect::get(value, &JsValue::from_str(name))
        .ok()
        .and_then(|value| value.as_string())
        .filter(|value| !value.is_empty())
}

fn unsupported_permissions() -> io::Error {
    io::Error::new(
        io::ErrorKind::Unsupported,
        "OPFS does not expose filesystem permissions",
    )
}
