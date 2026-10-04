//! Serialize and decerealize Krpc messages.

// Copied from <https://github.com/raptorswing/rustydht-lib/blob/main/src/packets/public.rs>

#![allow(missing_docs)]

mod internal;

use std::convert::TryInto;
use std::net::{Ipv4Addr, SocketAddrV4};

use crate::common::{ID_SIZE, Id, Node};

use super::InvalidIdSize;

#[derive(Debug, PartialEq, Clone)]
pub(crate) struct Message {
    pub transaction_id: u32,

    /// The version of the requester or responder.
    pub version: Option<[u8; 4]>,

    /// The IP address and port ("SocketAddr") of the requester as seen from the responder's point of view.
    /// This should be set only on response, but is defined at this level with the other common fields to avoid defining yet another layer on the response objects.
    pub requester_ip: Option<SocketAddrV4>,

    pub message_type: MessageType,

    /// For bep0043. When set true on a request, indicates that the requester can't reply to requests and that responders should not add requester to their routing tables.
    /// Should only be set on requests - undefined behavior when set on a response.
    pub read_only: bool,
}

#[derive(Debug, PartialEq, Clone)]
pub enum MessageType {
    Request(RequestSpecific),

    Response(ResponseSpecific),

    Error(ErrorSpecific),
}

#[derive(Debug, PartialEq, Clone)]
pub struct ErrorSpecific {
    pub code: i32,
    pub description: String,
}

/// An incoming KRPC request, as observed by the local node.
///
/// Mostly useful as the input to a custom [`RequestFilter`](crate::RequestFilter)
/// when the node is running in server mode.
#[derive(Debug, PartialEq, Clone)]
pub struct RequestSpecific {
    /// The [`Id`] claimed by the node sending the request.
    pub requester_id: Id,
    /// The body of the request, indicating which operation the sender wants performed.
    pub request_type: RequestTypeSpecific,
}

/// The kind of KRPC request, with operation-specific arguments.
#[derive(Debug, PartialEq, Clone)]
pub enum RequestTypeSpecific {
    /// Liveness check; expects no payload back.
    Ping,
    /// Find the closest known nodes to a target [`Id`].
    FindNode(FindNodeRequestArguments),
    /// Look up announced peers for an info hash ([BEP_0005](https://www.bittorrent.org/beps/bep_0005.html)).
    GetPeers(GetPeersRequestArguments),
    /// Look up signed peers for an info hash (proposed signed-peers BEP).
    GetSignedPeers(GetPeersRequestArguments),
    /// Get a value (immutable or mutable) by target ([BEP_0044](https://www.bittorrent.org/beps/bep_0044.html)).
    GetValue(GetValueRequestArguments),

    /// A storage request — see [`PutRequestSpecific`] for the variants.
    Put(PutRequest),
}

/// A storage request paired with the write token obtained from a previous lookup.
#[derive(Debug, PartialEq, Clone)]
pub struct PutRequest {
    /// Opaque write token previously handed out by the storing node.
    pub token: Box<[u8]>,
    /// The specific kind of write being performed.
    pub put_request_type: PutRequestSpecific,
}

/// The kind of storage request, with operation-specific arguments.
///
/// This is the parameter passed to [`Dht::put`](crate::Dht::put) for low-level
/// access. The high-level wrappers ([`Dht::put_immutable`](crate::Dht::put_immutable),
/// [`Dht::put_mutable`](crate::Dht::put_mutable),
/// [`Dht::announce_peer`](crate::Dht::announce_peer)) construct one of these
/// internally.
#[derive(Debug, PartialEq, Clone)]
pub enum PutRequestSpecific {
    /// Announce that the sender is a peer for an info hash ([BEP_0005](https://www.bittorrent.org/beps/bep_0005.html)).
    AnnouncePeer(AnnouncePeerRequestArguments),
    /// Announce a signed peer for an info hash (proposed signed-peers BEP).
    AnnounceSignedPeer(AnnounceSignedPeerRequestArguments),
    /// Store an immutable value, addressed by its sha1 hash ([BEP_0044](https://www.bittorrent.org/beps/bep_0044.html)).
    PutImmutable(PutImmutableRequestArguments),
    /// Store a mutable value, addressed by an ed25519 public key (and optional salt) ([BEP_0044](https://www.bittorrent.org/beps/bep_0044.html)).
    PutMutable(PutMutableRequestArguments),
}

impl PutRequestSpecific {
    /// The target [`Id`] this storage request will be routed to.
    ///
    /// For peer announcements this is the info hash; for value puts it's the
    /// content-addressed (immutable) or key-derived (mutable) target.
    pub fn target(&self) -> &Id {
        match self {
            PutRequestSpecific::AnnouncePeer(AnnouncePeerRequestArguments {
                info_hash, ..
            }) => info_hash,
            PutRequestSpecific::AnnounceSignedPeer(AnnounceSignedPeerRequestArguments {
                info_hash,
                ..
            }) => info_hash,
            PutRequestSpecific::PutMutable(PutMutableRequestArguments { target, .. }) => target,
            PutRequestSpecific::PutImmutable(PutImmutableRequestArguments { target, .. }) => target,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum ResponseSpecific {
    Ping(PingResponseArguments),
    FindNode(FindNodeResponseArguments),
    GetPeers(GetPeersResponseArguments),
    GetSignedPeers(GetSignedPeersResponseArguments),
    GetImmutable(GetImmutableResponseArguments),
    GetMutable(GetMutableResponseArguments),
    NoValues(NoValuesResponseArguments),
    NoMoreRecentValue(NoMoreRecentValueResponseArguments),
}

// === PING ===
#[derive(Debug, PartialEq, Clone)]
pub struct PingResponseArguments {
    pub responder_id: Id,
}

// === FIND_NODE ===
#[derive(Debug, PartialEq, Clone)]
pub struct FindNodeRequestArguments {
    pub target: Id,
}

#[derive(Debug, PartialEq, Clone)]
pub struct FindNodeResponseArguments {
    pub responder_id: Id,
    pub nodes: Box<[Node]>,
}

// Get anything

#[derive(Debug, PartialEq, Clone)]
pub struct GetValueRequestArguments {
    pub target: Id,
    pub seq: Option<i64>,
    // A bit of a hack, using this to carry an optional
    // salt in the query.request field of [crate::query]
    // not really encoded, decoded or sent over the wire.
    pub salt: Option<Box<[u8]>>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct NoValuesResponseArguments {
    pub responder_id: Id,
    pub token: Box<[u8]>,
    pub nodes: Option<Box<[Node]>>,
}

// === Get Peers ===

#[derive(Debug, PartialEq, Clone)]
pub struct GetPeersRequestArguments {
    pub info_hash: Id,
}

#[derive(Debug, PartialEq, Clone)]
pub struct GetPeersResponseArguments {
    pub responder_id: Id,
    pub token: Box<[u8]>,
    pub values: Vec<SocketAddrV4>,
    pub nodes: Option<Box<[Node]>>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct GetSignedPeersResponseArguments {
    pub responder_id: Id,
    pub token: Box<[u8]>,
    pub peers: Vec<([u8; 32], u64, [u8; 64])>,
    pub nodes: Option<Box<[Node]>>,
}

// === Announce Peer ===

#[derive(Debug, PartialEq, Clone)]
pub struct AnnouncePeerRequestArguments {
    pub info_hash: Id,
    pub port: u16,
    pub implied_port: Option<bool>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct AnnounceSignedPeerRequestArguments {
    pub info_hash: Id,
    pub t: u64,
    pub k: [u8; 32],
    pub sig: [u8; 64],
}

// === Get Immutable ===

#[derive(Debug, PartialEq, Clone)]
pub struct GetImmutableResponseArguments {
    pub responder_id: Id,
    pub token: Box<[u8]>,
    pub nodes: Option<Box<[Node]>>,
    pub v: Box<[u8]>,
}

// === Get Mutable ===

#[derive(Debug, PartialEq, Clone)]
pub struct GetMutableResponseArguments {
    pub responder_id: Id,
    pub token: Box<[u8]>,
    pub nodes: Option<Box<[Node]>>,
    pub v: Box<[u8]>,
    pub k: [u8; 32],
    pub seq: i64,
    pub sig: [u8; 64],
}

#[derive(Debug, PartialEq, Clone)]
pub struct NoMoreRecentValueResponseArguments {
    pub responder_id: Id,
    pub token: Box<[u8]>,
    pub nodes: Option<Box<[Node]>>,
    pub seq: i64,
}

// === Put Immutable ===

#[derive(Debug, PartialEq, Clone)]
pub struct PutImmutableRequestArguments {
    pub target: Id,
    pub v: Box<[u8]>,
}

// === Put Mutable ===

#[derive(Debug, PartialEq, Clone)]
pub struct PutMutableRequestArguments {
    pub target: Id,
    pub v: Box<[u8]>,
    pub k: [u8; 32],
    pub seq: i64,
    pub sig: [u8; 64],
    pub salt: Option<Box<[u8]>>,
    pub cas: Option<i64>,
}

impl Message {
    fn into_serde_message(self) -> internal::DHTMessage {
        internal::DHTMessage {
            transaction_id: self.transaction_id.to_be_bytes().to_vec(),
            version: self.version,
            ip: self
                .requester_ip
                .map(|sockaddr| sockaddr_to_bytes(&sockaddr)),
            read_only: if self.read_only { Some(1) } else { Some(0) },
            variant: match self.message_type {
                MessageType::Request(RequestSpecific {
                    requester_id,
                    request_type,
                }) => internal::DHTMessageVariant::Request(match request_type {
                    RequestTypeSpecific::Ping => internal::DHTRequestSpecific::Ping {
                        arguments: internal::DHTPingRequestArguments {
                            id: requester_id.into(),
                        },
                    },
                    RequestTypeSpecific::FindNode(find_node_args) => {
                        internal::DHTRequestSpecific::FindNode {
                            arguments: internal::DHTFindNodeRequestArguments {
                                id: requester_id.into(),
                                target: find_node_args.target.into(),
                            },
                        }
                    }
                    RequestTypeSpecific::GetPeers(get_peers_args) => {
                        internal::DHTRequestSpecific::GetPeers {
                            arguments: internal::DHTGetPeersRequestArguments {
                                id: requester_id.into(),
                                info_hash: get_peers_args.info_hash.into(),
                            },
                        }
                    }
                    RequestTypeSpecific::GetSignedPeers(get_peers_args) => {
                        internal::DHTRequestSpecific::GetSignedPeers {
                            arguments: internal::DHTGetPeersRequestArguments {
                                id: requester_id.into(),
                                info_hash: get_peers_args.info_hash.into(),
                            },
                        }
                    }
                    RequestTypeSpecific::GetValue(get_mutable_args) => {
                        internal::DHTRequestSpecific::GetValue {
                            arguments: internal::DHTGetValueRequestArguments {
                                id: requester_id.into(),
                                target: get_mutable_args.target.into(),
                                seq: get_mutable_args.seq,
                            },
                        }
                    }
                    RequestTypeSpecific::Put(PutRequest {
                        token,
                        put_request_type,
                    }) => match put_request_type {
                        PutRequestSpecific::AnnouncePeer(announce_peer_args) => {
                            internal::DHTRequestSpecific::AnnouncePeer {
                                arguments: internal::DHTAnnouncePeerRequestArguments {
                                    id: requester_id.into(),
                                    token,

                                    info_hash: announce_peer_args.info_hash.into(),
                                    port: announce_peer_args.port,
                                    implied_port: if announce_peer_args.implied_port.is_some() {
                                        Some(1)
                                    } else {
                                        Some(0)
                                    },
                                },
                            }
                        }
                        PutRequestSpecific::AnnounceSignedPeer(announce_signed_peer_args) => {
                            internal::DHTRequestSpecific::AnnounceSignedPeer {
                                arguments: internal::DHTAnnounceSignedPeerRequestArguments {
                                    id: requester_id.into(),
                                    token,

                                    info_hash: announce_signed_peer_args.info_hash.into(),
                                    t: announce_signed_peer_args.t as i64,
                                    k: announce_signed_peer_args.k,
                                    sig: announce_signed_peer_args.sig,
                                },
                            }
                        }
                        PutRequestSpecific::PutImmutable(put_immutable_arguments) => {
                            internal::DHTRequestSpecific::PutValue {
                                arguments: internal::DHTPutValueRequestArguments {
                                    id: requester_id.into(),
                                    token,

                                    target: put_immutable_arguments.target.into(),
                                    v: put_immutable_arguments.v,
                                    k: None,
                                    seq: None,
                                    sig: None,
                                    salt: None,
                                    cas: None,
                                },
                            }
                        }
                        PutRequestSpecific::PutMutable(put_mutable_arguments) => {
                            internal::DHTRequestSpecific::PutValue {
                                arguments: internal::DHTPutValueRequestArguments {
                                    id: requester_id.into(),
                                    token,

                                    target: put_mutable_arguments.target.into(),
                                    v: put_mutable_arguments.v,
                                    k: Some(put_mutable_arguments.k),
                                    seq: Some(put_mutable_arguments.seq),
                                    sig: Some(put_mutable_arguments.sig),
                                    salt: put_mutable_arguments.salt,
                                    cas: put_mutable_arguments.cas,
                                },
                            }
                        }
                    },
                }),

                MessageType::Response(res) => internal::DHTMessageVariant::Response(match res {
                    ResponseSpecific::Ping(ping_args) => internal::DHTResponseSpecific::Ping {
                        arguments: internal::DHTPingResponseArguments {
                            id: ping_args.responder_id.into(),
                        },
                    },
                    ResponseSpecific::FindNode(find_node_args) => {
                        internal::DHTResponseSpecific::FindNode {
                            arguments: internal::DHTFindNodeResponseArguments {
                                id: find_node_args.responder_id.into(),
                                nodes: nodes4_to_bytes(&find_node_args.nodes),
                            },
                        }
                    }
                    ResponseSpecific::GetPeers(get_peers_args) => {
                        internal::DHTResponseSpecific::GetPeers {
                            arguments: internal::DHTGetPeersResponseArguments {
                                id: get_peers_args.responder_id.into(),
                                token: get_peers_args.token,
                                nodes: get_peers_args
                                    .nodes
                                    .as_ref()
                                    .map(|nodes| nodes4_to_bytes(nodes)),
                                values: peers_to_bytes(&get_peers_args.values),
                            },
                        }
                    }
                    ResponseSpecific::GetSignedPeers(get_peers_args) => {
                        internal::DHTResponseSpecific::GetSignedPeers {
                            arguments: internal::DHTGetSignedPeersResponseArguments {
                                id: get_peers_args.responder_id.into(),
                                token: get_peers_args.token,
                                nodes: get_peers_args
                                    .nodes
                                    .as_ref()
                                    .map(|nodes| nodes4_to_bytes(nodes)),
                                peers: signed_peers_to_bytes(&get_peers_args.peers),
                            },
                        }
                    }
                    ResponseSpecific::NoValues(no_values_arguments) => {
                        internal::DHTResponseSpecific::NoValues {
                            arguments: internal::DHTNoValuesResponseArguments {
                                id: no_values_arguments.responder_id.into(),
                                token: no_values_arguments.token,
                                nodes: no_values_arguments
                                    .nodes
                                    .as_ref()
                                    .map(|nodes| nodes4_to_bytes(nodes)),
                            },
                        }
                    }
                    ResponseSpecific::GetImmutable(get_immutable_args) => {
                        internal::DHTResponseSpecific::GetImmutable {
                            arguments: internal::DHTGetImmutableResponseArguments {
                                id: get_immutable_args.responder_id.into(),
                                token: get_immutable_args.token,
                                nodes: get_immutable_args
                                    .nodes
                                    .as_ref()
                                    .map(|nodes| nodes4_to_bytes(nodes)),
                                v: get_immutable_args.v,
                            },
                        }
                    }
                    ResponseSpecific::GetMutable(get_mutable_args) => {
                        internal::DHTResponseSpecific::GetMutable {
                            arguments: internal::DHTGetMutableResponseArguments {
                                id: get_mutable_args.responder_id.into(),
                                token: get_mutable_args.token,
                                nodes: get_mutable_args
                                    .nodes
                                    .as_ref()
                                    .map(|nodes| nodes4_to_bytes(nodes)),
                                v: get_mutable_args.v,
                                k: get_mutable_args.k,
                                seq: get_mutable_args.seq,
                                sig: get_mutable_args.sig,
                            },
                        }
                    }
                    ResponseSpecific::NoMoreRecentValue(args) => {
                        internal::DHTResponseSpecific::NoMoreRecentValue {
                            arguments: internal::DHTNoMoreRecentValueResponseArguments {
                                id: args.responder_id.into(),
                                token: args.token,
                                nodes: args.nodes.as_ref().map(|nodes| nodes4_to_bytes(nodes)),
                                seq: args.seq,
                            },
                        }
                    }
                }),

                MessageType::Error(err) => {
                    internal::DHTMessageVariant::Error(internal::DHTErrorSpecific {
                        error_info: (err.code, err.description),
                    })
                }
            },
        }
    }

    fn from_serde_message(msg: internal::DHTMessage) -> Result<Message, DecodeMessageError> {
        Ok(Message {
            transaction_id: match *msg.transaction_id.as_slice() {
                [a, b] => u16::from_be_bytes([a, b]) as u32,
                [a, b, c, d] => u32::from_be_bytes([a, b, c, d]),
                _ => return Err(DecodeMessageError::InvalidTransactionIdSize),
            },
            version: msg.version,
            requester_ip: match msg.ip {
                Some(ip) => Some(bytes_to_sockaddr(ip)?),
                _ => None,
            },
            read_only: if let Some(read_only) = msg.read_only {
                read_only > 0
            } else {
                false
            },
            message_type: match msg.variant {
                internal::DHTMessageVariant::Request(req_variant) => {
                    MessageType::Request(match req_variant {
                        internal::DHTRequestSpecific::Ping { arguments } => RequestSpecific {
                            requester_id: Id::from_bytes(arguments.id)?,
                            request_type: RequestTypeSpecific::Ping,
                        },
                        internal::DHTRequestSpecific::FindNode { arguments } => RequestSpecific {
                            requester_id: Id::from_bytes(arguments.id)?,
                            request_type: RequestTypeSpecific::FindNode(FindNodeRequestArguments {
                                target: Id::from_bytes(arguments.target)?,
                            }),
                        },
                        internal::DHTRequestSpecific::GetPeers { arguments } => RequestSpecific {
                            requester_id: Id::from_bytes(arguments.id)?,
                            request_type: RequestTypeSpecific::GetPeers(GetPeersRequestArguments {
                                info_hash: Id::from_bytes(arguments.info_hash)?,
                            }),
                        },
                        internal::DHTRequestSpecific::GetSignedPeers { arguments } => {
                            RequestSpecific {
                                requester_id: Id::from_bytes(arguments.id)?,
                                request_type: RequestTypeSpecific::GetSignedPeers(
                                    GetPeersRequestArguments {
                                        info_hash: Id::from_bytes(arguments.info_hash)?,
                                    },
                                ),
                            }
                        }

                        internal::DHTRequestSpecific::GetValue { arguments } => RequestSpecific {
                            requester_id: Id::from_bytes(arguments.id)?,

                            request_type: RequestTypeSpecific::GetValue(GetValueRequestArguments {
                                target: Id::from_bytes(arguments.target)?,
                                seq: arguments.seq,
                                salt: None,
                            }),
                        },
                        internal::DHTRequestSpecific::AnnouncePeer { arguments } => {
                            RequestSpecific {
                                requester_id: Id::from_bytes(arguments.id)?,
                                request_type: RequestTypeSpecific::Put(PutRequest {
                                    token: arguments.token,
                                    put_request_type: PutRequestSpecific::AnnouncePeer(
                                        AnnouncePeerRequestArguments {
                                            implied_port: arguments
                                                .implied_port
                                                .map(|implied_port| implied_port != 0),
                                            info_hash: arguments.info_hash.into(),
                                            port: arguments.port,
                                        },
                                    ),
                                }),
                            }
                        }
                        internal::DHTRequestSpecific::AnnounceSignedPeer { arguments } => {
                            RequestSpecific {
                                requester_id: Id::from_bytes(arguments.id)?,

                                request_type: RequestTypeSpecific::Put(PutRequest {
                                    token: arguments.token,
                                    put_request_type: PutRequestSpecific::AnnounceSignedPeer(
                                        AnnounceSignedPeerRequestArguments {
                                            info_hash: Id::from_bytes(arguments.info_hash)?,
                                            t: arguments.t as u64,
                                            k: arguments.k,
                                            sig: arguments.sig,
                                        },
                                    ),
                                }),
                            }
                        }
                        internal::DHTRequestSpecific::PutValue { arguments } => {
                            if let Some(k) = arguments.k {
                                RequestSpecific {
                                    requester_id: Id::from_bytes(arguments.id)?,

                                    request_type: RequestTypeSpecific::Put(PutRequest {
                                        token: arguments.token,
                                        put_request_type: PutRequestSpecific::PutMutable(
                                            PutMutableRequestArguments {
                                                target: Id::from_bytes(arguments.target)?,
                                                v: arguments.v,
                                                k,
                                                seq: arguments.seq.expect(
                                                    "Put mutable message to have sequence number",
                                                ),
                                                sig: arguments.sig.expect(
                                                    "Put mutable message to have a signature",
                                                ),
                                                salt: arguments.salt,
                                                cas: arguments.cas,
                                            },
                                        ),
                                    }),
                                }
                            } else {
                                RequestSpecific {
                                    requester_id: Id::from_bytes(arguments.id)?,

                                    request_type: RequestTypeSpecific::Put(PutRequest {
                                        token: arguments.token,
                                        put_request_type: PutRequestSpecific::PutImmutable(
                                            PutImmutableRequestArguments {
                                                target: Id::from_bytes(arguments.target)?,
                                                v: arguments.v,
                                            },
                                        ),
                                    }),
                                }
                            }
                        }
                    })
                }

                internal::DHTMessageVariant::Response(res_variant) => {
                    MessageType::Response(match res_variant {
                        internal::DHTResponseSpecific::Ping { arguments } => {
                            ResponseSpecific::Ping(PingResponseArguments {
                                responder_id: Id::from_bytes(arguments.id)?,
                            })
                        }
                        internal::DHTResponseSpecific::FindNode { arguments } => {
                            ResponseSpecific::FindNode(FindNodeResponseArguments {
                                responder_id: Id::from_bytes(arguments.id)?,
                                nodes: bytes_to_nodes4(&arguments.nodes)?,
                            })
                        }
                        internal::DHTResponseSpecific::GetPeers { arguments } => {
                            ResponseSpecific::GetPeers(GetPeersResponseArguments {
                                responder_id: Id::from_bytes(arguments.id)?,
                                token: arguments.token,
                                nodes: match arguments.nodes {
                                    Some(nodes) => Some(bytes_to_nodes4(nodes)?),
                                    None => None,
                                },
                                values: bytes_to_peers(arguments.values)?,
                            })
                        }
                        internal::DHTResponseSpecific::GetSignedPeers { arguments } => {
                            ResponseSpecific::GetSignedPeers(GetSignedPeersResponseArguments {
                                responder_id: Id::from_bytes(arguments.id)?,
                                token: arguments.token,
                                nodes: match arguments.nodes {
                                    Some(nodes) => Some(bytes_to_nodes4(nodes)?),
                                    None => None,
                                },
                                peers: bytes_to_signed_peers(arguments.peers)?,
                            })
                        }
                        internal::DHTResponseSpecific::NoValues { arguments } => {
                            ResponseSpecific::NoValues(NoValuesResponseArguments {
                                responder_id: Id::from_bytes(arguments.id)?,
                                token: arguments.token,
                                nodes: match arguments.nodes {
                                    Some(nodes) => Some(bytes_to_nodes4(nodes)?),
                                    None => None,
                                },
                            })
                        }
                        internal::DHTResponseSpecific::GetImmutable { arguments } => {
                            ResponseSpecific::GetImmutable(GetImmutableResponseArguments {
                                responder_id: Id::from_bytes(arguments.id)?,
                                token: arguments.token,
                                nodes: match arguments.nodes {
                                    Some(nodes) => Some(bytes_to_nodes4(nodes)?),
                                    None => None,
                                },
                                v: arguments.v,
                            })
                        }
                        internal::DHTResponseSpecific::GetMutable { arguments } => {
                            ResponseSpecific::GetMutable(GetMutableResponseArguments {
                                responder_id: Id::from_bytes(arguments.id)?,
                                token: arguments.token,
                                nodes: match arguments.nodes {
                                    Some(nodes) => Some(bytes_to_nodes4(nodes)?),
                                    None => None,
                                },
                                v: arguments.v,
                                k: arguments.k,
                                seq: arguments.seq,
                                sig: arguments.sig,
                            })
                        }
                        internal::DHTResponseSpecific::NoMoreRecentValue { arguments } => {
                            ResponseSpecific::NoMoreRecentValue(
                                NoMoreRecentValueResponseArguments {
                                    responder_id: Id::from_bytes(arguments.id)?,
                                    token: arguments.token,
                                    nodes: match arguments.nodes {
                                        Some(nodes) => Some(bytes_to_nodes4(nodes)?),
                                        None => None,
                                    },
                                    seq: arguments.seq,
                                },
                            )
                        }
                    })
                }

                internal::DHTMessageVariant::Error(err) => MessageType::Error(ErrorSpecific {
                    code: err.error_info.0,
                    description: err.error_info.1,
                }),
            },
        })
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, serde_bencode::Error> {
        self.clone().into_serde_message().to_bytes()
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Message, DecodeMessageError> {
        if bytes.len() < 15 {
            return Err(DecodeMessageError::TooShort);
        } else if bytes[0] != 100 {
            return Err(DecodeMessageError::NotBencodeDictionary);
        }

        Message::from_serde_message(internal::DHTMessage::from_bytes(bytes)?)
    }

    /// Return the Id of the sender of the Message
    ///
    /// This is less straightforward than it seems because not *all* messages are sent
    /// with an Id (all are except Error messages). This is reflected in the structure
    /// of DHT Messages, and makes it a bit annoying to learn the sender's Id without
    /// unraveling the entire message. This method is a convenience method to extract
    /// the sender (or "author") Id from the guts of any Message.
    pub fn get_author_id(&self) -> Option<Id> {
        let id = match &self.message_type {
            MessageType::Request(arguments) => arguments.requester_id,
            MessageType::Response(response_variant) => match response_variant {
                ResponseSpecific::Ping(arguments) => arguments.responder_id,
                ResponseSpecific::FindNode(arguments) => arguments.responder_id,
                ResponseSpecific::GetPeers(arguments) => arguments.responder_id,
                ResponseSpecific::GetSignedPeers(arguments) => arguments.responder_id,
                ResponseSpecific::GetImmutable(arguments) => arguments.responder_id,
                ResponseSpecific::GetMutable(arguments) => arguments.responder_id,
                ResponseSpecific::NoValues(arguments) => arguments.responder_id,
                ResponseSpecific::NoMoreRecentValue(arguments) => arguments.responder_id,
            },
            MessageType::Error(_) => {
                return None;
            }
        };

        Some(id)
    }

    /// If the response contains a closer nodes to the target, return that!
    pub fn get_closer_nodes(&self) -> Option<&[Node]> {
        match &self.message_type {
            MessageType::Response(response_variant) => match response_variant {
                ResponseSpecific::Ping(_) => None,
                ResponseSpecific::FindNode(arguments) => Some(&arguments.nodes),
                ResponseSpecific::GetPeers(arguments) => arguments.nodes.as_deref(),
                ResponseSpecific::GetSignedPeers(arguments) => arguments.nodes.as_deref(),
                ResponseSpecific::GetMutable(arguments) => arguments.nodes.as_deref(),
                ResponseSpecific::GetImmutable(arguments) => arguments.nodes.as_deref(),
                ResponseSpecific::NoValues(arguments) => arguments.nodes.as_deref(),
                ResponseSpecific::NoMoreRecentValue(arguments) => arguments.nodes.as_deref(),
            },
            _ => None,
        }
    }

    pub fn get_token(&self) -> Option<(Id, &[u8])> {
        match &self.message_type {
            MessageType::Response(response_variant) => match response_variant {
                ResponseSpecific::Ping(_) => None,
                ResponseSpecific::FindNode(_) => None,
                ResponseSpecific::GetPeers(arguments) => {
                    Some((arguments.responder_id, &arguments.token))
                }
                ResponseSpecific::GetSignedPeers(arguments) => {
                    Some((arguments.responder_id, &arguments.token))
                }
                ResponseSpecific::GetImmutable(arguments) => {
                    Some((arguments.responder_id, &arguments.token))
                }
                ResponseSpecific::GetMutable(arguments) => {
                    Some((arguments.responder_id, &arguments.token))
                }
                ResponseSpecific::NoValues(arguments) => {
                    Some((arguments.responder_id, &arguments.token))
                }
                ResponseSpecific::NoMoreRecentValue(arguments) => {
                    Some((arguments.responder_id, &arguments.token))
                }
            },
            _ => None,
        }
    }
}

fn bytes_to_sockaddr<T: AsRef<[u8]>>(bytes: T) -> Result<SocketAddrV4, DecodeMessageError> {
    let bytes = bytes.as_ref();
    match bytes.len() {
        6 => {
            let ip = Ipv4Addr::new(bytes[0], bytes[1], bytes[2], bytes[3]);

            let port_bytes_as_array: [u8; 2] = bytes[4..6]
                .try_into()
                .map_err(|_| DecodeMessageError::InvalidPortEncoding)?;

            let port: u16 = u16::from_be_bytes(port_bytes_as_array);

            Ok(SocketAddrV4::new(ip, port))
        }
        18 => Err(DecodeMessageError::Ipv6Unsupported),
        _ => Err(DecodeMessageError::InvalidSocketAddrEncodingLength),
    }
}

pub fn sockaddr_to_bytes(sockaddr: &SocketAddrV4) -> [u8; 6] {
    let mut bytes = [0u8; 6];

    bytes[0..4].copy_from_slice(&sockaddr.ip().octets());

    bytes[4..6].copy_from_slice(&sockaddr.port().to_be_bytes());

    bytes
}

const NODE_BYTE_SIZE: usize = ID_SIZE + 6;

fn nodes4_to_bytes(nodes: &[Node]) -> Box<[u8]> {
    let mut bytes = Vec::with_capacity(NODE_BYTE_SIZE * nodes.len());

    for node in nodes {
        bytes.extend_from_slice(node.id().as_bytes());
        bytes.extend_from_slice(&sockaddr_to_bytes(&node.address()));
    }

    bytes.into_boxed_slice()
}

fn bytes_to_nodes4<T: AsRef<[u8]>>(bytes: T) -> Result<Box<[Node]>, DecodeMessageError> {
    let bytes = bytes.as_ref();

    if bytes.len() % NODE_BYTE_SIZE != 0 {
        return Err(DecodeMessageError::InvalidNodes4);
    }

    let expected_num = bytes.len() / NODE_BYTE_SIZE;
    let mut to_ret = Vec::with_capacity(expected_num);
    for i in 0..bytes.len() / NODE_BYTE_SIZE {
        let i = i * NODE_BYTE_SIZE;
        let id = Id::from_bytes(&bytes[i..i + ID_SIZE])?;
        let sockaddr = bytes_to_sockaddr(&bytes[i + ID_SIZE..i + NODE_BYTE_SIZE])?;
        let node = Node::new(id, sockaddr);
        to_ret.push(node);
    }

    Ok(to_ret.into_boxed_slice())
}

fn peers_to_bytes(peers: &[SocketAddrV4]) -> Vec<serde_bytes::ByteBuf> {
    peers
        .iter()
        .map(|p| serde_bytes::ByteBuf::from(sockaddr_to_bytes(p)))
        .collect()
}

fn bytes_to_peers<T: AsRef<[serde_bytes::ByteBuf]>>(
    bytes: T,
) -> Result<Vec<SocketAddrV4>, DecodeMessageError> {
    let bytes = bytes.as_ref();
    bytes.iter().map(bytes_to_sockaddr).collect()
}

#[allow(clippy::type_complexity)]
fn bytes_to_signed_peers<T: AsRef<[serde_bytes::ByteBuf]>>(
    bytes: T,
) -> Result<Vec<([u8; 32], u64, [u8; 64])>, DecodeMessageError> {
    let bytes = bytes.as_ref();

    bytes.iter().map(bytes_to_signed_peer).collect()
}

fn bytes_to_signed_peer<T: AsRef<[u8]>>(
    bytes: T,
) -> Result<([u8; 32], u64, [u8; 64]), DecodeMessageError> {
    let bytes = bytes.as_ref();

    if !bytes.len().is_multiple_of(104) {
        return Err(DecodeMessageError::InvalidSignedPeersEncodingLength);
    }

    let t: [u8; 8] = bytes[32..40].try_into().expect("infallible");

    Ok((
        bytes[0..32].try_into().expect("infallible"),
        u64::from_be_bytes(t),
        bytes[40..104].try_into().expect("infallible"),
    ))
}

fn signed_peers_to_bytes(peers: &[([u8; 32], u64, [u8; 64])]) -> Vec<serde_bytes::ByteBuf> {
    peers
        .iter()
        .map(|p| serde_bytes::ByteBuf::from(signed_peer_to_bytes(p)))
        .collect()
}

fn signed_peer_to_bytes(peer: &([u8; 32], u64, [u8; 64])) -> [u8; 104] {
    let mut bytes = [0; 104];

    bytes[0..32].copy_from_slice(&peer.0);
    bytes[32..40].copy_from_slice(&peer.1.to_be_bytes());
    bytes[40..].copy_from_slice(&peer.2);

    bytes
}

#[n0_error::stack_error(derive, from_sources, std_sources)]
/// Mainline crate error enum.
pub enum DecodeMessageError {
    #[error("Expected message to be longer than 15 characters")]
    /// Expected message to be longer than 15 characters
    TooShort,

    #[error("Expected message to start with 'd'")]
    /// Expected message to start with 'd'
    NotBencodeDictionary,

    #[error("Wrong number of bytes for nodes")]
    /// Wrong number of bytes for nodes
    InvalidNodes4,

    #[error("wrong number of bytes for port")]
    /// wrong number of bytes for port
    InvalidPortEncoding,

    #[error("IPv6 is not yet implemented")]
    /// IPv6 is not yet implemented
    Ipv6Unsupported,

    #[error("Wrong number of bytes for sockaddr")]
    /// Wrong number of bytes for sockaddr
    InvalidSocketAddrEncodingLength,

    #[error(transparent)]
    /// Failed to parse packet bytes
    BencodeError(serde_bencode::Error),

    #[error(transparent)]
    /// Invalid Id size
    InvalidIdSize(InvalidIdSize),

    #[error("Invalid transaction id size (expected `[u8;2]` or `[u8;4]`)")]
    /// Invalid transaction id size (expected `[u8;2]` or `[u8;4]`)
    InvalidTransactionIdSize,

    #[error("Wrong number of bytes for signed peers")]
    /// Wrong number of bytes for signed peers
    InvalidSignedPeersEncodingLength,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ping_request() {
        let original_msg = Message {
            transaction_id: 258,
            version: None,
            requester_ip: None,
            read_only: false,
            message_type: MessageType::Request(RequestSpecific {
                requester_id: Id::random(),
                request_type: RequestTypeSpecific::Ping,
            }),
        };

        let serde_msg = original_msg.clone().into_serde_message();
        let bytes = serde_msg.to_bytes().unwrap();
        let parsed_serde_msg = internal::DHTMessage::from_bytes(&bytes).unwrap();
        let parsed_msg = Message::from_serde_message(parsed_serde_msg).unwrap();
        assert_eq!(parsed_msg, original_msg);
    }

    #[test]
    fn test_ping_response() {
        let original_msg = Message {
            transaction_id: 258,
            version: Some([0xde, 0xad, 0, 1]),
            requester_ip: Some("99.100.101.102:1030".parse().unwrap()),
            read_only: false,
            message_type: MessageType::Response(ResponseSpecific::Ping(PingResponseArguments {
                responder_id: Id::random(),
            })),
        };

        let serde_msg = original_msg.clone().into_serde_message();
        let bytes = serde_msg.to_bytes().unwrap();
        let parsed_serde_msg = internal::DHTMessage::from_bytes(&bytes).unwrap();
        let parsed_msg = Message::from_serde_message(parsed_serde_msg).unwrap();
        assert_eq!(parsed_msg, original_msg);
    }

    #[test]
    fn test_find_node_request() {
        let original_msg = Message {
            transaction_id: 258,
            version: Some([0x62, 0x61, 0x72, 0x66]),
            requester_ip: None,
            read_only: false,
            message_type: MessageType::Request(RequestSpecific {
                requester_id: Id::random(),
                request_type: RequestTypeSpecific::FindNode(FindNodeRequestArguments {
                    target: Id::random(),
                }),
            }),
        };

        let serde_msg = original_msg.clone().into_serde_message();
        let bytes = serde_msg.to_bytes().unwrap();
        let parsed_serde_msg = internal::DHTMessage::from_bytes(&bytes).unwrap();
        let parsed_msg = Message::from_serde_message(parsed_serde_msg).unwrap();
        assert_eq!(parsed_msg, original_msg);
    }

    #[test]
    fn test_find_node_request_read_only() {
        let original_msg = Message {
            transaction_id: 258,
            version: Some([0x62, 0x61, 0x72, 0x66]),
            requester_ip: None,
            read_only: true,
            message_type: MessageType::Request(RequestSpecific {
                requester_id: Id::random(),
                request_type: RequestTypeSpecific::FindNode(FindNodeRequestArguments {
                    target: Id::random(),
                }),
            }),
        };

        let serde_msg = original_msg.clone().into_serde_message();
        let bytes = serde_msg.to_bytes().unwrap();
        let parsed_serde_msg = internal::DHTMessage::from_bytes(&bytes).unwrap();
        let parsed_msg = Message::from_serde_message(parsed_serde_msg).unwrap();
        assert_eq!(parsed_msg, original_msg);
    }

    #[test]
    fn test_find_node_response() {
        let original_msg = Message {
            transaction_id: 258,
            version: Some([1, 2, 3, 4]),
            requester_ip: Some("50.51.52.53:5455".parse().unwrap()),
            read_only: false,
            message_type: MessageType::Response(ResponseSpecific::FindNode(
                FindNodeResponseArguments {
                    responder_id: Id::random(),
                    nodes: [Node::new(Id::random(), "49.50.52.52:5354".parse().unwrap())].into(),
                },
            )),
        };

        let serde_msg = original_msg.clone().into_serde_message();
        let bytes = serde_msg.to_bytes().unwrap();
        let parsed_serde_msg = internal::DHTMessage::from_bytes(&bytes).unwrap();
        let parsed_msg = Message::from_serde_message(parsed_serde_msg).unwrap();
        assert_eq!(parsed_msg.get_author_id(), original_msg.get_author_id());
        assert_eq!(
            parsed_msg.get_closer_nodes().map(|nodes| nodes
                .iter()
                .map(|n| (n.id(), n.address()))
                .collect::<Vec<_>>()),
            original_msg.get_closer_nodes().map(|nodes| nodes
                .iter()
                .map(|n| (n.id(), n.address()))
                .collect::<Vec<_>>())
        );
    }

    #[test]
    fn test_get_peers_request() {
        let original_msg = Message {
            transaction_id: 258,
            version: Some([72, 73, 0, 1]),
            requester_ip: None,
            read_only: false,
            message_type: MessageType::Request(RequestSpecific {
                requester_id: Id::random(),
                request_type: RequestTypeSpecific::GetPeers(GetPeersRequestArguments {
                    info_hash: Id::random(),
                }),
            }),
        };

        let serde_msg = original_msg.clone().into_serde_message();
        let bytes = serde_msg.to_bytes().unwrap();
        let parsed_serde_msg = internal::DHTMessage::from_bytes(&bytes).unwrap();
        let parsed_msg = Message::from_serde_message(parsed_serde_msg).unwrap();
        assert_eq!(parsed_msg, original_msg);
    }

    #[test]
    fn test_get_peers_response() {
        let original_msg = Message {
            transaction_id: 3,
            version: Some([1, 2, 3, 4]),
            requester_ip: Some("50.51.52.53:5455".parse().unwrap()),
            read_only: true,
            message_type: MessageType::Response(ResponseSpecific::NoValues(
                NoValuesResponseArguments {
                    responder_id: Id::random(),
                    token: [99, 100, 101, 102].into(),
                    nodes: Some(
                        [Node::new(Id::random(), "49.50.52.52:5354".parse().unwrap())].into(),
                    ),
                },
            )),
        };

        let serde_msg = original_msg.clone().into_serde_message();
        let bytes = serde_msg.to_bytes().unwrap();
        let parsed_serde_msg = internal::DHTMessage::from_bytes(&bytes).unwrap();
        let parsed_msg = Message::from_serde_message(parsed_serde_msg).unwrap();

        assert_eq!(parsed_msg.transaction_id, original_msg.transaction_id);
        assert_eq!(parsed_msg.version, original_msg.version);
        assert_eq!(parsed_msg.requester_ip, original_msg.requester_ip);
        assert_eq!(parsed_msg.get_author_id(), original_msg.get_author_id());
        assert_eq!(
            parsed_msg.get_closer_nodes().map(|nodes| nodes
                .iter()
                .map(|n| (n.id(), n.address()))
                .collect::<Vec<_>>()),
            original_msg.get_closer_nodes().map(|nodes| nodes
                .iter()
                .map(|n| (n.id(), n.address()))
                .collect::<Vec<_>>())
        );
    }

    #[test]
    fn test_get_peers_response_peers() {
        let original_msg = Message {
            transaction_id: 3,
            version: Some([1, 2, 3, 4]),
            requester_ip: Some("50.51.52.53:5455".parse().unwrap()),
            read_only: false,
            message_type: MessageType::Response(ResponseSpecific::GetPeers(
                GetPeersResponseArguments {
                    responder_id: Id::random(),
                    token: vec![99, 100, 101, 102].into(),
                    nodes: None,
                    values: ["123.123.123.123:123".parse().unwrap()].into(),
                },
            )),
        };

        let serde_msg = original_msg.clone().into_serde_message();
        let bytes = serde_msg.to_bytes().unwrap();
        let parsed_serde_msg = internal::DHTMessage::from_bytes(&bytes).unwrap();
        let parsed_msg = Message::from_serde_message(parsed_serde_msg).unwrap();
        assert_eq!(parsed_msg, original_msg);
    }

    #[test]
    fn test_get_peers_response_neither() {
        let serde_message = internal::DHTMessage {
            ip: None,
            read_only: None,
            transaction_id: vec![1, 2], // test u16
            version: None,
            variant: internal::DHTMessageVariant::Response(
                internal::DHTResponseSpecific::NoValues {
                    arguments: internal::DHTNoValuesResponseArguments {
                        id: Id::random().into(),
                        token: vec![0, 1].into(),
                        nodes: None,
                    },
                },
            ),
        };
        let parsed_msg = Message::from_serde_message(serde_message).unwrap();
        assert!(matches!(
            parsed_msg.message_type,
            MessageType::Response(ResponseSpecific::NoValues(NoValuesResponseArguments { .. }))
        ));
    }

    #[test]
    fn test_get_immutable_request() {
        let original_msg = Message {
            transaction_id: 258,
            version: Some([72, 73, 0, 1]),
            requester_ip: None,
            read_only: false,
            message_type: MessageType::Request(RequestSpecific {
                requester_id: Id::random(),
                request_type: RequestTypeSpecific::GetValue(GetValueRequestArguments {
                    target: Id::random(),
                    seq: Some(1231),
                    salt: None,
                }),
            }),
        };

        let serde_msg = original_msg.clone().into_serde_message();
        let bytes = serde_msg.to_bytes().unwrap();
        let parsed_serde_msg = internal::DHTMessage::from_bytes(&bytes).unwrap();
        let parsed_msg = Message::from_serde_message(parsed_serde_msg).unwrap();
        assert_eq!(parsed_msg, original_msg);
    }

    #[test]
    fn test_get_immutable_response() {
        let original_msg = Message {
            transaction_id: 3,
            version: Some([1, 2, 3, 4]),
            requester_ip: Some("50.51.52.53:5455".parse().unwrap()),
            read_only: false,
            message_type: MessageType::Response(ResponseSpecific::GetImmutable(
                GetImmutableResponseArguments {
                    responder_id: Id::random(),
                    token: [99, 100, 101, 102].into(),
                    nodes: None,
                    v: [99, 100, 101, 102].into(),
                },
            )),
        };

        let serde_msg = original_msg.clone().into_serde_message();
        let bytes = serde_msg.to_bytes().unwrap();
        let parsed_serde_msg = internal::DHTMessage::from_bytes(&bytes).unwrap();
        let parsed_msg = Message::from_serde_message(parsed_serde_msg).unwrap();
        assert_eq!(parsed_msg, original_msg);
    }

    #[test]
    fn test_put_immutable_request() {
        let original_msg = Message {
            transaction_id: 3,
            version: Some([1, 2, 3, 4]),
            requester_ip: Some("50.51.52.53:5455".parse().unwrap()),
            read_only: false,
            message_type: MessageType::Request(RequestSpecific {
                requester_id: Id::random(),
                request_type: RequestTypeSpecific::Put(PutRequest {
                    token: [99, 100, 101, 102].into(),
                    put_request_type: PutRequestSpecific::PutImmutable(
                        PutImmutableRequestArguments {
                            target: Id::random(),
                            v: [99, 100, 101, 102].into(),
                        },
                    ),
                }),
            }),
        };

        let serde_msg = original_msg.clone().into_serde_message();
        let bytes = serde_msg.to_bytes().unwrap();
        let parsed_serde_msg = internal::DHTMessage::from_bytes(&bytes).unwrap();
        let parsed_msg = Message::from_serde_message(parsed_serde_msg).unwrap();
        assert_eq!(parsed_msg, original_msg);
    }

    #[test]
    fn test_put_mutable_request() {
        let original_msg = Message {
            transaction_id: 3,
            version: Some([1, 2, 3, 4]),
            requester_ip: Some("50.51.52.53:5455".parse().unwrap()),
            read_only: false,
            message_type: MessageType::Request(RequestSpecific {
                requester_id: Id::random(),
                request_type: RequestTypeSpecific::Put(PutRequest {
                    token: [99, 100, 101, 102].into(),
                    put_request_type: PutRequestSpecific::PutMutable(PutMutableRequestArguments {
                        target: Id::random(),
                        v: [99, 100, 101, 102].into(),
                        k: [100; 32],
                        seq: 100,
                        sig: [0; 64],
                        salt: Some([0, 2, 4, 8].into()),
                        cas: Some(100),
                    }),
                }),
            }),
        };

        let serde_msg = original_msg.clone().into_serde_message();
        let bytes = serde_msg.to_bytes().unwrap();
        let parsed_serde_msg = internal::DHTMessage::from_bytes(&bytes).unwrap();
        let parsed_msg = Message::from_serde_message(parsed_serde_msg).unwrap();
        assert_eq!(parsed_msg, original_msg);
    }
}
