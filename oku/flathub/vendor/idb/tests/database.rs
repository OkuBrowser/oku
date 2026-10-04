use idb::{DatabaseEvent, Factory, ObjectStoreParams, TransactionMode};
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test]
async fn test_database_name_and_version() {
    let factory = Factory::new().unwrap();
    factory.delete("test").unwrap().await.unwrap();

    let open_request = factory.open("test", Some(1)).unwrap();
    let database = open_request.await.unwrap();

    assert_eq!(database.name(), "test");
    assert_eq!(database.version(), Ok(1));

    database.close();
    factory.delete("test").unwrap().await.unwrap();
}

#[wasm_bindgen_test]
async fn test_database_store_names() {
    let factory = Factory::new().unwrap();
    factory.delete("test").unwrap().await.unwrap();

    let mut open_request = factory.open("test", Some(1)).unwrap();
    open_request.on_upgrade_needed(|event| {
        let database = event.database().unwrap();

        database
            .create_object_store("store1", ObjectStoreParams::new())
            .unwrap();
        database
            .create_object_store("store2", ObjectStoreParams::new())
            .unwrap();
        database
            .create_object_store("store3", ObjectStoreParams::new())
            .unwrap();
    });

    let database = open_request.await.unwrap();

    let store_names = database.store_names();
    assert_eq!(store_names.len(), 3);
    assert!(store_names.contains(&"store1".to_string()));
    assert!(store_names.contains(&"store2".to_string()));
    assert!(store_names.contains(&"store3".to_string()));

    database.close();
    factory.delete("test").unwrap().await.unwrap();
}

#[wasm_bindgen_test]
async fn test_database_transaction() {
    let factory = Factory::new().unwrap();
    factory.delete("test").unwrap().await.unwrap();

    let mut open_request = factory.open("test", Some(1)).unwrap();
    open_request.on_upgrade_needed(|event| {
        let database = event.database().unwrap();

        database
            .create_object_store("store1", ObjectStoreParams::new())
            .unwrap();
        database
            .create_object_store("store2", ObjectStoreParams::new())
            .unwrap();
        database
            .create_object_store("store3", ObjectStoreParams::new())
            .unwrap();
    });

    let database = open_request.await.unwrap();

    let read_transaction = database.transaction(&["store1"], TransactionMode::ReadOnly);
    assert!(
        read_transaction.is_ok(),
        "read transaction should be ok: {}",
        read_transaction.unwrap_err()
    );
    let read_transaction = read_transaction.unwrap();

    assert_eq!(read_transaction.mode(), Ok(TransactionMode::ReadOnly));
    assert_eq!(read_transaction.store_names(), vec!["store1"]);

    let write_transaction = database.transaction(&["store2", "store3"], TransactionMode::ReadWrite);
    assert!(
        write_transaction.is_ok(),
        "write transaction should be ok: {}",
        write_transaction.unwrap_err()
    );
    let write_transaction = write_transaction.unwrap();

    assert_eq!(write_transaction.mode(), Ok(TransactionMode::ReadWrite));
    assert_eq!(write_transaction.store_names(), vec!["store2", "store3"]);

    database.close();
    factory.delete("test").unwrap().await.unwrap();
}

#[wasm_bindgen_test]
async fn test_database_delete_object_store() {
    let factory = Factory::new().unwrap();
    factory.delete("test").unwrap().await.unwrap();

    let mut open_request = factory.open("test", Some(1)).unwrap();
    open_request.on_upgrade_needed(|event| {
        let database = event.database().unwrap();

        database
            .create_object_store("store1", ObjectStoreParams::new())
            .unwrap();
        database
            .create_object_store("store2", ObjectStoreParams::new())
            .unwrap();
        database
            .create_object_store("store3", ObjectStoreParams::new())
            .unwrap();
    });

    let mut database = open_request.await.unwrap();
    database.on_version_change(|event| event.database().expect("database").close());

    let mut open_request = factory.open("test", Some(2)).unwrap();
    open_request.on_upgrade_needed(|event| {
        let database = event.database().unwrap();

        let store_names = database.store_names();
        assert_eq!(store_names.len(), 3);
        assert!(store_names.contains(&"store1".to_string()));
        assert!(store_names.contains(&"store2".to_string()));
        assert!(store_names.contains(&"store3".to_string()));

        database.delete_object_store("store2").unwrap();
    });

    let database = open_request.await.unwrap();

    let store_names = database.store_names();
    assert_eq!(store_names.len(), 2);
    assert!(store_names.contains(&"store1".to_string()));
    assert!(store_names.contains(&"store3".to_string()));

    database.close();
    factory.delete("test").unwrap().await.unwrap();
}
