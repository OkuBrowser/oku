//! DNS packet construction and response parsing using `simple_dns`.

use std::net::{Ipv4Addr, Ipv6Addr};

use n0_error::{e, stack_error};
use simple_dns::{
    CLASS, Name, Packet, PacketFlag, QCLASS, QTYPE, Question, RCODE, TYPE, header_buffer,
    rdata::{A, AAAA, RData, SVCB},
};

use crate::{
    CaaRecordData, HttpsRecordData, MxRecordData, Record, RecordKind, ResponseCode, SrvRecordData,
    SvcbRecordData, TxtRecordData,
};

/// Errors that can occur while building a query packet or parsing a response.
///
/// Mapped to [`crate::Error`] by the resolver layer.
#[allow(missing_docs)]
#[stack_error(derive, add_meta, std_sources)]
pub(super) enum QueryError {
    #[error("invalid domain name: {name}")]
    BuildQuery { name: String },
    #[error("malformed DNS response")]
    Malformed {},
    #[error("response did not match query")]
    Unexpected {},
    #[error("domain name does not exist (NXDOMAIN)")]
    NxDomain {},
    #[error("server returned error: {rcode:?}")]
    ServerFailure { rcode: RCODE },
}

/// Maps a `simple_dns` [`RCODE`] onto the public [`ResponseCode`].
pub(super) fn response_code(rcode: RCODE) -> ResponseCode {
    match rcode {
        RCODE::ServerFailure => ResponseCode::ServerFailure,
        RCODE::Refused => ResponseCode::Refused,
        RCODE::FormatError => ResponseCode::FormatError,
        RCODE::NotImplemented => ResponseCode::NotImplemented,
        _ => ResponseCode::Other,
    }
}

/// Length of a fixed DNS message header (RFC 1035 Section 4.1.1).
///
/// A buffer shorter than this cannot be a DNS message. The header-flag helpers
/// index into the first four bytes without bounds-checking, so callers on raw
/// network input guard against shorter buffers first.
const DNS_HEADER_LEN: usize = 12;

/// Smallest wire size of a question: a root name, a type, and a class.
const MIN_QUESTION_LEN: usize = 1 + 2 + 2;

/// Smallest wire size of a resource record.
///
/// A root name, a type, a class, a TTL, and a zero rdata length.
const MIN_RECORD_LEN: usize = 1 + 2 + 2 + 4 + 2;

/// Parses a message from the network.
///
/// `simple_dns` reserves each section's vector from the header counts before
/// it reads a record, so a datagram of a dozen bytes claiming 65535 records
/// in every section has it allocate tens of megabytes and then fail on the
/// first record. The counts are checked against the smallest wire form of a
/// question and a record first, so a message can only make the parser reserve
/// what its own length could hold.
pub(super) fn parse_packet(data: &[u8]) -> Result<Packet<'_>, QueryError> {
    if data.len() < DNS_HEADER_LEN {
        return Err(e!(QueryError::Malformed));
    }
    let questions = usize::from(header_buffer::questions_unchecked(data));
    let records = usize::from(header_buffer::answers_unchecked(data))
        + usize::from(header_buffer::name_servers_unchecked(data))
        + usize::from(header_buffer::additional_records_unchecked(data));
    let min_len = DNS_HEADER_LEN + questions * MIN_QUESTION_LEN + records * MIN_RECORD_LEN;
    if data.len() < min_len {
        return Err(e!(QueryError::Malformed));
    }
    Packet::parse(data).map_err(|_| e!(QueryError::Malformed))
}

/// EDNS(0) advertised UDP payload size.
///
/// 1232 bytes is the current recommended safe value per RFC 6891 and the
/// DNS flag day 2020 recommendations. This avoids IP fragmentation on
/// common path MTUs while allowing responses much larger than the
/// original 512-byte RFC 1035 limit.
const EDNS_UDP_PAYLOAD_SIZE: u16 = 1232;

/// Builds a DNS query packet for the given host and query type.
///
/// The query includes an EDNS(0) OPT record advertising support for
/// responses up to [`EDNS_UDP_PAYLOAD_SIZE`] bytes over UDP.
///
/// Returns `(query_id, wire_bytes)`.
pub(super) fn build_query(host: &str, qtype: TYPE) -> Result<(u16, Vec<u8>), QueryError> {
    let id: u16 = rand::random();
    let mut packet = Packet::new_query(id);
    packet.set_flags(PacketFlag::RECURSION_DESIRED);

    let name = Name::new(host).map_err(|_| {
        e!(QueryError::BuildQuery {
            name: host.to_string()
        })
    })?;
    let question = Question::new(name, QTYPE::TYPE(qtype), QCLASS::CLASS(CLASS::IN), false);
    packet.questions.push(question);

    // Add EDNS(0) OPT record to advertise larger UDP payload support.
    *packet.opt_mut() = Some(simple_dns::rdata::OPT {
        udp_packet_size: EDNS_UDP_PAYLOAD_SIZE,
        version: 0,
        opt_codes: vec![],
    });

    let bytes = packet.build_bytes_vec().map_err(|_| {
        e!(QueryError::BuildQuery {
            name: host.to_string()
        })
    })?;
    Ok((id, bytes))
}

/// Returns the RCODE if `data` is a failure worth trying another nameserver for.
///
/// Those are SERVFAIL, REFUSED, and FORMERR. Any other response yields `None`.
///
/// SERVFAIL and REFUSED mean the server cannot answer for the name. A FORMERR
/// means it rejected the query itself, usually the EDNS(0) OPT record; the
/// caller retries that server without EDNS (see [`strip_edns`]) before reaching
/// here, so a surviving FORMERR likewise means another nameserver should be
/// tried.
///
/// Reads only the header RCODE, so it works on the raw response before the
/// packet is parsed or validated. A spoofed RCODE is harmless: at worst it makes
/// the race try another server, and the eventual response is still checked by
/// [`check_response`].
pub(super) fn retryable_failure_rcode(data: &[u8]) -> Option<RCODE> {
    // `header_buffer::rcode` indexes `data[2..4]` without a bounds check, so a
    // response shorter than a DNS header would panic. This runs on raw,
    // unvalidated bytes before `check_response`, so guard the length here.
    if data.len() < DNS_HEADER_LEN {
        return None;
    }
    match header_buffer::rcode(data) {
        Ok(rcode @ (RCODE::ServerFailure | RCODE::Refused | RCODE::FormatError)) => Some(rcode),
        _ => None,
    }
}

/// Returns whether `data` is a response with a FORMERR (format error) RCODE.
///
/// A FORMERR from a server or middlebox often means it could not parse our
/// query, commonly because of the EDNS(0) OPT record; the caller retries the
/// same server without EDNS (see [`strip_edns`]) before moving on. Reads only
/// the header RCODE, so it runs on raw bytes before full validation.
pub(super) fn is_format_error(data: &[u8]) -> bool {
    data.len() >= DNS_HEADER_LEN && matches!(header_buffer::rcode(data), Ok(RCODE::FormatError))
}

/// Returns whether a datagram answers `query`, for the UDP receive loop.
///
/// A datagram that does not is one the socket should keep listening past.
/// Each query goes out from a fresh source port, so an off-path attacker who
/// guesses that port can deliver whatever they like to it, and returning the
/// first thing that arrives would let a spoofed datagram end the query with a
/// mismatch before the real answer lands (hickory's GHSA-v44v-c8m4-gc43).
///
/// The transaction id must match and the QR bit must be set. A datagram that
/// then parses must echo our question, name, type, and class, so one that
/// carries no question or someone else's is ignored rather than taken as the
/// answer to ours. One that does not parse is accepted once it carries our id:
/// a truncated or malformed reply from the server itself then fails the lookup
/// at once rather than at the timeout, and a truncated one still brings in the
/// TCP query.
///
/// The one response accepted without a question is a FORMERR. A middlebox that
/// could not parse the query, usually over its EDNS(0) OPT record, may answer
/// with a bare header, and all a FORMERR does is have the caller retry the
/// same server without EDNS. A spoofed one costs a round trip and nothing
/// else.
pub(super) fn answers_query(response: &[u8], query: &[u8]) -> bool {
    let Ok(id) = header_buffer::id(response) else {
        return false;
    };
    if header_buffer::id(query).ok() != Some(id)
        || !header_buffer::has_flags(response, PacketFlag::RESPONSE).unwrap_or(false)
    {
        return false;
    }
    let Ok(packet) = parse_packet(response) else {
        return true;
    };
    let Ok(sent) = Packet::parse(query) else {
        return true;
    };
    if packet.questions.is_empty() && packet.rcode() == RCODE::FormatError {
        return true;
    }
    packet.questions.len() == sent.questions.len()
        && packet
            .questions
            .iter()
            .zip(&sent.questions)
            .all(|(got, want)| {
                names_equal(&got.qname, &want.qname)
                    && got.qtype == want.qtype
                    && got.qclass == want.qclass
            })
}

/// Rebuilds `query` without its EDNS(0) OPT record, or `None` if it has none.
///
/// RFC 6891 Section 6.2.2 calls for retrying without EDNS when a server rejects
/// the OPT record. The transaction ID, flags, and question are preserved so the
/// response to the stripped query still passes [`check_response`].
pub(super) fn strip_edns(query: &[u8]) -> Option<Vec<u8>> {
    let mut packet = Packet::parse(query).ok()?;
    packet.opt()?;
    *packet.opt_mut() = None;
    packet.build_bytes_vec().ok()
}

/// Derives the RFC 2308 negative-caching TTL from a response's authority SOA.
///
/// On a NODATA or NXDOMAIN answer an authoritative server includes an SOA record
/// in the authority section; the negative result should be cached for no longer
/// than `min(SOA MINIMUM, SOA record TTL)`. Returns `None` when no SOA is
/// present, so the caller can fall back to a fixed default.
pub(super) fn negative_ttl(data: &[u8]) -> Option<u32> {
    let packet = parse_packet(data).ok()?;
    packet.name_servers.iter().find_map(|rr| match &rr.rdata {
        RData::SOA(soa) => Some(rr.ttl.min(soa.minimum)),
        _ => None,
    })
}

/// Maximum CNAME chain depth to prevent infinite loops.
pub(super) const MAX_CNAME_DEPTH: usize = 8;

/// Returns whether `a` and `b` are the same DNS name.
///
/// Labels are compared without regard to ASCII case, as RFC 4343 requires.
/// `simple_dns` derives `PartialEq` for [`Name`] over the raw label bytes, so
/// `Example.COM` and `example.com` compare unequal there, while a server may
/// well return an owner name in a case other than the one we asked in.
pub(super) fn names_equal(a: &Name<'_>, b: &Name<'_>) -> bool {
    a.get_labels().len() == b.get_labels().len()
        && a.as_bytes()
            .zip(b.as_bytes())
            .all(|(a, b)| a.eq_ignore_ascii_case(b))
}

/// Returns every name in the CNAME chain starting at `start_name`.
///
/// The start name comes first, then each CNAME target, ending at the final
/// canonical name.
///
/// Records of the queried type can be attached to any name along the chain, not
/// just the final one, so extraction matches against the whole chain the way a
/// recursive resolver does. Bounded by [`MAX_CNAME_DEPTH`]. Only records of
/// class IN take part; see [`parse_response`].
fn cname_chain<'a>(packet: &'a Packet<'a>, start_name: &Name<'a>) -> Vec<Name<'a>> {
    let mut chain = vec![start_name.clone()];
    let mut current = start_name.clone();
    for _ in 0..MAX_CNAME_DEPTH {
        let Some(RData::CNAME(cname)) = packet
            .answers
            .iter()
            .find(|rr| rr.class == CLASS::IN && names_equal(&rr.name, &current))
            .map(|rr| &rr.rdata)
        else {
            break;
        };
        current = cname.0.clone();
        chain.push(current.clone());
    }
    chain
}

/// Returns the name to query next when a response leaves its CNAME chain unresolved.
///
/// That is the case when the answer section carries a CNAME chain from `qname`
/// but no record of `qtype` on any name along it. A record of the type on a
/// name off the chain is not an answer to this question, so it does not make
/// the chain resolved: following the chain is still the way to the answer.
///
/// The caller issues a new query for the returned target.
pub(super) fn unresolved_cname_target(
    packet: &Packet<'_>,
    qname: &Name<'_>,
    qtype: TYPE,
) -> Option<String> {
    let chain = cname_chain(packet, qname);
    let answered = packet.answers.iter().any(|rr| {
        rr.class == CLASS::IN
            && rr.rdata.type_code() == qtype
            && chain.iter().any(|name| names_equal(name, &rr.name))
    });
    if answered {
        return None;
    }
    // The chain always starts with `qname`.
    let last = chain.last()?;
    (!names_equal(last, qname)).then(|| last.to_string())
}

/// Validates a response against the query that produced it, then surfaces its RCODE.
///
/// Per RFC 5452 a response is only trustworthy if it echoes the query: the QR
/// flag must be set, the transaction ID must match, and the response must carry
/// exactly the one question we asked, by name, type, and class. Without the
/// question check an off-path attacker who guesses the 16-bit ID could supply
/// its own question and have its answers accepted for an arbitrary name. A
/// `NameError` RCODE maps to [`QueryError::NxDomain`] and any other non-`NoError`
/// RCODE to [`QueryError::ServerFailure`].
pub(super) fn check_response(
    packet: &Packet,
    expected_id: u16,
    expected_name: &Name<'_>,
    expected_type: TYPE,
) -> Result<(), QueryError> {
    if !packet.has_flags(PacketFlag::RESPONSE) {
        return Err(e!(QueryError::Unexpected));
    }
    if packet.id() != expected_id {
        return Err(e!(QueryError::Unexpected));
    }
    // The response must echo exactly the question we sent (RFC 1035 4.1.2).
    match packet.questions.as_slice() {
        [question]
            if names_equal(&question.qname, expected_name)
                && question.qtype == QTYPE::TYPE(expected_type)
                && question.qclass == QCLASS::CLASS(CLASS::IN) => {}
        _ => return Err(e!(QueryError::Unexpected)),
    }
    match packet.rcode() {
        RCODE::NoError => Ok(()),
        RCODE::NameError => Err(e!(QueryError::NxDomain)),
        rcode => Err(e!(QueryError::ServerFailure { rcode })),
    }
}

/// Extracts matching records from a validated DNS response, following CNAME chains.
///
/// The caller must already have validated the response with [`check_response`];
/// this walks the CNAME chain anchored on the question and extracts records of
/// the requested type using `extract` at every name along the chain. Returns the
/// records and the minimum TTL across them.
///
/// Every query is for class IN, and a record of another class is a different
/// record even under the same name and type, so records of any other class are
/// left out rather than returned as answers to an IN question.
fn parse_response<T>(
    data: &[u8],
    extract: impl Fn(&Name<'_>, &RData<'_>) -> Option<T>,
) -> Result<(Vec<T>, u32), QueryError> {
    let packet = parse_packet(data)?;

    // The chain is empty only for a response with no question section, which
    // `check_response` already rejects; extraction then matches nothing.
    let chain = packet
        .questions
        .first()
        .map(|question| cname_chain(&packet, &question.qname))
        .unwrap_or_default();

    let mut results = Vec::new();
    let mut min_ttl = u32::MAX;
    for rr in &packet.answers {
        if rr.class != CLASS::IN || !chain.iter().any(|name| names_equal(name, &rr.name)) {
            continue;
        }
        // A CNAME along the chain bounds the cached lifetime too: a short-TTL
        // CNAME must not be served past its intended life because the final
        // records happen to carry a longer TTL. Fold its TTL even though it is
        // not itself an extracted record. This bounds the CNAMEs present in this
        // response; a chain followed across separate queries only passes its
        // final response here, so those intermediate CNAME TTLs are not folded.
        if matches!(rr.rdata, RData::CNAME(_)) {
            min_ttl = min_ttl.min(rr.ttl);
        } else if let Some(val) = extract(&rr.name, &rr.rdata) {
            results.push(val);
            min_ttl = min_ttl.min(rr.ttl);
        }
    }
    if min_ttl == u32::MAX {
        min_ttl = 0;
    }
    Ok((results, min_ttl))
}

/// Parses the records of `kind` from a DNS response, following CNAME chains.
///
/// One dispatch replaces a parse function per kind: it picks the `extract`
/// closure for `kind` and runs [`parse_response`], returning the matching
/// [`Record`]s and the minimum TTL across them.
pub(super) fn parse_records(
    data: &[u8],
    kind: RecordKind,
) -> Result<(Vec<Record>, u32), QueryError> {
    match kind {
        RecordKind::A => parse_response(data, |_, rdata| match rdata {
            RData::A(A { address }) => Some(Record::A(Ipv4Addr::from(*address))),
            _ => None,
        }),
        RecordKind::Aaaa => parse_response(data, |_, rdata| match rdata {
            RData::AAAA(AAAA { address }) => Some(Record::Aaaa(Ipv6Addr::from(*address))),
            _ => None,
        }),
        RecordKind::Txt => parse_response(data, |_, rdata| match rdata {
            RData::TXT(txt) => Some(Record::Txt(extract_txt_record_data(txt))),
            _ => None,
        }),
        RecordKind::Ns => parse_response(data, |_, rdata| match rdata {
            RData::NS(ns) => Some(Record::Ns(ns.0.to_string())),
            _ => None,
        }),
        RecordKind::Srv => parse_response(data, |_, rdata| match rdata {
            RData::SRV(srv) => Some(Record::Srv(SrvRecordData {
                priority: srv.priority,
                weight: srv.weight,
                port: srv.port,
                target: srv.target.to_string(),
            })),
            _ => None,
        }),
        RecordKind::Mx => parse_response(data, |_, rdata| match rdata {
            RData::MX(mx) => Some(Record::Mx(MxRecordData {
                preference: mx.preference,
                exchange: mx.exchange.to_string(),
            })),
            _ => None,
        }),
        RecordKind::Caa => parse_response(data, |_, rdata| match rdata {
            RData::CAA(caa) => Some(Record::Caa(CaaRecordData {
                flag: caa.flag,
                tag: caa.tag.to_string(),
                value: caa.value.as_ref().into(),
            })),
            _ => None,
        }),
        RecordKind::Svcb => parse_response(data, |_, rdata| match rdata {
            RData::SVCB(svcb) => Some(Record::Svcb(extract_svcb(svcb))),
            _ => None,
        }),
        RecordKind::Https => parse_response(data, |name, rdata| match rdata {
            RData::HTTPS(https) => Some(Record::Https(HttpsRecordData::new(
                name.to_string(),
                extract_svcb(&https.0),
            ))),
            _ => None,
        }),
    }
}

/// Wraps an SVCB or HTTPS record for return.
///
/// HTTPS records reuse the SVCB rdata, so both kinds share this. The parsed
/// record is cloned into an owned form; [`SvcbRecordData`] exposes its common
/// parameters through accessors.
fn extract_svcb(svcb: &SVCB<'_>) -> SvcbRecordData {
    SvcbRecordData::new(svcb.clone().into_owned())
}

/// Extracts the raw content of a TXT record into `TxtRecordData`.
///
/// Converts the TXT record's character strings into raw bytes. In iroh's
/// DNS encoding, each TXT record typically contains a single character
/// string (one `key=value` attribute), and each attribute is published as
/// a separate TXT ResourceRecord.
///
/// We use `String::try_from` which concatenates all character strings in
/// the record into one byte sequence. This preserves the raw content
/// without the destructive key=value parsing that `TXT::attributes()` does
/// (which uses a HashMap and would lose ordering and deduplicate keys).
fn extract_txt_record_data(txt: &simple_dns::rdata::TXT<'_>) -> TxtRecordData {
    // Preserve each character-string as its own raw bytes, matching the
    // `TxtRecordData` "list of character strings" contract. The previous
    // `String::try_from` concatenated all strings into one and required UTF-8,
    // so a multi-string record collapsed and any binary TXT (some SPF/DKIM)
    // silently became empty. `iter_raw` yields each string split on its first
    // `=`; reconstruct the original bytes so the round trip is exact.
    txt.iter_raw()
        .map(|(key, value)| match value {
            Some(value) => {
                let mut bytes = Vec::with_capacity(key.len() + 1 + value.len());
                bytes.extend_from_slice(key);
                bytes.push(b'=');
                bytes.extend_from_slice(value);
                bytes.into_boxed_slice()
            }
            None => Box::<[u8]>::from(key),
        })
        .collect()
}

/// Returns true if the response has the TC (truncation) flag set.
///
/// Reads only the header flags, so it works even on a truncated packet whose
/// body fails to parse. A buffer too short to hold a header is treated as not
/// truncated; it will fail to parse downstream.
pub(super) fn is_truncated(data: &[u8]) -> bool {
    // As in `server_failure_rcode`, guard against a short buffer: this runs on
    // the raw UDP datagram before any validation, and `header_buffer::has_flags`
    // would panic indexing `data[2..4]` on fewer than four bytes.
    if data.len() < DNS_HEADER_LEN {
        return false;
    }
    header_buffer::has_flags(data, PacketFlag::TRUNCATION).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, Ipv6Addr};

    use simple_dns::{
        CLASS, CharacterString, Name, Packet, PacketFlag, QCLASS, QTYPE, Question, ResourceRecord,
        rdata::{A, CAA, CNAME, MX, RData, SRV, TXT},
    };

    use super::*;

    /// Build a query packet for `host` with type A, return (id, bytes).
    fn make_query(host: &str) -> (u16, Vec<u8>) {
        build_query(host, TYPE::A).unwrap()
    }

    /// Parses `data` down to its A record addresses and TTL.
    ///
    /// Lets the A-focused tests compare against a plain `Vec<Ipv4Addr>`.
    fn parse_a_addrs(data: &[u8]) -> (Vec<Ipv4Addr>, u32) {
        let (records, ttl) = parse_records(data, RecordKind::A).unwrap();
        let addrs = records
            .into_iter()
            .filter_map(|r| match r {
                Record::A(ip) => Some(ip),
                _ => None,
            })
            .collect();
        (addrs, ttl)
    }

    /// Build a response with A records for `name`.
    fn a_response(id: u16, name: &str, addrs: &[Ipv4Addr]) -> Vec<u8> {
        let mut packet = Packet::new_reply(id);
        packet.set_flags(PacketFlag::RECURSION_DESIRED | PacketFlag::RECURSION_AVAILABLE);
        // Echo the question back (needed for CNAME resolution in parse functions).
        packet.questions.push(Question::new(
            Name::new_unchecked(name),
            QTYPE::TYPE(TYPE::A),
            QCLASS::CLASS(CLASS::IN),
            false,
        ));
        for addr in addrs {
            packet.answers.push(ResourceRecord::new(
                Name::new_unchecked(name),
                CLASS::IN,
                300,
                RData::A(A {
                    address: u32::from(*addr),
                }),
            ));
        }
        packet.build_bytes_vec().unwrap()
    }

    /// Builds a response with a CNAME from `alias` to `canonical`.
    ///
    /// The A records sit under the canonical name, which is the common case of
    /// a nameserver answering the alias and the target in one response.
    fn cname_with_a_response(id: u16, alias: &str, canonical: &str, addrs: &[Ipv4Addr]) -> Vec<u8> {
        let mut packet = Packet::new_reply(id);
        packet.set_flags(PacketFlag::RECURSION_DESIRED | PacketFlag::RECURSION_AVAILABLE);
        packet.questions.push(Question::new(
            Name::new_unchecked(alias),
            QTYPE::TYPE(TYPE::A),
            QCLASS::CLASS(CLASS::IN),
            false,
        ));
        // CNAME record
        packet.answers.push(ResourceRecord::new(
            Name::new_unchecked(alias),
            CLASS::IN,
            300,
            RData::CNAME(CNAME(Name::new_unchecked(canonical))),
        ));
        // A records for the canonical name
        for addr in addrs {
            packet.answers.push(ResourceRecord::new(
                Name::new_unchecked(canonical),
                CLASS::IN,
                300,
                RData::A(A {
                    address: u32::from(*addr),
                }),
            ));
        }
        packet.build_bytes_vec().unwrap()
    }

    /// Build a response containing only a CNAME (no final A record).
    fn cname_only_response(id: u16, alias: &str, canonical: &str) -> Vec<u8> {
        let mut packet = Packet::new_reply(id);
        packet.set_flags(PacketFlag::RECURSION_DESIRED | PacketFlag::RECURSION_AVAILABLE);
        packet.questions.push(Question::new(
            Name::new_unchecked(alias),
            QTYPE::TYPE(TYPE::A),
            QCLASS::CLASS(CLASS::IN),
            false,
        ));
        packet.answers.push(ResourceRecord::new(
            Name::new_unchecked(alias),
            CLASS::IN,
            300,
            RData::CNAME(CNAME(Name::new_unchecked(canonical))),
        ));
        packet.build_bytes_vec().unwrap()
    }

    #[test]
    fn build_query_includes_edns_opt() {
        let (_, bytes) = build_query("example.com", TYPE::A).unwrap();
        let packet = Packet::parse(&bytes).unwrap();
        let opt = packet.opt().expect("query should include OPT record");
        assert_eq!(opt.udp_packet_size, 1232);
        assert_eq!(opt.version, 0);
    }

    #[test]
    fn strip_edns_removes_opt_and_preserves_question() {
        let (id, bytes) = build_query("example.com", TYPE::A).unwrap();
        assert!(Packet::parse(&bytes).unwrap().opt().is_some());

        let stripped = strip_edns(&bytes).expect("query has EDNS to strip");
        let packet = Packet::parse(&stripped).unwrap();
        assert!(packet.opt().is_none(), "OPT should be removed");
        assert_eq!(packet.id(), id, "transaction ID preserved");
        let question = &packet.questions[0];
        assert_eq!(question.qname, Name::new_unchecked("example.com"));
        assert_eq!(question.qtype, QTYPE::TYPE(TYPE::A));

        // A query already without EDNS yields None.
        assert!(strip_edns(&stripped).is_none());
    }

    #[test]
    fn formerr_is_detected_and_retryable() {
        let (id, _) = make_query("example.com");
        let mut resp = a_response(id, "example.com", &[Ipv4Addr::new(1, 2, 3, 4)]);
        // A NoError response is neither a FORMERR nor otherwise retryable.
        assert!(!is_format_error(&resp));
        assert_eq!(retryable_failure_rcode(&resp), None);

        // The RCODE is the low nibble of header byte 3. FORMERR = 1.
        resp[3] = (resp[3] & 0xF0) | 0x01;
        assert!(is_format_error(&resp));
        assert_eq!(retryable_failure_rcode(&resp), Some(RCODE::FormatError));

        // SERVFAIL (2) and REFUSED (5) are retryable but not FORMERR.
        resp[3] = (resp[3] & 0xF0) | 0x02;
        assert!(!is_format_error(&resp));
        assert_eq!(retryable_failure_rcode(&resp), Some(RCODE::ServerFailure));
    }

    #[test]
    fn negative_ttl_is_min_of_soa_ttl_and_minimum() {
        let (id, _) = make_query("nx.example.com");
        let mut packet = Packet::new_reply(id);
        packet.set_flags(PacketFlag::RESPONSE);
        packet.questions.push(Question::new(
            Name::new_unchecked("nx.example.com"),
            QTYPE::TYPE(TYPE::A),
            QCLASS::CLASS(CLASS::IN),
            false,
        ));
        packet.name_servers.push(ResourceRecord::new(
            Name::new_unchecked("example.com"),
            CLASS::IN,
            900,
            RData::SOA(simple_dns::rdata::SOA {
                mname: Name::new_unchecked("ns.example.com"),
                rname: Name::new_unchecked("hostmaster.example.com"),
                serial: 1,
                refresh: 3600,
                retry: 600,
                expire: 604800,
                minimum: 300,
            }),
        ));
        let bytes = packet.build_bytes_vec().unwrap();
        // min(record TTL 900, SOA minimum 300) = 300.
        assert_eq!(negative_ttl(&bytes), Some(300));

        // A response with no SOA in the authority section yields None.
        let plain = a_response(id, "example.com", &[Ipv4Addr::new(1, 2, 3, 4)]);
        assert_eq!(negative_ttl(&plain), None);
    }

    #[test]
    fn parse_a_no_cname() {
        let (id, _) = make_query("example.com");
        let resp = a_response(id, "example.com", &[Ipv4Addr::new(1, 2, 3, 4)]);
        let (addrs, ttl) = parse_a_addrs(&resp);
        assert_eq!(addrs, [Ipv4Addr::new(1, 2, 3, 4)]);
        assert_eq!(ttl, 300);
    }

    #[test]
    fn parse_a_with_cname_in_response() {
        let (id, _) = make_query("alias.example.com");
        let resp = cname_with_a_response(
            id,
            "alias.example.com",
            "real.example.com",
            &[Ipv4Addr::new(10, 0, 0, 1), Ipv4Addr::new(10, 0, 0, 2)],
        );
        let (addrs, _) = parse_a_addrs(&resp);
        assert_eq!(
            addrs,
            [Ipv4Addr::new(10, 0, 0, 1), Ipv4Addr::new(10, 0, 0, 2)]
        );
    }

    #[test]
    fn parse_a_with_chained_cname() {
        // alias -> middle -> real, A records on real
        let (id, _) = make_query("alias.example.com");
        let mut packet = Packet::new_reply(id);
        packet.set_flags(PacketFlag::RECURSION_DESIRED | PacketFlag::RECURSION_AVAILABLE);
        packet.questions.push(Question::new(
            Name::new_unchecked("alias.example.com"),
            QTYPE::TYPE(TYPE::A),
            QCLASS::CLASS(CLASS::IN),
            false,
        ));
        packet.answers.push(ResourceRecord::new(
            Name::new_unchecked("alias.example.com"),
            CLASS::IN,
            300,
            RData::CNAME(CNAME(Name::new_unchecked("middle.example.com"))),
        ));
        packet.answers.push(ResourceRecord::new(
            Name::new_unchecked("middle.example.com"),
            CLASS::IN,
            300,
            RData::CNAME(CNAME(Name::new_unchecked("real.example.com"))),
        ));
        packet.answers.push(ResourceRecord::new(
            Name::new_unchecked("real.example.com"),
            CLASS::IN,
            300,
            RData::A(A {
                address: u32::from(Ipv4Addr::new(5, 6, 7, 8)),
            }),
        ));
        let resp = packet.build_bytes_vec().unwrap();
        let (addrs, _) = parse_a_addrs(&resp);
        assert_eq!(addrs, [Ipv4Addr::new(5, 6, 7, 8)]);
    }

    /// Returns the target to query next for `qname`, an A question.
    fn cname_target(packet: &Packet<'_>, qname: &str) -> Option<String> {
        unresolved_cname_target(packet, &Name::new_unchecked(qname), TYPE::A)
    }

    #[test]
    fn cname_target_extracts_target_for_recursive_follow() {
        let id = 1234;
        let resp = cname_only_response(id, "alias.example.com", "real.example.com");
        let packet = Packet::parse(&resp).unwrap();
        let target = cname_target(&packet, "alias.example.com");
        assert_eq!(target.as_deref(), Some("real.example.com"));
    }

    /// The chain is resolved by a record of the type on a chain name only.
    ///
    /// One on an unrelated name is not an answer to the question, so the
    /// chain is still followed; one on the target settles it.
    #[test]
    fn cname_target_ignores_records_off_the_chain() {
        let id = 1234;
        let mut packet = reply_with_question(id, "alias.example.com", TYPE::A);
        packet.answers.push(ResourceRecord::new(
            Name::new_unchecked("alias.example.com"),
            CLASS::IN,
            300,
            RData::CNAME(CNAME(Name::new_unchecked("real.example.com"))),
        ));
        packet.answers.push(ResourceRecord::new(
            Name::new_unchecked("unrelated.example.com"),
            CLASS::IN,
            300,
            RData::A(A {
                address: u32::from(Ipv4Addr::new(6, 6, 6, 6)),
            }),
        ));
        let resp = packet.build_bytes_vec().unwrap();
        let parsed = Packet::parse(&resp).unwrap();
        assert_eq!(
            cname_target(&parsed, "alias.example.com").as_deref(),
            Some("real.example.com")
        );

        packet.answers.push(ResourceRecord::new(
            Name::new_unchecked("real.example.com"),
            CLASS::IN,
            300,
            RData::A(A {
                address: u32::from(Ipv4Addr::new(7, 7, 7, 7)),
            }),
        ));
        let resp = packet.build_bytes_vec().unwrap();
        let parsed = Packet::parse(&resp).unwrap();
        assert_eq!(cname_target(&parsed, "alias.example.com"), None);
    }

    /// Builds a reply echoing one question for `name`/`qtype`, plus an A answer.
    ///
    /// Gives `check_response` something to validate.
    fn reply_with_question(id: u16, name: &str, qtype: TYPE) -> Packet<'static> {
        let mut packet = Packet::new_reply(id);
        packet.set_flags(PacketFlag::RECURSION_DESIRED | PacketFlag::RECURSION_AVAILABLE);
        packet.questions.push(Question::new(
            Name::new_unchecked(name).into_owned(),
            QTYPE::TYPE(qtype),
            QCLASS::CLASS(CLASS::IN),
            false,
        ));
        packet
    }

    #[test]
    fn check_response_accepts_a_matching_reply() {
        let name = Name::new_unchecked("example.com");
        let packet = reply_with_question(42, "example.com", TYPE::A);
        assert!(check_response(&packet, 42, &name, TYPE::A).is_ok());
    }

    #[test]
    fn check_response_rejects_mismatches() {
        let name = Name::new_unchecked("example.com");

        // Wrong transaction id.
        let packet = reply_with_question(42, "example.com", TYPE::A);
        assert!(matches!(
            check_response(&packet, 7, &name, TYPE::A),
            Err(QueryError::Unexpected { .. })
        ));

        // Question name does not echo the query.
        let packet = reply_with_question(42, "attacker.example", TYPE::A);
        assert!(matches!(
            check_response(&packet, 42, &name, TYPE::A),
            Err(QueryError::Unexpected { .. })
        ));

        // Question type does not match.
        let packet = reply_with_question(42, "example.com", TYPE::AAAA);
        assert!(matches!(
            check_response(&packet, 42, &name, TYPE::A),
            Err(QueryError::Unexpected { .. })
        ));

        // No question section at all.
        let mut packet = reply_with_question(42, "example.com", TYPE::A);
        packet.questions.clear();
        assert!(matches!(
            check_response(&packet, 42, &name, TYPE::A),
            Err(QueryError::Unexpected { .. })
        ));
    }

    /// Names are matched without regard to case throughout.
    ///
    /// The echoed question, the owner names along the CNAME chain, and the
    /// CNAME target may all come back in a case other than the one queried.
    #[test]
    fn names_match_case_insensitively() {
        let name = Name::new_unchecked("example.com");
        let packet = reply_with_question(42, "EXAMPLE.com", TYPE::A);
        assert!(check_response(&packet, 42, &name, TYPE::A).is_ok());

        // The owner names in the answer differ in case from the question.
        let (id, _) = make_query("alias.example.com");
        let resp = cname_with_a_response(
            id,
            "Alias.Example.COM",
            "REAL.example.com",
            &[Ipv4Addr::new(10, 0, 0, 3)],
        );
        let (addrs, _) = parse_a_addrs(&resp);
        assert_eq!(addrs, [Ipv4Addr::new(10, 0, 0, 3)]);

        // A CNAME whose target is the query name in another case leaves the
        // chain where it started rather than looping back on itself.
        let resp = cname_only_response(id, "alias.example.com", "ALIAS.example.com");
        let packet = Packet::parse(&resp).unwrap();
        assert_eq!(cname_target(&packet, "alias.example.com"), None);
    }

    #[test]
    fn cname_target_returns_none_when_no_cname() {
        let id = 1234;
        let resp = a_response(id, "example.com", &[Ipv4Addr::new(1, 2, 3, 4)]);
        let packet = Packet::parse(&resp).unwrap();
        let target = cname_target(&packet, "example.com");
        assert_eq!(target, None);
    }

    /// Builds a reply for `name` carrying a single `rdata` answer.
    ///
    /// Lets a parse test drive [`parse_records`] against one record of a kind.
    fn reply_with_answer(name: &str, qtype: TYPE, rdata: RData<'_>) -> Vec<u8> {
        let mut packet = Packet::new_reply(1);
        packet.set_flags(PacketFlag::RECURSION_DESIRED | PacketFlag::RECURSION_AVAILABLE);
        packet.questions.push(Question::new(
            Name::new_unchecked(name),
            QTYPE::TYPE(qtype),
            QCLASS::CLASS(CLASS::IN),
            false,
        ));
        packet.answers.push(ResourceRecord::new(
            Name::new_unchecked(name),
            CLASS::IN,
            300,
            rdata,
        ));
        packet.build_bytes_vec().unwrap()
    }

    #[test]
    fn parse_ns_record() {
        let resp = reply_with_answer(
            "example.com",
            TYPE::NS,
            RData::NS(Name::new_unchecked("ns1.example.com").into()),
        );
        let (records, ttl) = parse_records(&resp, RecordKind::Ns).unwrap();
        assert_eq!(ttl, 300);
        let [Record::Ns(name)] = records.as_slice() else {
            panic!("expected one NS record, got {records:?}");
        };
        assert_eq!(name, "ns1.example.com");
    }

    #[test]
    fn parse_srv_record() {
        let resp = reply_with_answer(
            "_sip._tcp.example.com",
            TYPE::SRV,
            RData::SRV(SRV {
                priority: 10,
                weight: 20,
                port: 5060,
                target: Name::new_unchecked("sip.example.com"),
            }),
        );
        let (records, _) = parse_records(&resp, RecordKind::Srv).unwrap();
        let [Record::Srv(srv)] = records.as_slice() else {
            panic!("expected one SRV record, got {records:?}");
        };
        assert_eq!(srv.priority, 10);
        assert_eq!(srv.weight, 20);
        assert_eq!(srv.port, 5060);
        assert_eq!(srv.target, "sip.example.com");
    }

    #[test]
    fn parse_mx_record() {
        let resp = reply_with_answer(
            "example.com",
            TYPE::MX,
            RData::MX(MX {
                preference: 5,
                exchange: Name::new_unchecked("mail.example.com"),
            }),
        );
        let (records, _) = parse_records(&resp, RecordKind::Mx).unwrap();
        let [Record::Mx(mx)] = records.as_slice() else {
            panic!("expected one MX record, got {records:?}");
        };
        assert_eq!(mx.preference, 5);
        assert_eq!(mx.exchange, "mail.example.com");
    }

    #[test]
    fn parse_caa_record() {
        let resp = reply_with_answer(
            "example.com",
            TYPE::CAA,
            RData::CAA(CAA {
                flag: 0,
                tag: CharacterString::new(b"issue").unwrap(),
                value: b"letsencrypt.org".as_slice().into(),
            }),
        );
        let (records, _) = parse_records(&resp, RecordKind::Caa).unwrap();
        let [Record::Caa(caa)] = records.as_slice() else {
            panic!("expected one CAA record, got {records:?}");
        };
        assert_eq!(caa.flag, 0);
        assert_eq!(caa.tag, "issue");
        assert_eq!(&*caa.value, b"letsencrypt.org");
    }

    /// Builds a ServiceMode SVCB rdata with every parameter we decode set.
    ///
    /// Lets the SVCB and HTTPS parse tests share one endpoint description.
    fn sample_svcb() -> SVCB<'static> {
        let mut svcb = SVCB::new(1, Name::new_unchecked("svc.example.com"));
        svcb.set_alpn(&["h2".try_into().unwrap(), "h3".try_into().unwrap()]);
        svcb.set_port(8443);
        svcb.set_ipv4hint(&[u32::from(Ipv4Addr::new(192, 0, 2, 1))]);
        svcb.set_ipv6hint(&[u128::from(Ipv6Addr::LOCALHOST)]);
        svcb
    }

    fn assert_sample_svcb(svcb: &SvcbRecordData) {
        assert_eq!(svcb.priority(), 1);
        assert_eq!(svcb.target(), "svc.example.com");
        assert_eq!(svcb.alpn().collect::<Vec<_>>(), ["h2", "h3"]);
        assert_eq!(svcb.port(), Some(8443));
        assert_eq!(
            svcb.ipv4hint().collect::<Vec<_>>(),
            [Ipv4Addr::new(192, 0, 2, 1)]
        );
        assert_eq!(svcb.ipv6hint().collect::<Vec<_>>(), [Ipv6Addr::LOCALHOST]);
    }

    #[test]
    fn parse_svcb_record() {
        let resp = reply_with_answer("svc.example.com", TYPE::SVCB, RData::SVCB(sample_svcb()));
        let (records, _) = parse_records(&resp, RecordKind::Svcb).unwrap();
        let [Record::Svcb(svcb)] = records.as_slice() else {
            panic!("expected one SVCB record, got {records:?}");
        };
        assert_sample_svcb(svcb);
    }

    #[test]
    fn parse_https_record() {
        let resp = reply_with_answer(
            "svc.example.com",
            TYPE::HTTPS,
            RData::HTTPS(sample_svcb().into()),
        );
        let (records, _) = parse_records(&resp, RecordKind::Https).unwrap();
        let [Record::Https(https)] = records.as_slice() else {
            panic!("expected one HTTPS record, got {records:?}");
        };
        assert_sample_svcb(https.svcb());
    }

    /// The [`HttpsRecordData`] helpers apply the RFC 9460 rules.
    ///
    /// A raw parameter read would miss all three: AliasMode versus ServiceMode,
    /// the root-target-uses-owner rule (Section 2.5.2), and the default
    /// `http/1.1` ALPN (Sections 7.1.1 and 9.5).
    #[test]
    fn https_record_helpers() {
        // Parse one HTTPS record from raw rdata; the owner is EXAMPLE_WIRE.
        fn https(rdata: &[u8]) -> HttpsRecordData {
            let resp = raw_response(TYPE_HTTPS, EXAMPLE_WIRE, EXAMPLE_WIRE, TYPE_HTTPS, rdata);
            let (records, _) = parse_records(&resp, RecordKind::Https).expect("vector parses");
            match records.as_slice() {
                [Record::Https(https)] => https.clone(),
                other => panic!("expected one HTTPS record, got {other:?}"),
            }
        }

        // ServiceMode (priority 1), root target, alpn=[h2]: the default http/1.1
        // is appended, and the root target resolves to the owner name.
        let svc = https(b"\x00\x01\x00\x00\x01\x00\x03\x02h2");
        assert!(svc.is_service() && !svc.is_alias());
        assert_eq!(svc.alpn().collect::<Vec<_>>(), ["h2"]);
        assert_eq!(svc.effective_alpn(), ["h2", "http/1.1"]);
        assert!(svc.supports_http2());
        assert!(!svc.supports_http3());
        assert_eq!(svc.effective_target(), "example.com");

        // no-default-alpn suppresses the http/1.1 default.
        let no_default = https(b"\x00\x01\x00\x00\x01\x00\x03\x02h2\x00\x02\x00\x00");
        assert!(no_default.no_default_alpn());
        assert_eq!(no_default.effective_alpn(), ["h2"]);

        // AliasMode (priority 0): no endpoint, so no ALPN; the target is used as-is.
        let alias = https(b"\x00\x00\x03svc\x07example\x03com\x00");
        assert!(alias.is_alias());
        assert!(alias.effective_alpn().is_empty());
        assert!(!alias.supports_http2());

        // A ServiceMode record with no `alpn` gets only the scheme default, so
        // supports_http2 must stay false. This pins the equivalence that lets
        // supports_http2 search the raw list rather than the effective set.
        let bare = https(b"\x00\x01\x00");
        assert_eq!(bare.effective_alpn(), ["http/1.1"]);
        assert!(!bare.supports_http2());
        assert!(!bare.supports_http3());
        assert_eq!(alias.effective_target(), "svc.example.com");
    }

    /// Only a datagram carrying our id, the QR bit, and our question answers a query.
    ///
    /// Everything else is what the UDP receive loop keeps listening past.
    #[test]
    fn answers_query_demands_id_qr_bit_and_echoed_question() {
        let (id, query) = make_query("example.com");
        let good = a_response(id, "example.com", &[Ipv4Addr::new(1, 2, 3, 4)]);
        assert!(answers_query(&good, &query));

        // Wrong transaction id.
        let other = a_response(id.wrapping_add(1), "example.com", &[]);
        assert!(!answers_query(&other, &query));

        // Our own query echoed back: right id, but no QR bit.
        assert!(!answers_query(&query, &query));

        // Someone else's question, or none at all (QDCOUNT=0).
        let forged = a_response(id, "attacker.example", &[Ipv4Addr::new(6, 6, 6, 6)]);
        assert!(!answers_query(&forged, &query));
        let mut empty = Packet::new_reply(id);
        assert!(!answers_query(&empty.build_bytes_vec().unwrap(), &query));

        // Except a FORMERR, which a middlebox may send as a bare header.
        *empty.rcode_mut() = RCODE::FormatError;
        assert!(answers_query(&empty.build_bytes_vec().unwrap(), &query));

        // Shorter than a header.
        assert!(!answers_query(&good[..8], &query));

        // Right id and QR bit but unparsable: accepted, so the server's own
        // malformed reply fails the lookup now rather than at the timeout.
        let mut cut = good.clone();
        cut.truncate(good.len() - 2);
        assert!(answers_query(&cut, &query));
    }

    /// A header that claims more records than the message could hold is rejected.
    ///
    /// Before the parser reserves a vector per section from those counts.
    #[test]
    fn parse_packet_rejects_overstated_section_counts() {
        let (id, _) = make_query("example.com");
        let mut resp = a_response(id, "example.com", &[Ipv4Addr::new(1, 2, 3, 4)]);
        assert!(parse_packet(&resp).is_ok());

        // ANCOUNT is header bytes 6 and 7; NSCOUNT 8 and 9; ARCOUNT 10 and 11.
        resp[6..8].copy_from_slice(&u16::MAX.to_be_bytes());
        assert!(matches!(
            parse_packet(&resp),
            Err(QueryError::Malformed { .. })
        ));

        // A bare header claiming one record of every kind is too short as well.
        let mut header = [0u8; DNS_HEADER_LEN];
        header[7] = 1;
        header[9] = 1;
        header[11] = 1;
        assert!(matches!(
            parse_packet(&header),
            Err(QueryError::Malformed { .. })
        ));

        // Shorter than a header is malformed rather than a panic.
        assert!(matches!(
            parse_packet(&header[..4]),
            Err(QueryError::Malformed { .. })
        ));
    }

    /// The header-peek helpers must not panic on a short buffer.
    ///
    /// They run on raw, unvalidated bytes before the response is checked.
    #[test]
    fn header_helpers_do_not_panic_on_short_input() {
        for data in [&[][..], &[0][..], &[0, 0][..], &[0, 0, 0][..]] {
            assert_eq!(retryable_failure_rcode(data), None);
            assert!(!is_format_error(data));
            assert!(!is_truncated(data));
        }
    }

    /// A TXT record keeps each character-string as its own raw bytes.
    ///
    /// That covers binary and `key=value` content, which concatenating them or
    /// dropping non-UTF-8 data would lose.
    #[test]
    fn parse_txt_preserves_character_strings() {
        let (id, _) = make_query("example.com");
        let mut packet = Packet::new_reply(id);
        packet.set_flags(PacketFlag::RECURSION_DESIRED | PacketFlag::RECURSION_AVAILABLE);
        packet.questions.push(Question::new(
            Name::new_unchecked("example.com"),
            QTYPE::TYPE(TYPE::TXT),
            QCLASS::CLASS(CLASS::IN),
            false,
        ));
        let mut txt = TXT::new();
        txt.add_char_string(CharacterString::new(b"first".as_slice()).unwrap());
        // Contains an `=` (exercises the split-and-rejoin) and a non-UTF-8 byte.
        txt.add_char_string(CharacterString::new(b"k=\xff\x00".as_slice()).unwrap());
        packet.answers.push(ResourceRecord::new(
            Name::new_unchecked("example.com"),
            CLASS::IN,
            300,
            RData::TXT(txt),
        ));
        let resp = packet.build_bytes_vec().unwrap();

        let (records, _) = parse_records(&resp, RecordKind::Txt).unwrap();
        let [Record::Txt(data)] = records.as_slice() else {
            panic!("expected one TXT record, got {records:?}");
        };
        let strings: Vec<&[u8]> = data.iter().collect();
        assert_eq!(strings, [b"first".as_slice(), b"k=\xff\x00".as_slice()]);
    }

    /// The `example.com` owner name in wire form.
    ///
    /// Used by the hand-assembled packets below for the question and answer.
    const EXAMPLE_WIRE: &[u8] = b"\x07example\x03com\x00";

    /// SVCB and HTTPS record type codes (RFC 9460 Section 14.1 and 14.2).
    const TYPE_SVCB: u16 = 64;
    const TYPE_HTTPS: u16 = 65;

    /// Assembles a minimal DNS response with one question and one answer.
    ///
    /// Gives a test full control over the answer's raw name bytes, type, and
    /// rdata.
    ///
    /// The builder in `simple_dns` will not emit malformed or non-canonical wire
    /// forms, so tests that need RFC Appendix D rdata vectors, a wrong rdlength,
    /// or a specific compression pointer assemble the bytes here and feed them
    /// straight through [`parse_records`].
    fn raw_response(
        qtype: u16,
        question_name: &[u8],
        answer_name: &[u8],
        rtype: u16,
        rdata: &[u8],
    ) -> Vec<u8> {
        let mut buf = Vec::new();
        // Header: id, flags (QR set), then qd=1, an=1, ns=0, ar=0.
        buf.extend_from_slice(&0x1234u16.to_be_bytes());
        buf.extend_from_slice(&0x8000u16.to_be_bytes());
        buf.extend_from_slice(&1u16.to_be_bytes());
        buf.extend_from_slice(&1u16.to_be_bytes());
        buf.extend_from_slice(&0u16.to_be_bytes());
        buf.extend_from_slice(&0u16.to_be_bytes());
        // Question section.
        buf.extend_from_slice(question_name);
        buf.extend_from_slice(&qtype.to_be_bytes());
        buf.extend_from_slice(&1u16.to_be_bytes()); // QCLASS IN
        // Answer section.
        buf.extend_from_slice(answer_name);
        buf.extend_from_slice(&rtype.to_be_bytes());
        buf.extend_from_slice(&1u16.to_be_bytes()); // CLASS IN
        buf.extend_from_slice(&300u32.to_be_bytes()); // TTL
        buf.extend_from_slice(&(rdata.len() as u16).to_be_bytes());
        buf.extend_from_slice(rdata);
        buf
    }

    /// Parses a raw SVCB or HTTPS rdata vector into its record data.
    ///
    /// Wraps it in a one-answer response first, so the RFC 9460 Appendix D
    /// vectors drive straight through [`parse_records`].
    fn parse_svcb_vector(rtype: u16, kind: RecordKind, rdata: &[u8]) -> SvcbRecordData {
        let resp = raw_response(rtype, EXAMPLE_WIRE, EXAMPLE_WIRE, rtype, rdata);
        let (records, _) = parse_records(&resp, kind).expect("vector should parse");
        match records.as_slice() {
            [Record::Svcb(svcb)] => svcb.clone(),
            [Record::Https(https)] => https.svcb().clone(),
            other => panic!("expected one SVCB/HTTPS record, got {other:?}"),
        }
    }

    /// RFC 9460 Appendix D.1: an AliasMode record.
    ///
    /// Priority 0, a target, and no parameters.
    #[test]
    fn parse_svcb_aliasmode_vector() {
        let svcb = parse_svcb_vector(
            TYPE_SVCB,
            RecordKind::Svcb,
            b"\x00\x00\x03foo\x07example\x03com\x00",
        );
        assert_eq!(svcb.priority(), 0);
        assert_eq!(svcb.target(), "foo.example.com");
        assert!(svcb.alpn().next().is_none());
        assert_eq!(svcb.port(), None);
        assert!(svcb.ipv4hint().next().is_none());
        assert!(svcb.ipv6hint().next().is_none());
    }

    /// RFC 9460 Appendix D.2.3: a ServiceMode record targeting the root.
    ///
    /// The root target surfaces as an empty target string.
    #[test]
    fn parse_svcb_root_target_vector() {
        let svcb = parse_svcb_vector(TYPE_SVCB, RecordKind::Svcb, b"\x00\x01\x00");
        assert_eq!(svcb.priority(), 1);
        assert_eq!(svcb.target(), "");
    }

    /// RFC 9460 Appendix D.2.4: a record carrying only the `port` parameter.
    #[test]
    fn parse_svcb_port_vector() {
        let svcb = parse_svcb_vector(
            TYPE_SVCB,
            RecordKind::Svcb,
            b"\x00\x10\x03foo\x07example\x03com\x00\x00\x03\x00\x02\x00\x35",
        );
        assert_eq!(svcb.priority(), 16);
        assert_eq!(svcb.target(), "foo.example.com");
        assert_eq!(svcb.port(), Some(53));
    }

    /// The D.2.4 vector parses identically through the HTTPS path.
    ///
    /// HTTPS records reuse the SVCB rdata format.
    #[test]
    fn parse_https_port_vector() {
        let https = parse_svcb_vector(
            TYPE_HTTPS,
            RecordKind::Https,
            b"\x00\x10\x03foo\x07example\x03com\x00\x00\x03\x00\x02\x00\x35",
        );
        assert_eq!(https.priority(), 16);
        assert_eq!(https.port(), Some(53));
    }

    /// RFC 9460 Appendix D.2.7: two IPv6 hints.
    #[test]
    fn parse_svcb_ipv6hint_vector() {
        let svcb = parse_svcb_vector(
            TYPE_SVCB,
            RecordKind::Svcb,
            b"\x00\x01\x03foo\x07example\x03com\x00\x00\x06\x00\x20\
              \x20\x01\x0d\xb8\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x01\
              \x20\x01\x0d\xb8\x00\x00\x00\x00\x00\x00\x00\x00\x00\x53\x00\x01",
        );
        assert_eq!(svcb.priority(), 1);
        assert_eq!(svcb.target(), "foo.example.com");
        assert_eq!(
            svcb.ipv6hint().collect::<Vec<_>>(),
            [
                Ipv6Addr::new(0x2001, 0x0db8, 0, 0, 0, 0, 0, 1),
                Ipv6Addr::new(0x2001, 0x0db8, 0, 0, 0, 0, 0x0053, 1),
            ]
        );
    }

    /// RFC 9460 Appendix D.2.10: `mandatory`, `alpn` and `ipv4hint` together.
    ///
    /// Stored out of presentation order, but sorted on the wire.
    #[test]
    fn parse_svcb_mandatory_alpn_ipv4hint_vector() {
        let svcb = parse_svcb_vector(
            TYPE_SVCB,
            RecordKind::Svcb,
            b"\x00\x10\x03foo\x07example\x03org\x00\
              \x00\x00\x00\x04\x00\x01\x00\x04\
              \x00\x01\x00\x09\x02h2\x05h3-19\
              \x00\x04\x00\x04\xc0\x00\x02\x01",
        );
        assert_eq!(svcb.priority(), 16);
        assert_eq!(svcb.target(), "foo.example.org");
        assert_eq!(svcb.alpn().collect::<Vec<_>>(), ["h2", "h3-19"]);
        assert_eq!(
            svcb.ipv4hint().collect::<Vec<_>>(),
            [Ipv4Addr::new(192, 0, 2, 1)]
        );
    }

    /// SvcParamKeys out of ascending order are a clean parse error.
    ///
    /// RFC 9460 Section 2.2 requires them strictly ascending on the wire, and
    /// `simple_dns` rejects the packet rather than panicking.
    #[test]
    fn parse_svcb_out_of_order_params_is_malformed() {
        // Priority 1, root target, then `port` (key 3) before `alpn` (key 1).
        let rdata = b"\x00\x01\x00\
                      \x00\x03\x00\x02\x00\x35\
                      \x00\x01\x00\x09\x02h2\x05h3-19";
        let resp = raw_response(TYPE_SVCB, EXAMPLE_WIRE, EXAMPLE_WIRE, TYPE_SVCB, rdata);
        assert!(matches!(
            parse_records(&resp, RecordKind::Svcb),
            Err(QueryError::Malformed { .. })
        ));
    }

    /// An A answer with an rdlength of 3 is a clean parse error.
    ///
    /// Four bytes are required, and a short read must not panic.
    #[test]
    fn parse_a_with_short_rdata_is_malformed() {
        let resp = raw_response(
            u16::from(TYPE::A),
            EXAMPLE_WIRE,
            EXAMPLE_WIRE,
            u16::from(TYPE::A),
            b"\x01\x02\x03",
        );
        assert!(matches!(
            parse_records(&resp, RecordKind::A),
            Err(QueryError::Malformed { .. })
        ));
    }

    /// An owner name that is a compression pointer resolves to its target.
    ///
    /// Pointing at the question name matches, so the record is returned.
    #[test]
    fn parse_a_with_compression_pointer_resolves() {
        // The question name begins at offset 12, right after the fixed header.
        let pointer = (0xc000u16 | 12u16).to_be_bytes();
        let resp = raw_response(
            u16::from(TYPE::A),
            EXAMPLE_WIRE,
            &pointer,
            u16::from(TYPE::A),
            &[10, 0, 0, 7],
        );
        let (addrs, _) = parse_a_addrs(&resp);
        assert_eq!(addrs, [Ipv4Addr::new(10, 0, 0, 7)]);
    }

    /// A self-referential compression pointer is a clean parse error.
    ///
    /// `simple_dns` follows only strictly-backward pointers, so one aimed at
    /// its own offset fails without looping.
    #[test]
    fn parse_a_with_circular_pointer_is_malformed() {
        // The answer name sits right after the 12-byte header and the question
        // (name + qtype + qclass); point it at its own offset.
        let answer_offset = 12 + EXAMPLE_WIRE.len() + 4;
        let pointer = (0xc000u16 | answer_offset as u16).to_be_bytes();
        let resp = raw_response(
            u16::from(TYPE::A),
            EXAMPLE_WIRE,
            &pointer,
            u16::from(TYPE::A),
            &[10, 0, 0, 7],
        );
        assert!(matches!(
            parse_records(&resp, RecordKind::A),
            Err(QueryError::Malformed { .. })
        ));
    }

    /// A record of a class other than IN is not an answer to an IN question.
    ///
    /// Neither as a record of the queried type nor as a link in the CNAME
    /// chain.
    #[test]
    fn parse_ignores_records_of_another_class() {
        let (id, _) = make_query("example.com");
        let mut packet = reply_with_question(id, "example.com", TYPE::A);
        packet.answers.push(ResourceRecord::new(
            Name::new_unchecked("example.com"),
            CLASS::CH,
            300,
            RData::A(A {
                address: u32::from(Ipv4Addr::new(6, 6, 6, 6)),
            }),
        ));
        packet.answers.push(ResourceRecord::new(
            Name::new_unchecked("example.com"),
            CLASS::CH,
            300,
            RData::CNAME(CNAME(Name::new_unchecked("other.example.com"))),
        ));
        packet.answers.push(ResourceRecord::new(
            Name::new_unchecked("other.example.com"),
            CLASS::IN,
            300,
            RData::A(A {
                address: u32::from(Ipv4Addr::new(7, 7, 7, 7)),
            }),
        ));
        let resp = packet.build_bytes_vec().unwrap();

        let (addrs, _) = parse_a_addrs(&resp);
        assert!(addrs.is_empty(), "got {addrs:?}");
        let packet = Packet::parse(&resp).unwrap();
        assert_eq!(cname_target(&packet, "example.com"), None);
    }

    /// A record on an intermediate name in the CNAME chain is still collected.
    ///
    /// The name is neither the query name nor the final canonical name, but
    /// recursive resolvers extract from the whole chain.
    #[test]
    fn parse_collects_records_on_intermediate_cname_name() {
        let (id, _) = make_query("alias.example.com");
        let mut packet = Packet::new_reply(id);
        packet.set_flags(PacketFlag::RECURSION_DESIRED | PacketFlag::RECURSION_AVAILABLE);
        packet.questions.push(Question::new(
            Name::new_unchecked("alias.example.com"),
            QTYPE::TYPE(TYPE::A),
            QCLASS::CLASS(CLASS::IN),
            false,
        ));
        packet.answers.push(ResourceRecord::new(
            Name::new_unchecked("alias.example.com"),
            CLASS::IN,
            300,
            RData::CNAME(CNAME(Name::new_unchecked("middle.example.com"))),
        ));
        packet.answers.push(ResourceRecord::new(
            Name::new_unchecked("middle.example.com"),
            CLASS::IN,
            300,
            RData::CNAME(CNAME(Name::new_unchecked("real.example.com"))),
        ));
        // The A record sits on the intermediate `middle`, not the final `real`.
        packet.answers.push(ResourceRecord::new(
            Name::new_unchecked("middle.example.com"),
            CLASS::IN,
            300,
            RData::A(A {
                address: u32::from(Ipv4Addr::new(9, 9, 9, 9)),
            }),
        ));
        let resp = packet.build_bytes_vec().unwrap();

        let (addrs, _) = parse_a_addrs(&resp);
        assert_eq!(addrs, [Ipv4Addr::new(9, 9, 9, 9)]);
    }

    /// The SVCB accessors surface the less common parameters.
    ///
    /// Namely the ECH config, the mandatory key list and the no-default-alpn
    /// flag.
    #[test]
    fn svcb_exposes_ech_mandatory_and_no_default_alpn() {
        use simple_dns::rdata::{SVCB, SVCParam};

        let mut svcb = SVCB::new(1, Name::new_unchecked("svc.example.com"));
        svcb.set_alpn(&["h2".try_into().unwrap()]);
        svcb.set_mandatory([1u16].into_iter());
        svcb.set_no_default_alpn();
        svcb.set_param(SVCParam::Ech(std::borrow::Cow::Borrowed(
            b"echconfig".as_slice(),
        )));
        let resp = reply_with_answer("svc.example.com", TYPE::SVCB, RData::SVCB(svcb));

        let (records, _) = parse_records(&resp, RecordKind::Svcb).unwrap();
        let [Record::Svcb(data)] = records.as_slice() else {
            panic!("expected one SVCB record, got {records:?}");
        };
        assert_eq!(data.mandatory().collect::<Vec<_>>(), vec![1]);
        assert!(data.no_default_alpn());
        assert_eq!(data.ech(), Some(b"echconfig".as_slice()));
    }

    /// `into_boxed_slices` hands over the character strings without copying.
    #[test]
    fn txt_into_boxed_slices_hands_over_storage() {
        let data = TxtRecordData::from(vec![
            b"a".to_vec().into_boxed_slice(),
            b"bb".to_vec().into_boxed_slice(),
        ]);
        let slices = data.into_boxed_slices();
        assert_eq!(slices.len(), 2);
        assert_eq!(&*slices[0], b"a");
        assert_eq!(&*slices[1], b"bb");
    }

    /// A short-TTL CNAME in the chain bounds the returned minimum TTL.
    ///
    /// The alias must not be cached past the CNAME's intended lifetime, even
    /// when the final records carry a longer TTL.
    #[test]
    fn cname_ttl_bounds_the_min_ttl() {
        let (id, _) = make_query("alias.example.com");
        let mut packet = Packet::new_reply(id);
        packet.set_flags(PacketFlag::RECURSION_DESIRED | PacketFlag::RECURSION_AVAILABLE);
        packet.questions.push(Question::new(
            Name::new_unchecked("alias.example.com"),
            QTYPE::TYPE(TYPE::A),
            QCLASS::CLASS(CLASS::IN),
            false,
        ));
        packet.answers.push(ResourceRecord::new(
            Name::new_unchecked("alias.example.com"),
            CLASS::IN,
            50,
            RData::CNAME(CNAME(Name::new_unchecked("real.example.com"))),
        ));
        packet.answers.push(ResourceRecord::new(
            Name::new_unchecked("real.example.com"),
            CLASS::IN,
            300,
            RData::A(A {
                address: u32::from(Ipv4Addr::new(1, 2, 3, 4)),
            }),
        ));
        let resp = packet.build_bytes_vec().unwrap();

        let (records, ttl) = parse_records(&resp, RecordKind::A).unwrap();
        assert_eq!(ttl, 50);
        assert_eq!(records.len(), 1);
    }
}
