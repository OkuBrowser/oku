//! Tickets for blobs.
use std::{collections::BTreeSet, net::SocketAddr, str::FromStr};

use iroh::{EndpointAddr, EndpointId, RelayUrl};
use iroh_tickets::{ParseError, Ticket};
use n0_error::Result;
use serde::{Deserialize, Serialize};

use crate::{BlobFormat, Hash, HashAndFormat};

/// A token containing everything to get a file from the provider.
///
/// It is a single item which can be easily serialized and deserialized.
#[derive(Debug, Clone, PartialEq, Eq, derive_more::Display)]
#[display("{}", Ticket::encode_string(self))]
pub struct BlobTicket {
    /// The provider to get a file from.
    addr: EndpointAddr,
    /// The format of the blob.
    format: BlobFormat,
    /// The hash to retrieve.
    hash: Hash,
}

impl From<BlobTicket> for HashAndFormat {
    fn from(val: BlobTicket) -> Self {
        HashAndFormat {
            hash: val.hash,
            format: val.format,
        }
    }
}

/// Wire format for [`BlobTicket`].
///
/// In the future we might have multiple variants (not versions, since they
/// might be both equally valid), so this is a single variant enum to force
/// postcard to add a discriminator.
#[derive(Serialize, Deserialize)]
enum TicketWireFormat {
    Variant0(Variant0BlobTicket),
}

// Legacy
#[derive(Serialize, Deserialize)]
struct Variant0BlobTicket {
    node: Variant0NodeAddr,
    format: BlobFormat,
    hash: Hash,
}

#[derive(Serialize, Deserialize)]
struct Variant0NodeAddr {
    endpoint_id: EndpointId,
    info: Variant0AddrInfo,
}

#[derive(Serialize, Deserialize)]
struct Variant0AddrInfo {
    relay_url: Option<RelayUrl>,
    direct_addresses: BTreeSet<SocketAddr>,
}

impl Ticket for BlobTicket {
    const KIND: &'static str = "blob";

    fn encode_bytes(&self) -> Vec<u8> {
        let data = TicketWireFormat::Variant0(Variant0BlobTicket {
            node: Variant0NodeAddr {
                endpoint_id: self.addr.id,
                info: Variant0AddrInfo {
                    relay_url: self.addr.relay_urls().next().cloned(),
                    direct_addresses: self.addr.ip_addrs().cloned().collect(),
                },
            },
            format: self.format,
            hash: self.hash,
        });
        postcard::to_stdvec(&data).expect("postcard serialization failed")
    }

    fn decode_bytes(bytes: &[u8]) -> std::result::Result<Self, ParseError> {
        let res: TicketWireFormat = postcard::from_bytes(bytes)?;
        let TicketWireFormat::Variant0(Variant0BlobTicket { node, format, hash }) = res;
        let mut addr = EndpointAddr::new(node.endpoint_id);
        if let Some(relay_url) = node.info.relay_url {
            addr = addr.with_relay_url(relay_url);
        }
        for ip_addr in node.info.direct_addresses {
            addr = addr.with_ip_addr(ip_addr);
        }
        Ok(Self { addr, format, hash })
    }
}

impl FromStr for BlobTicket {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ticket::decode_string(s)
    }
}

impl BlobTicket {
    /// Creates a new ticket.
    pub fn new(addr: EndpointAddr, hash: Hash, format: BlobFormat) -> Self {
        Self { hash, format, addr }
    }

    /// The hash of the item this ticket can retrieve.
    pub fn hash(&self) -> Hash {
        self.hash
    }

    /// The [`EndpointAddr`] of the provider for this ticket.
    pub fn addr(&self) -> &EndpointAddr {
        &self.addr
    }

    /// The [`BlobFormat`] for this ticket.
    pub fn format(&self) -> BlobFormat {
        self.format
    }

    pub fn hash_and_format(&self) -> HashAndFormat {
        HashAndFormat {
            hash: self.hash,
            format: self.format,
        }
    }

    /// True if the ticket is for a collection and should retrieve all blobs in it.
    pub fn recursive(&self) -> bool {
        self.format.is_hash_seq()
    }

    /// Get the contents of the ticket, consuming it.
    pub fn into_parts(self) -> (EndpointAddr, Hash, BlobFormat) {
        let BlobTicket { addr, hash, format } = self;
        (addr, hash, format)
    }
}

impl Serialize for BlobTicket {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            serializer.serialize_str(&self.to_string())
        } else {
            let BlobTicket {
                addr: node,
                format,
                hash,
            } = self;
            (node, format, hash).serialize(serializer)
        }
    }
}

impl<'de> Deserialize<'de> for BlobTicket {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        if deserializer.is_human_readable() {
            let s = String::deserialize(deserializer)?;
            Self::from_str(&s).map_err(serde::de::Error::custom)
        } else {
            let (peer, format, hash) = Deserialize::deserialize(deserializer)?;
            Ok(Self::new(peer, hash, format))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use iroh::{PublicKey, SecretKey, TransportAddr};
    use iroh_test::{assert_eq_hex, hexdump::parse_hexdump};

    use super::*;

    fn make_ticket() -> BlobTicket {
        let hash = Hash::new(b"hi there");
        let peer = SecretKey::generate().public();
        let addr = SocketAddr::from_str("127.0.0.1:1234").unwrap();
        BlobTicket {
            hash,
            addr: EndpointAddr::from_parts(peer, [TransportAddr::Ip(addr)]),
            format: BlobFormat::HashSeq,
        }
    }

    #[test]
    fn test_ticket_postcard() {
        let ticket = make_ticket();
        let bytes = postcard::to_stdvec(&ticket).unwrap();
        let ticket2: BlobTicket = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(ticket2, ticket);
    }

    #[test]
    fn test_ticket_json() {
        let ticket = make_ticket();
        let json = serde_json::to_string(&ticket).unwrap();
        let ticket2: BlobTicket = serde_json::from_str(&json).unwrap();
        assert_eq!(ticket2, ticket);
    }

    #[test]
    fn test_ticket_base32() {
        let hash =
            Hash::from_str("0b84d358e4c8be6c38626b2182ff575818ba6bd3f4b90464994be14cb354a072")
                .unwrap();
        let endpoint_id =
            PublicKey::from_str("ae58ff8833241ac82d6ff7611046ed67b5072d142c588d0063e942d9a75502b6")
                .unwrap();

        let ticket = BlobTicket {
            addr: EndpointAddr::new(endpoint_id),
            format: BlobFormat::Raw,
            hash,
        };
        let encoded = ticket.to_string();
        let stripped = encoded.strip_prefix("blob").unwrap();
        let base32 = data_encoding::BASE32_NOPAD
            .decode(stripped.to_ascii_uppercase().as_bytes())
            .unwrap();
        let expected = parse_hexdump("
            00 # discriminator for variant 0
            ae58ff8833241ac82d6ff7611046ed67b5072d142c588d0063e942d9a75502b6 # endpoint id, 32 bytes, see above
            00 # relay url
            00 # number of addresses (0)
            00 # format (raw)
            0b84d358e4c8be6c38626b2182ff575818ba6bd3f4b90464994be14cb354a072 # hash, 32 bytes, see above
        ").unwrap();
        assert_eq_hex!(base32, expected);
    }
}
