use netlink_packet_core::{NLM_F_DUMP, NLM_F_REQUEST, NetlinkMessage, NetlinkPayload};
use netlink_packet_route::{
    RouteNetlinkMessage,
    address::{AddressAttribute, AddressFlags, AddressMessage},
    link::{LinkAttribute, LinkMessage},
};
use netlink_sys::{Socket, SocketAddr, protocols::NETLINK_ROUTE};
use std::io::ErrorKind;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::{
    collections::HashMap,
    io, thread,
    time::{Duration, Instant},
};

#[cfg(feature = "gateway")]
use netlink_packet_route::AddressFamily;
#[cfg(feature = "gateway")]
use netlink_packet_route::neighbour::{NeighbourAddress, NeighbourAttribute, NeighbourMessage};
#[cfg(feature = "gateway")]
use netlink_packet_route::route::{RouteAddress, RouteAttribute, RouteMessage};

const SEQ_BASE: u32 = 0x6E_64_65_76; // "ndev"
const RECV_BUFSZ: usize = 1 << 20; // 1MB
const RECV_TIMEOUT: Duration = Duration::from_secs(2);
const NLMSG_ALIGNTO: usize = 4;
const MIN_NLMSG_HEADER_LEN: usize = 16;

#[inline]
fn nlmsg_align(n: usize) -> usize {
    (n + NLMSG_ALIGNTO - 1) & !(NLMSG_ALIGNTO - 1)
}

fn open_route_socket() -> io::Result<Socket> {
    let mut sock = Socket::new(NETLINK_ROUTE)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("netlink open: {e}")))?;
    sock.bind_auto()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("bind_auto: {e}")))?;
    sock.set_non_blocking(true).ok();
    Ok(sock)
}

fn send_dump(sock: &mut Socket, msg: RouteNetlinkMessage, seq: u32) -> io::Result<()> {
    let mut nl = NetlinkMessage::from(msg);
    nl.header.flags = NLM_F_REQUEST | NLM_F_DUMP;
    nl.header.sequence_number = seq;
    nl.header.port_number = 0;

    // Finalize to set length
    nl.finalize();

    let blen = nl.buffer_len();
    if blen < MIN_NLMSG_HEADER_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("netlink message too short: buffer_len={}", blen),
        ));
    }

    let mut buf = vec![0; blen];
    nl.serialize(&mut buf);

    let kernel = SocketAddr::new(0, 0);
    sock.send_to(&buf, &kernel, 0)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("netlink send: {e}")))?;
    Ok(())
}

fn recv_multi(
    sock: &mut Socket,
    expect_seq: u32,
) -> io::Result<Vec<NetlinkMessage<RouteNetlinkMessage>>> {
    let mut out = Vec::new();
    let mut buf = vec![0u8; RECV_BUFSZ];
    let kernel = SocketAddr::new(0, 0);
    let deadline = Instant::now() + RECV_TIMEOUT;

    loop {
        match sock.recv_from(&mut &mut buf[..], 0) {
            Ok((size, from)) => {
                let _ = from == kernel;
                let mut offset = 0usize;

                while offset < size {
                    if size - offset < MIN_NLMSG_HEADER_LEN {
                        break;
                    }

                    let bytes = &buf[offset..size];

                    let msg =
                        NetlinkMessage::<RouteNetlinkMessage>::deserialize(bytes).map_err(|e| {
                            io::Error::new(
                                io::ErrorKind::InvalidData,
                                format!("deserialize: {e:?}"),
                            )
                        })?;

                    let consumed = msg.header.length as usize;
                    if consumed < MIN_NLMSG_HEADER_LEN || offset + consumed > size {
                        break;
                    }

                    if msg.header.sequence_number != expect_seq {
                        offset += nlmsg_align(consumed);
                        continue;
                    }

                    match &msg.payload {
                        NetlinkPayload::Done(_) => {
                            return Ok(out);
                        }
                        NetlinkPayload::Error(e) => {
                            if let Some(code) = e.code {
                                return Err(io::Error::new(
                                    io::ErrorKind::Other,
                                    format!("netlink error: code={}", code),
                                ));
                            }
                            // code==None: possibly ACK ... ignore
                        }
                        NetlinkPayload::Noop | NetlinkPayload::Overrun(_) => { /* skip */ }
                        _ => out.push(msg),
                    }

                    // Align to 4-byte boundary
                    offset += nlmsg_align(consumed);
                }
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                if Instant::now() >= deadline {
                    // timeout
                    return Ok(out);
                }
                thread::sleep(Duration::from_millis(5));
            }
            Err(e) => return Err(e),
        }
    }
}

pub fn dump_links() -> io::Result<Vec<LinkMessage>> {
    let mut sock = open_route_socket()?;
    let seq = SEQ_BASE ^ 0x01;
    send_dump(
        &mut sock,
        RouteNetlinkMessage::GetLink(LinkMessage::default()),
        seq,
    )?;
    let msgs = recv_multi(&mut sock, seq)?;
    let mut out = Vec::new();
    for m in msgs {
        if let NetlinkPayload::InnerMessage(RouteNetlinkMessage::NewLink(link)) = m.payload {
            out.push(link);
        }
    }
    Ok(out)
}

pub fn dump_addrs() -> io::Result<Vec<AddressMessage>> {
    let mut sock = open_route_socket()?;
    let seq = SEQ_BASE ^ 0x02;
    send_dump(
        &mut sock,
        RouteNetlinkMessage::GetAddress(AddressMessage::default()),
        seq,
    )?;
    let msgs = recv_multi(&mut sock, seq)?;
    let mut out = Vec::new();
    for m in msgs {
        if let NetlinkPayload::InnerMessage(RouteNetlinkMessage::NewAddress(addr)) = m.payload {
            out.push(addr);
        }
    }
    Ok(out)
}

#[cfg(feature = "gateway")]
pub fn dump_routes() -> io::Result<Vec<RouteMessage>> {
    let mut sock = open_route_socket()?;
    let seq = SEQ_BASE ^ 0x03;
    send_dump(
        &mut sock,
        RouteNetlinkMessage::GetRoute(RouteMessage::default()),
        seq,
    )?;
    let msgs = recv_multi(&mut sock, seq)?;
    let mut out = Vec::new();
    for m in msgs {
        if let NetlinkPayload::InnerMessage(RouteNetlinkMessage::NewRoute(rt)) = m.payload {
            out.push(rt);
        }
    }
    Ok(out)
}

#[cfg(feature = "gateway")]
pub fn dump_neigh() -> io::Result<Vec<NeighbourMessage>> {
    let mut sock = open_route_socket()?;
    let seq = SEQ_BASE ^ 0x04;
    send_dump(
        &mut sock,
        RouteNetlinkMessage::GetNeighbour(NeighbourMessage::default()),
        seq,
    )?;
    let msgs = recv_multi(&mut sock, seq)?;
    let mut out = Vec::new();
    for m in msgs {
        if let NetlinkPayload::InnerMessage(RouteNetlinkMessage::NewNeighbour(n)) = m.payload {
            out.push(n);
        }
    }
    Ok(out)
}

fn mac_from_link(link: &LinkMessage) -> Option<[u8; 6]> {
    for nla in &link.attributes {
        if let LinkAttribute::Address(bytes) = nla {
            if bytes.len() == 6 {
                return Some([bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5]]);
            }
        }
    }
    None
}

fn name_from_link(link: &LinkMessage) -> Option<String> {
    for nla in &link.attributes {
        if let LinkAttribute::IfName(n) = nla {
            return Some(n.clone());
        }
    }
    None
}

fn ip_from_addr(addr: &AddressMessage) -> Option<(IpAddr, u8, u32)> {
    let pfx = addr.header.prefix_len;
    let mut ip_out = None;
    let mut flags: Option<AddressFlags> = None;
    for nla in &addr.attributes {
        match nla {
            AddressAttribute::Local(ip) | AddressAttribute::Address(ip) => {
                ip_out = Some(*ip);
            }
            AddressAttribute::Flags(f) => {
                flags = Some(*f);
            }
            _ => {}
        }
    }
    let addr_flags = flags
        .map(|f| f.bits())
        .unwrap_or(addr.header.flags.bits() as u32);
    ip_out.map(|ip| (ip, pfx, addr_flags))
}

#[cfg(feature = "gateway")]
fn route_addr_to_ip(a: &RouteAddress) -> Option<IpAddr> {
    match a {
        RouteAddress::Inet(v4) => Some(IpAddr::V4(*v4)),
        RouteAddress::Inet6(v6) => Some(IpAddr::V6(*v6)),
        _ => None,
    }
}

#[cfg(feature = "gateway")]
fn route_extract(rt: &RouteMessage) -> (Option<IpAddr>, Option<u8>, Option<IpAddr>, Option<u32>) {
    // (dst, prefix, gateway, oif)
    let mut dst: Option<IpAddr> = None;
    let pfx: Option<u8> = Some(rt.header.destination_prefix_length);
    let mut gw: Option<IpAddr> = None;
    let mut oif: Option<u32> = None;

    for nla in &rt.attributes {
        match nla {
            RouteAttribute::Destination(a) => dst = route_addr_to_ip(a),
            RouteAttribute::Gateway(a) => gw = route_addr_to_ip(a),
            RouteAttribute::Oif(i) => oif = Some(*i),
            _ => {}
        }
    }

    // if dst is None and pfx is 0, it means default route
    if dst.is_none() && pfx == Some(0) {
        dst = match rt.header.address_family {
            AddressFamily::Inet => Some(IpAddr::V4(Ipv4Addr::UNSPECIFIED)),
            AddressFamily::Inet6 => Some(IpAddr::V6(Ipv6Addr::UNSPECIFIED)),
            _ => None,
        };
    }

    (dst, pfx, gw, oif)
}

#[cfg(feature = "gateway")]
fn neigh_addr_to_ip(a: &NeighbourAddress) -> Option<IpAddr> {
    match a {
        NeighbourAddress::Inet(v4) => Some(IpAddr::V4(*v4)),
        NeighbourAddress::Inet6(v6) => Some(IpAddr::V6(*v6)),
        #[allow(unreachable_patterns)]
        _ => None,
    }
}

#[cfg(feature = "gateway")]
fn neigh_extract(n: &NeighbourMessage) -> (Option<IpAddr>, Option<[u8; 6]>, Option<u32>) {
    let mut ip = None;
    let mut mac = None;
    let mut oif = Some(n.header.ifindex as u32);

    for nla in &n.attributes {
        match nla {
            NeighbourAttribute::Destination(a) => {
                ip = neigh_addr_to_ip(a);
            }
            // Link-layer address (MAC)
            NeighbourAttribute::LinkLayerAddress(v) => {
                if v.len() == 6 {
                    mac = Some([v[0], v[1], v[2], v[3], v[4], v[5]]);
                }
            }
            NeighbourAttribute::IfIndex(i) => oif = Some(*i as u32),
            _ => {}
        }
    }
    (ip, mac, oif)
}

fn mtu_from_link(link: &LinkMessage) -> Option<u32> {
    for nla in &link.attributes {
        if let LinkAttribute::Mtu(m) = nla {
            return Some(*m);
        }
    }
    None
}

#[derive(Debug, Clone)]
pub struct IfRow {
    pub index: u32,
    pub name: String,
    pub mac: Option<[u8; 6]>,
    pub ipv4: Vec<(Ipv4Addr, u8)>,
    pub ipv6: Vec<(Ipv6Addr, u8)>,
    pub ipv6_addr_flags: Vec<u32>,
    pub flags: u32,
    pub mtu: Option<u32>,
}

pub fn collect_interfaces() -> io::Result<Vec<IfRow>> {
    let links = dump_links()?;
    let addrs = dump_addrs()?;

    let mut base: HashMap<u32, IfRow> = HashMap::new();
    for l in links {
        let idx = l.header.index as u32;
        let name = name_from_link(&l).unwrap_or_else(|| idx.to_string());
        let mac = mac_from_link(&l);
        let flags = l.header.flags.bits();
        let mtu_nl = mtu_from_link(&l);
        base.insert(
            idx,
            IfRow {
                index: idx,
                name,
                mac,
                ipv4: vec![],
                ipv6: vec![],
                ipv6_addr_flags: vec![],
                flags,
                mtu: mtu_nl,
            },
        );
    }

    for a in addrs {
        let idx = a.header.index as u32;
        if let Some((ip, pfx, addr_flags)) = ip_from_addr(&a) {
            if let Some(row) = base.get_mut(&idx) {
                match ip {
                    IpAddr::V4(v4) => row.ipv4.push((v4, pfx)),
                    IpAddr::V6(v6) => {
                        row.ipv6.push((v6, pfx));
                        row.ipv6_addr_flags.push(addr_flags);
                    }
                }
            }
        }
    }

    Ok(base.into_values().collect())
}

#[cfg(feature = "gateway")]
#[derive(Debug, Clone)]
pub struct GwRow {
    #[allow(dead_code)]
    pub ifindex: u32,
    pub gw_v4: Vec<Ipv4Addr>,
    pub gw_v6: Vec<Ipv6Addr>,
    pub mac: Option<[u8; 6]>,
}

#[cfg(feature = "gateway")]
pub fn collect_routes() -> io::Result<HashMap<u32, GwRow>> {
    let routes = dump_routes()?;
    let neighs = dump_neigh().unwrap_or_default();

    let mut m: HashMap<u32, GwRow> = HashMap::new();
    for rt in routes {
        let (_dst, pfx, gw, oif) = route_extract(&rt);
        // default route only
        if pfx != Some(0) {
            continue;
        }
        let oif = match oif {
            Some(i) => i,
            None => continue,
        };
        if let Some(gwip) = gw {
            let e = m.entry(oif).or_insert(GwRow {
                ifindex: oif,
                gw_v4: vec![],
                gw_v6: vec![],
                mac: None,
            });
            match gwip {
                IpAddr::V4(v4) => {
                    if !e.gw_v4.contains(&v4) {
                        e.gw_v4.push(v4);
                    }
                }
                IpAddr::V6(v6) => {
                    if !e.gw_v6.contains(&v6) {
                        e.gw_v6.push(v6);
                    }
                }
            }
        }
    }

    for n in neighs {
        let (ip, mac, ifi) = neigh_extract(&n);
        let ifi = match ifi {
            Some(i) => i,
            None => continue,
        };
        if let Some(row) = m.get_mut(&ifi) {
            if let (Some(m6), Some(ip)) = (mac, ip) {
                let hit = match ip {
                    IpAddr::V4(v4) => row.gw_v4.contains(&v4),
                    IpAddr::V6(v6) => row.gw_v6.contains(&v6),
                };
                if hit {
                    row.mac = Some(m6);
                }
            }
        }
    }

    Ok(m)
}
