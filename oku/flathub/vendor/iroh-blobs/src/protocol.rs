//! Protocol for transferring content-addressed blobs over [`iroh`] p2p QUIC connections.
//!
//! # Participants
//!
//! The protocol is a request/response protocol with two parties, a *provider* that
//! serves blobs and a *getter* that requests blobs.
//!
//! # Goals
//!
//! - Be paranoid about data integrity.
//!
//!   Data integrity is considered more important than performance. Data will be validated both on
//!   the provider and getter side. A well behaved provider will never send invalid data. Responses
//!   to range requests contain sufficient information to validate the data.
//!
//!   Note: Validation using blake3 is extremely fast, so in almost all scenarios the validation
//!   will not be the bottleneck even if we validate both on the provider and getter side.
//!
//! - Do not limit the size of blobs or collections.
//!
//!   Blobs can be of arbitrary size, up to terabytes. Likewise, collections can contain an
//!   arbitrary number of links. A well behaved implementation will not require the entire blob or
//!   collection to be in memory at once.
//!
//! - Be efficient when transferring large blobs, including range requests.
//!
//!   It is possible to request entire blobs or ranges of blobs, where the minimum granularity is a
//!   chunk group of 16KiB or 16 blake3 chunks. The worst case overhead when doing range requests
//!   is about two chunk groups per range.
//!
//! - Be efficient when transferring multiple tiny blobs.
//!
//!   For tiny blobs the overhead of sending the blob hashes and the round-trip time for each blob
//!   would be prohibitive.
//!
//! To avoid roundtrips, the protocol allows grouping multiple blobs into *collections*.
//! The semantic meaning of a collection is up to the application. For the purpose
//! of this protocol, a collection is just a grouping of related blobs.
//!
//! # Non-goals
//!
//! - Do not attempt to be generic in terms of the used hash function.
//!
//!   The protocol makes extensive use of the [blake3](https://crates.io/crates/blake3) hash
//!   function and it's special properties such as blake3 verified streaming.
//!
//! - Do not support graph traversal.
//!
//!   The protocol only supports collections that directly contain blobs. If you have deeply nested
//!   graph data, you will need to either do multiple requests or flatten the graph into a single
//!   temporary collection.
//!
//! - Do not support discovery.
//!
//!   The protocol does not yet have a discovery mechanism for asking the provider what ranges are
//!   available for a given blob. Currently you have to have some out-of-band knowledge about what
//!   node has data for a given hash, or you can just try to retrieve the data and see if it is
//!   available.
//!
//! A discovery protocol is planned in the future though.
//!
//! # Requests
//!
//! ## Getter defined requests
//!
//! In this case the getter knows the hash of the blob it wants to retrieve and
//! whether it wants to retrieve a single blob or a collection.
//!
//! The getter needs to define exactly what it wants to retrieve and send the
//! request to the provider.
//!
//! The provider will then respond with the bao encoded bytes for the requested
//! data and then close the connection. It will immediately close the connection
//! in case some data is not available or invalid.
//!
//! ## Provider defined requests
//!
//! In this case the getter sends a blob to the provider. This blob can contain
//! some kind of query. The exact details of the query are up to the application.
//!
//! The provider evaluates the query and responds with a serialized request in
//! the same format as the getter defined requests, followed by the bao encoded
//! data. From then on the protocol is the same as for getter defined requests.
//!
//! ## Specifying the required data
//!
//! A [`GetRequest`] contains a hash and a specification of what data related to
//! that hash is required. The specification is using a [`ChunkRangesSeq`] which
//! has a compact representation on the wire but is otherwise identical to a
//! sequence of sets of ranges.
//!
//! In the following, we describe how the [`GetRequest`] is to be created for
//! different common scenarios.
//!
//! Under the hood, this is using the [`ChunkRangesSeq`] type, but the most
//! convenient way to create a [`GetRequest`] is to use the builder API.
//!
//! Ranges are always given in terms of 1024 byte blake3 chunks, *not* in terms
//! of bytes or chunk groups. The reason for this is that chunks are the fundamental
//! unit of hashing in BLAKE3. Addressing anything smaller than a chunk is not
//! possible, and combining multiple chunks is merely an optimization to reduce
//! metadata overhead.
//!
//! ### Individual blobs
//!
//! In the easiest case, the getter just wants to retrieve a single blob. In this
//! case, the getter specifies [`ChunkRangesSeq`] that contains a single element.
//! This element is the set of all chunks to indicate that we
//! want the entire blob, no matter how many chunks it has.
//!
//! Since this is a very common case, there is a convenience method
//! [`GetRequest::blob`] that only requires the hash of the blob.
//!
//! ```rust
//! # use iroh_blobs::protocol::GetRequest;
//! # let hash: iroh_blobs::Hash = [0; 32].into();
//! let request = GetRequest::blob(hash);
//! ```
//!
//! ### Ranges of blobs
//!
//! In this case, we have a (possibly large) blob and we want to retrieve only
//! some ranges of chunks. This is useful in similar cases as HTTP range requests.
//!
//! We still need just a single element in the [`ChunkRangesSeq`], since we are
//! still only interested in a single blob. However, this element contains all
//! the chunk ranges we want to retrieve.
//!
//! For example, if we want to retrieve chunks 0-10 of a blob, we would
//! create a [`ChunkRangesSeq`] like this:
//!
//! ```rust
//! # use iroh_blobs::protocol::{GetRequest, ChunkRanges, ChunkRangesExt};
//! # let hash: iroh_blobs::Hash = [0; 32].into();
//! let request = GetRequest::builder()
//!     .root(ChunkRanges::chunks(..10))
//!     .build(hash);
//! ```
//!
//! While not that common, it is also possible to request multiple ranges of a
//! single blob. For example, if we want to retrieve chunks `0-10` and `100-110`
//! of a large file, we would create a [`GetRequest`] like this:
//!
//! ```rust
//! # use iroh_blobs::protocol::{GetRequest, ChunkRanges, ChunkRangesExt, ChunkRangesSeq};
//! # let hash: iroh_blobs::Hash = [0; 32].into();
//! let request = GetRequest::builder()
//!     .root(ChunkRanges::chunks(..10) | ChunkRanges::chunks(100..110))
//!     .build(hash);
//! ```
//!
//! This is all great, but in most cases we are not interested in chunks but
//! in bytes. The [`ChunkRanges`] type has a constructor that allows providing
//! byte ranges instead of chunk ranges. These will be rounded up to the
//! nearest chunk.
//!
//! ```rust
//! # use iroh_blobs::protocol::{GetRequest, ChunkRanges, ChunkRangesExt, ChunkRangesSeq};
//! # let hash: iroh_blobs::Hash = [0; 32].into();
//! let request = GetRequest::builder()
//!     .root(ChunkRanges::bytes(..1000) | ChunkRanges::bytes(10000..11000))
//!     .build(hash);
//! ```
//!
//! There are also methods to request a single chunk or a single byte offset,
//! as well as a special constructor for the last chunk of a blob.
//!
//! ```rust
//! # use iroh_blobs::protocol::{GetRequest, ChunkRanges, ChunkRangesExt, ChunkRangesSeq};
//! # let hash: iroh_blobs::Hash = [0; 32].into();
//! let request = GetRequest::builder()
//!     .root(ChunkRanges::offset(1) | ChunkRanges::last_chunk())
//!     .build(hash);
//! ```
//!
//! To specify chunk ranges, we use the [`ChunkRanges`] type alias.
//! This is actually the [`RangeSet`] type from the
//! [range_collections](https://crates.io/crates/range_collections) crate. This
//! type supports efficient boolean operations on sets of non-overlapping ranges.
//!
//! The [`RangeSet2`] type is a type alias for [`RangeSet`] that can store up to
//! 2 boundaries without allocating. This is sufficient for most use cases.
//!
//! [`RangeSet`]: range_collections::range_set::RangeSet
//! [`RangeSet2`]: range_collections::range_set::RangeSet2
//!
//! ### Hash sequences
//!
//! In this case the provider has a hash sequence that refers multiple blobs.
//! We want to retrieve all blobs in the hash sequence.
//!
//! When used for hash sequences, the first element of a [`ChunkRangesSeq`] refers
//! to the hash seq itself, and all subsequent elements refer to the blobs
//! in the hash seq. When a [`ChunkRangesSeq`] specifies ranges for more than
//! one blob, the provider will interpret this as a request for a hash seq.
//!
//! One thing to note is that we might not yet know how many blobs are in the
//! hash sequence. Therefore, it is not possible to download an entire hash seq
//! by just specifying [`ChunkRanges::all()`] for all children.
//!
//! Instead, [`ChunkRangesSeq`] allows defining infinite sequences of range sets.
//! The [`ChunkRangesSeq::all()`] method returns a [`ChunkRangesSeq`] that, when iterated
//! over, will yield [`ChunkRanges::all()`] forever.
//!
//! So a get request to download a hash sequence blob and all its children
//! would look like this:
//!
//! ```rust
//! # use iroh_blobs::protocol::{ChunkRanges, ChunkRangesExt, GetRequest};
//! # let hash: iroh_blobs::Hash = [0; 32].into();
//! let request = GetRequest::builder()
//!     .root(ChunkRanges::all())
//!     .build_open(hash); // repeats the last range forever
//! ```
//!
//! Downloading an entire hash seq is also a very common case, so there is a
//! convenience method [`GetRequest::all`] that only requires the hash of the
//! hash sequence blob.
//!
//! ```rust
//! # use iroh_blobs::protocol::{ChunkRanges, ChunkRangesExt, GetRequest};
//! # let hash: iroh_blobs::Hash = [0; 32].into();
//! let request = GetRequest::all(hash);
//! ```
//!
//! ### Parts of hash sequences
//!
//! The most complex common case is when we have retrieved a hash seq and
//! it's children, but were interrupted before we could retrieve all children.
//!
//! In this case we need to specify the hash seq we want to retrieve, but
//! exclude the children and parts of children that we already have.
//!
//! For example, if we have a hash with 3 children, and we already have
//! the first child and the first 1000000 chunks of the second child.
//!
//! We would create a [`GetRequest`] like this:
//!
//! ```rust
//! # use iroh_blobs::protocol::{GetRequest, ChunkRanges, ChunkRangesExt};
//! # let hash: iroh_blobs::Hash = [0; 32].into();
//! let request = GetRequest::builder()
//!     .child(1, ChunkRanges::chunks(1000000..)) // we don't need the first child;
//!     .next(ChunkRanges::all()) // we need the second child and all subsequent children completely
//!     .build_open(hash);
//! ```
//!
//! ### Requesting chunks for each child
//!
//! The ChunkRangesSeq allows some scenarios that are not covered above. E.g. you
//! might want to request a hash seq and the first chunk of each child blob to
//! do something like mime type detection.
//!
//! You do not know how many children the collection has, so you need to use
//! an infinite sequence.
//!
//! ```rust
//! # use iroh_blobs::protocol::{GetRequest, ChunkRanges, ChunkRangesExt, ChunkRangesSeq};
//! # let hash: iroh_blobs::Hash = [0; 32].into();
//! let request = GetRequest::builder()
//!     .root(ChunkRanges::all())
//!     .next(ChunkRanges::chunk(1)) // the first chunk of each child)
//!     .build_open(hash);
//! ```
//!
//! ### Requesting a single child
//!
//! It is of course possible to request a single child of a collection. E.g.
//! the following would download the second child of a collection:
//!
//! ```rust
//! # use iroh_blobs::protocol::{GetRequest, ChunkRanges, ChunkRangesExt};
//! # let hash: iroh_blobs::Hash = [0; 32].into();
//! let request = GetRequest::builder()
//!     .child(1, ChunkRanges::all()) // we need the second child completely
//!     .build(hash);
//! ```
//!
//! However, if you already have the collection, you might as well locally
//! look up the hash of the child and request it directly.
//!
//! ```rust
//! # use iroh_blobs::protocol::{GetRequest, ChunkRanges, ChunkRangesSeq};
//! # let child_hash: iroh_blobs::Hash = [0; 32].into();
//! let request = GetRequest::blob(child_hash);
//! ```
//!
//! ### Why ChunkRanges and ChunkRangesSeq?
//!
//! You might wonder why we have [`ChunkRangesSeq`], when a simple
//! sequence of [`ChunkRanges`] might also do.
//!
//! The [`ChunkRangesSeq`] type exist to provide an efficient
//! representation of the request on the wire. In the wire encoding of [`ChunkRangesSeq`],
//! [`ChunkRanges`] are encoded alternating intervals of selected and non-selected chunks.
//! This results in smaller numbers that will result in fewer bytes on the wire when using
//! the [postcard](https://crates.io/crates/postcard) encoding format that uses variable
//! length integers.
//!
//! Likewise, the [`ChunkRangesSeq`] type
//! does run length encoding to remove repeating elements. It also allows infinite
//! sequences of [`ChunkRanges`] to be encoded, unlike a simple sequence of
//! [`ChunkRanges`]s.
//!
//! [`ChunkRangesSeq`] should be efficient even in case of very fragmented availability
//! of chunks, like a download from multiple providers that was frequently interrupted.
//!
//! # Responses
//!
//! The response stream contains the bao encoded bytes for the requested data.
//! The data will be sent in the order in which it was requested, so ascending
//! chunks for each blob, and blobs in the order in which they appear in the
//! hash seq.
//!
//! For details on the bao encoding, see the [bao specification](https://github.com/oconnor663/bao/blob/master/docs/spec.md)
//! and the [bao-tree](https://crates.io/crates/bao-tree) crate. The bao-tree crate
//! is identical to the bao crate, except that it allows combining multiple BLAKE3
//! chunks to chunk groups for efficiency.
//!
//! For a complete response, the chunks are guaranteed to completely cover the
//! requested ranges.
//!
//! Reasons for not retrieving a complete response are two-fold:
//!
//! - the connection to the provider was interrupted, or the provider encountered
//!   an internal error. In this case the provider will close the entire noq connection.
//!
//! - the provider does not have the requested data, or discovered on send that the
//!   requested data is not valid.
//!
//! In this case the provider will close just the stream used to send the response.
//! The exact location of the missing data can be retrieved from the error.
//!
//! # Requesting multiple unrelated blobs
//!
//! Let's say you don't have a hash sequence on the provider side, but you
//! nevertheless want to request multiple unrelated blobs in a single request.
//!
//! For this, there is the [`GetManyRequest`] type, which also comes with a
//! builder API.
//!
//! ```rust
//! # use iroh_blobs::protocol::{GetManyRequest, ChunkRanges, ChunkRangesExt};
//! # let hash1: iroh_blobs::Hash = [0; 32].into();
//! # let hash2: iroh_blobs::Hash = [1; 32].into();
//! GetManyRequest::builder()
//!     .hash(hash1, ChunkRanges::all())
//!     .hash(hash2, ChunkRanges::all())
//!     .build();
//! ```
//! If you accidentally or intentionally request ranges for the same hash
//! multiple times, they will be merged into a single [`ChunkRanges`].
//!
//! ```rust
//! # use iroh_blobs::protocol::{GetManyRequest, ChunkRanges, ChunkRangesExt};
//! # let hash1: iroh_blobs::Hash = [0; 32].into();
//! # let hash2: iroh_blobs::Hash = [1; 32].into();
//! GetManyRequest::builder()
//!     .hash(hash1, ChunkRanges::chunk(1))
//!     .hash(hash2, ChunkRanges::all())
//!     .hash(hash1, ChunkRanges::last_chunk())
//!     .build();
//! ```
//!
//! This is mostly useful for requesting multiple tiny blobs in a single request.
//! For large or even medium sized blobs, multiple requests are not expensive.
//! Multiple requests just create multiple streams on the same connection,
//! which is *very* cheap in QUIC.
//!
//! In case nodes are permanently exchanging data, it is somewhat valuable to
//! keep a connection open and reuse it for multiple requests. However, creating
//! a new connection is also very cheap, so you would only do this to optimize
//! a large existing system that has demonstrated performance issues.
//!
//! If in doubt, just use multiple requests and multiple connections.
use std::{
    io,
    ops::{Bound, RangeBounds},
};

use bao_tree::{io::round_up_to_chunks, ChunkNum};
use builder::GetRequestBuilder;
use derive_more::From;
use iroh::endpoint::VarInt;
use postcard::experimental::max_size::MaxSize;
use range_collections::{range_set::RangeSetEntry, RangeSet2};
use serde::{Deserialize, Serialize};
mod range_spec;
pub use bao_tree::ChunkRanges;
use n0_error::stack_error;
pub use range_spec::{ChunkRangesSeq, NonEmptyRequestRangeSpecIter, RangeSpec};

use crate::{api::blobs::Bitfield, util::RecvStreamExt, BlobFormat, Hash, HashAndFormat};

/// Maximum message size is limited to 100MiB for now.
pub const MAX_MESSAGE_SIZE: usize = 1024 * 1024;

/// Error code for a permission error
pub const ERR_PERMISSION: VarInt = VarInt::from_u32(1u32);
/// Error code for when a request is aborted due to a rate limit
pub const ERR_LIMIT: VarInt = VarInt::from_u32(2u32);
/// Error code for when a request is aborted due to internal error
pub const ERR_INTERNAL: VarInt = VarInt::from_u32(3u32);

/// The ALPN used with quic for the iroh blobs protocol.
pub const ALPN: &[u8] = b"/iroh-bytes/4";

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone, From)]
/// A request to the provider
pub enum Request {
    /// A get request for a blob or collection
    Get(GetRequest),
    Observe(ObserveRequest),
    Slot2,
    Slot3,
    Slot4,
    Slot5,
    Slot6,
    Slot7,
    /// The inverse of a get request - push data to the provider
    ///
    /// Note that providers will in many cases reject this request, e.g. if
    /// they don't have write access to the store or don't want to ingest
    /// unknown data.
    Push(PushRequest),
    /// Get multiple blobs in a single request, from a single provider
    ///
    /// This is identical to a [`GetRequest`] for a [`crate::hashseq::HashSeq`], but the provider
    /// does not need to have the hash seq.
    GetMany(GetManyRequest),
}

/// This must contain the request types in the same order as the full requests
#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone, Copy, MaxSize)]
pub enum RequestType {
    Get,
    Observe,
    Slot2,
    Slot3,
    Slot4,
    Slot5,
    Slot6,
    Slot7,
    Push,
    GetMany,
}

impl Request {
    pub async fn read_async<R: crate::util::RecvStream>(
        reader: &mut R,
    ) -> io::Result<(Self, usize)> {
        let request_type = reader.read_u8().await?;
        let request_type: RequestType = postcard::from_bytes(std::slice::from_ref(&request_type))
            .map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "failed to deserialize request type",
            )
        })?;
        Ok(match request_type {
            RequestType::Get => {
                let (r, size) = reader
                    .read_to_end_as::<GetRequest>(MAX_MESSAGE_SIZE)
                    .await?;
                (r.into(), size)
            }
            RequestType::GetMany => {
                let (r, size) = reader
                    .read_to_end_as::<GetManyRequest>(MAX_MESSAGE_SIZE)
                    .await?;
                (r.into(), size)
            }
            RequestType::Observe => {
                let (r, size) = reader
                    .read_to_end_as::<ObserveRequest>(MAX_MESSAGE_SIZE)
                    .await?;
                (r.into(), size)
            }
            RequestType::Push => {
                let r = reader
                    .read_length_prefixed::<PushRequest>(MAX_MESSAGE_SIZE)
                    .await?;
                let size = postcard::experimental::serialized_size(&r).unwrap();
                (r.into(), size)
            }
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "failed to deserialize request type",
                ));
            }
        })
    }
}

/// A get request
#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone, Hash)]
pub struct GetRequest {
    /// blake3 hash
    pub hash: Hash,
    /// The range of data to request
    ///
    /// The first element is the parent, all subsequent elements are children.
    pub ranges: ChunkRangesSeq,
}

impl From<HashAndFormat> for GetRequest {
    fn from(value: HashAndFormat) -> Self {
        match value.format {
            BlobFormat::Raw => Self::blob(value.hash),
            BlobFormat::HashSeq => Self::all(value.hash),
        }
    }
}

impl GetRequest {
    pub fn builder() -> GetRequestBuilder {
        GetRequestBuilder::default()
    }

    pub fn content(&self) -> HashAndFormat {
        HashAndFormat {
            hash: self.hash,
            format: if self.ranges.is_blob() {
                BlobFormat::Raw
            } else {
                BlobFormat::HashSeq
            },
        }
    }

    /// Request a blob or collection with specified ranges
    pub fn new(hash: Hash, ranges: ChunkRangesSeq) -> Self {
        Self { hash, ranges }
    }

    /// Request a collection and all its children
    pub fn all(hash: impl Into<Hash>) -> Self {
        Self {
            hash: hash.into(),
            ranges: ChunkRangesSeq::all(),
        }
    }

    /// Request just a single blob
    pub fn blob(hash: impl Into<Hash>) -> Self {
        Self {
            hash: hash.into(),
            ranges: ChunkRangesSeq::from_ranges([ChunkRanges::all()]),
        }
    }

    /// Request ranges from a single blob
    pub fn blob_ranges(hash: Hash, ranges: ChunkRanges) -> Self {
        Self {
            hash,
            ranges: ChunkRangesSeq::from_ranges([ranges]),
        }
    }
}

/// A push request contains a description of what to push, but will be followed
/// by the data to push.
#[derive(
    Deserialize, Serialize, Debug, PartialEq, Eq, Clone, derive_more::From, derive_more::Deref,
)]
pub struct PushRequest(GetRequest);

impl PushRequest {
    pub fn new(hash: Hash, ranges: ChunkRangesSeq) -> Self {
        Self(GetRequest::new(hash, ranges))
    }
}

/// A GetMany request is a request to get multiple blobs via a single request.
///
/// It is identical to a [`GetRequest`] for a HashSeq, but the HashSeq is provided
/// by the requester.
#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone)]
pub struct GetManyRequest {
    /// The hashes of the blobs to get
    pub hashes: Vec<Hash>,
    /// The ranges of data to request
    ///
    /// There is no range request for the parent, since we just sent the hashes
    /// and therefore have the parent already.
    pub ranges: ChunkRangesSeq,
}

impl<I: Into<Hash>> FromIterator<I> for GetManyRequest {
    fn from_iter<T: IntoIterator<Item = I>>(iter: T) -> Self {
        let mut res = iter.into_iter().map(Into::into).collect::<Vec<Hash>>();
        res.sort();
        res.dedup();
        let n = res.len() as u64;
        Self {
            hashes: res,
            ranges: ChunkRangesSeq(smallvec::smallvec![
                (0, ChunkRanges::all()),
                (n, ChunkRanges::empty())
            ]),
        }
    }
}

impl GetManyRequest {
    pub fn new(hashes: Vec<Hash>, ranges: ChunkRangesSeq) -> Self {
        Self { hashes, ranges }
    }

    pub fn builder() -> builder::GetManyRequestBuilder {
        builder::GetManyRequestBuilder::default()
    }
}

/// A request to observe a raw blob bitfield.
#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone, Hash)]
pub struct ObserveRequest {
    /// blake3 hash
    pub hash: Hash,
    /// ranges to observe.
    pub ranges: RangeSpec,
}

impl ObserveRequest {
    pub fn new(hash: Hash) -> Self {
        Self {
            hash,
            ranges: RangeSpec::all(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq)]
pub struct ObserveItem {
    pub size: u64,
    pub ranges: ChunkRanges,
}

impl From<&Bitfield> for ObserveItem {
    fn from(value: &Bitfield) -> Self {
        Self {
            size: value.size,
            ranges: value.ranges.clone(),
        }
    }
}

impl From<&ObserveItem> for Bitfield {
    fn from(value: &ObserveItem) -> Self {
        Self {
            size: value.size,
            ranges: value.ranges.clone(),
        }
    }
}

/// Reasons to close connections or stop streams.
///
/// A QUIC **connection** can be *closed* and a **stream** can request the other side to
/// *stop* sending data.  Both closing and stopping have an associated `error_code`, closing
/// also adds a `reason` as some arbitrary bytes.
///
/// This enum exists so we have a single namespace for `error_code`s used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum Closed {
    /// The [`RecvStream`] was dropped.
    ///
    /// Used implicitly when a [`RecvStream`] is dropped without explicit call to
    /// [`RecvStream::stop`].  We don't use this explicitly but this is here as
    /// documentation as to what happened to `0`.
    ///
    /// [`RecvStream`]: iroh::endpoint::RecvStream
    /// [`RecvStream::stop`]: iroh::endpoint::RecvStream::stop
    StreamDropped = 0,
    /// The provider is terminating.
    ///
    /// When a provider terminates all connections and associated streams are closed.
    ProviderTerminating = 1,
    /// The provider has received the request.
    ///
    /// Only a single request is allowed on a stream, if more data is received after this a
    /// provider may send this error code in a STOP_STREAM frame.
    RequestReceived = 2,
}

impl Closed {
    /// The close reason as bytes. This is a valid utf8 string describing the reason.
    pub fn reason(&self) -> &'static [u8] {
        match self {
            Closed::StreamDropped => b"stream dropped",
            Closed::ProviderTerminating => b"provider terminating",
            Closed::RequestReceived => b"request received",
        }
    }
}

impl From<Closed> for VarInt {
    fn from(source: Closed) -> Self {
        VarInt::from(source as u16)
    }
}

/// Unknown error_code, can not be converted into [`Closed`].
#[stack_error(derive, add_meta)]
#[error("Unknown error_code: {code}")]
pub struct UnknownErrorCode {
    code: u64,
}

impl TryFrom<VarInt> for Closed {
    type Error = UnknownErrorCode;

    fn try_from(value: VarInt) -> std::result::Result<Self, Self::Error> {
        match value.into_inner() {
            0 => Ok(Self::StreamDropped),
            1 => Ok(Self::ProviderTerminating),
            2 => Ok(Self::RequestReceived),
            val => Err(n0_error::e!(UnknownErrorCode { code: val })),
        }
    }
}

pub trait ChunkRangesExt {
    fn last_chunk() -> Self;
    fn chunk(offset: u64) -> Self;
    fn bytes(ranges: impl RangeBounds<u64>) -> Self;
    fn chunks(ranges: impl RangeBounds<u64>) -> Self;
    fn offset(offset: u64) -> Self;
}

impl ChunkRangesExt for ChunkRanges {
    fn last_chunk() -> Self {
        ChunkRanges::from(ChunkNum(u64::MAX)..)
    }

    /// Create a chunk range that contains a single chunk.
    fn chunk(offset: u64) -> Self {
        ChunkRanges::from(ChunkNum(offset)..ChunkNum(offset + 1))
    }

    /// Create a range of chunks that contains the given byte ranges.
    /// The byte ranges are rounded up to the nearest chunk size.
    fn bytes(ranges: impl RangeBounds<u64>) -> Self {
        round_up_to_chunks(&bounds_from_range(ranges, |v| v))
    }

    /// Create a range of chunks from u64 chunk bounds.
    ///
    /// This is equivalent but more convenient than using the ChunkNum newtype.
    fn chunks(ranges: impl RangeBounds<u64>) -> Self {
        bounds_from_range(ranges, ChunkNum)
    }

    /// Create a chunk range that contains a single byte offset.
    fn offset(offset: u64) -> Self {
        Self::bytes(offset..offset + 1)
    }
}

// todo: move to range_collections
pub(crate) fn bounds_from_range<R, T, F>(range: R, f: F) -> RangeSet2<T>
where
    R: RangeBounds<u64>,
    T: RangeSetEntry,
    F: Fn(u64) -> T,
{
    let from = match range.start_bound() {
        Bound::Included(start) => Some(*start),
        Bound::Excluded(start) => {
            let Some(start) = start.checked_add(1) else {
                return RangeSet2::empty();
            };
            Some(start)
        }
        Bound::Unbounded => None,
    };
    let to = match range.end_bound() {
        Bound::Included(end) => end.checked_add(1),
        Bound::Excluded(end) => Some(*end),
        Bound::Unbounded => None,
    };
    match (from, to) {
        (Some(from), Some(to)) => RangeSet2::from(f(from)..f(to)),
        (Some(from), None) => RangeSet2::from(f(from)..),
        (None, Some(to)) => RangeSet2::from(..f(to)),
        (None, None) => RangeSet2::all(),
    }
}

pub mod builder {
    use std::collections::BTreeMap;

    use bao_tree::ChunkRanges;

    use super::ChunkRangesSeq;
    use crate::{
        protocol::{GetManyRequest, GetRequest},
        Hash,
    };

    #[derive(Debug, Clone, Default)]
    pub struct ChunkRangesSeqBuilder {
        ranges: BTreeMap<u64, ChunkRanges>,
    }

    #[derive(Debug, Clone, Default)]
    pub struct GetRequestBuilder {
        builder: ChunkRangesSeqBuilder,
    }

    impl GetRequestBuilder {
        /// Add a range to the request.
        pub fn offset(mut self, offset: u64, ranges: impl Into<ChunkRanges>) -> Self {
            self.builder = self.builder.offset(offset, ranges);
            self
        }

        /// Add a range to the request.
        pub fn child(mut self, child: u64, ranges: impl Into<ChunkRanges>) -> Self {
            self.builder = self.builder.offset(child + 1, ranges);
            self
        }

        /// Specify ranges for the root blob (the HashSeq)
        pub fn root(mut self, ranges: impl Into<ChunkRanges>) -> Self {
            self.builder = self.builder.offset(0, ranges);
            self
        }

        /// Specify ranges for the next offset.
        pub fn next(mut self, ranges: impl Into<ChunkRanges>) -> Self {
            self.builder = self.builder.next(ranges);
            self
        }

        /// Build a get request for the given hash, with the ranges specified in the builder.
        pub fn build(self, hash: impl Into<Hash>) -> GetRequest {
            let ranges = self.builder.build();
            GetRequest::new(hash.into(), ranges)
        }

        /// Build a get request for the given hash, with the ranges specified in the builder
        /// and the last non-empty range repeating indefinitely.
        pub fn build_open(self, hash: impl Into<Hash>) -> GetRequest {
            let ranges = self.builder.build_open();
            GetRequest::new(hash.into(), ranges)
        }
    }

    impl ChunkRangesSeqBuilder {
        /// Add a range to the request.
        pub fn offset(self, offset: u64, ranges: impl Into<ChunkRanges>) -> Self {
            self.at_offset(offset, ranges.into())
        }

        /// Specify ranges for the next offset.
        pub fn next(self, ranges: impl Into<ChunkRanges>) -> Self {
            let offset = self.next_offset_value();
            self.at_offset(offset, ranges.into())
        }

        /// Build a get request for the given hash, with the ranges specified in the builder.
        pub fn build(self) -> ChunkRangesSeq {
            ChunkRangesSeq::from_ranges(self.build0())
        }

        /// Build a get request for the given hash, with the ranges specified in the builder
        /// and the last non-empty range repeating indefinitely.
        pub fn build_open(self) -> ChunkRangesSeq {
            ChunkRangesSeq::from_ranges_infinite(self.build0())
        }

        /// Add ranges at the given offset.
        fn at_offset(mut self, offset: u64, ranges: ChunkRanges) -> Self {
            self.ranges
                .entry(offset)
                .and_modify(|v| *v |= ranges.clone())
                .or_insert(ranges);
            self
        }

        /// Build the request.
        fn build0(mut self) -> impl Iterator<Item = ChunkRanges> {
            let mut ranges = Vec::new();
            self.ranges.retain(|_, v| !v.is_empty());
            let until_key = self.next_offset_value();
            for offset in 0..until_key {
                ranges.push(self.ranges.remove(&offset).unwrap_or_default());
            }
            ranges.into_iter()
        }

        /// Get the next offset value.
        fn next_offset_value(&self) -> u64 {
            self.ranges
                .last_key_value()
                .map(|(k, _)| *k + 1)
                .unwrap_or_default()
        }
    }

    #[derive(Debug, Clone, Default)]
    pub struct GetManyRequestBuilder {
        ranges: BTreeMap<Hash, ChunkRanges>,
    }

    impl GetManyRequestBuilder {
        /// Specify ranges for the given hash.
        ///
        /// Note that if you specify a hash that is already in the request, the ranges will be
        /// merged with the existing ranges.
        pub fn hash(mut self, hash: impl Into<Hash>, ranges: impl Into<ChunkRanges>) -> Self {
            let ranges = ranges.into();
            let hash = hash.into();
            self.ranges
                .entry(hash)
                .and_modify(|v| *v |= ranges.clone())
                .or_insert(ranges);
            self
        }

        /// Build a `GetManyRequest`.
        pub fn build(self) -> GetManyRequest {
            let (hashes, ranges): (Vec<Hash>, Vec<ChunkRanges>) = self
                .ranges
                .into_iter()
                .filter(|(_, v)| !v.is_empty())
                .unzip();
            let ranges = ChunkRangesSeq::from_ranges(ranges);
            GetManyRequest { hashes, ranges }
        }
    }

    #[cfg(test)]
    mod tests {
        use bao_tree::ChunkNum;

        use super::*;
        use crate::protocol::{ChunkRangesExt, GetManyRequest};

        #[test]
        fn chunk_ranges_ext() {
            let ranges = ChunkRanges::bytes(1..2)
                | ChunkRanges::chunks(100..=200)
                | ChunkRanges::offset(1024 * 10)
                | ChunkRanges::chunk(1024)
                | ChunkRanges::last_chunk();
            assert_eq!(
                ranges,
                ChunkRanges::from(ChunkNum(0)..ChunkNum(1)) // byte range 1..2
                    | ChunkRanges::from(ChunkNum(10)..ChunkNum(11)) // chunk at byte offset 1024*10
                    | ChunkRanges::from(ChunkNum(100)..ChunkNum(201)) // chunk range 100..=200
                    | ChunkRanges::from(ChunkNum(1024)..ChunkNum(1025)) // chunk 1024
                    | ChunkRanges::last_chunk() // last chunk
            );
        }

        #[test]
        fn get_request_builder() {
            let hash = [0; 32];
            let request = GetRequest::builder()
                .root(ChunkRanges::all())
                .next(ChunkRanges::all())
                .next(ChunkRanges::bytes(0..100))
                .build(hash);
            assert_eq!(request.hash.as_bytes(), &hash);
            assert_eq!(
                request.ranges,
                ChunkRangesSeq::from_ranges([
                    ChunkRanges::all(),
                    ChunkRanges::all(),
                    ChunkRanges::from(..ChunkNum(1)),
                ])
            );

            let request = GetRequest::builder()
                .root(ChunkRanges::all())
                .child(2, ChunkRanges::bytes(0..100))
                .build(hash);
            assert_eq!(request.hash.as_bytes(), &hash);
            assert_eq!(
                request.ranges,
                ChunkRangesSeq::from_ranges([
                    ChunkRanges::all(),               // root
                    ChunkRanges::empty(),             // child 0
                    ChunkRanges::empty(),             // child 1
                    ChunkRanges::from(..ChunkNum(1))  // child 2,
                ])
            );

            let request = GetRequest::builder()
                .root(ChunkRanges::all())
                .next(ChunkRanges::bytes(0..1024) | ChunkRanges::last_chunk())
                .build_open(hash);
            assert_eq!(request.hash.as_bytes(), &[0; 32]);
            assert_eq!(
                request.ranges,
                ChunkRangesSeq::from_ranges_infinite([
                    ChunkRanges::all(),
                    ChunkRanges::from(..ChunkNum(1)) | ChunkRanges::last_chunk(),
                ])
            );
        }

        #[test]
        fn get_many_request_builder() {
            let hash1 = [0; 32];
            let hash2 = [1; 32];
            let hash3 = [2; 32];
            let request = GetManyRequest::builder()
                .hash(hash1, ChunkRanges::all())
                .hash(hash2, ChunkRanges::empty()) // will be ignored!
                .hash(hash3, ChunkRanges::bytes(0..100))
                .build();
            assert_eq!(
                request.hashes,
                vec![Hash::from([0; 32]), Hash::from([2; 32])]
            );
            assert_eq!(
                request.ranges,
                ChunkRangesSeq::from_ranges([
                    ChunkRanges::all(),               // hash 0
                    ChunkRanges::from(..ChunkNum(1)), // hash 2
                ])
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use iroh_test::{assert_eq_hex, hexdump::parse_hexdump};
    use postcard::experimental::max_size::MaxSize;

    use super::{GetRequest, Request, RequestType};
    use crate::Hash;

    #[test]
    fn request_wire_format() {
        let hash: Hash = [0xda; 32].into();
        let cases = [
            (
                Request::from(GetRequest::blob(hash)),
                r"
                    00 # enum variant for GetRequest
                    dadadadadadadadadadadadadadadadadadadadadadadadadadadadadadadada # the hash
                    020001000100 # the ChunkRangesSeq
            ",
            ),
            (
                Request::from(GetRequest::all(hash)),
                r"
                    00 # enum variant for GetRequest
                    dadadadadadadadadadadadadadadadadadadadadadadadadadadadadadadada # the hash
                    01000100 # the ChunkRangesSeq
            ",
            ),
        ];
        for (case, expected_hex) in cases {
            let expected = parse_hexdump(expected_hex).unwrap();
            let bytes = postcard::to_stdvec(&case).unwrap();
            assert_eq_hex!(bytes, expected);
        }
    }

    #[test]
    fn request_type_size() {
        assert_eq!(RequestType::POSTCARD_MAX_SIZE, 1);
    }
}
