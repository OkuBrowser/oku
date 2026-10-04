//! Behavior tests ported from the reference resolver and the DNS RFCs.
//!
//! Ported tests live here, in one module, rather than in the per-file unit-test
//! blocks next to the code they exercise. That keeps the scenarios modeled on an
//! external reference together and separate from the crate's own unit tests.
//!
//! Every test cites its origin in a comment: the hickory source file and
//! function it is modeled on, or the RFC and section it exercises. The reference
//! is hickory-proto 0.26.1 and hickory-resolver 0.26.1.
//!
//! The tests here drive the public API against [`spawn_mock`], a mock UDP
//! nameserver, so they exercise the full lookup path (search-list expansion,
//! CNAME following, parsing, and caching) rather than an internal function.

mod resolver;

use std::{
    net::{Ipv4Addr, SocketAddr},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use simple_dns::{
    CLASS, CharacterString, Name, Packet, PacketFlag, RCODE, ResourceRecord,
    rdata::{A, CAA, CNAME, MX, NS, RData, SRV, TXT},
};
use tokio::{net::UdpSocket, task::JoinHandle};

use crate::{DnsProtocol, DnsResolver, Nameserver};

/// TTL, in seconds, stamped on every answer the mock nameserver returns.
const ANSWER_TTL: u32 = 300;

/// A mock UDP nameserver for the ported end-to-end tests.
///
/// Bound to a loopback port, it answers each incoming query with the bytes its
/// handler returns and counts the queries it receives, so a test can assert both
/// the resolved records and how many network queries the lookup issued (for
/// example that a cached result served the second lookup).
pub(crate) struct MockServer {
    /// The loopback address the server is bound to.
    addr: SocketAddr,
    /// Queries received so far, shared with the receive loop.
    queries: Arc<AtomicUsize>,
    /// The receive loop, aborted when the server is dropped.
    task: JoinHandle<()>,
}

impl Drop for MockServer {
    fn drop(&mut self) {
        // The receive loop never returns on its own; abort it with the server.
        self.task.abort();
    }
}

impl MockServer {
    /// The loopback address the mock nameserver is bound to.
    pub(crate) fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// The number of queries the mock nameserver has received so far.
    pub(crate) fn query_count(&self) -> usize {
        self.queries.load(Ordering::SeqCst)
    }
}

/// Binds a mock UDP nameserver on a loopback port.
///
/// Each query is answered with the bytes `handler` returns for it.
///
/// `handler` receives the parsed query packet and returns the response bytes to
/// send, or `None` to drop the query without replying. Build the response with
/// [`reply`] and the record constructors in this module.
pub(crate) async fn spawn_mock<F>(handler: F) -> MockServer
where
    F: Fn(&Packet) -> Option<Vec<u8>> + Send + Sync + 'static,
{
    let socket = UdpSocket::bind("127.0.0.1:0").await.expect("bind mock ns");
    let addr = socket.local_addr().expect("mock ns local addr");
    let queries = Arc::new(AtomicUsize::new(0));
    let task_queries = queries.clone();
    let handler = Arc::new(handler);
    let task = tokio::spawn(async move {
        let mut buf = vec![0u8; 512];
        loop {
            let (len, peer) = match socket.recv_from(&mut buf).await {
                Ok(pair) => pair,
                Err(_) => break,
            };
            task_queries.fetch_add(1, Ordering::SeqCst);
            let Ok(query) = Packet::parse(&buf[..len]) else {
                continue;
            };
            if let Some(response) = handler(&query) {
                let _ = socket.send_to(&response, peer).await;
            }
        }
    });
    MockServer {
        addr,
        queries,
        task,
    }
}

/// Builds a resolver that queries only `addr`, over UDP.
///
/// A default builder adds no system config and no fallback tier, so the lookup
/// is hermetic and reaches only the mock.
pub(crate) fn resolver_for(addr: SocketAddr) -> DnsResolver {
    DnsResolver::builder()
        .nameserver(Nameserver::new(addr, DnsProtocol::Udp))
        .build()
}

/// Builds response bytes for `query`.
///
/// Echoes the question, sets `rcode`, and attaches `answers` to the answer
/// section.
///
/// The question echo is what [`crate::resolver`] validates the response against,
/// so a reply built here passes the id, question, and class checks.
pub(crate) fn reply(
    query: &Packet,
    rcode: RCODE,
    answers: Vec<ResourceRecord<'static>>,
) -> Vec<u8> {
    let mut packet = Packet::new_reply(query.id());
    packet.set_flags(PacketFlag::RECURSION_DESIRED | PacketFlag::RECURSION_AVAILABLE);
    *packet.rcode_mut() = rcode;
    for question in &query.questions {
        packet.questions.push(question.clone());
    }
    packet.answers = answers;
    packet.build_bytes_vec().expect("reply builds")
}

/// The qname of the first question in `query`, as a string.
pub(crate) fn qname(query: &Packet) -> Option<String> {
    Some(query.questions.first()?.qname.to_string())
}

/// An A answer mapping `name` to `ip`.
pub(crate) fn a(name: &str, ip: Ipv4Addr) -> ResourceRecord<'static> {
    ResourceRecord::new(
        Name::new_unchecked(name).into_owned(),
        CLASS::IN,
        ANSWER_TTL,
        RData::A(A {
            address: u32::from(ip),
        }),
    )
}

/// A CNAME answer aliasing `name` to `target`.
pub(crate) fn cname(name: &str, target: &str) -> ResourceRecord<'static> {
    ResourceRecord::new(
        Name::new_unchecked(name).into_owned(),
        CLASS::IN,
        ANSWER_TTL,
        RData::CNAME(CNAME(Name::new_unchecked(target).into_owned())),
    )
}

/// An NS answer naming `target` as a nameserver for `name`.
pub(crate) fn ns(name: &str, target: &str) -> ResourceRecord<'static> {
    ResourceRecord::new(
        Name::new_unchecked(name).into_owned(),
        CLASS::IN,
        ANSWER_TTL,
        RData::NS(NS(Name::new_unchecked(target).into_owned())),
    )
}

/// An SRV answer for `name`.
pub(crate) fn srv(
    name: &str,
    priority: u16,
    weight: u16,
    port: u16,
    target: &str,
) -> ResourceRecord<'static> {
    ResourceRecord::new(
        Name::new_unchecked(name).into_owned(),
        CLASS::IN,
        ANSWER_TTL,
        RData::SRV(SRV {
            priority,
            weight,
            port,
            target: Name::new_unchecked(target).into_owned(),
        }),
    )
}

/// An MX answer for `name`.
pub(crate) fn mx(name: &str, preference: u16, exchange: &str) -> ResourceRecord<'static> {
    ResourceRecord::new(
        Name::new_unchecked(name).into_owned(),
        CLASS::IN,
        ANSWER_TTL,
        RData::MX(MX {
            preference,
            exchange: Name::new_unchecked(exchange).into_owned(),
        }),
    )
}

/// A CAA answer for `name`.
pub(crate) fn caa(name: &str, flag: u8, tag: &str, value: &[u8]) -> ResourceRecord<'static> {
    ResourceRecord::new(
        Name::new_unchecked(name).into_owned(),
        CLASS::IN,
        ANSWER_TTL,
        RData::CAA(CAA {
            flag,
            tag: CharacterString::new(tag.as_bytes())
                .expect("valid CAA tag")
                .into_owned(),
            value: value.to_vec().into(),
        }),
    )
}

/// A TXT answer for `name`, one character-string per element of `strings`.
pub(crate) fn txt(name: &str, strings: &[&[u8]]) -> ResourceRecord<'static> {
    let mut record = TXT::new();
    for string in strings {
        record.add_char_string(CharacterString::new(string).expect("valid TXT string"));
    }
    ResourceRecord::new(
        Name::new_unchecked(name).into_owned(),
        CLASS::IN,
        ANSWER_TTL,
        RData::TXT(record.into_owned()),
    )
}
