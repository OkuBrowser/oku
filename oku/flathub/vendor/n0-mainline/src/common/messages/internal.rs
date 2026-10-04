use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DHTMessage {
    #[serde(rename = "t", with = "serde_bytes")]
    pub transaction_id: Vec<u8>,

    #[serde(default)]
    #[serde(rename = "v", with = "serde_bytes")]
    pub version: Option<[u8; 4]>,

    #[serde(flatten)]
    pub variant: DHTMessageVariant,

    #[serde(default)]
    #[serde(with = "serde_bytes")]
    // Ipv6 is not supported anyways.
    pub ip: Option<[u8; 6]>,

    #[serde(default)]
    #[serde(rename = "ro")]
    pub read_only: Option<i32>,
}

impl DHTMessage {
    pub fn from_bytes(bytes: &[u8]) -> Result<DHTMessage, serde_bencode::Error> {
        let obj = serde_bencode::from_bytes(bytes)?;
        Ok(obj)
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, serde_bencode::Error> {
        serde_bencode::to_bytes(self)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "y")]
pub enum DHTMessageVariant {
    #[serde(rename = "q")]
    Request(DHTRequestSpecific),

    #[serde(rename = "r")]
    Response(DHTResponseSpecific),

    #[serde(rename = "e")]
    Error(DHTErrorSpecific),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "q")]
pub enum DHTRequestSpecific {
    #[serde(rename = "ping")]
    Ping {
        #[serde(rename = "a")]
        arguments: DHTPingRequestArguments,
    },

    #[serde(rename = "find_node")]
    FindNode {
        #[serde(rename = "a")]
        arguments: DHTFindNodeRequestArguments,
    },

    #[serde(rename = "get_peers")]
    GetPeers {
        #[serde(rename = "a")]
        arguments: DHTGetPeersRequestArguments,
    },

    #[serde(rename = "get_signed_peers")]
    GetSignedPeers {
        #[serde(rename = "a")]
        arguments: DHTGetPeersRequestArguments,
    },

    #[serde(rename = "announce_peer")]
    AnnouncePeer {
        #[serde(rename = "a")]
        arguments: DHTAnnouncePeerRequestArguments,
    },

    #[serde(rename = "announce_signed_peer")]
    AnnounceSignedPeer {
        #[serde(rename = "a")]
        arguments: DHTAnnounceSignedPeerRequestArguments,
    },

    #[serde(rename = "get")]
    GetValue {
        #[serde(rename = "a")]
        arguments: DHTGetValueRequestArguments,
    },

    #[serde(rename = "put")]
    PutValue {
        #[serde(rename = "a")]
        arguments: DHTPutValueRequestArguments,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)] // This means order matters! Order these from most to least detailed
pub enum DHTResponseSpecific {
    GetMutable {
        #[serde(rename = "r")]
        arguments: DHTGetMutableResponseArguments,
    },

    NoMoreRecentValue {
        #[serde(rename = "r")]
        arguments: DHTNoMoreRecentValueResponseArguments,
    },

    GetImmutable {
        #[serde(rename = "r")]
        arguments: DHTGetImmutableResponseArguments,
    },

    GetPeers {
        #[serde(rename = "r")]
        arguments: DHTGetPeersResponseArguments,
    },

    GetSignedPeers {
        #[serde(rename = "r")]
        arguments: DHTGetSignedPeersResponseArguments,
    },

    NoValues {
        #[serde(rename = "r")]
        arguments: DHTNoValuesResponseArguments,
    },

    FindNode {
        #[serde(rename = "r")]
        arguments: DHTFindNodeResponseArguments,
    },

    Ping {
        #[serde(rename = "r")]
        arguments: DHTPingResponseArguments,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DHTErrorSpecific {
    #[serde(rename = "e")]
    pub error_info: (i32, String),
}

// === PING ===

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DHTPingRequestArguments {
    #[serde(with = "serde_bytes")]
    pub id: [u8; 20],
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DHTPingResponseArguments {
    #[serde(with = "serde_bytes")]
    pub id: [u8; 20],
}

// === FIND NODE ===

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DHTFindNodeRequestArguments {
    #[serde(with = "serde_bytes")]
    pub id: [u8; 20],

    #[serde(with = "serde_bytes")]
    pub target: [u8; 20],
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DHTFindNodeResponseArguments {
    #[serde(with = "serde_bytes")]
    pub id: [u8; 20],

    #[serde(with = "serde_bytes")]
    pub nodes: Box<[u8]>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DHTNoValuesResponseArguments {
    #[serde(with = "serde_bytes")]
    pub id: [u8; 20],

    #[serde(with = "serde_bytes")]
    pub token: Box<[u8]>,

    #[serde(with = "serde_bytes")]
    #[serde(default)]
    pub nodes: Option<Box<[u8]>>,
}

// === Get Peers ===

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DHTGetPeersRequestArguments {
    #[serde(with = "serde_bytes")]
    pub id: [u8; 20],

    #[serde(with = "serde_bytes")]
    pub info_hash: [u8; 20],
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DHTGetPeersResponseArguments {
    #[serde(with = "serde_bytes")]
    pub id: [u8; 20],

    #[serde(with = "serde_bytes")]
    pub token: Box<[u8]>,

    #[serde(with = "serde_bytes")]
    #[serde(default)]
    pub nodes: Option<Box<[u8]>>,

    // values are not optional, because if they are missing this missing
    // we can just treat this as DHTNoValuesResponseArguments
    pub values: Vec<ByteBuf>,
}

// === Get Signed Peers ===

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DHTGetSignedPeersResponseArguments {
    #[serde(with = "serde_bytes")]
    pub id: [u8; 20],

    #[serde(with = "serde_bytes")]
    pub token: Box<[u8]>,

    #[serde(with = "serde_bytes")]
    #[serde(default)]
    pub nodes: Option<Box<[u8]>>,

    // peers are not optional, because if they are missing this missing
    // we can just treat this as DHTNoValuesResponseArguments
    pub peers: Vec<ByteBuf>,
}

// === Announce Peer ===

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DHTAnnouncePeerRequestArguments {
    #[serde(with = "serde_bytes")]
    pub id: [u8; 20],

    #[serde(with = "serde_bytes")]
    pub info_hash: [u8; 20],

    pub port: u16,

    #[serde(with = "serde_bytes")]
    pub token: Box<[u8]>,

    #[serde(default)]
    pub implied_port: Option<u8>,
}

/// === Announce Signed Peer ===

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DHTAnnounceSignedPeerRequestArguments {
    #[serde(with = "serde_bytes")]
    pub id: [u8; 20],

    #[serde(with = "serde_bytes")]
    pub info_hash: [u8; 20],

    #[serde(with = "serde_bytes")]
    pub token: Box<[u8]>,

    #[serde(with = "serde_bytes")]
    pub k: [u8; 32],

    #[serde(with = "serde_bytes")]
    pub sig: [u8; 64],

    pub t: i64,
}

// === Get Value ===

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DHTGetValueRequestArguments {
    #[serde(with = "serde_bytes")]
    pub id: [u8; 20],

    #[serde(with = "serde_bytes")]
    pub target: [u8; 20],

    #[serde(default)]
    pub seq: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DHTGetImmutableResponseArguments {
    #[serde(with = "serde_bytes")]
    pub id: [u8; 20],

    #[serde(with = "serde_bytes")]
    pub token: Box<[u8]>,

    #[serde(with = "serde_bytes")]
    #[serde(default)]
    pub nodes: Option<Box<[u8]>>,

    #[serde(with = "serde_bytes")]
    pub v: Box<[u8]>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DHTNoMoreRecentValueResponseArguments {
    #[serde(with = "serde_bytes")]
    pub id: [u8; 20],

    #[serde(with = "serde_bytes")]
    pub token: Box<[u8]>,

    #[serde(with = "serde_bytes")]
    #[serde(default)]
    pub nodes: Option<Box<[u8]>>,

    pub seq: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DHTGetMutableResponseArguments {
    #[serde(with = "serde_bytes")]
    pub id: [u8; 20],

    #[serde(with = "serde_bytes")]
    pub token: Box<[u8]>,

    #[serde(with = "serde_bytes")]
    #[serde(default)]
    pub nodes: Option<Box<[u8]>>,

    #[serde(with = "serde_bytes")]
    pub v: Box<[u8]>,

    #[serde(with = "serde_bytes")]
    pub k: [u8; 32],

    #[serde(with = "serde_bytes")]
    pub sig: [u8; 64],

    pub seq: i64,
}

// === Put Value ===

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DHTPutValueRequestArguments {
    #[serde(with = "serde_bytes")]
    pub id: [u8; 20],

    #[serde(with = "serde_bytes")]
    pub target: [u8; 20],

    #[serde(with = "serde_bytes")]
    pub token: Box<[u8]>,

    #[serde(with = "serde_bytes")]
    pub v: Box<[u8]>,

    #[serde(with = "serde_bytes")]
    #[serde(default)]
    pub k: Option<[u8; 32]>,

    #[serde(with = "serde_bytes")]
    #[serde(default)]
    pub sig: Option<[u8; 64]>,

    #[serde(default)]
    pub seq: Option<i64>,

    #[serde(default)]
    pub cas: Option<i64>,

    #[serde(with = "serde_bytes")]
    #[serde(default)]
    pub salt: Option<Box<[u8]>>,
}
