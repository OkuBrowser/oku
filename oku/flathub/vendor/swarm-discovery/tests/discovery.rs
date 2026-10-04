fn main() {
    #[cfg(target_os = "linux")]
    test();
}

#[cfg(target_os = "linux")]
fn test() {
    use if_addrs::get_if_addrs;
    use ipc_channel::ipc::{channel, IpcReceiver, IpcSender};
    use netsim_embed::{
        declare_machines, machine, run_tests, DelayBuffer, Ipv4Range, MachineId, Netsim, NetworkId,
    };
    use serde::{Deserialize, Serialize};
    use std::{
        collections::{BTreeMap, BTreeSet},
        net::IpAddr,
        thread,
        time::{Duration, Instant},
    };
    use swarm_discovery::{Discoverer, Protocol};
    use tokio::runtime::Builder;
    use tracing_subscriber::{fmt, EnvFilter};

    #[derive(Clone, Debug, Serialize, Deserialize)]
    enum Disco {
        Discover {
            host: String,
            peer: String,
            addrs: Vec<(IpAddr, u16)>,
            txt: BTreeMap<String, Option<String>>,
        },
        Forget {
            host: String,
            peer: String,
        },
    }

    #[machine]
    fn disco(
        (protocol, tau, phi, peer_id, port, rcv, snd): (
            Protocol,
            Duration,
            f32,
            String,
            u16,
            IpcReceiver<()>,
            IpcSender<Disco>,
        ),
    ) {
        let rt = Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("build runtime");

        let addrs = get_if_addrs()
            .expect("get_if_addrs")
            .into_iter()
            .map(|iface| iface.addr.ip())
            .collect::<Vec<_>>();

        let mut attributes = BTreeMap::new();
        attributes.insert("name".to_string(), Some(format!("peer={peer_id}")));
        attributes.insert("føø".to_string(), Some("bär".to_string()));
        attributes.insert("bool".to_string(), None);

        let _guard = Discoverer::new("swarm".to_owned(), peer_id.clone())
            .with_protocol(protocol)
            .with_addrs(port + 10, addrs.iter().take(1).copied())
            .with_addrs(port, addrs)
            .with_callback(move |pid, peer| {
                let msg = if peer.is_expiry() {
                    Disco::Forget {
                        host: peer_id.clone(),
                        peer: pid.to_owned(),
                    }
                } else {
                    Disco::Discover {
                        host: peer_id.clone(),
                        peer: pid.to_owned(),
                        addrs: peer.addrs().to_owned(),
                        txt: peer
                            .txt_attributes()
                            .map(|(k, v)| (k.to_string(), v.map(ToString::to_string)))
                            .collect(),
                    }
                };
                snd.send(msg).expect("send");
            })
            .with_cadence(tau)
            .with_response_rate(phi)
            .with_txt_attributes(attributes.into_iter())
            .unwrap()
            .spawn(rt.handle())
            .expect("discoverer spawn");

        rcv.recv().expect("recv");
    }

    async fn spawn(
        sim: &mut Netsim<String, String>,
        net: NetworkId,
        protocol: Protocol,
        tau: Duration,
        phi: f32,
        num: usize,
        tx_f: IpcSender<Disco>,
    ) -> (MachineId, IpcSender<()>) {
        let (tx_d, rx_d) = channel().expect("channel");
        let tx_f = tx_f.clone();

        let mut delay = DelayBuffer::new();
        delay.set_delay(Duration::from_millis(10));
        let id = sim
            .spawn(
                disco,
                (
                    protocol,
                    tau,
                    phi,
                    format!("peer_id{}", num),
                    1234 + num as u16,
                    rx_d,
                    tx_f,
                ),
                Some(delay),
            )
            .await;
        sim.plug(id, net, None).await;

        (id, tx_d)
    }

    async fn discover(protocol: Protocol) {
        let mut sim = Netsim::<String, String>::new();
        let net = sim.spawn_network(Ipv4Range::random_local_subnet());
        sim.network(net)
            .set_count_filter(Some(Box::new(|bytes: &[u8]| {
                bytes.len() >= 60 && bytes[16..24] == [224, 0, 0, 251, 20, 233, 20, 233]
            })));

        // one channel for the discoveries
        let (tx_f, rx_f) = channel().expect("channel");

        const N: usize = 100;
        let tau = Duration::from_millis(2000);
        let phi = 5f32;

        let mut discovered = BTreeMap::<String, BTreeSet<String>>::new();
        let mut channels = Vec::with_capacity(N);
        for i in 0..N {
            let (_, tx_d) = spawn(&mut sim, net, protocol, tau, phi, i, tx_f.clone()).await;
            discovered.insert(format!("peer_id{}", i), BTreeSet::new());
            channels.push(tx_d);
            thread::sleep(Duration::from_millis(10));
        }
        tracing::info!("waiting for discovery");

        let start = Instant::now();
        let mut discoveries = 0;
        loop {
            let Disco::Discover { host, peer, .. } =
                rx_f.try_recv_timeout(Duration::from_secs(5)).unwrap()
            else {
                continue;
            };
            discoveries += 1;
            tracing::info!(
                "{} discovered {} ({} remaining)",
                host,
                peer,
                discovered.len()
            );
            if discovered.len() < 5 {
                tracing::info!("discovered: {:?}", discovered.keys().collect::<Vec<_>>());
            }
            if let Some(set) = discovered.get_mut(&host) {
                set.insert(peer.to_owned());
                if set.len() == N {
                    discovered.remove(&host);
                }
                if discovered.is_empty() {
                    break;
                }
            }
        }
        let elapsed = start.elapsed();

        let forwarded = sim.network(net).num_forwarded();
        tracing::info!(%forwarded, ?elapsed, %discoveries, "discovery complete");

        // phase 1 was getting a swarm of N nodes going - during mass startup
        // there are lots of duplicates, so we expect a lot of forwarded packets
        // the real test is now adding a node and observing its discoveries while
        // seeing basically no duplicates

        let (_, tx_d) = spawn(&mut sim, net, protocol, tau, phi, N, tx_f).await;
        let peer_id = format!("peer_id{}", N);
        let mut discovered = BTreeSet::new();
        let start = Instant::now();
        discoveries = 0;
        while discovered.len() < (N + 1) {
            let Disco::Discover { host, peer, .. } =
                rx_f.try_recv_timeout(Duration::from_secs(5)).unwrap()
            else {
                continue;
            };
            discoveries += 1;
            if host != peer_id {
                continue;
            }
            tracing::info!("{} discovered {}", host, peer);
            discovered.insert(peer);
        }
        let elapsed = start.elapsed();

        for tx_d in channels {
            tx_d.send(()).expect("send");
        }
        tx_d.send(()).expect("send");

        let net = sim.network(net);
        let max =
            // number of responses
            (elapsed.as_secs_f32() * phi).ceil() as usize * (N + 1)
            // number of queries
            + (elapsed.as_secs_f32() / tau.as_secs_f32()).ceil() as usize * (N + 1);
        let forwarded2 = net.num_forwarded() - forwarded;
        tracing::info!(%forwarded2, ?elapsed, %discoveries, "additional discovery complete");
        assert!(forwarded2 < max, "forwarded {} >= {}", forwarded2, max);
        assert_eq!(net.num_disabled(), 0);
        assert_eq!(net.num_invalid(), 0);
        assert_eq!(net.num_unroutable(), 0);
    }

    fn discover_udp() {
        Builder::new_current_thread()
            .build()
            .expect("build runtime")
            .block_on(discover(Protocol::Udp));
    }

    fn discover_tcp() {
        Builder::new_current_thread()
            .build()
            .expect("build runtime")
            .block_on(discover(Protocol::Tcp));
    }

    fn gc() {
        let rt = Builder::new_current_thread()
            .build()
            .expect("build runtime");

        let mut sim = Netsim::<String, String>::new();
        let net = sim.spawn_network(Ipv4Range::random_local_subnet());

        // one channel for the discoveries
        let (tx_f, rx_f) = channel().expect("channel");

        let (_, tx1) = rt.block_on(spawn(
            &mut sim,
            net,
            Protocol::Udp,
            Duration::from_secs(1),
            1f32,
            0,
            tx_f.clone(),
        ));

        let (_, tx2) = rt.block_on(spawn(
            &mut sim,
            net,
            Protocol::Udp,
            Duration::from_secs(1),
            1f32,
            1,
            tx_f.clone(),
        ));

        let (_, tx3) = rt.block_on(spawn(
            &mut sim,
            net,
            Protocol::Udp,
            Duration::from_secs(1),
            1f32,
            2,
            tx_f.clone(),
        ));

        let mut discovered = BTreeSet::new();
        while discovered.len() < 9 {
            let Disco::Discover { host, peer, .. } =
                rx_f.try_recv_timeout(Duration::from_secs(5)).unwrap()
            else {
                continue;
            };
            tracing::info!("{} discovered {}", host, peer);
            discovered.insert((host, peer));
        }

        // kill the first peer
        tx1.send(()).expect("send");

        let mut forgotten = BTreeSet::new();
        while forgotten.len() < 2 {
            let Disco::Forget { host, peer } =
                rx_f.try_recv_timeout(Duration::from_secs(5)).unwrap()
            else {
                continue;
            };
            assert_ne!(host, "peer_id0");
            assert_eq!(peer, "peer_id0");
            tracing::info!("{} forgot {}", host, peer);
            forgotten.insert(host);
        }

        tx2.send(()).expect("send");
        tx3.send(()).expect("send");
    }

    fn simple() {
        let rt = Builder::new_current_thread()
            .build()
            .expect("build runtime");

        let mut sim = Netsim::<String, String>::new();
        let net = sim.spawn_network(Ipv4Range::random_local_subnet());

        // one channel for the discoveries
        let (tx_f, rx_f) = channel().expect("channel");

        let (_, tx1) = rt.block_on(spawn(
            &mut sim,
            net,
            Protocol::Udp,
            Duration::from_secs(1),
            1f32,
            0,
            tx_f.clone(),
        ));

        let (_, tx2) = rt.block_on(spawn(
            &mut sim,
            net,
            Protocol::Udp,
            Duration::from_secs(1),
            1f32,
            1,
            tx_f.clone(),
        ));

        let Disco::Discover {
            host,
            peer,
            addrs,
            txt,
        } = rx_f.try_recv_timeout(Duration::from_secs(5)).expect("recv")
        else {
            panic!("no discovery");
        };
        assert!(host.starts_with("peer_id"));
        assert!(peer.starts_with("peer_id"));
        assert!(addrs.len() > 1);
        let mut addr_map = BTreeMap::new();
        for (ip, port) in addrs {
            addr_map.entry(port).or_insert_with(Vec::new).push(ip);
        }
        assert_eq!(addr_map.len(), 2);
        let mut port_iter = addr_map.keys();
        let port1 = port_iter.next().expect("port1");
        let port2 = port_iter.next().expect("port2");
        assert_eq!(addr_map.get(port2).expect("port2").len(), 1);
        let addr = addr_map.get(port2).expect("port2")[0];
        let addrs = addr_map.get(port1).expect("port1");
        assert!(addrs.len() > 1);
        assert!(addrs.iter().any(|a| *a == addr));

        let mut expected_txt = BTreeMap::new();
        expected_txt.insert("føø".to_string(), Some("bär".to_string()));
        expected_txt.insert("name".to_string(), Some(format!("peer={peer}")));
        expected_txt.insert("bool".to_string(), None);
        assert_eq!(txt, expected_txt, "txt mismatch");

        tx1.send(()).expect("send");
        tx2.send(()).expect("send");
    }

    fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_test_writer()
        .init();

    declare_machines!(disco);
    run_tests!(discover_udp, discover_tcp, gc, simple);
}
