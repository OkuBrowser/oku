use idb::{
    DatabaseEvent, Factory, IndexParams, KeyPath, KeyRange, ObjectStoreParams, Query,
    TransactionMode,
};
use serde::Serialize;
use serde_json::Value;
use serde_wasm_bindgen::Serializer;
use wasm_bindgen::JsValue;
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test]
async fn test_index_read() {
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

    // Count values using email index
    let transaction = database
        .transaction(&["employees"], TransactionMode::ReadOnly)
        .unwrap();

    let store = transaction.object_store("employees").unwrap();

    let index = store.index("email").unwrap();

    let count = index.count(None).unwrap().await;
    assert_eq!(count, Ok(2), "count should be 2: {count:?}");

    // Read values using email index
    let transaction = database
        .transaction(&["employees"], TransactionMode::ReadOnly)
        .unwrap();

    let store = transaction.object_store("employees").unwrap();

    let index = store.index("email").unwrap();

    let stored_employee1 = index
        .get(JsValue::from_str("john@example.com"))
        .unwrap()
        .await;
    assert!(
        stored_employee1.is_ok(),
        "stored employee 1 should be ok: {}",
        stored_employee1.unwrap_err()
    );
    let stored_employee1 = stored_employee1.unwrap().unwrap();

    let stored_employee1: Value = serde_wasm_bindgen::from_value(stored_employee1).unwrap();

    assert_eq!(stored_employee1["name"], "John Doe");
    assert_eq!(stored_employee1["email"], "john@example.com");
    assert_eq!(
        stored_employee1["id"],
        serde_wasm_bindgen::from_value::<Value>(id1.clone()).unwrap()
    );

    let stored_employee2 = index
        .get(JsValue::from_str("jane@example.com"))
        .unwrap()
        .await;
    assert!(
        stored_employee2.is_ok(),
        "stored employee 2 should be ok: {}",
        stored_employee2.unwrap_err()
    );
    let stored_employee2 = stored_employee2.unwrap().unwrap();

    let stored_employee2: Value = serde_wasm_bindgen::from_value(stored_employee2).unwrap();

    assert_eq!(stored_employee2["name"], "Jane Doe");
    assert_eq!(stored_employee2["email"], "jane@example.com");
    assert_eq!(
        stored_employee2["id"],
        serde_wasm_bindgen::from_value::<Value>(id2.clone()).unwrap()
    );

    // Read all values where email id starts with `ja` until the emails that start with `jo` (not including `jo`)
    let transaction = database
        .transaction(&["employees"], TransactionMode::ReadOnly)
        .unwrap();

    let store = transaction.object_store("employees").unwrap();

    let index = store.index("email").unwrap();

    let stored_employees = index
        .get_all(
            Some(Query::KeyRange(
                KeyRange::bound(
                    &JsValue::from_str("ja"),
                    &JsValue::from_str("jo"),
                    Some(false),
                    Some(true),
                )
                .unwrap(),
            )),
            None,
        )
        .unwrap()
        .await
        .unwrap();

    assert_eq!(stored_employees.len(), 1);

    let stored_employee: Value =
        serde_wasm_bindgen::from_value(stored_employees[0].clone()).unwrap();
    assert_eq!(stored_employee["name"], "Jane Doe");
    assert_eq!(stored_employee["email"], "jane@example.com");
    assert_eq!(
        stored_employee["id"],
        serde_wasm_bindgen::from_value::<Value>(id2.clone()).unwrap()
    );

    database.close();
    factory.delete("test").unwrap().await.unwrap();
}
