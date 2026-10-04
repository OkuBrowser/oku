use crate::provider::VoxProvider;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

#[derive(Clone, Debug, Default)]
/// A provider of the Vox build system that reads & writes from memory.
pub struct RamProvider {
    files: Arc<Mutex<HashMap<std::path::PathBuf, String>>>,
}
impl VoxProvider for RamProvider {
    fn read_to_string(&self, path: impl AsRef<std::path::Path>) -> miette::Result<String> {
        self.files
            .try_lock()
            .map_err(|e| miette::miette!("{}", e))?
            .get(&path.as_ref().to_path_buf())
            .ok_or(miette::miette!("File not found … "))
            .cloned()
    }
    fn write_file(
        &self,
        path: impl AsRef<std::path::Path> + Clone,
        contents: impl AsRef<[u8]>,
    ) -> miette::Result<()> {
        self.files
            .try_lock()
            .map_err(|e| miette::miette!("{}", e))?
            .insert(
                path.as_ref().to_path_buf(),
                String::from_utf8_lossy(contents.as_ref()).to_string(),
            );
        Ok(())
    }
    fn remove_file(&self, path: impl AsRef<std::path::Path>) -> miette::Result<()> {
        self.files
            .try_lock()
            .map_err(|e| miette::miette!("{}", e))?
            .remove(&path.as_ref().to_path_buf());
        Ok(())
    }
    fn list_vox_files(&self) -> miette::Result<Vec<std::path::PathBuf>> {
        Ok(self
            .files
            .try_lock()
            .map_err(|e| miette::miette!("{}", e))?
            .clone()
            .into_keys()
            .filter(|x| Some("vox") == x.extension().map(|y| y.to_str()).flatten())
            .collect())
    }
    fn list_snippets(&self) -> miette::Result<Vec<std::path::PathBuf>> {
        Ok(self
            .files
            .try_lock()
            .map_err(|e| miette::miette!("{}", e))?
            .clone()
            .into_keys()
            .filter(|x| x.starts_with("snippets/"))
            .collect())
    }
}
impl RamProvider {
    /// Create a new Vox provider that reads & writes from memory.
    pub fn new(initial_files: Option<HashMap<std::path::PathBuf, String>>) -> Self {
        Self {
            files: Arc::new(Mutex::new(initial_files.unwrap_or_default())),
        }
    }
}
