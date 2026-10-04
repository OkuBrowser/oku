use idb::{
    builder::{DatabaseBuilder, IndexBuilder, ObjectStoreBuilder},
    Factory, KeyPath, TransactionMode,
};
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test]
async fn test_database_builder_name_and_version() {
    let factory = Factory::new().unwrap();
    factory.delete("test").unwrap().await.unwrap();

    let database = DatabaseBuilder::new("test")
        .version(1)
        .build()
        .await
        .unwrap();

    assert_eq!(database.name(), "test");
    assert_eq!(database.version(), Ok(1));

    database.close();

    factory.delete("test").unwrap().await.unwrap();
}

#[wasm_bindgen_test]
async fn test_database_builder_remove() {
    let factory = Factory::new().unwrap();
    factory.delete("test").unwrap().await.unwrap();

    let database = DatabaseBuilder::new("test")
        .version(1)
        .add_object_store(ObjectStoreBuilder::new("store1"))
        .add_object_store(ObjectStoreBuilder::new("store2"))
        .add_object_store(ObjectStoreBuilder::new("store3"))
        .add_object_store(ObjectStoreBuilder::new("store4"))
        .remove_object_store("store4")
        .build()
        .await
        .unwrap();

    let store_names = database.store_names();
    assert_eq!(store_names.len(), 3);
    assert!(store_names.contains(&"store1".to_string()));
    assert!(store_names.contains(&"store2".to_string()));
    assert!(store_names.contains(&"store3".to_string()));

    database.close();
    factory.delete("test").unwrap().await.unwrap();
}

#[wasm_bindgen_test]
async fn test_database_builder_reopen_remove() {
    let factory = Factory::new().unwrap();
    factory.delete("test").unwrap().await.unwrap();

    let database = DatabaseBuilder::new("test")
        .version(1)
        .add_object_store(ObjectStoreBuilder::new("store1"))
        .add_object_store(ObjectStoreBuilder::new("store2"))
        .build()
        .await
        .unwrap();

    let store_names = database.store_names();
    assert_eq!(store_names.len(), 2);
    assert!(store_names.contains(&"store1".to_string()));
    assert!(store_names.contains(&"store2".to_string()));

    database.close();

    let database = DatabaseBuilder::new("test")
        .version(2)
        .add_object_store(ObjectStoreBuilder::new("store1"))
        .add_object_store(ObjectStoreBuilder::new("store2"))
        .remove_object_store("store2")
        .build()
        .await
        .unwrap();

    let store_names = database.store_names();
    assert_eq!(store_names.len(), 1);
    assert!(store_names.contains(&"store1".to_string()));
    database.close();

    factory.delete("test").unwrap().await.unwrap();
}

#[wasm_bindgen_test]
async fn test_database_builder_store_names() {
    let factory = Factory::new().unwrap();
    factory.delete("test").unwrap().await.unwrap();

    let database = DatabaseBuilder::new("test")
        .version(1)
        .add_object_store(ObjectStoreBuilder::new("store1"))
        .add_object_store(ObjectStoreBuilder::new("store2"))
        .add_object_store(ObjectStoreBuilder::new("store3"))
        .build()
        .await
        .unwrap();

    let store_names = database.store_names();
    assert_eq!(store_names.len(), 3);
    assert!(store_names.contains(&"store1".to_string()));
    assert!(store_names.contains(&"store2".to_string()));
    assert!(store_names.contains(&"store3".to_string()));

    database.close();
    factory.delete("test").unwrap().await.unwrap();
}

#[wasm_bindgen_test]
async fn test_database_builder_rename() {
    let factory = Factory::new().unwrap();
    factory.delete("test").unwrap().await.unwrap();

    let database = DatabaseBuilder::new("test")
        .version(1)
        .add_object_store(ObjectStoreBuilder::new("store1"))
        .add_object_store(ObjectStoreBuilder::new("store2"))
        .add_object_store(ObjectStoreBuilder::new("store3_to_rename_twice"))
        .rename_object_store("store3_to_rename_twice", "store3_to_rename_once_more")
        .rename_object_store("store3_to_rename_once_more", "store3")
        .build()
        .await
        .unwrap();

    let store_names = database.store_names();
    assert_eq!(store_names.len(), 3);
    assert!(store_names.contains(&"store1".to_string()));
    assert!(store_names.contains(&"store2".to_string()));
    assert!(store_names.contains(&"store3".to_string()));

    database.close();
    factory.delete("test").unwrap().await.unwrap();
}

#[wasm_bindgen_test]
async fn test_database_builder_reopen_rename() {
    let factory = Factory::new().unwrap();
    factory.delete("test").unwrap().await.unwrap();

    let database = DatabaseBuilder::new("test")
        .version(1)
        .add_object_store(ObjectStoreBuilder::new("store_to_rename"))
        .build()
        .await
        .unwrap();

    let store_names = database.store_names();
    assert_eq!(store_names.len(), 1);
    assert!(store_names.contains(&"store_to_rename".to_string()));

    database.close();

    let database = DatabaseBuilder::new("test")
        .version(2)
        .add_object_store(ObjectStoreBuilder::new("store_to_rename"))
        .rename_object_store("store_to_rename", "store")
        .build()
        .await
        .unwrap();

    let store_names = database.store_names();
    assert_eq!(store_names.len(), 1);
    assert!(store_names.contains(&"store".to_string()));
    database.close();

    factory.delete("test").unwrap().await.unwrap();
}

#[wasm_bindgen_test]
async fn test_database_builder_reopen_remove_rename() {
    let factory = Factory::new().unwrap();
    factory.delete("test").unwrap().await.unwrap();

    let database = DatabaseBuilder::new("test")
        .version(1)
        .add_object_store(ObjectStoreBuilder::new("store"))
        .add_object_store(ObjectStoreBuilder::new("store_to_rename"))
        .build()
        .await
        .unwrap();

    let store_names = database.store_names();
    assert_eq!(store_names.len(), 2);
    assert!(store_names.contains(&"store".to_string()));
    assert!(store_names.contains(&"store_to_rename".to_string()));

    database.close();

    let database = DatabaseBuilder::new("test")
        .version(2)
        .add_object_store(ObjectStoreBuilder::new("store"))
        .add_object_store(ObjectStoreBuilder::new("store_to_rename"))
        .remove_object_store("store")
        .rename_object_store("store_to_rename", "store")
        .build()
        .await
        .unwrap();

    let store_names = database.store_names();
    assert_eq!(store_names.len(), 1);
    assert!(store_names.contains(&"store".to_string()));
    database.close();

    factory.delete("test").unwrap().await.unwrap();
}

#[wasm_bindgen_test]
async fn test_database_builder_store_names_with_index() {
    let factory = Factory::new().unwrap();
    factory.delete("test").unwrap().await.unwrap();

    let database = DatabaseBuilder::new("test")
        .version(1)
        .add_object_store(
            ObjectStoreBuilder::new("store1").add_index(IndexBuilder::new(
                "index1".to_string(),
                KeyPath::new_single("id"),
            )),
        )
        .add_object_store(
            ObjectStoreBuilder::new("store2").add_index(IndexBuilder::new(
                "index2".to_string(),
                KeyPath::new_single("id"),
            )),
        )
        .add_object_store(
            ObjectStoreBuilder::new("store3").add_index(IndexBuilder::new(
                "index3".to_string(),
                KeyPath::new_single("id"),
            )),
        )
        .build()
        .await
        .unwrap();

    let store_names = database.store_names();
    assert_eq!(store_names.len(), 3);
    assert!(store_names.contains(&"store1".to_string()));
    assert!(store_names.contains(&"store2".to_string()));
    assert!(store_names.contains(&"store3".to_string()));

    let transaction = database
        .transaction(&["store1", "store2", "store3"], TransactionMode::ReadOnly)
        .unwrap();

    let store1 = transaction.object_store("store1").unwrap();
    let index1 = store1.index_names();
    assert_eq!(1, index1.len());
    assert!(index1.contains(&"index1".to_string()));

    let store2 = transaction.object_store("store2").unwrap();
    let index2 = store2.index_names();
    assert_eq!(1, index2.len());
    assert!(index2.contains(&"index2".to_string()));

    let store3 = transaction.object_store("store3").unwrap();
    let index3 = store3.index_names();
    assert_eq!(1, index3.len());
    assert!(index3.contains(&"index3".to_string()));

    database.close();
    factory.delete("test").unwrap().await.unwrap();
}

#[wasm_bindgen_test]
async fn test_database_builder_reopen() {
    let factory = Factory::new().unwrap();
    factory.delete("test").unwrap().await.unwrap();

    let database = DatabaseBuilder::new("test")
        .version(1)
        .add_object_store(
            ObjectStoreBuilder::new("store")
                .add_index(IndexBuilder::new(
                    "index0".to_string(),
                    KeyPath::new_single("id"),
                ))
                .add_index(
                    IndexBuilder::new("index1".to_string(), KeyPath::new_single("id")).unique(true),
                ),
        )
        .build()
        .await
        .unwrap();

    let transaction = database
        .transaction(&["store"], TransactionMode::ReadOnly)
        .unwrap();

    let store = transaction.object_store("store").unwrap();
    let index = store.index_names();
    assert_eq!(index, vec!["index0", "index1"]);
    assert!(store.index("index1").unwrap().unique());

    transaction.abort().unwrap();
    database.close();

    let database = DatabaseBuilder::new("test")
        .version(2)
        .add_object_store(
            ObjectStoreBuilder::new("store")
                .add_index(IndexBuilder::new(
                    "index0".to_string(),
                    KeyPath::new_single("id"),
                ))
                .add_index(IndexBuilder::new(
                    "index1".to_string(),
                    KeyPath::new_single("id"),
                )),
        )
        .build()
        .await
        .unwrap();

    let transaction = database
        .transaction(&["store"], TransactionMode::ReadOnly)
        .unwrap();

    let store = transaction.object_store("store").unwrap();
    let index = store.index_names();
    assert_eq!(index, vec!["index0", "index1"]);
    assert!(!store.index("index1").unwrap().unique());

    transaction.abort().unwrap();
    database.close();
}
