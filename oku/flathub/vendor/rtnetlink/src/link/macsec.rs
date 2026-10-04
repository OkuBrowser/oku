// SPDX-License-Identifier: MIT

use netlink_packet_core::DefaultNla;
use netlink_packet_route::link::{
    InfoMacSec, MacSecCipherId, MacSecOffload, MacSecValidate,
};

use crate::{
    link::LinkMessageBuilder,
    packet_route::link::{InfoData, InfoKind},
};

/// Represent MACsec interface.
/// Example code on creating a MACsec interface
/// ```no_run
/// use rtnetlink::{new_connection, LinkMacSec};
///
/// #[tokio::main]
/// async fn main() -> Result<(), String> {
///     let (connection, handle, _) = new_connection().unwrap();
///     tokio::spawn(connection);
///
///     handle
///         .link()
///         .add(
///             LinkMacSec::new("macsec0", 10)
///                 .up()
///                 .build(),
///         )
///         .execute()
///         .await
///         .map_err(|e| format!("{e}"))
/// }
/// ```
///
/// Please check LinkMessageBuilder::<LinkMacSec> for more detail.
#[derive(Debug)]
pub struct LinkMacSec;

impl LinkMacSec {
    /// Wrapper of `LinkMessageBuilder::<LinkMacSec>::new().link().mode()`
    pub fn new(name: &str, base_iface_index: u32) -> LinkMessageBuilder<Self> {
        LinkMessageBuilder::<LinkMacSec>::new(name).link(base_iface_index)
    }
}

impl LinkMessageBuilder<LinkMacSec> {
    /// Create [LinkMessageBuilder] for MACSEC
    pub fn new(name: &str) -> Self {
        LinkMessageBuilder::<LinkMacSec>::new_with_info_kind(InfoKind::MacSec)
            .name(name.to_string())
    }

    pub fn append_info_data(mut self, info: InfoMacSec) -> Self {
        if let InfoData::MacSec(infos) = self
            .info_data
            .get_or_insert_with(|| InfoData::MacSec(Vec::new()))
        {
            infos.push(info);
        }
        self
    }

    pub fn sci(self, sci: u64) -> Self {
        self.append_info_data(InfoMacSec::Sci(sci))
    }

    pub fn port(self, port: u16) -> Self {
        self.append_info_data(InfoMacSec::Port(port))
    }

    pub fn icv_len(self, icv_len: u8) -> Self {
        self.append_info_data(InfoMacSec::IcvLen(icv_len))
    }

    pub fn cipher_suite(self, cipher_suite: MacSecCipherId) -> Self {
        self.append_info_data(InfoMacSec::CipherSuite(cipher_suite))
    }

    pub fn window(self, window: u32) -> Self {
        self.append_info_data(InfoMacSec::Window(window))
    }

    pub fn encoding_sa(self, encoding_sa: u8) -> Self {
        self.append_info_data(InfoMacSec::EncodingSa(encoding_sa))
    }

    pub fn encrypt(self, encrypt: bool) -> Self {
        self.append_info_data(InfoMacSec::Encrypt(if encrypt { 1 } else { 0 }))
    }

    pub fn protect(self, protect: bool) -> Self {
        self.append_info_data(InfoMacSec::Protect(if protect { 1 } else { 0 }))
    }

    pub fn inc_sci(self, inc_sci: bool) -> Self {
        self.append_info_data(InfoMacSec::IncSci(if inc_sci { 1 } else { 0 }))
    }

    pub fn es(self, es: bool) -> Self {
        self.append_info_data(InfoMacSec::Es(if es { 1 } else { 0 }))
    }

    pub fn scb(self, scb: bool) -> Self {
        self.append_info_data(InfoMacSec::Scb(if scb { 1 } else { 0 }))
    }

    pub fn replay_protect(self, replay_protect: bool) -> Self {
        self.append_info_data(InfoMacSec::ReplayProtect(if replay_protect {
            1
        } else {
            0
        }))
    }

    pub fn validation(self, validation: MacSecValidate) -> Self {
        self.append_info_data(InfoMacSec::Validation(validation))
    }

    pub fn offload(self, offload: MacSecOffload) -> Self {
        self.append_info_data(InfoMacSec::Offload(offload))
    }

    pub fn other(self, other: DefaultNla) -> Self {
        self.append_info_data(InfoMacSec::Other(other))
    }
}
