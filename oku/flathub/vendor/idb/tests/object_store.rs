use idb::{
    DatabaseEvent, Factory, IndexParams, KeyPath, ObjectStoreParams, Query, TransactionMode,
};
use serde::Serialize;
use serde_json::Value;
use serde_wasm_bindgen::Serializer;
use wasm_bindgen::JsValue;
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test]
async fn test_object_store_metadata() {
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

    assert_eq!(store.name(), "employees");
    assert_eq!(store.key_path(), Ok(Some(KeyPath::new_single("id"))));
    assert_eq!(store.index_names().len(), 1);
    assert!(store.auto_increment());

    let index = store.index("email");
    assert!(index.is_ok(), "index should be ok: {}", index.unwrap_err());
    let index = index.unwrap();

    assert_eq!(index.name(), "email");
    assert_eq!(index.key_path(), Ok(Some(KeyPath::new_single("email"))));
    assert!(index.unique());
    assert!(!index.multi_entry());

    database.close();
    factory.delete("test").unwrap().await.unwrap();
}

#[wasm_bindgen_test]
async fn test_object_store_crud() {
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

    // Add a value to store
    let transaction = database
        .transaction(&["employees"], TransactionMode::ReadWrite)
        .unwrap();

    let store = transaction.object_store("employees").unwrap();

    assert_eq!(store.count(None).unwrap().await, Ok(0));

    let employee = serde_json::json!({
        "name": "John Doe",
        "email": "john@example.com",
    });

    let id = store
        .add(
            &employee.serialize(&Serializer::json_compatible()).unwrap(),
            None,
        )
        .unwrap()
        .await;
    assert!(id.is_ok(), "id should be ok: {}", id.unwrap_err());
    let id = id.unwrap();

    transaction.commit().unwrap().await.unwrap();

    // Read the value back
    let transaction = database
        .transaction(&["employees"], TransactionMode::ReadOnly)
        .unwrap();

    let store = transaction.object_store("employees").unwrap();

    assert_eq!(store.count(None).unwrap().await, Ok(1));

    let stored_employee = store.get(id.clone()).unwrap().await;
    assert!(
        stored_employee.is_ok(),
        "stored employee should be ok: {}",
        stored_employee.unwrap_err()
    );
    let stored_employee = stored_employee.unwrap().unwrap();

    let stored_employee: Value = serde_wasm_bindgen::from_value(stored_employee).unwrap();

    assert_eq!(stored_employee["name"], "John Doe");
    assert_eq!(stored_employee["email"], "john@example.com");
    assert_eq!(
        stored_employee["id"],
        serde_wasm_bindgen::from_value::<Value>(id.clone()).unwrap()
    );

    transaction.await.unwrap();

    // Update the value
    let transaction = database
        .transaction(&["employees"], TransactionMode::ReadWrite)
        .unwrap();

    let store = transaction.object_store("employees").unwrap();

    let employee = serde_json::json!({
        "id": serde_wasm_bindgen::from_value::<Value>(id.clone()).unwrap(),
        "name": "John Doe",
        "email": "johndoe@example.com",
    });

    let put = store
        .put(
            &employee.serialize(&Serializer::json_compatible()).unwrap(),
            None,
        )
        .unwrap()
        .await;
    assert!(put.is_ok(), "put should be ok: {}", put.unwrap_err());

    transaction.commit().unwrap().await.unwrap();

    // Read the value back
    let transaction = database
        .transaction(&["employees"], TransactionMode::ReadOnly)
        .unwrap();

    let store = transaction.object_store("employees").unwrap();

    assert_eq!(store.count(None).unwrap().await, Ok(1));

    let stored_employee = store.get(id.clone()).unwrap().await;
    assert!(
        stored_employee.is_ok(),
        "stored employee should be ok: {}",
        stored_employee.unwrap_err()
    );
    let stored_employee = stored_employee.unwrap().unwrap();

    let stored_employee: Value = serde_wasm_bindgen::from_value(stored_employee).unwrap();

    assert_eq!(stored_employee["name"], "John Doe");
    assert_eq!(stored_employee["email"], "johndoe@example.com");
    assert_eq!(
        stored_employee["id"],
        serde_wasm_bindgen::from_value::<Value>(id.clone()).unwrap()
    );

    transaction.await.unwrap();

    // Delete the value
    let transaction = database
        .transaction(&["employees"], TransactionMode::ReadWrite)
        .unwrap();

    let store = transaction.object_store("employees").unwrap();

    let delete = store.delete(id.clone()).unwrap().await;
    assert!(
        delete.is_ok(),
        "delete should be ok: {}",
        delete.unwrap_err()
    );

    transaction.commit().unwrap().await.unwrap();

    // Read the value back
    let transaction = database
        .transaction(&["employees"], TransactionMode::ReadOnly)
        .unwrap();

    let store = transaction.object_store("employees").unwrap();

    assert_eq!(store.count(None).unwrap().await, Ok(0));

    transaction.await.unwrap();

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

    // Read the values back
    let transaction = database
        .transaction(&["employees"], TransactionMode::ReadOnly)
        .unwrap();

    let store = transaction.object_store("employees").unwrap();

    assert_eq!(store.count(None).unwrap().await, Ok(2));

    let stored_employees = store.get_all(None, None).unwrap().await;
    assert!(
        stored_employees.is_ok(),
        "stored employees should be ok: {}",
        stored_employees.unwrap_err()
    );
    let stored_employees = stored_employees.unwrap();

    assert_eq!(stored_employees.len(), 2);

    let stored_employee1 = store
        .get_all(Some(Query::Key(id1)), None)
        .unwrap()
        .await
        .unwrap();

    assert_eq!(stored_employee1.len(), 1);
    assert_eq!(
        serde_wasm_bindgen::from_value::<Value>(stored_employees[0].clone()).unwrap(),
        serde_wasm_bindgen::from_value::<Value>(stored_employee1[0].clone()).unwrap()
    );

    let stored_employee2 = store
        .get_all(Some(Query::Key(id2)), None)
        .unwrap()
        .await
        .unwrap();

    assert_eq!(stored_employee2.len(), 1);
    assert_eq!(
        serde_wasm_bindgen::from_value::<Value>(stored_employees[1].clone()).unwrap(),
        serde_wasm_bindgen::from_value::<Value>(stored_employee2[0].clone()).unwrap()
    );

    transaction.await.unwrap();

    // Clear the store
    let transaction = database
        .transaction(&["employees"], TransactionMode::ReadWrite)
        .unwrap();

    let store = transaction.object_store("employees").unwrap();

    let clear = store.clear().unwrap().await;
    assert!(clear.is_ok(), "clear should be ok: {}", clear.unwrap_err());

    transaction.commit().unwrap().await.unwrap();

    // Read the values back
    let transaction = database
        .transaction(&["employees"], TransactionMode::ReadOnly)
        .unwrap();

    let store = transaction.object_store("employees").unwrap();

    assert_eq!(store.count(None).unwrap().await, Ok(0));

    database.close();
    factory.delete("test").unwrap().await.unwrap();
}

#[wasm_bindgen_test]
async fn test_duplicate_add_fail() {
    let factory = Factory::new().unwrap();
    factory.delete("test").unwrap().await.unwrap();

    let mut open_request = factory.open("test", Some(1)).unwrap();
    open_request.on_upgrade_needed(|event| {
        let database = event.database().unwrap();

        let mut store_params = ObjectStoreParams::new();
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
        .transaction(&["employees"], TransactionMode::ReadWrite)
        .unwrap();

    let store = transaction.object_store("employees").unwrap();

    let employee = serde_json::json!({
        "id": 1,
        "name": "John Doe",
        "email": "john@example.com",
    });
    store
        .add(
            &employee.serialize(&Serializer::json_compatible()).unwrap(),
            None,
        )
        .unwrap()
        .await
        .unwrap();

    let employee = serde_json::json!({
        "id": 1,
        "name": "Jane Doe",
        "email": "jane@example.com",
    });

    let error = store
        .add(
            &employee.serialize(&Serializer::json_compatible()).unwrap(),
            None,
        )
        .unwrap()
        .await;

    assert!(
        error.is_err(),
        "adding duplicate id should fail: {}",
        error.unwrap_err()
    );

    database.close();
    factory.delete("test").unwrap().await.unwrap();
}

#[wasm_bindgen_test]
async fn test_zero_key_get() {
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

    // Add a value to store
    let transaction = database
        .transaction(&["employees"], TransactionMode::ReadWrite)
        .unwrap();

    let store = transaction.object_store("employees").unwrap();

    assert_eq!(store.count(None).unwrap().await, Ok(0));

    let employee = serde_json::json!({
        "id": 0,
        "name": "John Doe",
        "email": "john@example.com",
    });

    store
        .add(
            &employee.serialize(&Serializer::json_compatible()).unwrap(),
            None,
        )
        .unwrap()
        .await
        .unwrap();

    transaction.commit().unwrap().await.unwrap();

    // Read the value back
    let transaction = database
        .transaction(&["employees"], TransactionMode::ReadOnly)
        .unwrap();

    let store = transaction.object_store("employees").unwrap();

    assert_eq!(store.count(None).unwrap().await, Ok(1));

    let stored_employee = store.get(JsValue::from(0)).unwrap().await;
    assert!(
        stored_employee.is_ok(),
        "stored employee should be ok: {}",
        stored_employee.unwrap_err()
    );
    let stored_employee = stored_employee.unwrap().unwrap();

    let stored_employee: Value = serde_wasm_bindgen::from_value(stored_employee).unwrap();

    assert_eq!(stored_employee["name"], "John Doe");
    assert_eq!(stored_employee["email"], "john@example.com");

    transaction.await.unwrap();

    database.close();
    factory.delete("test").unwrap().await.unwrap();
}

#[wasm_bindgen_test]
async fn test_bulk_insertion() {
    let factory = Factory::new().unwrap();
    factory.delete("test").unwrap().await.unwrap();

    let mut open_request = factory.open("test", Some(1)).unwrap();
    open_request.on_upgrade_needed(|event| {
        let database = event.database().unwrap();

        let mut store_params = ObjectStoreParams::new();
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

    // Add a value to store
    let transaction = database
        .transaction(&["employees"], TransactionMode::ReadWrite)
        .unwrap();

    let store = transaction.object_store("employees").unwrap();

    let employee1 = serde_json::json!({
        "id": 1,
        "name": "John Doe",
        "email": "john@example.com",
    });
    let employee2 = serde_json::json!({
        "id": 2,
        "name": "Jane Doe",
        "email": "jane@example.com",
    });

    let _ = store.add(
        &employee1.serialize(&Serializer::json_compatible()).unwrap(),
        None,
    );
    let _ = store
        .add(
            &employee2.serialize(&Serializer::json_compatible()).unwrap(),
            None,
        )
        .unwrap()
        .await
        .unwrap();

    transaction.commit().unwrap().await.unwrap();

    // Read the values back
    let transaction = database
        .transaction(&["employees"], TransactionMode::ReadOnly)
        .unwrap();

    let store = transaction.object_store("employees").unwrap();

    assert_eq!(store.count(None).unwrap().await, Ok(2));

    let stored_employees = store.get_all(None, None).unwrap().await;
    assert!(
        stored_employees.is_ok(),
        "stored employees should be ok: {}",
        stored_employees.unwrap_err()
    );
    let stored_employees = stored_employees.unwrap();

    assert_eq!(stored_employees.len(), 2);

    let stored_employee1 = store
        .get_all(Some(Query::Key(1.into())), None)
        .unwrap()
        .await
        .unwrap();

    assert_eq!(stored_employee1.len(), 1);
    assert_eq!(
        serde_wasm_bindgen::from_value::<Value>(stored_employees[0].clone()).unwrap(),
        serde_wasm_bindgen::from_value::<Value>(stored_employee1[0].clone()).unwrap()
    );

    let stored_employee2 = store
        .get_all(Some(Query::Key(2.into())), None)
        .unwrap()
        .await
        .unwrap();

    assert_eq!(stored_employee2.len(), 1);
    assert_eq!(
        serde_wasm_bindgen::from_value::<Value>(stored_employees[1].clone()).unwrap(),
        serde_wasm_bindgen::from_value::<Value>(stored_employee2[0].clone()).unwrap()
    );

    transaction.await.unwrap();

    // Clear the store
    let transaction = database
        .transaction(&["employees"], TransactionMode::ReadWrite)
        .unwrap();

    let store = transaction.object_store("employees").unwrap();

    let clear = store.clear().unwrap().await;
    assert!(clear.is_ok(), "clear should be ok: {}", clear.unwrap_err());

    transaction.commit().unwrap().await.unwrap();

    // Read the values back
    let transaction = database
        .transaction(&["employees"], TransactionMode::ReadOnly)
        .unwrap();

    let store = transaction.object_store("employees").unwrap();

    assert_eq!(store.count(None).unwrap().await, Ok(0));

    database.close();
    factory.delete("test").unwrap().await.unwrap();
}
