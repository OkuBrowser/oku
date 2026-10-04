use bytes::Bytes;
use chrono::DateTime;
use chrono::FixedOffset;
use chrono::SecondsFormat;
use chrono::Utc;
use ipld_core::ipld::Ipld;
use libp2p_identity::PeerId;
use libp2p_identity::PublicKey;
use libp2p_identity::{DecodingError, Keypair, SigningError};
use prost::Message;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::BTreeMap;

mod generate;

const SIGNATURE_V2_BASE: &[u8] = &[
    0x69, 0x70, 0x6e, 0x73, 0x2d, 0x73, 0x69, 0x67, 0x6e, 0x61, 0x74, 0x75, 0x72, 0x65, 0x3a,
];

/// libp2p inlines a public key into the PeerID (via an `identity` multihash) when its protobuf
/// encoding is at most this many bytes; larger keys are referenced by a sha2-256 hash instead.
const MAX_INLINE_KEY_LENGTH: usize = 42;

/// Errors produced when creating, decoding, or validating an IPNS [`Record`].
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// The record exceeds the 10 KiB IPNS size limit.
    RecordTooLarge,
    /// The record is missing its V2 signature.
    MissingSignature,
    /// The record is missing its data field.
    EmptyData,
    /// The signing key does not correspond to the IPNS name.
    NameMismatch,
    /// The IPNS name does not inline a public key and the record omits one.
    MissingPublicKey,
    /// The V2 signature failed verification.
    InvalidSignature,
    /// The record's EOL validity has elapsed.
    Expired,
    /// The dag-cbor data does not match the record's protobuf fields.
    DataMismatch,
    /// Unrecognized validity type.
    InvalidValidityType,
    /// Malformed protobuf.
    ProtoEncoding(prost::EncodeError),
    /// Malformed protobuf.
    ProtoDecoding(prost::DecodeError),
    /// Malformed dag-cbor data.
    Cbor(Box<dyn std::error::Error + Send + Sync + 'static>),
    /// Malformed EOL validity timestamp.
    InvalidValidity(chrono::ParseError),
    /// A key or signing operation failed.
    SigningError(SigningError),
    /// Invalid public key.
    InvalidPublicKey(DecodingError),
    /// Malformed multihash or peer id.
    Multihash(multihash::Error),
    /// A metadata key collides with a reserved IPNS field name.
    ReservedMetadataKey(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::RecordTooLarge => write!(f, "record exceeds the 10 KiB limit"),
            Error::MissingSignature => write!(f, "record is missing a V2 signature"),
            Error::EmptyData => write!(f, "record is missing its data field"),
            Error::NameMismatch => write!(f, "public key does not match the IPNS name"),
            Error::MissingPublicKey => {
                write!(
                    f,
                    "record omits pubKey but the IPNS name does not inline one"
                )
            }
            Error::InvalidSignature => write!(f, "signature is invalid"),
            Error::Expired => write!(f, "record has expired"),
            Error::DataMismatch => write!(f, "dag-cbor data does not match the protobuf fields"),
            Error::InvalidValidityType => write!(f, "invalid validity type"),
            Error::ProtoEncoding(e) => write!(f, "protobuf error: {e}"),
            Error::ProtoDecoding(e) => write!(f, "protobuf error: {e}"),
            Error::Cbor(e) => write!(f, "dag-cbor error: {e}"),
            Error::InvalidValidity(e) => write!(f, "invalid validity timestamp: {e}"),
            Error::SigningError(e) => write!(f, "signing error: {e}"),
            Error::InvalidPublicKey(e) => write!(f, "invalid public key: {e}"),
            Error::Multihash(e) => write!(f, "invalid multihash: {e}"),
            Error::ReservedMetadataKey(k) => write!(f, "metadata key `{k}` is reserved"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::ProtoEncoding(e) => Some(e),
            Error::ProtoDecoding(e) => Some(e),
            Error::SigningError(e) => Some(e),
            Error::InvalidPublicKey(e) => Some(e),
            Error::Multihash(e) => Some(e),
            Error::Cbor(e) => Some(&**e),
            Error::InvalidValidity(e) => Some(e),
            _ => None,
        }
    }
}

impl From<chrono::ParseError> for Error {
    fn from(e: chrono::ParseError) -> Self {
        Error::InvalidValidity(e)
    }
}

impl From<Error> for std::io::Error {
    fn from(e: Error) -> Self {
        std::io::Error::new(std::io::ErrorKind::InvalidData, e)
    }
}

impl From<prost::DecodeError> for Error {
    fn from(e: prost::DecodeError) -> Self {
        Error::ProtoDecoding(e)
    }
}

impl From<prost::EncodeError> for Error {
    fn from(e: prost::EncodeError) -> Self {
        Error::ProtoEncoding(e)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(i32)]
pub enum ValidityType {
    EOL = 0,
}

impl std::fmt::Display for ValidityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EOL")
    }
}

impl Serialize for ValidityType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_i32(*self as i32)
    }
}

impl<'de> Deserialize<'de> for ValidityType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let i = i32::deserialize(deserializer)?;
        ValidityType::try_from(i).map_err(serde::de::Error::custom)
    }
}

impl TryFrom<i32> for ValidityType {
    type Error = Error;
    fn try_from(i: i32) -> Result<Self, Self::Error> {
        match i {
            0 => Ok(ValidityType::EOL),
            _ => Err(Error::InvalidValidityType),
        }
    }
}

impl From<ValidityType> for i32 {
    fn from(ty: ValidityType) -> Self {
        ty as i32
    }
}

#[derive(Clone, Debug)]
pub struct Record {
    data: Option<Bytes>,

    value: Option<Bytes>,
    validity_type: ValidityType,
    validity: Option<Bytes>,
    sequence: Option<u64>,
    ttl: Option<u64>,

    public_key: Option<Bytes>,

    signature_v1: Option<Bytes>,
    signature_v2: Option<Bytes>,
}

impl From<generate::ipns_pb::IpnsEntry> for Record {
    fn from(entry: generate::ipns_pb::IpnsEntry) -> Self {
        Record {
            data: entry.data,
            value: entry.value,
            validity_type: entry
                .validity_type
                .and_then(|v| ValidityType::try_from(v).ok())
                .unwrap_or(ValidityType::EOL),
            validity: entry.validity,
            sequence: entry.sequence,
            ttl: entry.ttl,
            public_key: entry.pub_key,
            signature_v1: entry.signature_v1,
            signature_v2: entry.signature_v2,
        }
    }
}

impl From<Record> for generate::ipns_pb::IpnsEntry {
    fn from(record: Record) -> Self {
        generate::ipns_pb::IpnsEntry {
            validity: record.validity,
            validity_type: Some(record.validity_type.into()),
            value: record.value,
            signature_v1: record.signature_v1,
            signature_v2: record.signature_v2,
            sequence: record.sequence,
            pub_key: record.public_key,
            ttl: record.ttl,
            data: record.data,
        }
    }
}

impl From<&Record> for generate::ipns_pb::IpnsEntry {
    fn from(record: &Record) -> Self {
        generate::ipns_pb::IpnsEntry {
            validity: record.validity.clone(),
            validity_type: Some(record.validity_type.into()),
            value: record.value.clone(),
            signature_v1: record.signature_v1.clone(),
            signature_v2: record.signature_v2.clone(),
            sequence: record.sequence,
            pub_key: record.public_key.clone(),
            ttl: record.ttl,
            data: record.data.clone(),
        }
    }
}

// Fields of the Bytes type are used here instead of Vec<u8> to ensure that
// these fields are (de)serialized into "byte string" CBOR values instead of simple arrays.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Data {
    #[serde(rename = "Value")]
    pub value: Bytes,

    #[serde(rename = "ValidityType")]
    pub validity_type: ValidityType,

    #[serde(rename = "Validity")]
    pub validity: Bytes,

    #[serde(rename = "Sequence")]
    pub sequence: u64,

    #[serde(rename = "TTL")]
    pub ttl: u64,

    /// Additional non-standard metadata keys carried in the dag-cbor map. Empty for typical
    /// records; preserved verbatim on decode/encode and covered by the V2 signature.
    #[serde(flatten)]
    pub metadata: BTreeMap<String, Ipld>,
}

impl Data {
    pub fn value(&self) -> &[u8] {
        &self.value
    }

    /// Non-standard metadata keys carried alongside the reserved IPNS fields.
    pub fn metadata(&self) -> &BTreeMap<String, Ipld> {
        &self.metadata
    }

    pub fn validity_type(&self) -> ValidityType {
        self.validity_type
    }

    pub fn validity(&self) -> &[u8] {
        &self.validity
    }

    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    pub fn ttl(&self) -> u64 {
        self.ttl
    }
}

impl Record {
    /// Creates and signs an IPNS record pointing at `value`, valid until the absolute `eol`
    /// (End-Of-Life) timestamp. `ttl` is a caching hint for resolvers.
    pub fn new(
        keypair: &Keypair,
        value: impl AsRef<[u8]>,
        eol: DateTime<Utc>,
        seq: u64,
        ttl: std::time::Duration,
    ) -> Result<Self, Error> {
        Self::new_with_metadata(keypair, value, eol, seq, ttl, BTreeMap::new())
    }

    /// Like [`Record::new`] but attaches additional dag-cbor `metadata` keys to the record. Keys
    /// must not collide with the reserved fields (`Value`, `Validity`, `ValidityType`, `Sequence`,
    /// `TTL`).
    pub fn new_with_metadata(
        keypair: &Keypair,
        value: impl AsRef<[u8]>,
        eol: DateTime<Utc>,
        seq: u64,
        ttl: std::time::Duration,
        metadata: BTreeMap<String, Ipld>,
    ) -> Result<Self, Error> {
        for reserved in ["Value", "Validity", "ValidityType", "Sequence", "TTL"] {
            if metadata.contains_key(reserved) {
                return Err(Error::ReservedMetadataKey(reserved.to_string()));
            }
        }

        let value = Bytes::from(value.as_ref().to_vec());

        let ttl = u64::try_from(ttl.as_nanos()).unwrap_or(u64::MAX);

        let validity = eol.to_rfc3339_opts(SecondsFormat::Nanos, true).into_bytes();

        let validity_type = ValidityType::EOL;

        let signature_v1_construct = {
            let mut data = Vec::with_capacity(value.len() + validity.len() + 3);

            data.extend(value.iter());
            data.extend(validity.iter());
            data.extend(validity_type.to_string().as_bytes());

            data
        };

        let signature_v1 = keypair
            .sign(&signature_v1_construct)
            .map_err(Error::SigningError)?;

        let document = Data {
            value: value.clone(),
            validity_type,
            validity: Bytes::from(validity.clone()),
            sequence: seq,
            ttl,
            metadata,
        };

        let data = serde_ipld_dagcbor::to_vec(&document)
            .map(Bytes::from)
            .map_err(|e| Error::Cbor(Box::new(e)))?;

        let signature_v2_construct = SIGNATURE_V2_BASE
            .iter()
            .chain(data.iter())
            .copied()
            .collect::<Vec<_>>();

        let signature_v2 = keypair
            .sign(&signature_v2_construct)
            .map_err(Error::SigningError)?;

        let encoded_public_key = keypair.public().encode_protobuf();
        let public_key = if encoded_public_key.len() > MAX_INLINE_KEY_LENGTH {
            encoded_public_key
        } else {
            Vec::new()
        };

        Ok(Record {
            data: Some(data),
            value: Some(value),
            validity_type,
            validity: Some(validity.into()),
            sequence: Some(seq),
            ttl: Some(ttl),
            public_key: Some(public_key.into()),
            signature_v1: (!signature_v1.is_empty()).then_some(signature_v1.into()),
            signature_v2: (!signature_v2.is_empty()).then_some(signature_v2.into()),
        })
    }

    pub fn decode(data: impl AsRef<[u8]>) -> Result<Self, Error> {
        let data = data.as_ref();

        if data.len() > 10 * 1024 {
            return Err(Error::RecordTooLarge);
        }
        let entry: generate::ipns_pb::IpnsEntry = prost::Message::decode(data)?;

        let record = entry.into();
        Ok(record)
    }

    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        let entry: generate::ipns_pb::IpnsEntry = self.into();
        let buffer = entry.encode_to_vec();
        Ok(buffer)
    }
}

impl Record {
    pub fn sequence(&self) -> u64 {
        self.sequence.unwrap_or_default()
    }

    pub fn validity_type(&self) -> ValidityType {
        self.validity_type
    }

    pub fn validity(&self) -> Result<DateTime<FixedOffset>, Error> {
        let validity_bytes = self.validity.as_deref().unwrap_or_default();
        let time = String::from_utf8_lossy(validity_bytes);
        Ok(chrono::DateTime::parse_from_rfc3339(&time)?)
    }

    pub fn ttl(&self) -> u64 {
        self.ttl.unwrap_or_default()
    }

    pub fn signature_v1(&self) -> &[u8] {
        self.signature_v1.as_deref().unwrap_or_default()
    }

    pub fn signature_v2(&self) -> &[u8] {
        self.signature_v2.as_deref().unwrap_or_default()
    }

    /// Whether the record carries a (legacy) V1 signature.
    pub fn has_signature_v1(&self) -> bool {
        self.signature_v1
            .as_ref()
            .is_some_and(|sig| !sig.is_empty())
    }

    /// Whether the record carries a V2 signature.
    pub fn has_signature_v2(&self) -> bool {
        self.signature_v2
            .as_ref()
            .is_some_and(|sig| !sig.is_empty())
    }

    pub fn data(&self) -> Result<Data, Error> {
        let Some(data) = self.data.as_ref() else {
            return Err(Error::EmptyData);
        };

        let data: Data =
            serde_ipld_dagcbor::from_slice(data).map_err(|e| Error::Cbor(Box::new(e)))?;

        if data.value != self.value.as_deref().unwrap_or_default()
            || data.validity != self.validity.as_deref().unwrap_or_default()
            || data.validity_type != self.validity_type
            || data.sequence != self.sequence()
            || data.ttl != self.ttl()
        {
            return Err(Error::DataMismatch);
        }

        Ok(data)
    }

    /// The raw IPNS value.
    pub fn value(&self) -> &[u8] {
        self.value.as_deref().unwrap_or_default()
    }

    pub fn verify_signature(&self, peer_id: PeerId) -> Result<(), Error> {
        use multihash::Multihash;

        if !self.has_signature_v2() {
            return Err(Error::MissingSignature);
        }

        if let Some(data) = self.data.as_ref()
            && data.is_empty()
        {
            return Err(Error::EmptyData);
        }

        let public_key_bytes = self.public_key.as_deref().unwrap_or_default();

        let public_key = if public_key_bytes.is_empty() {
            let mh = Multihash::<64>::from_bytes(&peer_id.to_bytes()).map_err(Error::Multihash)?;
            // small keys are inlined in the name via an identity (code 0) multihash; anything
            // else (e.g. an RSA name) carries no inlined key, so the record must embed one.
            if mh.code() != 0 {
                return Err(Error::MissingPublicKey);
            }
            PublicKey::try_decode_protobuf(mh.digest())
        } else {
            PublicKey::try_decode_protobuf(public_key_bytes)
        }
        .map_err(Error::InvalidPublicKey)?;

        if PeerId::from_public_key(&public_key) != peer_id {
            return Err(Error::NameMismatch);
        }

        self.data()?;

        let data_bytes = self.data.as_deref().unwrap_or_default();

        let signature_v2 = SIGNATURE_V2_BASE
            .iter()
            .chain(data_bytes.iter())
            .copied()
            .collect::<Vec<_>>();

        if !public_key.verify(&signature_v2, self.signature_v2()) {
            return Err(Error::InvalidSignature);
        }

        Ok(())
    }

    /// Fully validates the record against `peer_id`: name binding, V2 signature, and that the EOL
    /// validity has not elapsed.
    pub fn verify(&self, peer_id: PeerId) -> Result<(), Error> {
        self.verify_signature(peer_id)?;

        if self.validity()?.with_timezone(&Utc) < Utc::now() {
            return Err(Error::Expired);
        }

        Ok(())
    }

    /// Orders this record against `other` by IPNS precedence: higher `sequence` wins, then the
    /// later EOL `validity`.
    /// Records should already be validated and refer to the same name.
    pub fn compare(&self, other: &Record) -> Result<std::cmp::Ordering, Error> {
        use std::cmp::Ordering;

        match self
            .has_signature_v2()
            .cmp(&other.has_signature_v2())
            .then_with(|| self.sequence.cmp(&other.sequence))
        {
            Ordering::Equal => Ok(self.validity()?.cmp(&other.validity()?)),
            ord => Ok(ord),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn record_for(kp: &Keypair, hours: i64) -> Record {
        Record::new(
            kp,
            b"/ipfs/bafkqaaa",
            Utc::now() + Duration::hours(hours),
            0,
            std::time::Duration::ZERO,
        )
        .unwrap()
    }

    #[test]
    fn valid_record_roundtrips_and_verifies() {
        let kp = Keypair::generate_ed25519();
        let peer = PeerId::from_public_key(&kp.public());
        let rec = record_for(&kp, 24);
        rec.verify(peer).unwrap();

        let decoded = Record::decode(rec.encode().unwrap()).unwrap();
        decoded.verify(peer).unwrap();
    }

    #[test]
    fn verify_rejects_expired_record_but_signature_still_checks() {
        let kp = Keypair::generate_ed25519();
        let peer = PeerId::from_public_key(&kp.public());
        let rec = record_for(&kp, -1);
        rec.verify_signature(peer).unwrap();
        assert!(rec.verify(peer).is_err());
    }

    #[test]
    fn embedded_pubkey_must_match_the_name() {
        let attacker = Keypair::generate_ed25519();
        let victim = Keypair::generate_ed25519();
        let attacker_peer = PeerId::from_public_key(&attacker.public());
        let victim_peer = PeerId::from_public_key(&victim.public());

        // a genuine attacker record, with the attacker's pubKey spliced into the protobuf
        // (field 7, tag 0x3a) to mimic a record that carries an embedded key.
        let mut bytes = record_for(&attacker, 24).encode().unwrap();
        let pk = attacker.public().encode_protobuf();
        bytes.push(0x3a);
        bytes.push(pk.len() as u8); // an ed25519 protobuf key is < 128 bytes
        bytes.extend_from_slice(&pk);

        let tampered = Record::decode(&bytes).unwrap();
        tampered.verify_signature(attacker_peer).unwrap();
        // the embedded key does not hash to the victim's name, so it must not validate for it.
        assert!(tampered.verify_signature(victim_peer).is_err());
    }

    #[test]
    fn create_and_verify_across_key_types() {
        // Ed25519/Secp256k1 inline into the name; ECDSA does not, so its record must embed the
        // public key. All must create and verify (and survive a wire round-trip).
        for kp in [
            Keypair::generate_ed25519(),
            Keypair::generate_secp256k1(),
            Keypair::generate_ecdsa(),
        ] {
            let peer = PeerId::from_public_key(&kp.public());
            let rec = Record::new(
                &kp,
                b"/ipfs/bafkqaaa",
                Utc::now() + Duration::hours(24),
                0,
                std::time::Duration::ZERO,
            )
            .unwrap();
            rec.verify(peer).unwrap();
            let decoded = Record::decode(rec.encode().unwrap()).unwrap();
            decoded.verify(peer).unwrap();
        }
    }

    #[test]
    fn compare_prefers_higher_sequence_then_later_validity() {
        use std::cmp::Ordering;
        let kp = Keypair::generate_ed25519();

        let seq0 = Record::new(
            &kp,
            b"/ipfs/bafkqaaa",
            Utc::now() + Duration::hours(24),
            0,
            std::time::Duration::ZERO,
        )
        .unwrap();
        let seq1 = Record::new(
            &kp,
            b"/ipfs/bafkqaaa",
            Utc::now() + Duration::hours(1),
            1,
            std::time::Duration::ZERO,
        )
        .unwrap();
        // higher sequence wins even with an earlier EOL
        assert_eq!(seq1.compare(&seq0).unwrap(), Ordering::Greater);
        assert_eq!(seq0.compare(&seq1).unwrap(), Ordering::Less);

        let near = Record::new(
            &kp,
            b"/ipfs/bafkqaaa",
            Utc::now() + Duration::hours(1),
            5,
            std::time::Duration::ZERO,
        )
        .unwrap();
        let far = Record::new(
            &kp,
            b"/ipfs/bafkqaaa",
            Utc::now() + Duration::hours(48),
            5,
            std::time::Duration::ZERO,
        )
        .unwrap();
        // equal sequence: the later EOL wins
        assert_eq!(far.compare(&near).unwrap(), Ordering::Greater);
    }

    #[test]
    fn compare_prefers_v2_over_v1_only() {
        use std::cmp::Ordering;
        let kp = Keypair::generate_ed25519();

        let with_v2 = record_for(&kp, 1);
        let mut v1_only = record_for(&kp, 48); // later EOL, but no V2
        v1_only.signature_v2 = None;
        assert!(with_v2.has_signature_v2() && !v1_only.has_signature_v2());

        // V2 presence outranks both sequence and the later validity
        assert_eq!(with_v2.compare(&v1_only).unwrap(), Ordering::Greater);
        assert_eq!(v1_only.compare(&with_v2).unwrap(), Ordering::Less);
    }

    #[test]
    fn metadata_roundtrips_and_is_signed() {
        let kp = Keypair::generate_ed25519();
        let peer = PeerId::from_public_key(&kp.public());
        let mut metadata = BTreeMap::new();
        metadata.insert("Foo".to_string(), Ipld::String("bar".into()));
        metadata.insert("Count".to_string(), Ipld::Integer(7));

        let rec = Record::new_with_metadata(
            &kp,
            b"/ipfs/bafkqaaa",
            Utc::now() + Duration::hours(24),
            0,
            std::time::Duration::ZERO,
            metadata.clone(),
        )
        .unwrap();
        rec.verify(peer).unwrap();

        // survives a wire round-trip, still verifies, and exposes the metadata unchanged
        let decoded = Record::decode(rec.encode().unwrap()).unwrap();
        decoded.verify(peer).unwrap();
        assert_eq!(decoded.data().unwrap().metadata(), &metadata);

        // reserved keys are rejected
        let mut bad = BTreeMap::new();
        bad.insert("TTL".to_string(), Ipld::Integer(1));
        assert!(matches!(
            Record::new_with_metadata(
                &kp,
                b"/x",
                Utc::now() + Duration::hours(1),
                0,
                std::time::Duration::ZERO,
                bad,
            ),
            Err(Error::ReservedMetadataKey(_))
        ));
    }
}
