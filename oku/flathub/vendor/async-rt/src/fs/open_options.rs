use super::{File, FileSystemOpenOptions, current};
use std::io;
use std::path::Path;

/// Options for opening a file.
#[derive(Clone, Debug, Default)]
pub struct OpenOptions {
    pub(super) read: bool,
    pub(super) write: bool,
    pub(super) append: bool,
    pub(super) truncate: bool,
    pub(super) create: bool,
    pub(super) create_new: bool,
}

impl OpenOptions {
    /// Creates a blank set of options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets whether the file is opened for reading.
    pub fn read(&mut self, read: bool) -> &mut Self {
        self.read = read;
        self
    }

    /// Sets whether the file is opened for writing.
    pub fn write(&mut self, write: bool) -> &mut Self {
        self.write = write;
        self
    }

    /// Sets whether writes are appended to the file.
    pub fn append(&mut self, append: bool) -> &mut Self {
        self.append = append;
        self
    }

    /// Sets whether an existing file is truncated when opened.
    pub fn truncate(&mut self, truncate: bool) -> &mut Self {
        self.truncate = truncate;
        self
    }

    /// Sets whether the file is created when it does not exist.
    pub fn create(&mut self, create: bool) -> &mut Self {
        self.create = create;
        self
    }

    /// Sets whether a new file is created and opening fails if it exists.
    pub fn create_new(&mut self, create_new: bool) -> &mut Self {
        self.create_new = create_new;
        self
    }

    /// Opens a file with these options.
    pub async fn open(&self, path: impl AsRef<Path>) -> io::Result<File> {
        current().open_with(self, path).await
    }
}
