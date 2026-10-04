use if_addrs::get_if_addrs;
use rand::{rng, RngExt};
use std::collections::HashSet;
use std::{
    io::{stderr, stdin},
    net::UdpSocket,
};
use swarm_discovery::Discoverer;
use tokio::runtime::Builder;
use tracing_subscriber::{fmt, EnvFilter};

/// To run this example:
/// In two (or more) different terminals run: `cargo run --example local`
/// and each one will discover the other peer and track a peer set containing them all
fn main() {
    // enable logging: use `RUST_LOG=debug` or similar to see logs on STDERR
    fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(stderr)
        .init();

    // create Tokio runtime
    let rt = Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("build runtime");

    // make up some peer ID
    let my_peer_id = format!("peer_id{}", rng().random_range(0..100));

    // get local addresses and make up some port
    let addrs = get_if_addrs()
        .expect("get_if_addrs")
        .into_iter()
        .map(|iface| iface.addr.ip())
        .collect::<Vec<_>>();
    let port = UdpSocket::bind((addrs[0], 0))
        .expect("bind")
        .local_addr()
        .expect("local_addr")
        .port();

    println!("my_peer_id: {}", my_peer_id);
    println!("addrs: {:?}", addrs);

    let mut peer_set: HashSet<String> = HashSet::new();
    peer_set.insert(my_peer_id.clone());
    println!("peer set: {:?}", peer_set);

    // start announcing and discovering
    let _guard = Discoverer::new_interactive("swarm".to_owned(), my_peer_id.clone())
        .with_addrs(port, addrs.iter().take(1).copied())
        .with_addrs(port + 1, addrs)
        .with_callback(move |peer_id, peer| {
            if peer_set.insert(peer_id.to_string()) {
                println!("new peer discovered {peer_id}: {:?}", peer);
                println!("peer set: {:?}", peer_set);
            }

            if peer.addrs().is_empty() {
                println!("peer removed: {peer_id}");
                peer_set.remove(peer_id);
                println!("peer set: {:?}", peer_set);
            }
        })
        .spawn(rt.handle())
        .expect("discoverer spawn");

    // end program when user presses Enter
    stdin().read_line(&mut String::new()).expect("read_line");
}
