use idb::{DatabaseEvent, Factory, ObjectStoreParams, TransactionMode, TransactionResult};
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test]
async fn test_transaction_commit() {
    let factory = Factory::new().unwrap();
    factory.delete("test").unwrap().await.unwrap();

    let mut open_request = factory.open("test", Some(1)).unwrap();
    open_request.on_upgrade_needed(|event| {
        let database = event.database().unwrap();

        database
            .create_object_store("store1", ObjectStoreParams::new())
            .unwrap();
    });

    let database = open_request.await.unwrap();

    let transaction = database
        .transaction(&["store1"], TransactionMode::ReadWrite)
        .unwrap();

    let store = transaction.object_store("store1").unwrap();
    let id = store
        .add(
            &serde_wasm_bindgen::to_value("hello").unwrap(),
            Some(&serde_wasm_bindgen::to_value("world").unwrap()),
        )
        .unwrap()
        .await;

    assert!(id.is_ok(), "id should be ok: {}", id.unwrap_err());
    let id = id.unwrap();
    assert_eq!(serde_wasm_bindgen::to_value("world").unwrap(), id);

    let commit = transaction.commit().unwrap().await;
    assert!(
        commit.is_ok(),
        "commit should be ok: {}",
        commit.unwrap_err()
    );
    let commit = commit.unwrap();
    assert!(matches!(commit, TransactionResult::Committed));

    let read_transaction = database
        .transaction(&["store1"], TransactionMode::ReadOnly)
        .unwrap();
    let read_store = read_transaction.object_store("store1").unwrap();

    let count = read_store.count(None).unwrap().await;
    assert!(count.is_ok(), "count should be ok: {}", count.unwrap_err());
    let count = count.unwrap();

    assert_eq!(count, 1);

    let value = read_store
        .get(serde_wasm_bindgen::to_value("world").unwrap())
        .unwrap()
        .await;
    assert!(value.is_ok(), "value should be ok: {}", value.unwrap_err());
    let value = value.unwrap().unwrap();

    assert_eq!(value, serde_wasm_bindgen::to_value("hello").unwrap());

    database.close();
    factory.delete("test").unwrap().await.unwrap();
}

#[wasm_bindgen_test]
async fn test_transaction_abort() {
    let factory = Factory::new().unwrap();
    factory.delete("test").unwrap().await.unwrap();

    let mut open_request = factory.open("test", Some(1)).unwrap();
    open_request.on_upgrade_needed(|event| {
        let database = event.database().unwrap();

        database
            .create_object_store("store1", ObjectStoreParams::new())
            .unwrap();
    });

    let database = open_request.await.unwrap();

    let transaction = database
        .transaction(&["store1"], TransactionMode::ReadWrite)
        .unwrap();

    let store = transaction.object_store("store1").unwrap();
    let id = store
        .add(
            &serde_wasm_bindgen::to_value("hello").unwrap(),
            Some(&serde_wasm_bindgen::to_value("world").unwrap()),
        )
        .unwrap()
        .await;

    assert!(id.is_ok(), "id should be ok: {}", id.unwrap_err());
    let id = id.unwrap();
    assert_eq!(serde_wasm_bindgen::to_value("world").unwrap(), id);

    let abort = transaction.abort().unwrap().await;
    assert!(abort.is_ok(), "abort should be ok: {}", abort.unwrap_err());
    let abort = abort.unwrap();
    assert!(matches!(abort, TransactionResult::Aborted));

    let read_transaction = database
        .transaction(&["store1"], TransactionMode::ReadOnly)
        .unwrap();
    let read_store = read_transaction.object_store("store1").unwrap();

    let count = read_store.count(None).unwrap().await;
    assert!(count.is_ok(), "count should be ok: {}", count.unwrap_err());
    let count = count.unwrap();

    assert_eq!(count, 0);

    database.close();
    factory.delete("test").unwrap().await.unwrap();
}

#[wasm_bindgen_test]
async fn test_transaction_error() {
    let factory = Factory::new().unwrap();
    factory.delete("test").unwrap().await.unwrap();

    let mut open_request = factory.open("test", Some(1)).unwrap();
    open_request.on_upgrade_needed(|event| {
        let database = event.database().unwrap();

        database
            .create_object_store("store1", ObjectStoreParams::new())
            .unwrap();
    });

    let mut database = open_request.await.unwrap();
    database.on_version_change(|event| {
        event.database().unwrap().close();
    });

    let transaction = database
        .transaction(&["store1"], TransactionMode::ReadWrite)
        .unwrap();

    let store = transaction.object_store("store1").unwrap();
    store
        .add(
            &serde_wasm_bindgen::to_value("hello").unwrap(),
            Some(&serde_wasm_bindgen::to_value("world").unwrap()),
        )
        .unwrap()
        .await
        .unwrap();

    let mut open_request = factory.open("test", Some(2)).unwrap();
    open_request.on_upgrade_needed(|event| {
        let database = event.database().unwrap();

        database
            .create_object_store("store2", ObjectStoreParams::new())
            .unwrap();
    });

    let database = open_request.await.unwrap();

    let error = transaction.commit();
    assert!(error.is_err(), "commit should fail"); // Note that the value may still get committed to the store depending on indexedDB implementation on the browser

    database.close();
    factory.delete("test").unwrap().await.unwrap();
}

#[wasm_bindgen_test]
async fn test_transaction_read_write_in_loop() {
    let factory = Factory::new().unwrap();
    factory.delete("test").unwrap().await.unwrap();

    let mut open_request = factory.open("test", Some(1)).unwrap();
    open_request.on_upgrade_needed(|event| {
        let database = event.database().unwrap();

        database
            .create_object_store("store1", ObjectStoreParams::new())
            .unwrap();
    });

    let database = open_request.await.unwrap();

    // 1. Insert 10 values in database

    let transaction = database
        .transaction(&["store1"], TransactionMode::ReadWrite)
        .unwrap();

    let store = transaction.object_store("store1").unwrap();

    for i in 0..10 {
        let id = store
            .add(
                &serde_wasm_bindgen::to_value("hello").unwrap(),
                Some(&serde_wasm_bindgen::to_value(&i.to_string()).unwrap()),
            )
            .unwrap()
            .await;

        assert!(id.is_ok(), "id should be ok: {}", id.unwrap_err());
        let id = id.unwrap();
        assert_eq!(serde_wasm_bindgen::to_value(&i.to_string()).unwrap(), id);
    }

    let commit = transaction.commit().unwrap().await;
    assert!(
        commit.is_ok(),
        "commit should be ok: {}",
        commit.unwrap_err()
    );

    // 2. Change all the values in database

    let transaction = database
        .transaction(&["store1"], TransactionMode::ReadWrite)
        .unwrap();

    let store = transaction.object_store("store1").unwrap();

    for i in 0..10 {
        let value = store
            .get(serde_wasm_bindgen::to_value(&i.to_string()).unwrap())
            .unwrap()
            .await;

        assert!(value.is_ok(), "value should be ok: {}", value.unwrap_err());
        let value = value.unwrap();

        assert!(value.is_some(), "value should be some");
        let value = value.unwrap();
        assert_eq!(value, serde_wasm_bindgen::to_value("hello").unwrap());

        let id = store
            .put(
                &serde_wasm_bindgen::to_value("hello").unwrap(),
                Some(&serde_wasm_bindgen::to_value(&i.to_string()).unwrap()),
            )
            .unwrap()
            .await;

        assert!(id.is_ok(), "id should be ok: {}", id.unwrap_err());
        let id = id.unwrap();
        assert_eq!(serde_wasm_bindgen::to_value(&i.to_string()).unwrap(), id);
    }

    transaction.commit().unwrap().await.unwrap();

    database.close();
    factory.delete("test").unwrap().await.unwrap();
}

#[wasm_bindgen_test]
async fn test_transaction_fail_event_loop_control() {
    let factory = Factory::new().unwrap();
    factory.delete("test").unwrap().await.unwrap();

    let mut open_request = factory.open("test", Some(1)).unwrap();
    open_request.on_upgrade_needed(|event| {
        let database = event.database().unwrap();

        database
            .create_object_store("store1", ObjectStoreParams::new())
            .unwrap();
    });

    let database = open_request.await.unwrap();

    // 1. Insert values in database

    let transaction = database
        .transaction(&["store1"], TransactionMode::ReadWrite)
        .unwrap();

    let store = transaction.object_store("store1").unwrap();

    let id1 = store
        .add(
            &serde_wasm_bindgen::to_value("hello").unwrap(),
            Some(&serde_wasm_bindgen::to_value(&0.to_string()).unwrap()),
        )
        .unwrap()
        .await;

    assert!(id1.is_ok(), "id1 should be ok: {}", id1.unwrap_err());

    gloo::timers::future::TimeoutFuture::new(50).await; // Gives the control back to event loop so that indexed db tries to auto-commit the transaction

    let id2 = store.add(
        &serde_wasm_bindgen::to_value("hello").unwrap(),
        Some(&serde_wasm_bindgen::to_value(&1.to_string()).unwrap()),
    );

    assert!(id2.is_err(), "id2 should be err: {:?}", id2);

    database.close();
    factory.delete("test").unwrap().await.unwrap();
}
