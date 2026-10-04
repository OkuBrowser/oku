use if_addrs::get_if_addrs;
use rand::{rng, RngExt};
use std::collections::HashSet;
use std::{
    io::{stderr, stdin},
    net::UdpSocket,
    sync::Arc,
    time::Duration,
};
use swarm_discovery::Discoverer;
use tokio::runtime::Builder;
use tokio::time;
use tracing_subscriber::{fmt, EnvFilter};

/// Example demonstrating multi-interface multicast support with dynamic interface management.
///
/// To run this example:
/// 1. In one terminal run: `cargo run --example multi_interface`
/// 2. In another terminal run: `cargo run --example multi_interface`
///
/// This example will:
/// - Join multicast on ALL available network interfaces
/// - Monitor for new network interfaces every 5 seconds
/// - Automatically add new interfaces as they come up
/// - Remove interfaces that go down
///
/// This is useful for systems where network interfaces may change dynamically,
/// such as VPN connections, Docker networks, or USB network adapters.
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
    println!("Using multi-interface multicast with dynamic interface monitoring");

    let mut peer_set: HashSet<String> = HashSet::new();
    peer_set.insert(my_peer_id.clone());
    println!("peer set: {:?}", peer_set);

    // Get initial local IPs for multicast
    let initial_ips: Vec<std::net::Ipv4Addr> = get_if_addrs()
        .expect("get_if_addrs")
        .into_iter()
        .filter(|iface| !iface.is_loopback())
        .map(|iface| iface.addr.ip())
        .filter_map(|ip| match ip {
            std::net::IpAddr::V4(ipv4) => Some(ipv4),
            std::net::IpAddr::V6(_) => None,
        })
        .collect();

    println!("Initial interfaces: {:?}", initial_ips);

    // start announcing and discovering with multi-interface support
    let guard = Arc::new(
        Discoverer::new_interactive("swarm".to_owned(), my_peer_id.clone())
            .with_addrs(port, addrs.iter().take(1).copied())
            .with_addrs(port + 1, addrs)
            .with_multicast_interfaces_v4(initial_ips.clone()) // Start with initial interfaces
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
            .expect("discoverer spawn"),
    );

    // Spawn interface monitoring task
    let guard_clone = guard.clone();
    let mut known_interfaces: HashSet<std::net::Ipv4Addr> = initial_ips.into_iter().collect();

    rt.spawn(async move {
        println!("\nStarting interface monitor (checking every 5 seconds)...");

        loop {
            time::sleep(Duration::from_secs(5)).await;

            // Get current network interfaces
            let current_interfaces: HashSet<std::net::Ipv4Addr> = match get_if_addrs() {
                Ok(addrs) => addrs
                    .into_iter()
                    .filter(|iface| !iface.is_loopback())
                    .map(|iface| iface.addr.ip())
                    .filter_map(|ip| match ip {
                        std::net::IpAddr::V4(ipv4) => Some(ipv4),
                        std::net::IpAddr::V6(_) => None,
                    })
                    .collect(),
                Err(e) => {
                    eprintln!("Failed to get interfaces: {}", e);
                    continue;
                }
            };

            // Check for new interfaces
            for new_if in current_interfaces.difference(&known_interfaces) {
                println!(
                    "📡 New interface detected: {} - adding to multicast",
                    new_if
                );
                guard_clone.add_interface_v4(*new_if);
            }

            // Check for removed interfaces
            for old_if in known_interfaces.difference(&current_interfaces) {
                println!("❌ Interface removed: {} - removing from multicast", old_if);
                guard_clone.remove_interface_v4(*old_if);
            }

            known_interfaces = current_interfaces;
        }
    });

    println!("\nPress Enter to exit...");
    println!("While running, try connecting/disconnecting VPN, USB network adapters, etc.");
    println!("The discoverer will automatically adapt to network changes!\n");

    // end program when user presses Enter
    stdin().read_line(&mut String::new()).expect("read_line");
}
