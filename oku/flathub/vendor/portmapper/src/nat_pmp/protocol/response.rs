//! A NAT-PMP response encoding and decoding.

use std::net::Ipv4Addr;

use n0_error::{e, stack_error};
use num_enum::{IntoPrimitive, TryFromPrimitive};

use super::{MapProtocol, Opcode, Version};

/// A NAT-PMP successful Response/Notification.
#[derive(Debug, PartialEq, Eq)]
pub enum Response {
    /// Response to a [`Opcode::DetermineExternalAddress`] request.
    PublicAddress {
        epoch_time: u32,
        public_ip: Ipv4Addr,
    },
    /// Response to a [`Opcode::MapUdp`] request.
    PortMap {
        /// Protocol for which the mapping was requested.
        proto: MapProtocol,
        /// Epoch time of the server.
        epoch_time: u32,
        /// Local port for which the mapping was created.
        private_port: u16,
        /// External port registered for this mapping.
        external_port: u16,
        /// Lifetime in seconds that can be assumed by this mapping.
        lifetime_seconds: u32,
    },
}

/// Result code obtained in a NAT-PMP response.
///
/// See [RFC 6886 Result Codes](https://datatracker.ietf.org/doc/html/rfc6886#section-3.5)
#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive, IntoPrimitive)]
#[repr(u16)]
pub enum ResultCode {
    /// A successful response.
    Success = 0,
    /// The sent version is not supported by the NAT-PMP server.
    UnsupportedVersion = 1,
    /// Functionality is supported but not allowerd: e.g. box supports mapping, but user has turned
    /// feature off.
    NotAuthorizedOrRefused = 2,
    /// Netfork failures, e.g. NAT device itself has not obtained a DHCP lease.
    NetworkFailure = 3,
    /// NAT-PMP server cannot create any more mappings at this time.
    OutOfResources = 4,
    /// Opcode is not supported by the server.
    UnsupportedOpcode = 5,
}

/// Errors that can occur when decoding a [`Response`] from a server.
#[allow(missing_docs)]
#[stack_error(derive, add_meta)]
#[non_exhaustive]
pub enum Error {
    /// Request is too short or is otherwise malformed.
    #[error("Response is malformed")]
    Malformed {},
    /// The [`Response::RESPONSE_INDICATOR`] is not present.
    #[error("Packet does not appear to be a response")]
    NotAResponse {},
    /// The received opcode is not recognized.
    #[error("Invalid Opcode received")]
    InvalidOpcode {},
    /// The received version is not recognized.
    #[error("Invalid version received")]
    InvalidVersion {},
    /// The received result code is not recognized.
    #[error("Invalid result code received")]
    InvalidResultCode {},
    /// Received an error code indicating the server does not support the sent version.
    #[error("Server does not support the version")]
    UnsupportedVersion {},
    /// Received an error code indicating the operation is supported but not authorized.
    #[error("Operation is supported but not authorized")]
    NotAuthorizedOrRefused {},
    /// Received an error code indicating the server experienced a network failure
    #[error("Server experienced a network failure")]
    NetworkFailure {},
    /// Received an error code indicating the server cannot create more mappings at this time.
    #[error("Server is out of resources")]
    OutOfResources {},
    /// Received an error code indicating the Opcode is not supported by the server.
    #[error("Server does not support this opcode")]
    UnsupportedOpcode {},
}

impl Response {
    /// Minimum size of an encoded [`Response`] sent by a server to this client.
    pub const MIN_SIZE: usize = // parts of a public ip response
        1 + // version
        1 + // opcode
        2 + // result code
        4 + // epoch time
        4; // lifetime

    /// Minimum size of an encoded [`Response`] sent by a server to this client.
    pub const MAX_SIZE: usize = // parts of mapping response
        1 + // version
        1 + // opcode
        2 + // result code
        4 + // epoch time
        2 + // private port
        2 + // public port
        4; // lifetime

    /// Indicator ORd into the [`Opcode`] to indicate a response packet.
    pub const RESPONSE_INDICATOR: u8 = 1u8 << 7;

    /// Decode a map response.
    fn decode_map(buf: &[u8], proto: MapProtocol) -> Result<Self, Error> {
        if buf.len() != Self::MAX_SIZE {
            return Err(e!(Error::Malformed));
        }

        let epoch_bytes = buf[4..8].try_into().expect("slice has the right len");
        let epoch_time = u32::from_be_bytes(epoch_bytes);

        let private_port_bytes = buf[8..10].try_into().expect("slice has the right len");
        let private_port = u16::from_be_bytes(private_port_bytes);

        let external_port_bytes = buf[10..12].try_into().expect("slice has the right len");
        let external_port = u16::from_be_bytes(external_port_bytes);

        let lifetime_bytes = buf[12..16].try_into().expect("slice has the right len");
        let lifetime_seconds = u32::from_be_bytes(lifetime_bytes);

        Ok(Response::PortMap {
            proto,
            epoch_time,
            private_port,
            external_port,
            lifetime_seconds,
        })
    }

    /// Decode a response.
    pub fn decode(buf: &[u8]) -> Result<Self, Error> {
        if buf.len() < Self::MIN_SIZE || buf.len() > Self::MAX_SIZE {
            return Err(e!(Error::Malformed));
        }
        let _: Version = buf[0].try_into().map_err(|_| e!(Error::InvalidVersion))?;
        let opcode = buf[1];
        if opcode & Self::RESPONSE_INDICATOR != Self::RESPONSE_INDICATOR {
            return Err(e!(Error::NotAResponse));
        }
        let opcode: Opcode = (opcode & !Self::RESPONSE_INDICATOR)
            .try_into()
            .map_err(|_| e!(Error::InvalidOpcode))?;

        let result_bytes =
            u16::from_be_bytes(buf[2..4].try_into().expect("slice has the right len"));
        let result_code = result_bytes
            .try_into()
            .map_err(|_| e!(Error::InvalidResultCode))?;

        match result_code {
            ResultCode::Success => Ok(()),
            ResultCode::UnsupportedVersion => Err(e!(Error::UnsupportedVersion)),
            ResultCode::NotAuthorizedOrRefused => Err(e!(Error::NotAuthorizedOrRefused)),
            ResultCode::NetworkFailure => Err(e!(Error::NetworkFailure)),
            ResultCode::OutOfResources => Err(e!(Error::OutOfResources)),
            ResultCode::UnsupportedOpcode => Err(e!(Error::UnsupportedOpcode)),
        }?;

        let response = match opcode {
            Opcode::DetermineExternalAddress => {
                let epoch_bytes = buf[4..8].try_into().expect("slice has the right len");
                let epoch_time = u32::from_be_bytes(epoch_bytes);
                let ip_bytes: [u8; 4] = buf[8..12].try_into().expect("slice has the right len");
                Response::PublicAddress {
                    epoch_time,
                    public_ip: ip_bytes.into(),
                }
            }
            Opcode::MapUdp => Self::decode_map(buf, MapProtocol::Udp)?,
            Opcode::MapTcp => Self::decode_map(buf, MapProtocol::Tcp)?,
        };

        Ok(response)
    }

    #[cfg(test)]
    fn random<R: rand::Rng>(opcode: Opcode, rng: &mut R) -> Self {
        use rand::RngExt;

        match opcode {
            Opcode::DetermineExternalAddress => {
                let octets: [u8; 4] = rng.random();
                Response::PublicAddress {
                    epoch_time: rng.random(),
                    public_ip: octets.into(),
                }
            }
            Opcode::MapUdp => Response::PortMap {
                proto: MapProtocol::Udp,
                epoch_time: rng.random(),
                private_port: rng.random(),
                external_port: rng.random(),
                lifetime_seconds: rng.random(),
            },
            Opcode::MapTcp => Response::PortMap {
                proto: MapProtocol::Tcp,
                epoch_time: rng.random(),
                private_port: rng.random(),
                external_port: rng.random(),
                lifetime_seconds: rng.random(),
            },
        }
    }

    #[cfg(test)]
    fn encode(&self) -> Vec<u8> {
        match self {
            Response::PublicAddress {
                epoch_time,
                public_ip,
            } => {
                let mut buf = Vec::with_capacity(Self::MIN_SIZE);
                // version
                buf.push(Version::NatPmp.into());
                // response indicator and opcode
                let opcode: u8 = Opcode::DetermineExternalAddress.into();
                buf.push(Response::RESPONSE_INDICATOR | opcode);
                // result code
                let result_code: u16 = ResultCode::Success.into();
                for b in result_code.to_be_bytes() {
                    buf.push(b);
                }
                // epoch
                for b in epoch_time.to_be_bytes() {
                    buf.push(b);
                }
                // public ip
                for b in public_ip.octets() {
                    buf.push(b)
                }
                buf
            }
            Response::PortMap {
                proto: _,
                epoch_time,
                private_port,
                external_port,
                lifetime_seconds,
            } => {
                let mut buf = Vec::with_capacity(Self::MAX_SIZE);
                // version
                buf.push(Version::NatPmp.into());
                // response indicator and opcode
                let opcode: u8 = Opcode::MapUdp.into();
                buf.push(Response::RESPONSE_INDICATOR | opcode);
                // result code
                let result_code: u16 = ResultCode::Success.into();
                for b in result_code.to_be_bytes() {
                    buf.push(b);
                }
                // epoch
                for b in epoch_time.to_be_bytes() {
                    buf.push(b);
                }
                // internal port
                for b in private_port.to_be_bytes() {
                    buf.push(b)
                }
                // external port
                for b in external_port.to_be_bytes() {
                    buf.push(b)
                }
                for b in lifetime_seconds.to_be_bytes() {
                    buf.push(b)
                }
                buf
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rand::SeedableRng;

    use super::*;

    #[test]
    fn test_decode_external_addr_response() {
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(42);

        let response = Response::random(Opcode::DetermineExternalAddress, &mut rng);
        let encoded = response.encode();
        assert_eq!(response, Response::decode(&encoded).unwrap());
    }

    #[test]
    fn test_encode_decode_map_response() {
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(42);

        let response = Response::random(Opcode::MapUdp, &mut rng);
        let encoded = response.encode();
        assert_eq!(response, Response::decode(&encoded).unwrap());
    }
}
