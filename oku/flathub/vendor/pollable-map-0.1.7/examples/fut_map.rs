use futures::future::BoxFuture;
use futures::StreamExt;
use pollable_map::futures::FutureMap;

fn main() {
    futures::executor::block_on(async {
        let mut fut_map = FutureMap::<u8, BoxFuture<'static, ()>>::new();

        fut_map.insert(
            1,
            Box::pin(async {
                println!("message from 1");
            }),
        );
        fut_map.insert(
            2,
            Box::pin(async {
                println!("message from 2");
            }),
        );

        while let Some((key, _)) = fut_map.next().await {
            println!("> future \"{}\" is finished", key);
        }

        println!("Done processing map")
    });
}
