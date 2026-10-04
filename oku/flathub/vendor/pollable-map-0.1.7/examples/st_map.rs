use futures::stream::BoxStream;
use futures::StreamExt;
use pollable_map::stream::StreamMap;

fn main() {
    futures::executor::block_on(async {
        let mut st_map = StreamMap::<u8, BoxStream<'static, ()>>::new();

        st_map.insert(
            1,
            futures::stream::once(async {
                println!("message from 1");
            })
            .boxed(),
        );
        st_map.insert(
            2,
            futures::stream::once(async {
                println!("message from 2");
            })
            .boxed(),
        );

        while let Some((key, _)) = st_map.next().await {
            println!("> future \"{}\" is finished", key);
        }

        println!("Done processing map")
    });
}
