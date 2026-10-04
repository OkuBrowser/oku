use idb::Factory;
use tokio::sync::oneshot;
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test]
async fn test_open_request_upgrade_needed() {
    let factory = Factory::new().unwrap();
    factory.delete("test").unwrap().await.unwrap();

    let mut open_request = factory.open("test", Some(1)).unwrap();

    let (sender, receiver) = oneshot::channel();
    open_request.on_upgrade_needed(move |event| {
        sender.send(event).expect("channel send");
    });

    let database = open_request.await.unwrap();

    let event = receiver.await.unwrap();

    assert_eq!(event.old_version(), Ok(0));
    assert_eq!(event.new_version(), Ok(Some(1)));

    database.close();
    factory.delete("test").unwrap().await.unwrap();
}

#[wasm_bindgen_test]
async fn test_open_request_blocked() {
    let factory = Factory::new().unwrap();
    factory.delete("test").unwrap().await.unwrap();

    let open_request = factory.open("test", Some(1)).unwrap();
    let database = open_request.await.unwrap();

    let mut blocking_open_request = factory.open("test", Some(2)).unwrap();

    let (sender, receiver) = oneshot::channel();
    blocking_open_request.on_blocked(move |event| {
        sender.send(event).expect("channel send");
    });

    let event = receiver.await.unwrap();

    assert_eq!(event.old_version(), Ok(1));
    assert_eq!(event.new_version(), Ok(Some(2)));

    database.close();

    let database = blocking_open_request.await.unwrap();
    assert_eq!(database.version(), Ok(2));

    database.close();
    factory.delete("test").unwrap().await.unwrap();
}
