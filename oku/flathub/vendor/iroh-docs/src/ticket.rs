//! Tickets for `iroh-docs` documents.

use iroh::EndpointAddr;
use iroh_tickets::{ParseError, Ticket};
use serde::{Deserialize, Serialize};

use crate::Capability;

/// Contains both a key (either secret or public) to a document, and a list of peers to join.
#[derive(Serialize, Deserialize, Clone, Debug, derive_more::Display)]
#[display("{}", Ticket::encode_string(self))]
pub struct DocTicket {
    /// either a public or private key
    pub capability: Capability,
    /// A list of nodes to contact.
    pub nodes: Vec<EndpointAddr>,
}

/// Wire format for [`DocTicket`].
///
/// In the future we might have multiple variants (not versions, since they
/// might be both equally valid), so this is a single variant enum to force
/// postcard to add a discriminator.
#[derive(Serialize, Deserialize)]
enum TicketWireFormat {
    Variant0(DocTicket),
}

impl Ticket for DocTicket {
    const KIND: &'static str = "doc";

    fn encode_bytes(&self) -> Vec<u8> {
        let data = TicketWireFormat::Variant0(self.clone());
        postcard::to_stdvec(&data).expect("postcard serialization failed")
    }

    fn decode_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
        let res: TicketWireFormat = postcard::from_bytes(bytes)?;
        let TicketWireFormat::Variant0(res) = res;
        if res.nodes.is_empty() {
            return Err(ParseError::verification_failed(
                "addressing info cannot be empty",
            ));
        }
        Ok(res)
    }
}

impl DocTicket {
    /// Create a new doc ticket
    pub fn new(capability: Capability, peers: Vec<EndpointAddr>) -> Self {
        Self {
            capability,
            nodes: peers,
        }
    }
}

impl std::str::FromStr for DocTicket {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ticket::decode_string(s)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use anyhow::{ensure, Context, Result};
    use iroh::PublicKey;

    use super::*;
    use crate::NamespaceId;

    #[test]
    fn test_ticket_base32() {
        let node_id =
            PublicKey::from_str("ae58ff8833241ac82d6ff7611046ed67b5072d142c588d0063e942d9a75502b6")
                .unwrap();
        let namespace_id = NamespaceId::from(
            &<[u8; 32]>::try_from(
                hex::decode("ae58ff8833241ac82d6ff7611046ed67b5072d142c588d0063e942d9a75502b6")
                    .unwrap(),
            )
            .unwrap(),
        );

        let ticket = DocTicket {
            capability: Capability::Read(namespace_id),
            nodes: vec![EndpointAddr::new(node_id)],
        };
        let s = ticket.to_string();
        let base32 = data_encoding::BASE32_NOPAD
            .decode(
                s.strip_prefix("doc")
                    .unwrap()
                    .to_ascii_uppercase()
                    .as_bytes(),
            )
            .unwrap();
        let expected = parse_hexdump("
            00 # variant
            01 # capability discriminator, 1 = read
            ae58ff8833241ac82d6ff7611046ed67b5072d142c588d0063e942d9a75502b6 # namespace id, 32 bytes, see above
            01 # one node
            ae58ff8833241ac82d6ff7611046ed67b5072d142c588d0063e942d9a75502b6 # node id, 32 bytes, see above
            00 # no addrs
        ").unwrap();
        assert_eq!(base32, expected);
    }

    /// Parses a commented multi line hexdump into a vector of bytes.
    ///
    /// This is useful to write wire level protocol tests.
    pub fn parse_hexdump(s: &str) -> Result<Vec<u8>> {
        let mut result = Vec::new();

        for (line_number, line) in s.lines().enumerate() {
            let data_part = line.split('#').next().unwrap_or("");
            let cleaned: String = data_part.chars().filter(|c| !c.is_whitespace()).collect();

            ensure!(
                cleaned.len().is_multiple_of(2),
                "Non-even number of hex chars detected on line {}.",
                line_number + 1
            );

            for i in (0..cleaned.len()).step_by(2) {
                let byte_str = &cleaned[i..i + 2];
                let byte = u8::from_str_radix(byte_str, 16)
                    .with_context(|| format!("Invalid hex data on line {}.", line_number + 1))?;

                result.push(byte);
            }
        }

        Ok(result)
    }
}
