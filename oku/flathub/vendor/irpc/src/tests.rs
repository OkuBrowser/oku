use std::vec;

#[tokio::test]
async fn test_map_filter() {
    use crate::channel::mpsc;
    let (tx, rx) = mpsc::channel::<u64>(100);
    // *2, filter multiples of 4, *3 if multiple of 8
    //
    // the transforms are applied in reverse order!
    let tx = tx
        .with_filter_map(|x: u64| {
            if x.is_multiple_of(8) {
                Some(x * 3)
            } else {
                None
            }
        })
        .with_filter(|x| x % 4 == 0)
        .with_map(|x: u64| x * 2);
    for i in 0..100 {
        tx.send(i).await.ok();
    }
    drop(tx);
    // /24, filter multiples of 3, /2 if even
    let mut rx = rx
        .map(|x: u64| x / 24)
        .filter(|x| x % 3 == 0)
        .filter_map(|x: u64| {
            if x.is_multiple_of(2) {
                Some(x / 2)
            } else {
                None
            }
        });
    let mut res = vec![];
    while let Ok(Some(x)) = rx.recv().await {
        res.push(x);
    }
    assert_eq!(res, vec![0, 3, 6, 9, 12]);
}
