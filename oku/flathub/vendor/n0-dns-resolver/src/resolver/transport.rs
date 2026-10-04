//! DNS transport implementations: UDP, TCP, TLS, and HTTPS.

#[cfg(with_rustls)]
use std::sync::Arc;
use std::{io, net::SocketAddr};

use n0_error::{e, stack_error};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tracing::debug;

use super::{pool::ConnPool, query};

/// A network or transport-level failure while querying a single nameserver.
///
/// The source of [`crate::Error::Transport`]; match on it for the specific cause.
#[stack_error(derive, add_meta, std_sources)]
#[non_exhaustive]
pub enum TransportError {
    /// A socket read or write failed.
    #[error("transport I/O failed")]
    Io {
        /// The underlying I/O error.
        #[error(from)]
        source: io::Error,
    },
    /// The query did not fit the 2-byte length prefix used for TCP and DoT.
    #[error("query too large for TCP framing")]
    QueryTooLarge {},
    /// A TCP or DoT frame did not answer the query.
    ///
    /// A connection that delivers one is dropped rather than returned to the
    /// pool, so a stale frame cannot be handed to the next query on it.
    #[error("response frame does not answer the query")]
    UnexpectedResponse {},
    /// The configured TLS server name is not a valid DNS name for SNI.
    #[cfg(transport_tls)]
    #[error("invalid TLS server name: {name}")]
    InvalidServerName {
        /// The rejected server name.
        name: String,
    },
    /// A DNS-over-HTTPS request failed at the HTTP layer.
    #[cfg(transport_https)]
    #[error("DNS-over-HTTPS request failed")]
    Http {
        /// The underlying reqwest error.
        #[error(from)]
        source: reqwest::Error,
    },
    /// The DNS-over-HTTPS client could not be constructed.
    #[cfg(transport_https)]
    #[error("failed to build HTTPS client")]
    BuildClient {
        /// The reqwest error from building the client.
        source: reqwest::Error,
    },
    /// A DNS-over-HTTPS response body grew past the largest DNS message.
    ///
    /// The body is dropped at that point rather than read to its end.
    #[cfg(transport_https)]
    #[error("DNS-over-HTTPS response body exceeds {MAX_HTTPS_BODY} bytes")]
    ResponseTooLarge {},
}

// TCP and DoT connections are pooled (see the `pool` module) and reused across
// queries, so a DoT handshake is paid once and amortized over repeated
// lookups to the same nameserver.
//
// UDP sockets are intentionally not reused (a new random source port per query
// helps prevent cache poisoning).

/// UDP receive buffer size.
///
/// Well above the advertised EDNS(0) payload of 1232 bytes, so a compliant
/// server's UDP response always fits and a larger answer arrives with the DNS TC
/// bit set (handled by retrying over TCP). A datagram that fills this buffer is
/// treated as possibly truncated and also retried over TCP.
const UDP_RECV_BUFFER: usize = 4096;

/// Sends a DNS query over UDP and reads the response.
///
/// Each query uses a fresh socket with a random ephemeral source port to
/// prevent cache poisoning. The socket keeps listening until a datagram arrives
/// from the nameserver that answers the query (see [`query::answers_query`]);
/// anything else that reaches the port is logged and ignored, so a spoofed or
/// stray datagram cannot end the query ahead of the real answer. The caller
/// bounds the wait with its timeout.
///
/// The returned flag is set when the datagram filled the receive buffer and may
/// be truncated, so the caller can retry over TCP.
pub(super) async fn udp_query(
    addr: SocketAddr,
    query: &[u8],
) -> Result<(Vec<u8>, bool), TransportError> {
    let unspecified: std::net::IpAddr = if addr.is_ipv6() {
        std::net::Ipv6Addr::UNSPECIFIED.into()
    } else {
        std::net::Ipv4Addr::UNSPECIFIED.into()
    };
    let bind_addr = SocketAddr::new(unspecified, 0);
    let socket = tokio::net::UdpSocket::bind(bind_addr).await?;
    socket.send_to(query, addr).await?;

    let mut buf = vec![0u8; UDP_RECV_BUFFER];
    loop {
        let (len, src) = socket.recv_from(&mut buf).await?;
        if src != addr {
            debug!(%addr, %src, "ignoring UDP datagram from an unexpected source");
            continue;
        }
        if !query::answers_query(&buf[..len], query) {
            debug!(%addr, len, "ignoring UDP datagram that does not answer the query");
            continue;
        }
        // A datagram that fills the whole buffer may have been truncated at the
        // socket by a sender that ignored our advertised EDNS payload size. The
        // DNS TC bit only covers server-side truncation, so flag this separately
        // and let the caller retry over TCP rather than parse a partial message.
        let maybe_truncated = len == buf.len();
        buf.truncate(len);
        return Ok((buf, maybe_truncated));
    }
}

/// Sends a length-prefixed DNS query on an established stream and reads the reply.
///
/// Uses the 2-byte length prefix framing from RFC 1035 Section 4.2.2. Shared by
/// TCP and DoT.
///
/// The frame read back must answer the query (see [`query::answers_query`]).
/// One query is in flight per connection, so the next frame can only be the
/// answer or something the server sent unasked, such as a second frame after
/// its last answer. Taking that as the answer would fail this query, and
/// leaving it unread would hand it to the next query on the connection once
/// pooled. The error makes the caller drop the connection instead.
async fn framed_query<S>(stream: &mut S, query: &[u8]) -> Result<Vec<u8>, TransportError>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let len = u16::try_from(query.len())
        .map_err(|_| e!(TransportError::QueryTooLarge))?
        .to_be_bytes();
    stream.write_all(&len).await?;
    stream.write_all(query).await?;
    stream.flush().await?;

    let resp_len = stream.read_u16().await? as usize;
    let mut buf = vec![0u8; resp_len];
    stream.read_exact(&mut buf).await?;
    if !query::answers_query(&buf, query) {
        return Err(e!(TransportError::UnexpectedResponse));
    }
    Ok(buf)
}

/// Sends a DNS query over TCP, reusing a pooled connection when one is available.
///
/// A pooled connection may have been closed by the server while idle, or hold
/// a frame the server sent after its last answer; either only surfaces on the
/// first use, so on failure we dial a fresh connection and retry the query
/// once.
pub(super) async fn tcp_query(
    pool: &ConnPool,
    addr: SocketAddr,
    query: &[u8],
) -> Result<Vec<u8>, TransportError> {
    if let Some(mut stream) = pool.checkout_tcp(addr)
        && let Ok(resp) = framed_query(&mut stream, query).await
    {
        pool.checkin_tcp(addr, stream);
        return Ok(resp);
    }
    let mut stream = tokio::net::TcpStream::connect(addr).await?;
    let resp = framed_query(&mut stream, query).await?;
    pool.checkin_tcp(addr, stream);
    Ok(resp)
}

/// Sends a DNS query over TLS (DNS-over-TLS, RFC 7858).
///
/// With `server_name`, that name is used for the TLS handshake and certificate
/// validation; without it the certificate is validated against the IP address,
/// which works for providers that list the IP in their certificate (e.g. Google
/// `8.8.8.8`, Cloudflare `1.1.1.1`) but not for those whose certificates only
/// cover a hostname.
///
/// Reuses a pooled connection when one is available, retrying once on a fresh
/// connection if a pooled one turns out to have been closed while idle.
#[cfg(transport_tls)]
pub(super) async fn tls_query(
    pool: &ConnPool,
    addr: SocketAddr,
    query: &[u8],
    tls_config: &Arc<rustls::ClientConfig>,
    server_name: Option<&str>,
) -> Result<Vec<u8>, TransportError> {
    let key = (addr, server_name.map(str::to_string));
    if let Some(mut stream) = pool.checkout_tls(&key)
        && let Ok(resp) = framed_query(&mut stream, query).await
    {
        pool.checkin_tls(key, stream);
        return Ok(resp);
    }

    let connector = tokio_rustls::TlsConnector::from(tls_config.clone());
    let tcp_stream = tokio::net::TcpStream::connect(addr).await?;
    // Use the explicit server name for SNI and validation if given, otherwise
    // validate against the IP the connection was made to.
    let sni = match server_name {
        Some(name) => rustls::pki_types::ServerName::try_from(name.to_string()).map_err(|_| {
            e!(TransportError::InvalidServerName {
                name: name.to_string()
            })
        })?,
        None => rustls::pki_types::ServerName::IpAddress(addr.ip().into()),
    };
    let mut stream = connector.connect(sni, tcp_stream).await?;
    let resp = framed_query(&mut stream, query).await?;
    pool.checkin_tls(key, stream);
    Ok(resp)
}

/// Builds a [`reqwest::Client`] for DNS-over-HTTPS queries.
///
/// `resolves` pins each named DoH host to a fixed address, so a hostname-based
/// DoH URL connects to that IP instead of being resolved recursively.
#[cfg(transport_https)]
pub(super) fn build_https_client(
    tls_config: &Arc<rustls::ClientConfig>,
    resolves: &[(String, SocketAddr)],
) -> Result<reqwest::Client, TransportError> {
    // reqwest wraps the argument in an `Option` and downcasts to
    // `Option<rustls::ClientConfig>`, so hand it a bare `ClientConfig` (not the
    // `Arc`), or it rejects it as an unknown backend at build time.
    let mut builder = reqwest::Client::builder().use_preconfigured_tls((**tls_config).clone());
    for (host, addr) in resolves {
        builder = builder.resolve(host, *addr);
    }
    builder
        .build()
        .map_err(|source| e!(TransportError::BuildClient { source }))
}

/// Largest DNS-over-HTTPS response body read, in bytes.
///
/// A DNS message is at most 65535 bytes, the most the length prefix of the
/// TCP framing can express, so a body past that cannot be one. Reading the
/// body to its end before checking would let a server have us buffer as much
/// as it cares to send; [`read_body`] stops at this point instead.
#[cfg(transport_https)]
const MAX_HTTPS_BODY: usize = u16::MAX as usize;

/// Reads a DNS-over-HTTPS response body, giving up once it exceeds [`MAX_HTTPS_BODY`].
#[cfg(transport_https)]
async fn read_body(mut response: reqwest::Response) -> Result<Vec<u8>, TransportError> {
    if response
        .content_length()
        .is_some_and(|len| len > MAX_HTTPS_BODY as u64)
    {
        return Err(e!(TransportError::ResponseTooLarge));
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if body.len() + chunk.len() > MAX_HTTPS_BODY {
            return Err(e!(TransportError::ResponseTooLarge));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

/// Sends a DNS query over HTTPS (DNS-over-HTTPS, RFC 8484).
///
/// With `server_name`, the URL is addressed by hostname (the client pins it to
/// `addr`); without it the URL is addressed by IP (e.g.
/// `https://1.1.1.1/dns-query`), which works only for providers whose
/// certificates include the IP address.
#[cfg(transport_https)]
pub(super) async fn https_query(
    addr: SocketAddr,
    server_name: Option<&str>,
    query: &[u8],
    client: &reqwest::Client,
) -> Result<Vec<u8>, TransportError> {
    let url = match server_name {
        Some(name) => format!("https://{name}:{}/dns-query", addr.port()),
        None => format!("https://{addr}/dns-query"),
    };
    let response = client
        .post(&url)
        .header("content-type", "application/dns-message")
        .header("accept", "application/dns-message")
        .body(query.to_vec())
        .send()
        .await?;

    read_body(response.error_for_status()?).await
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use simple_dns::{
        CLASS, Name, Packet, PacketFlag, QCLASS, QTYPE, Question, ResourceRecord, TYPE,
        rdata::{A, RData},
    };
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    use super::*;
    use crate::Record;

    /// Parses a response down to its A record addresses and TTL.
    ///
    /// Lets the transport tests compare against a plain `Vec<Ipv4Addr>`.
    fn parse_a_addrs(data: &[u8]) -> (Vec<Ipv4Addr>, u32) {
        let (records, ttl) =
            super::super::query::parse_records(data, crate::RecordKind::A).unwrap();
        let addrs = records
            .into_iter()
            .filter_map(|r| match r {
                Record::A(ip) => Some(ip),
                _ => None,
            })
            .collect();
        (addrs, ttl)
    }

    fn build_a_response(id: u16, addrs: &[Ipv4Addr]) -> Vec<u8> {
        build_a_response_for(id, "example.com", addrs)
    }

    /// Builds an A response for `name`, echoing a question for it.
    fn build_a_response_for(id: u16, name: &str, addrs: &[Ipv4Addr]) -> Vec<u8> {
        let mut packet = Packet::new_reply(id);
        packet.set_flags(PacketFlag::RECURSION_DESIRED | PacketFlag::RECURSION_AVAILABLE);
        // Echo the question section, as a real server does.
        packet.questions.push(Question::new(
            Name::new_unchecked(name),
            QTYPE::TYPE(TYPE::A),
            QCLASS::CLASS(CLASS::IN),
            false,
        ));
        for addr in addrs {
            let rdata = RData::A(A {
                address: u32::from(*addr),
            });
            packet.answers.push(ResourceRecord::new(
                Name::new_unchecked(name),
                CLASS::IN,
                300,
                rdata,
            ));
        }
        packet.build_bytes_vec().unwrap()
    }

    /// Spawns a mock UDP server that echoes back an A response for any query.
    async fn mock_udp_server(addrs: &[Ipv4Addr]) -> (SocketAddr, tokio::task::JoinHandle<()>) {
        let server = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let server_addr = server.local_addr().unwrap();
        let addrs = addrs.to_vec();
        let handle = tokio::spawn(async move {
            let mut buf = vec![0u8; 4096];
            let (len, client_addr) = server.recv_from(&mut buf).await.unwrap();
            let id = Packet::parse(&buf[..len]).unwrap().id();
            server
                .send_to(&build_a_response(id, &addrs), client_addr)
                .await
                .unwrap();
        });
        (server_addr, handle)
    }

    /// Spawns a mock TCP server that echoes back an A response for any query.
    async fn mock_tcp_server(addrs: &[Ipv4Addr]) -> (SocketAddr, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let server_addr = listener.local_addr().unwrap();
        let addrs = addrs.to_vec();
        let handle = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let query_len = stream.read_u16().await.unwrap() as usize;
            let mut query_buf = vec![0u8; query_len];
            stream.read_exact(&mut query_buf).await.unwrap();
            let id = Packet::parse(&query_buf).unwrap().id();
            let resp = build_a_response(id, &addrs);
            stream
                .write_all(&(resp.len() as u16).to_be_bytes())
                .await
                .unwrap();
            stream.write_all(&resp).await.unwrap();
            stream.flush().await.unwrap();
        });
        (server_addr, handle)
    }

    fn build_query() -> (u16, Vec<u8>) {
        super::super::query::build_query("example.com", TYPE::A).unwrap()
    }

    #[tokio::test]
    async fn test_udp_query() {
        let (addr, handle) = mock_udp_server(&[Ipv4Addr::new(93, 184, 216, 34)]).await;
        let (_, query) = build_query();
        let (addrs, _) = parse_a_addrs(&udp_query(addr, &query).await.unwrap().0);
        assert_eq!(addrs, [Ipv4Addr::new(93, 184, 216, 34)]);
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn test_tcp_query() {
        let (addr, handle) = mock_tcp_server(&[Ipv4Addr::new(93, 184, 216, 34)]).await;
        let (_, query) = build_query();
        let pool = ConnPool::new();
        let (addrs, _) = parse_a_addrs(&tcp_query(&pool, addr, &query).await.unwrap());
        assert_eq!(addrs, [Ipv4Addr::new(93, 184, 216, 34)]);
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn test_udp_multiple_records() {
        let expected = [
            Ipv4Addr::new(1, 2, 3, 4),
            Ipv4Addr::new(5, 6, 7, 8),
            Ipv4Addr::new(9, 10, 11, 12),
        ];
        let (addr, handle) = mock_udp_server(&expected).await;
        let (_, query) = build_query();
        let (addrs, ttl) = parse_a_addrs(&udp_query(addr, &query).await.unwrap().0);
        assert_eq!(addrs, expected);
        assert_eq!(ttl, 300);
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn test_tcp_large_response() {
        let expected: Vec<Ipv4Addr> = (0..50).map(|i| Ipv4Addr::new(10, 0, 0, i)).collect();
        let (addr, handle) = mock_tcp_server(&expected).await;
        let (_, query) = build_query();
        let pool = ConnPool::new();
        let (addrs, _) = parse_a_addrs(&tcp_query(&pool, addr, &query).await.unwrap());
        assert_eq!(addrs, expected);
        handle.await.unwrap();
    }

    /// The socket keeps listening past datagrams that do not answer the query.
    ///
    /// An off-path attacker who guesses the source port can deliver anything to
    /// it. The mock sends, ahead of the real answer, an answer from another
    /// socket, one with a wrong id, one for a different question, and a short
    /// datagram. Each would have ended the query with a mismatch or the wrong
    /// records had it been accepted.
    #[tokio::test]
    async fn udp_query_ignores_datagrams_that_do_not_answer_the_query() {
        let expected = Ipv4Addr::new(93, 184, 216, 34);
        let spoofed = Ipv4Addr::new(6, 6, 6, 6);
        let server = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let addr = server.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            let mut buf = vec![0u8; 4096];
            let (len, peer) = server.recv_from(&mut buf).await.unwrap();
            let id = Packet::parse(&buf[..len]).unwrap().id();

            let other = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
            other
                .send_to(&build_a_response(id, &[spoofed]), peer)
                .await
                .unwrap();
            for datagram in [
                build_a_response(id.wrapping_add(1), &[spoofed]),
                build_a_response_for(id, "attacker.example", &[spoofed]),
                vec![0u8; 4],
                build_a_response(id, &[expected]),
            ] {
                server.send_to(&datagram, peer).await.unwrap();
            }
        });

        let (_, query) = build_query();
        let (addrs, _) = parse_a_addrs(&udp_query(addr, &query).await.unwrap().0);
        assert_eq!(addrs, [expected]);
        handle.await.unwrap();
    }

    /// A DoH body past the largest DNS message is dropped, not buffered.
    ///
    /// One that just fits is read whole.
    #[cfg(transport_https)]
    #[tokio::test]
    async fn https_body_is_capped_at_the_dns_message_maximum() {
        let fits = vec![0u8; MAX_HTTPS_BODY];
        let response = reqwest::Response::from(http::Response::new(fits.clone()));
        assert_eq!(read_body(response).await.unwrap(), fits);

        let too_large = vec![0u8; MAX_HTTPS_BODY + 1];
        let response = reqwest::Response::from(http::Response::new(too_large));
        assert!(matches!(
            read_body(response).await,
            Err(TransportError::ResponseTooLarge { .. })
        ));
    }

    /// A frame that does not answer the query is an error, not the answer.
    #[tokio::test]
    async fn framed_query_rejects_a_frame_for_another_query() {
        let (mut client, mut server) = tokio::io::duplex(4096);
        let (id, query) = build_query();
        let stale = build_a_response(id.wrapping_add(1), &[Ipv4Addr::new(6, 6, 6, 6)]);
        server
            .write_all(&(stale.len() as u16).to_be_bytes())
            .await
            .unwrap();
        server.write_all(&stale).await.unwrap();

        assert!(matches!(
            framed_query(&mut client, &query).await,
            Err(TransportError::UnexpectedResponse { .. })
        ));
    }

    /// A pooled connection holding a stale frame is dropped, not reused.
    ///
    /// The mock answers the first query and then sends a frame nobody asked
    /// for. The next query on the pooled connection reads that frame, must not
    /// take it as its answer, and gets a fresh connection instead, which is why
    /// the mock accepts twice.
    #[tokio::test]
    async fn tcp_query_drops_a_pooled_connection_with_a_stale_frame() {
        let expected = Ipv4Addr::new(93, 184, 216, 34);
        let spoofed = Ipv4Addr::new(6, 6, 6, 6);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            /// Reads one framed query and returns its id.
            async fn read_query(stream: &mut tokio::net::TcpStream) -> u16 {
                let len = stream.read_u16().await.unwrap() as usize;
                let mut buf = vec![0u8; len];
                stream.read_exact(&mut buf).await.unwrap();
                Packet::parse(&buf).unwrap().id()
            }
            /// Writes one framed message.
            async fn write_frame(stream: &mut tokio::net::TcpStream, frame: &[u8]) {
                stream
                    .write_all(&(frame.len() as u16).to_be_bytes())
                    .await
                    .unwrap();
                stream.write_all(frame).await.unwrap();
                stream.flush().await.unwrap();
            }

            let (mut first, _) = listener.accept().await.unwrap();
            let id = read_query(&mut first).await;
            write_frame(&mut first, &build_a_response(id, &[expected])).await;
            write_frame(&mut first, &build_a_response(id ^ 1, &[spoofed])).await;

            let (mut second, _) = listener.accept().await.unwrap();
            let id = read_query(&mut second).await;
            write_frame(&mut second, &build_a_response(id, &[expected])).await;
            // Keep the first connection open until the second query is done,
            // so the stale frame is read rather than a reset.
            drop(first);
        });

        let pool = ConnPool::new();
        let (_, query) = build_query();
        let (addrs, _) = parse_a_addrs(&tcp_query(&pool, addr, &query).await.unwrap());
        assert_eq!(addrs, [expected]);

        let (_, query) = build_query();
        let (addrs, _) = parse_a_addrs(&tcp_query(&pool, addr, &query).await.unwrap());
        assert_eq!(addrs, [expected]);
        handle.await.unwrap();
    }

    /// A datagram that exactly fills the receive buffer is flagged as truncated.
    ///
    /// It may have been cut off, so the caller can retry over TCP.
    #[tokio::test]
    async fn udp_query_flags_full_buffer_as_maybe_truncated() {
        let server = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let addr = server.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            let mut buf = [0u8; 512];
            let (len, peer) = server.recv_from(&mut buf).await.unwrap();
            // A real answer, padded out to the buffer.
            let id = Packet::parse(&buf[..len]).unwrap().id();
            let mut reply = build_a_response(id, &[Ipv4Addr::new(1, 2, 3, 4)]);
            reply.resize(UDP_RECV_BUFFER, 0);
            server.send_to(&reply, peer).await.unwrap();
        });
        let (_, query) = build_query();
        let (resp, maybe_truncated) = udp_query(addr, &query).await.unwrap();
        assert_eq!(resp.len(), UDP_RECV_BUFFER);
        assert!(maybe_truncated);
        handle.await.unwrap();
    }
}
