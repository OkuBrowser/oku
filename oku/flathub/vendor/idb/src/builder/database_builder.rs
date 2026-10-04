use std::collections::HashSet;

use indexmap::{IndexMap, IndexSet};

use crate::{Database, DatabaseEvent as _, Error, Event as _, Factory, Request as _};

use super::ObjectStoreBuilder;

/// Builder for databases.
#[derive(Debug)]
pub struct DatabaseBuilder {
    name: String,
    version: Option<u32>,
    object_stores: IndexMap<String, ObjectStoreBuilder>,
    /// Maps the new name to the old name and the builder.
    object_stores_to_rename: IndexMap<String, (String, ObjectStoreBuilder)>,
    object_stores_to_remove: IndexSet<String>,
}

impl DatabaseBuilder {
    /// Creates a new instance of [`DatabaseBuilder`].
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            version: None,
            object_stores: Default::default(),
            object_stores_to_rename: Default::default(),
            object_stores_to_remove: Default::default(),
        }
    }

    /// Sets the version of the database.
    pub fn version(mut self, version: u32) -> Self {
        self.version = Some(version);
        self
    }

    /// Adds an object store.
    pub fn add_object_store(mut self, object_store: ObjectStoreBuilder) -> Self {
        let name = object_store.name().to_owned();
        self.object_stores.insert(name, object_store);
        self
    }

    /// Removes an object store.
    pub fn remove_object_store(mut self, object_store_name: &str) -> Self {
        self.object_stores.shift_remove(object_store_name);
        self.object_stores_to_remove
            .insert(object_store_name.to_owned());
        self
    }

    /// Renames the object store with the given name to the new given name.
    pub fn rename_object_store(mut self, old_name: &str, new_name: &str) -> Self {
        let mut old_name = old_name.to_owned();

        let mut previous_object_store = self
            .object_stores
            .shift_remove(&old_name)
            .or_else(|| {
                // In case we're in a chain of renames, a store previously added for rename was added with the now old
                // name
                self.object_stores_to_rename.shift_remove(&old_name).map(
                    |(previous_old_name, store)| {
                        old_name = previous_old_name;
                        store
                    },
                )
            })
            .expect("cannot rename an object store which does not exist");

        previous_object_store.set_name(new_name);

        self.object_stores_to_rename
            .insert(new_name.to_owned(), (old_name, previous_object_store));

        self
    }

    /// Builds the database.
    pub async fn build(mut self) -> Result<Database, Error> {
        let factory = Factory::new()?;
        let mut request = factory.open(&self.name, self.version)?;

        request.on_upgrade_needed(move |event| {
            let request = event.target().expect("open database request");
            let database = event.database().expect("database");

            let mut existing_store_names = database.store_names();

            // Explicitly removed object stores
            for store_to_remove in self.object_stores_to_remove.iter() {
                if existing_store_names.contains(store_to_remove) {
                    database
                        .delete_object_store(store_to_remove)
                        .expect("object store deletion");
                }
            }

            let stores_to_retain = self
                .object_stores
                .keys()
                .cloned()
                .chain(self.object_stores_to_rename.keys().cloned())
                .collect::<HashSet<_>>();

            // For each store to rename, rename if it exists locally, just add it otherwise
            for (new_name, (old_name, store)) in self.object_stores_to_rename.into_iter() {
                if !existing_store_names.contains(&old_name) {
                    self.object_stores.insert(new_name, store);
                } else {
                    request
                        .transaction()
                        .expect("transaction")
                        .object_store(&old_name)
                        .expect("object store")
                        .to_owned()
                        .set_name(&new_name);
                    existing_store_names = database.store_names()
                }
            }

            for object_store in self.object_stores.into_values() {
                object_store
                    .apply(&database, &request)
                    .expect("object store creation");
            }

            // Object stores removed implicitly by not adding them
            for db_store_name in existing_store_names {
                if !stores_to_retain.contains(&db_store_name)
                    && !self.object_stores_to_remove.contains(&db_store_name)
                {
                    database
                        .delete_object_store(&db_store_name)
                        .expect("object store deletion");
                }
            }
        });

        let mut database = request.await?;

        database.on_version_change(|event| {
            let database = event.database().expect("database");
            database.close();
        });

        Ok(database)
    }
}
