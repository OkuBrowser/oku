//! Definitions and utilities to interact with a PCP server.

use std::{net::Ipv4Addr, num::NonZeroU16, time::Duration};

use n0_error::{e, stack_error};
use netwatch::UdpSocket;
use rand::Rng;
use tracing::{debug, trace};

use crate::{Protocol, defaults::PCP_RECV_TIMEOUT as RECV_TIMEOUT};

mod protocol;

/// Use the recommended port mapping lifetime for PMP, which is 2 hours. See
/// <https://datatracker.ietf.org/doc/html/rfc6886#section-3.3>
const MAPPING_REQUESTED_LIFETIME_SECONDS: u32 = 60 * 60;

/// A mapping successfully registered with a PCP server.
#[derive(Debug)]
pub struct Mapping {
    /// Protocol for this mapping.
    protocol: protocol::MapProtocol,
    /// Local ip used to create this mapping.
    local_ip: Ipv4Addr,
    /// Local port used to create this mapping.
    local_port: NonZeroU16,
    /// Gateway address used to registered this mapping.
    gateway: Ipv4Addr,
    /// External port of the mapping.
    external_port: NonZeroU16,
    /// External address of the mapping.
    external_address: Ipv4Addr,
    /// Allowed time for this mapping as informed by the server.
    lifetime_seconds: u32,
    /// The nonce of the mapping, used for modifications with the PCP server, for example releasing
    /// the mapping.
    nonce: [u8; 12],
}

#[allow(missing_docs)]
#[stack_error(derive, add_meta, from_sources)]
#[non_exhaustive]
pub enum Error {
    #[error("received nonce does not match sent request")]
    NonceMissmatch {},
    #[error("received mapping does not match the requested protocol")]
    ProtocolMissmatch {},
    #[error("received mapping is for a local port that does not match the requested one")]
    PortMissmatch {},
    #[error("received 0 external port for mapping")]
    ZeroExternalPort {},
    #[error("received external address is not ipv4")]
    NotIpv4 {},
    #[error("received an announce response for a map request")]
    InvalidAnnounce {},
    #[error("IO error during PCP")]
    Io {
        #[error(std_err)]
        source: std::io::Error,
    },
    #[error("Protocol error during PCP")]
    Protocol { source: protocol::Error },
}

impl super::mapping::PortMapped for Mapping {
    fn external(&self) -> (Ipv4Addr, NonZeroU16) {
        (self.external_address, self.external_port)
    }

    fn half_lifetime(&self) -> Duration {
        Duration::from_secs((self.lifetime_seconds / 2).into())
    }
}

impl Mapping {
    /// Attempt to registered a new mapping with the PCP server on the provided gateway.
    pub async fn new(
        protocol: Protocol,
        local_ip: Ipv4Addr,
        local_port: NonZeroU16,
        gateway: Ipv4Addr,
        preferred_external_address: Option<(Ipv4Addr, NonZeroU16)>,
    ) -> Result<Self, Error> {
        // create the socket and send the request
        let socket = UdpSocket::bind_full((local_ip, 0))?;
        socket.connect((gateway, protocol::SERVER_PORT).into())?;

        let mut nonce = [0u8; 12];
        rand::rng().fill_bytes(&mut nonce);

        let (requested_address, requested_port) = match preferred_external_address {
            Some((ip, port)) => (Some(ip), Some(port.into())),
            None => (None, None),
        };

        let protocol = match protocol {
            Protocol::Udp => protocol::MapProtocol::Udp,
            Protocol::Tcp => protocol::MapProtocol::Tcp,
        };
        let req = protocol::Request::mapping(
            nonce,
            protocol,
            local_port.into(),
            local_ip,
            requested_port,
            requested_address,
            MAPPING_REQUESTED_LIFETIME_SECONDS,
        );

        socket.send(&req.encode()).await?;

        // wait for the response and decode it
        let mut buffer = vec![0; protocol::Response::MAX_SIZE];
        let read = tokio::time::timeout(RECV_TIMEOUT, socket.recv(&mut buffer))
            .await
            .map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::TimedOut, "read timeout".to_string())
            })??;
        let response = protocol::Response::decode(&buffer[..read])?;

        // verify that the response is correct and matches the request
        let protocol::Response {
            lifetime_seconds,
            epoch_time: _,
            data,
        } = response;

        match data {
            protocol::OpcodeData::MapData(map_data) => {
                let protocol::MapData {
                    nonce: received_nonce,
                    protocol: received_protocol,
                    local_port: received_local_port,
                    external_port,
                    external_address,
                } = map_data;

                if nonce != received_nonce {
                    return Err(e!(Error::NonceMissmatch));
                }

                if received_protocol != protocol {
                    return Err(e!(Error::ProtocolMissmatch));
                }

                let sent_port: u16 = local_port.into();
                if received_local_port != sent_port {
                    return Err(e!(Error::PortMissmatch));
                }
                let external_port = external_port
                    .try_into()
                    .map_err(|_| e!(Error::ZeroExternalPort))?;

                let external_address = external_address
                    .to_ipv4_mapped()
                    .ok_or(e!(Error::NotIpv4))?;

                Ok(Mapping {
                    protocol: received_protocol,
                    external_port,
                    external_address,
                    lifetime_seconds,
                    nonce,
                    local_ip,
                    local_port,
                    gateway,
                })
            }
            protocol::OpcodeData::Announce => Err(e!(Error::InvalidAnnounce)),
        }
    }

    pub async fn release(self) -> Result<(), Error> {
        let Mapping {
            protocol,
            nonce,
            local_ip,
            local_port,
            gateway,
            ..
        } = self;

        // create the socket and send the request
        let socket = UdpSocket::bind_full((local_ip, 0))?;
        socket.connect((gateway, protocol::SERVER_PORT).into())?;

        let local_port = local_port.into();
        let req = protocol::Request::mapping(nonce, protocol, local_port, local_ip, None, None, 0);

        socket.send(&req.encode()).await?;

        // mapping deletion is a notification, no point in waiting for the response
        Ok(())
    }
}

/// Probes the local gateway for PCP support.
pub async fn probe_available(local_ip: Ipv4Addr, gateway: Ipv4Addr) -> bool {
    match probe_available_fallible(local_ip, gateway).await {
        Ok(response) => {
            trace!("probe response: {response:?}");
            let protocol::Response {
                lifetime_seconds: _,
                epoch_time: _,
                data,
            } = response;
            match data {
                protocol::OpcodeData::Announce => true,
                _ => {
                    debug!("server returned an unexpected response type for probe");
                    // missbehaving server is not useful
                    false
                }
            }
        }
        Err(e) => {
            debug!("probe failed: {e}");
            false
        }
    }
}

async fn probe_available_fallible(
    local_ip: Ipv4Addr,
    gateway: Ipv4Addr,
) -> Result<protocol::Response, Error> {
    // create the socket and send the request
    let socket = UdpSocket::bind_full((local_ip, 0))?;
    socket.connect((gateway, protocol::SERVER_PORT).into())?;
    let req = protocol::Request::announce(local_ip.to_ipv6_mapped());
    socket.send(&req.encode()).await?;

    // wait for the response and decode it
    let mut buffer = vec![0; protocol::Response::MAX_SIZE];
    let read = tokio::time::timeout(RECV_TIMEOUT, socket.recv(&mut buffer))
        .await
        .map_err(|_| {
            e!(
                Error::Io,
                std::io::Error::new(std::io::ErrorKind::TimedOut, "read timeout".to_string())
            )
        })??;
    let response = protocol::Response::decode(&buffer[..read])?;

    Ok(response)
}
