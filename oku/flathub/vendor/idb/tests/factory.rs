use std::future::IntoFuture;

use idb::Factory;
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test]
fn test_factory_new() {
    let factory = Factory::new();
    assert!(
        factory.is_ok(),
        "Factory::new() should be Ok(): {}",
        factory.unwrap_err()
    );
}

#[wasm_bindgen_test]
async fn test_factory_open_delete() {
    let factory = Factory::new().unwrap();

    let open_request = factory.open("test", None);
    assert!(
        open_request.is_ok(),
        "Factory::open() should be Ok(): {}",
        open_request.unwrap_err()
    );

    let database = open_request.unwrap().await;
    assert!(
        database.is_ok(),
        "OpenRequest::into_future() should be Ok(): {}",
        database.unwrap_err()
    );
    let database = database.unwrap();

    database.close();

    let delete = factory.delete("test").unwrap().await;
    assert!(
        delete.is_ok(),
        "Factory::delete() should be Ok(): {}",
        delete.unwrap_err()
    );
}

#[wasm_bindgen_test]
async fn test_factory_open_request_drop() {
    let factory = Factory::new().unwrap();

    let open_request = factory.open("test", None);
    assert!(
        open_request.is_ok(),
        "Factory::open() should be Ok(): {}",
        open_request.unwrap_err()
    );

    let open_request = open_request.unwrap();
    drop(open_request);
}

#[wasm_bindgen_test]
async fn test_factory_open_request_future_drop() {
    let factory = Factory::new().unwrap();

    let open_request = factory.open("test", None);
    assert!(
        open_request.is_ok(),
        "Factory::open() should be Ok(): {}",
        open_request.unwrap_err()
    );

    let fut = open_request.unwrap().into_future();
    drop(fut);
}
