use std::collections::HashSet;

use crate::{request::OpenDatabaseRequest, Database, Error, KeyPath, ObjectStoreParams, Request};

use super::IndexBuilder;

/// Builder for object stores.
#[derive(Debug)]
pub struct ObjectStoreBuilder {
    name: String,
    auto_increment: Option<bool>,
    key_path: Option<KeyPath>,
    indexes: Vec<IndexBuilder>,
}

impl ObjectStoreBuilder {
    /// Creates a new instance of [`ObjectStoreBuilder`].
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            auto_increment: None,
            key_path: None,
            indexes: Vec::new(),
        }
    }

    /// Returns the name of the object store.
    pub fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn set_name(&mut self, name: &str) {
        self.name = name.to_owned()
    }

    /// Sets the `auto_increment` flag.
    pub fn auto_increment(mut self, auto_increment: bool) -> Self {
        self.auto_increment = Some(auto_increment);
        self
    }

    /// Sets the key path.
    pub fn key_path(mut self, key_path: Option<KeyPath>) -> Self {
        self.key_path = key_path;
        self
    }

    /// Adds an index.
    pub fn add_index(mut self, index: IndexBuilder) -> Self {
        self.indexes.push(index);
        self
    }

    pub(crate) fn apply(
        self,
        database: &Database,
        request: &OpenDatabaseRequest,
    ) -> Result<(), Error> {
        let mut index_names: HashSet<_> = self
            .indexes
            .iter()
            .map(|index| index.name().to_owned())
            .collect();

        let object_store = if database.store_names().contains(&self.name) {
            let transaction = request
                .transaction()
                .ok_or_else(|| Error::TransactionNotFound)?;

            transaction.object_store(&self.name)
        } else {
            let mut params = ObjectStoreParams::new();

            if let Some(auto_increment) = self.auto_increment {
                params.auto_increment(auto_increment);
            }

            if let Some(key_path) = self.key_path {
                params.key_path(Some(key_path));
            }

            database.create_object_store(&self.name, params)
        }?;

        for index in self.indexes {
            index.apply(&object_store)?;
        }

        let db_index_names = object_store.index_names();
        let mut indexes_to_remove = Vec::new();

        for db_index_name in db_index_names {
            if !index_names.remove(db_index_name.as_str()) {
                indexes_to_remove.push(db_index_name);
            }
        }

        for index_name in indexes_to_remove {
            object_store.delete_index(&index_name)?;
        }

        Ok(())
    }
}
