//! Manual, loopback-only comparison of UDP-loss behaviour with hickory.
//!
//! The assertions for this crate's retransmit and TCP-fallback behaviour live
//! beside the resolver. This ignored test is a lightweight way to print the
//! same comparison while updating those decisions:
//!
//! ```sh
//! cargo test --test lossy_compare -- --ignored --nocapture
//! ```

use std::{
    collections::HashMap,
    future::Future,
    net::{Ipv4Addr, SocketAddr},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use hickory_resolver::{
    TokioResolver,
    config::{NameServerConfig, ResolverConfig},
    net::runtime::TokioRuntimeProvider,
    proto::rr::RData as HickoryRData,
};
use n0_dns_resolver::{DnsProtocol, DnsResolver, Nameserver};
use simple_dns::{
    CLASS, Packet, PacketFlag, ResourceRecord,
    rdata::{A, RData},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream, UdpSocket},
    task::JoinHandle,
};

const ANSWER: Ipv4Addr = Ipv4Addr::new(10, 77, 0, 1);
const LOOKUP_CAP: Duration = Duration::from_secs(7);

#[derive(Clone, Copy)]
enum UdpPolicy {
    Answer,
    DropFirst(u32),
    Blackhole,
}

impl UdpPolicy {
    fn drops(self, id: u16, seen: &Mutex<HashMap<u16, u32>>) -> bool {
        match self {
            Self::Answer => false,
            Self::Blackhole => true,
            Self::DropFirst(n) => {
                let mut seen = seen.lock().expect("poisoned");
                let count = seen.entry(id).or_insert(0);
                *count += 1;
                *count <= n
            }
        }
    }
}

#[derive(Default)]
struct Counts {
    n0_udp: u64,
    n0_tcp: u64,
    hickory_udp: u64,
    hickory_tcp: u64,
}

impl Counts {
    fn record(&mut self, query: &Packet<'_>, tcp: bool) {
        let Some(question) = query.questions.first() else {
            return;
        };
        match (question.qname.to_string().starts_with("n0-"), tcp) {
            (true, false) => self.n0_udp += 1,
            (true, true) => self.n0_tcp += 1,
            (false, false) => self.hickory_udp += 1,
            (false, true) => self.hickory_tcp += 1,
        }
    }
}

fn reply(query: &Packet<'_>) -> Vec<u8> {
    let mut reply = Packet::new_reply(query.id());
    reply.set_flags(PacketFlag::RECURSION_DESIRED | PacketFlag::RECURSION_AVAILABLE);
    let question = query.questions.first().expect("query has a question");
    reply.questions.push(question.clone());
    reply.answers.push(ResourceRecord::new(
        question.qname.clone(),
        CLASS::IN,
        300,
        RData::A(A {
            address: u32::from(ANSWER),
        }),
    ));
    reply.build_bytes_vec().expect("reply builds")
}

struct TestNameserver {
    addr: SocketAddr,
    counts: Arc<Mutex<Counts>>,
    udp: JoinHandle<()>,
    tcp: JoinHandle<()>,
}

impl Drop for TestNameserver {
    fn drop(&mut self) {
        self.udp.abort();
        self.tcp.abort();
    }
}

impl TestNameserver {
    async fn spawn(policy: UdpPolicy) -> Self {
        let (tcp, udp) = 'bind: {
            for _ in 0..16 {
                let tcp = TcpListener::bind("127.0.0.1:0").await.expect("bind TCP");
                if let Ok(udp) = UdpSocket::bind(tcp.local_addr().expect("TCP address")).await {
                    break 'bind (tcp, udp);
                }
            }
            panic!("no port free for both UDP and TCP");
        };
        let addr = tcp.local_addr().expect("TCP address");
        let counts = Arc::new(Mutex::new(Counts::default()));
        let udp_counts = counts.clone();
        let udp = tokio::spawn(async move {
            let seen = Mutex::new(HashMap::new());
            let mut buf = vec![0u8; 4096];
            loop {
                let Ok((len, peer)) = udp.recv_from(&mut buf).await else {
                    return;
                };
                let Ok(query) = Packet::parse(&buf[..len]) else {
                    continue;
                };
                udp_counts.lock().expect("poisoned").record(&query, false);
                if !policy.drops(query.id(), &seen) {
                    let _ = udp.send_to(&reply(&query), peer).await;
                }
            }
        });
        let tcp_counts = counts.clone();
        let tcp = tokio::spawn(async move {
            while let Ok((stream, _)) = tcp.accept().await {
                let counts = tcp_counts.clone();
                tokio::spawn(async move {
                    let _ = serve_tcp(stream, counts).await;
                });
            }
        });
        Self {
            addr,
            counts,
            udp,
            tcp,
        }
    }
}

async fn serve_tcp(mut stream: TcpStream, counts: Arc<Mutex<Counts>>) -> std::io::Result<()> {
    loop {
        let len = stream.read_u16().await? as usize;
        let mut buf = vec![0; len];
        stream.read_exact(&mut buf).await?;
        let Ok(query) = Packet::parse(&buf) else {
            return Ok(());
        };
        counts.lock().expect("poisoned").record(&query, true);
        let response = reply(&query);
        stream
            .write_all(&(response.len() as u16).to_be_bytes())
            .await?;
        stream.write_all(&response).await?;
    }
}

fn n0(server: SocketAddr) -> DnsResolver {
    DnsResolver::builder()
        .nameserver(Nameserver::new(server, DnsProtocol::Udp))
        .build()
}

fn hickory(server: SocketAddr) -> TokioResolver {
    let mut server_config = NameServerConfig::udp_and_tcp(server.ip());
    for connection in &mut server_config.connections {
        connection.port = server.port();
    }
    TokioResolver::builder_with_config(
        ResolverConfig::from_parts(None, vec![], vec![server_config]),
        TokioRuntimeProvider::default(),
    )
    .build()
    .expect("build hickory resolver")
}

async fn measure<F>(lookup: F) -> Option<Duration>
where
    F: Future<Output = Option<Ipv4Addr>>,
{
    let start = Instant::now();
    tokio::time::timeout(LOOKUP_CAP, lookup)
        .await
        .ok()
        .flatten()
        .map(|answer| {
            assert_eq!(answer, ANSWER);
            start.elapsed()
        })
}

async fn compare(label: &str, policy: UdpPolicy) {
    let server = TestNameserver::spawn(policy).await;
    let n0 = n0(server.addr);
    let hickory = hickory(server.addr);
    let n0_time = measure(async {
        n0.lookup_ipv4("n0-lossy.test.")
            .await
            .ok()?
            .first()
            .copied()
    })
    .await;
    let hickory_time = measure(async {
        let lookup = hickory.ipv4_lookup("hickory-lossy.test.").await.ok()?;
        lookup
            .answers()
            .iter()
            .find_map(|answer| match &answer.data {
                HickoryRData::A(address) => Some(address.0),
                _ => None,
            })
    })
    .await;
    let counts = server.counts.lock().expect("poisoned");
    let format_time = |time: Option<Duration>| match time {
        Some(time) => format!("{:.1} ms", time.as_secs_f64() * 1000.0),
        None => "failed".to_owned(),
    };
    println!(
        "{label:24}  n0 {:>10} ({}/{})  hickory {:>10} ({}/{})",
        format_time(n0_time),
        counts.n0_udp,
        counts.n0_tcp,
        format_time(hickory_time),
        counts.hickory_udp,
        counts.hickory_tcp,
    );
}

/// Prints latency and (UDP/TCP) query counts for deterministic UDP loss.
#[tokio::test]
#[ignore = "manual comparison; run with --ignored --nocapture"]
async fn compare_lossy_udp() {
    println!("scenario                  n0 latency (udp/tcp)  hickory latency (udp/tcp)");
    compare("no loss", UdpPolicy::Answer).await;
    compare("first datagram dropped", UdpPolicy::DropFirst(1)).await;
    compare("first two datagrams dropped", UdpPolicy::DropFirst(2)).await;
    compare("UDP black-holed", UdpPolicy::Blackhole).await;
}
