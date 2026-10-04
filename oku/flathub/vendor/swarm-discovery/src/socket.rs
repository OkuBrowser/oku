use crate::IpClass;
use hickory_proto::op::Message;
use socket2::{Domain, Protocol, Socket, Type};
use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4},
    sync::{Arc, RwLock},
};
use thiserror::Error;
use tokio::net::UdpSocket;

pub const MDNS_PORT: u16 = 5353;
pub const MDNS_IPV4: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 251);
pub const MDNS_IPV6: Ipv6Addr = Ipv6Addr::new(0xff02, 0, 0, 0, 0, 0, 0, 0xfb);

#[derive(Debug)]
#[doc(hidden)]
pub enum IP {
    Ipv4,
    Ipv6,
}

impl std::fmt::Display for IP {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IP::Ipv4 => write!(f, "IPv4"),
            IP::Ipv6 => write!(f, "IPv6"),
        }
    }
}

/// Errors that can occur when creating and configuring an IPv4 or IPv6 socket.
#[derive(Debug, Error)]
pub enum SocketError {
    #[error("{domain}: error creating new socket for")]
    NewSocket {
        domain: IP,
        #[source]
        source: std::io::Error,
    },
    #[error("{domain}: error setting the reuse address")]
    ReuseAddress {
        domain: IP,
        #[source]
        source: std::io::Error,
    },
    #[cfg(unix)]
    #[error("{domain}: error setting the reuse port")]
    ReusePort {
        domain: IP,
        #[source]
        source: std::io::Error,
    },
    #[error("{domain}: error binding the socket for")]
    Bind {
        domain: IP,
        #[source]
        source: std::io::Error,
    },
    #[error("{domain}: error setting multicast loop")]
    SetMulticastLoop {
        domain: IP,
        #[source]
        source: std::io::Error,
    },
    #[error("{domain}: error joining multicast")]
    JoinMulticast {
        domain: IP,
        #[source]
        source: std::io::Error,
    },
    #[error("{domain}: error setting the multicast ttl")]
    MulticastTtl {
        domain: IP,
        #[source]
        source: std::io::Error,
    },
    #[error("{domain}: error setting the multicast interface")]
    SetMulticastIf {
        domain: IP,
        #[source]
        source: std::io::Error,
    },
    #[error("{domain}: error setting the socket to non-blocking mode")]
    SetNonBlocking {
        domain: IP,
        #[source]
        source: std::io::Error,
    },
    #[error("{domain}: error creating a UDP socket from a standard socket")]
    UdpSocket {
        domain: IP,
        #[source]
        source: std::io::Error,
    },
    #[error("Cannot bind to IPv4 or IPv6")]
    CannotBind,
}

pub fn socket_v4(interface_addr: Option<Ipv4Addr>) -> Result<UdpSocket, SocketError> {
    // Make sure we bind to a specific interface if specified
    let bind_addr = match interface_addr {
        Some(addr) => SocketAddrV4::new(addr, MDNS_PORT).into(),
        None => SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, MDNS_PORT).into(),
    };

    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP)).map_err(|source| {
        SocketError::NewSocket {
            domain: IP::Ipv4,
            source,
        }
    })?;
    socket
        .set_reuse_address(true)
        .map_err(|source| SocketError::ReuseAddress {
            domain: IP::Ipv4,
            source,
        })?;
    // Best-effort: some platforms (e.g. esp-idf/lwIP) declare SO_REUSEPORT but
    // leave it unimplemented (setsockopt returns ENOPROTOOPT). It is not needed
    // for a single process binding the mDNS port, and a genuine "port in use"
    // still surfaces at bind(), so tolerate a failure here instead of aborting.
    #[cfg(unix)]
    if let Err(e) = socket.set_reuse_port(true) {
        tracing::debug!("set_reuse_port (IPv4) failed, continuing: {e}");
    }
    socket
        .bind(&bind_addr)
        .map_err(|source| SocketError::Bind {
            domain: IP::Ipv4,
            source,
        })?;
    socket
        .set_multicast_loop_v4(true)
        .map_err(|source| SocketError::SetMulticastLoop {
            domain: IP::Ipv4,
            source,
        })?;

    // Join the multicast group once, on the interface of the default route.
    //
    // Group membership is per interface, so packets arriving on any other
    // interface are not delivered to this socket. IP_MULTICAST_ALL would
    // paper over this on Linux when another membership exists, but that
    // socket option is Linux-only; on macOS and other BSD-derived systems
    // reception from non-default interfaces does not work. The per-interface
    // sockets cannot take over reception either: they are bound to a unicast
    // address and therefore never match multicast-destined packets. Reception
    // on further interfaces is instead set up by
    // `Sockets::join_group_on_main_v4`, which joins the group per interface on
    // this main wildcard socket.
    socket
        .join_multicast_v4(&MDNS_IPV4, &interface_addr.unwrap_or(Ipv4Addr::UNSPECIFIED))
        .map_err(|source| SocketError::JoinMulticast {
            domain: IP::Ipv4,
            source,
        })?;

    // Pin multicast egress to the requested interface. Binding to the
    // interface address alone does not reliably select the outgoing
    // interface for multicast: the kernel falls back to the routing table,
    // which can pick a different interface or fail with "no route to host"
    // (for example for interfaces without a multicast route entry).
    if let Some(addr) = interface_addr {
        socket
            .set_multicast_if_v4(&addr)
            .map_err(|source| SocketError::SetMulticastIf {
                domain: IP::Ipv4,
                source,
            })?;
    }

    socket
        .set_multicast_ttl_v4(16)
        .map_err(|source| SocketError::MulticastTtl {
            domain: IP::Ipv4,
            source,
        })?;
    socket
        .set_nonblocking(true)
        .map_err(|source| SocketError::SetNonBlocking {
            domain: IP::Ipv4,
            source,
        })?;
    UdpSocket::from_std(std::net::UdpSocket::from(socket)).map_err(|source| {
        SocketError::UdpSocket {
            domain: IP::Ipv4,
            source,
        }
    })
}

pub fn socket_v6() -> Result<UdpSocket, SocketError> {
    let socket = Socket::new(Domain::IPV6, Type::DGRAM, Some(Protocol::UDP)).map_err(|source| {
        SocketError::NewSocket {
            domain: IP::Ipv6,
            source,
        }
    })?;
    socket
        .set_reuse_address(true)
        .map_err(|source| SocketError::ReuseAddress {
            domain: IP::Ipv6,
            source,
        })?;
    // See the IPv4 path: set_reuse_port is best-effort (unimplemented on some
    // platforms, e.g. esp-idf/lwIP), so tolerate a failure here.
    #[cfg(unix)]
    if let Err(e) = socket.set_reuse_port(true) {
        tracing::debug!("set_reuse_port (IPv6) failed, continuing: {e}");
    }
    socket
        .bind(&SocketAddr::from((Ipv6Addr::UNSPECIFIED, MDNS_PORT)).into())
        .map_err(|source| SocketError::Bind {
            domain: IP::Ipv6,
            source,
        })?;
    socket
        .set_multicast_loop_v6(true)
        .map_err(|source| SocketError::SetMulticastLoop {
            domain: IP::Ipv6,
            source,
        })?;

    // Join multicast on the default interface (interface index 0)
    socket
        .join_multicast_v6(&MDNS_IPV6, 0)
        .map_err(|source| SocketError::JoinMulticast {
            domain: IP::Ipv6,
            source,
        })?;

    socket
        .set_nonblocking(true)
        .map_err(|source| SocketError::SetNonBlocking {
            domain: IP::Ipv6,
            source,
        })?;
    UdpSocket::from_std(std::net::UdpSocket::from(socket)).map_err(|source| {
        SocketError::UdpSocket {
            domain: IP::Ipv6,
            source,
        }
    })
}

#[derive(Clone, Debug)]
pub struct Sockets {
    v4: Option<Arc<UdpSocket>>,
    v6: Option<Arc<UdpSocket>>,
    interface_sockets_v4: Arc<RwLock<HashMap<Ipv4Addr, Arc<UdpSocket>>>>,
}

impl Sockets {
    pub fn new(class: IpClass, multicast_interfaces: Vec<Ipv4Addr>) -> Result<Self, SocketError> {
        // Create interface-specific sockets for multi-interface mode
        let mut interface_sockets_v4 = HashMap::new();
        for addr in &multicast_interfaces {
            match socket_v4(Some(*addr)) {
                Ok(socket) => {
                    tracing::debug!("Created interface-specific socket for {}", addr);
                    interface_sockets_v4.insert(*addr, Arc::new(socket));
                }
                Err(e) => {
                    tracing::warn!("Failed to create interface socket for {}: {}", addr, e);
                }
            }
        }
        let interface_sockets_v4 = Arc::new(RwLock::new(interface_sockets_v4));

        let sockets = match class {
            IpClass::Auto => {
                let socket = Self {
                    v4: socket_v4(None).ok().map(Arc::new),
                    v6: socket_v6().ok().map(Arc::new),
                    interface_sockets_v4: interface_sockets_v4.clone(),
                };
                if socket.v4.is_none() && socket.v6.is_none() {
                    return Err(SocketError::CannotBind);
                }
                socket
            }
            _ => Self {
                v4: class
                    .has_v4()
                    .then(|| socket_v4(None).map(Arc::new))
                    .transpose()?,
                v6: class
                    .has_v6()
                    .then(|| socket_v6().map(Arc::new))
                    .transpose()?,
                interface_sockets_v4: interface_sockets_v4.clone(),
            },
        };
        for addr in sockets.get_all_interface_addresses_v4() {
            sockets.join_group_on_main_v4(addr);
        }
        Ok(sockets)
    }

    pub fn v4(&self) -> Option<Arc<UdpSocket>> {
        self.v4.as_ref().map(Arc::clone)
    }

    pub fn v6(&self) -> Option<Arc<UdpSocket>> {
        self.v6.as_ref().map(Arc::clone)
    }

    /// Joins the multicast group on the given interface on the main wildcard
    /// IPv4 socket.
    ///
    /// Group membership is per interface: the wildcard socket initially only
    /// joins the group on the interface of the default route, so multicast
    /// arriving on other interfaces is not delivered to it (except on
    /// platforms with `IP_MULTICAST_ALL` semantics and an existing
    /// system-wide membership). The per-interface sockets cannot take over
    /// reception either, because they are bound to a unicast address and
    /// therefore never match multicast-destined packets. Joining the group
    /// per interface on the wildcard socket makes reception work on all
    /// interfaces.
    fn join_group_on_main_v4(&self, addr: Ipv4Addr) {
        if let Some(v4) = &self.v4 {
            match v4.join_multicast_v4(MDNS_IPV4, addr) {
                Ok(()) => tracing::debug!("joined multicast group on interface {}", addr),
                // Already being a member on this interface (e.g. the default
                // interface joined at socket creation) is not a problem.
                Err(e) => tracing::debug!("could not join multicast group on {}: {}", addr, e),
            }
        }
    }

    /// Leaves the multicast group on the given interface on the main wildcard
    /// IPv4 socket. See [`Self::join_group_on_main_v4`].
    fn leave_group_on_main_v4(&self, addr: Ipv4Addr) {
        if let Some(v4) = &self.v4 {
            if let Err(e) = v4.leave_multicast_v4(MDNS_IPV4, addr) {
                tracing::debug!("could not leave multicast group on {}: {}", addr, e);
            }
        }
    }

    /// Add a new IPv4 interface for multicast operations.
    /// Returns Ok(()) if the socket was successfully created and added.
    pub fn add_interface_v4(&self, addr: Ipv4Addr) -> Result<(), SocketError> {
        // Check if interface already exists
        if self
            .interface_sockets_v4
            .read()
            .unwrap()
            .contains_key(&addr)
        {
            return Ok(());
        }

        // Create the interface-specific socket for sending
        let socket = socket_v4(Some(addr))?;

        {
            let mut interfaces = self.interface_sockets_v4.write().unwrap();
            // need to recheck since we dropped the lock in between
            interfaces.entry(addr).or_insert_with(|| {
                tracing::info!("Added interface {} for multicast", addr);
                Arc::new(socket)
            });
        }
        self.join_group_on_main_v4(addr);
        Ok(())
    }

    /// Remove an IPv4 interface from multicast operations.
    /// Returns true if the interface was found and removed.
    pub fn remove_interface_v4(&self, addr: Ipv4Addr) -> bool {
        let mut interfaces = self.interface_sockets_v4.write().unwrap();

        if interfaces.contains_key(&addr) {
            let socket = interfaces.remove(&addr);
            drop(interfaces);
            // drop socket outside the lock
            drop(socket);
            self.leave_group_on_main_v4(addr);
            tracing::info!("Removed interface {} from multicast", addr);

            true
        } else {
            false
        }
    }

    /// Get the socket for a specific IPv4 interface
    pub fn get_interface_socket_v4(&self, addr: Ipv4Addr) -> Option<Arc<UdpSocket>> {
        let interfaces = self.interface_sockets_v4.read().unwrap();
        match interfaces.get(&addr) {
            Some(sock) => Some(Arc::clone(sock)),
            None => None,
        }
    }

    /// Get all interface addresses that have sockets
    pub fn get_all_interface_addresses_v4(&self) -> Vec<Ipv4Addr> {
        let interfaces = self.interface_sockets_v4.read().unwrap();
        interfaces.keys().copied().collect()
    }

    pub async fn send_msg(&self, msg: &Message, mode: Mode) {
        let bytes = match msg.to_vec() {
            Ok(b) => b,
            Err(e) => {
                tracing::warn!("error serializing mDNS: {}", e);
                return;
            }
        };

        // Use multi-interface mode only for IPv4 when interface sockets are available
        let use_multi_interface = !self.interface_sockets_v4.read().unwrap().is_empty()
            && matches!(mode, Mode::V4 | Mode::Any);

        if use_multi_interface {
            tracing::debug!(
                "Using multi-interface mode for IPv4 sending, {} interfaces available",
                self.interface_sockets_v4.read().unwrap().len()
            );
            self.send_msg_multi_interface_v4(&bytes, msg).await;

            // If mode is Any, also send on IPv6 if available
            if matches!(mode, Mode::Any) {
                if let Some(v6) = &self.v6 {
                    if let Err(e) = v6.send_to(&bytes, (MDNS_IPV6, MDNS_PORT)).await {
                        tracing::warn!("error sending mDNS on IPv6: {}", e);
                    } else {
                        tracing::debug!(
                            q = msg.queries.len(),
                            an = msg.answers.len(),
                            ad = msg.additionals.len(),
                            "sent {} bytes on IPv6",
                            bytes.len()
                        );
                    }
                }
            }
        } else {
            // Single interface mode or IPv6-only
            let (socket, addr) = match mode {
                Mode::V4 => (self.v4.as_ref().unwrap(), IpAddr::from(MDNS_IPV4)),
                Mode::V6 => (self.v6.as_ref().unwrap(), IpAddr::from(MDNS_IPV6)),
                Mode::Any => {
                    if let Some(v4) = &self.v4 {
                        (v4, IpAddr::from(MDNS_IPV4))
                    } else {
                        (self.v6.as_ref().unwrap(), IpAddr::from(MDNS_IPV6))
                    }
                }
            };
            if let Err(e) = socket.send_to(&bytes, (addr, MDNS_PORT)).await {
                tracing::warn!("error sending mDNS: {}", e);
            } else {
                tracing::debug!(
                    q = msg.queries.len(),
                    an = msg.answers.len(),
                    ad = msg.additionals.len(),
                    "sent {} bytes",
                    bytes.len()
                );
            }
        }
    }

    async fn send_msg_multi_interface_v4(&self, bytes: &[u8], msg: &Message) {
        let mut sent_count = 0;

        // Send on all IPv4 interface-specific sockets
        let interfaces = self.interface_sockets_v4.read().unwrap().clone();
        for (addr, socket) in interfaces.iter() {
            if let Err(e) = socket.send_to(bytes, (MDNS_IPV4, MDNS_PORT)).await {
                tracing::error!("error sending mDNS on interface {}: {}", addr, e);
            } else {
                tracing::debug!(
                    addr = %addr,
                    q = msg.queries.len(),
                    an = msg.answers.len(),
                    ad = msg.additionals.len(),
                    "sent {} bytes on interface",
                    bytes.len()
                );
                sent_count += 1;
            }
        }

        if sent_count == 0 {
            tracing::error!("failed to send mDNS on any IPv4 interface in multi-interface mode");
        }
    }
}

#[derive(Debug)]
pub enum Mode {
    V4,
    V6,
    Any,
}
