use idb::{
    CursorDirection, DatabaseEvent, Factory, IndexParams, KeyPath, ObjectStoreParams,
    TransactionMode,
};
use serde::Serialize;
use serde_wasm_bindgen::Serializer;
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test]
async fn test_cursor_next_advance_and_get() {
    let factory = Factory::new().unwrap();
    factory.delete("test").unwrap().await.unwrap();

    let mut open_request = factory.open("test", Some(1)).unwrap();
    open_request.on_upgrade_needed(|event| {
        let database = event.database().unwrap();

        let mut store_params = ObjectStoreParams::new();
        store_params.auto_increment(true);
        store_params.key_path(Some(KeyPath::new_single("id")));

        let store = database
            .create_object_store("employees", store_params)
            .unwrap();

        let mut index_params = IndexParams::new();
        index_params.unique(true);

        store
            .create_index("email", KeyPath::new_single("email"), Some(index_params))
            .unwrap();
    });

    let database = open_request.await.unwrap();

    // Insert multiple values
    let transaction = database
        .transaction(&["employees"], TransactionMode::ReadWrite)
        .unwrap();

    let store = transaction.object_store("employees").unwrap();

    let employee1 = serde_json::json!({
        "name": "John Doe",
        "email": "john@example.com",
    });
    let employee2 = serde_json::json!({
        "name": "Jane Doe",
        "email": "jane@example.com",
    });

    let id1 = store
        .add(
            &employee1.serialize(&Serializer::json_compatible()).unwrap(),
            None,
        )
        .unwrap()
        .await
        .unwrap();
    let id2 = store
        .add(
            &employee2.serialize(&Serializer::json_compatible()).unwrap(),
            None,
        )
        .unwrap()
        .await
        .unwrap();

    transaction.commit().unwrap().await.unwrap();

    // Test cursor flow (with next)
    let transaction = database
        .transaction(&["employees"], TransactionMode::ReadOnly)
        .unwrap();

    let store = transaction.object_store("employees").unwrap();

    let mut cursor = store
        .open_cursor(None, Some(CursorDirection::Next))
        .unwrap()
        .await
        .unwrap()
        .unwrap()
        .into_managed();

    assert_eq!(Ok(Some(id1.clone())), cursor.key());
    cursor.next(None).await.unwrap();
    assert_eq!(Ok(Some(id2.clone())), cursor.key());
    cursor.next(None).await.unwrap();
    assert_eq!(Ok(None), cursor.key());

    transaction.await.unwrap();

    // Test cursor flow (with next, in a loop)
    let transaction = database
        .transaction(&["employees"], TransactionMode::ReadOnly)
        .unwrap();

    let store = transaction.object_store("employees").unwrap();

    let mut cursor = store
        .open_cursor(None, Some(CursorDirection::Next))
        .unwrap()
        .await
        .unwrap()
        .unwrap()
        .into_managed();

    let mut num_employees = 0;
    loop {
        match cursor.value().unwrap() {
            Some(_) => {
                num_employees += 1;
                cursor.next(None).await.unwrap();
            }
            None => break,
        }
    }
    assert_eq!(2, num_employees);

    transaction.await.unwrap();

    // Test cursor flow (with advance)
    let transaction = database
        .transaction(&["employees"], TransactionMode::ReadOnly)
        .unwrap();

    let store = transaction.object_store("employees").unwrap();

    let mut cursor = store
        .open_cursor(None, Some(CursorDirection::Next))
        .unwrap()
        .await
        .unwrap()
        .unwrap()
        .into_managed();

    assert_eq!(Ok(Some(id1)), cursor.key());
    cursor.advance(1).await.unwrap();
    assert_eq!(Ok(Some(id2)), cursor.key());
    cursor.advance(1).await.unwrap();
    assert_eq!(Ok(None), cursor.key());

    transaction.await.unwrap();

    database.close();
    factory.delete("test").unwrap().await.unwrap();
}

#[wasm_bindgen_test]
async fn test_cursor_delete() {
    let factory = Factory::new().unwrap();
    factory.delete("test").unwrap().await.unwrap();

    let mut open_request = factory.open("test", Some(1)).unwrap();
    open_request.on_upgrade_needed(|event| {
        let database = event.database().unwrap();

        let mut store_params = ObjectStoreParams::new();
        store_params.auto_increment(true);
        store_params.key_path(Some(KeyPath::new_single("id")));

        let store = database
            .create_object_store("employees", store_params)
            .unwrap();

        let mut index_params = IndexParams::new();
        index_params.unique(true);

        store
            .create_index("email", KeyPath::new_single("email"), Some(index_params))
            .unwrap();
    });

    let database = open_request.await.unwrap();

    // Insert multiple values
    let transaction = database
        .transaction(&["employees"], TransactionMode::ReadWrite)
        .unwrap();

    let store = transaction.object_store("employees").unwrap();

    let employee1 = serde_json::json!({
        "name": "John Doe",
        "email": "john@example.com",
    });
    let employee2 = serde_json::json!({
        "name": "Jane Doe",
        "email": "jane@example.com",
    });

    let id1 = store
        .add(
            &employee1.serialize(&Serializer::json_compatible()).unwrap(),
            None,
        )
        .unwrap()
        .await
        .unwrap();
    let id2 = store
        .add(
            &employee2.serialize(&Serializer::json_compatible()).unwrap(),
            None,
        )
        .unwrap()
        .await
        .unwrap();

    transaction.commit().unwrap().await.unwrap();

    // Delete the second key using cursor
    let transaction = database
        .transaction(&["employees"], TransactionMode::ReadWrite)
        .unwrap();

    let store = transaction.object_store("employees").unwrap();

    let mut cursor = store
        .open_cursor(None, Some(CursorDirection::Next))
        .unwrap()
        .await
        .unwrap()
        .unwrap()
        .into_managed();

    assert_eq!(Ok(Some(id1.clone())), cursor.key());
    cursor.next(None).await.unwrap();
    assert_eq!(Ok(Some(id2.clone())), cursor.key());

    cursor.delete().await.unwrap();

    cursor.next(None).await.unwrap();
    assert_eq!(Ok(None), cursor.key());

    transaction.commit().unwrap().await.unwrap();

    // Get count of values in store
    let transaction = database
        .transaction(&["employees"], TransactionMode::ReadOnly)
        .unwrap();

    let store = transaction.object_store("employees").unwrap();

    let count = store.count(None).unwrap().await.unwrap();
    assert_eq!(1, count);

    database.close();
    factory.delete("test").unwrap().await.unwrap();
}

#[wasm_bindgen_test]
async fn test_cursor_with_zero_matches() {
    let factory = Factory::new().unwrap();
    factory.delete("test").unwrap().await.unwrap();

    let mut open_request = factory.open("test", Some(1)).unwrap();
    open_request.on_upgrade_needed(|event| {
        let database = event.database().unwrap();

        let mut store_params = ObjectStoreParams::new();
        store_params.auto_increment(true);
        store_params.key_path(Some(KeyPath::new_single("id")));

        let store = database
            .create_object_store("employees", store_params)
            .unwrap();

        let mut index_params = IndexParams::new();
        index_params.unique(true);

        store
            .create_index("email", KeyPath::new_single("email"), Some(index_params))
            .unwrap();
    });

    let database = open_request.await.unwrap();

    let transaction = database
        .transaction(&["employees"], TransactionMode::ReadOnly)
        .unwrap();

    let store = transaction.object_store("employees").unwrap();

    assert!(store
        .open_cursor(None, None)
        .unwrap()
        .await
        .unwrap()
        .is_none());

    database.close();
    factory.delete("test").unwrap().await.unwrap();
}
