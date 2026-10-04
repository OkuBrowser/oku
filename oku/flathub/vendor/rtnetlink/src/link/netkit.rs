// SPDX-License-Identifier: MIT

use crate::{
    packet_route::link::{
        InfoData, InfoKind, InfoNetkit, NetkitMode, NetkitPolicy, NetkitScrub,
    },
    LinkMessageBuilder, LinkUnspec,
};

#[derive(Debug)]
/// Represent netkit virtual interface.
/// Netkit devices are used for container networking and BPF programs.
///
/// Example code on creating a netkit pair:
/// ```no_run
/// use rtnetlink::{new_connection, LinkNetkit, packet_route::link::NetkitMode};
/// #[tokio::main]
/// async fn main() -> Result<(), String> {
///     let (connection, handle, _) = new_connection().unwrap();
///     tokio::spawn(connection);
///
///     handle
///         .link()
///         .add(LinkNetkit::new("netkit0", "netkit0-peer", NetkitMode::L3).build())
///         .execute()
///         .await
///         .map_err(|e| format!("{e}"))
/// }
/// ```
///
/// Please check LinkMessageBuilder::<LinkNetkit> for more detail.
pub struct LinkNetkit;

impl LinkNetkit {
    /// Equal to `LinkMessageBuilder::<LinkNetkit>::new(name, peer, mode)`
    pub fn new(
        name: &str,
        peer: &str,
        mode: NetkitMode,
    ) -> LinkMessageBuilder<Self> {
        LinkMessageBuilder::<LinkNetkit>::new(name, peer, mode)
    }
}

impl LinkMessageBuilder<LinkNetkit> {
    /// Create [LinkMessageBuilder] for netkit
    pub fn new(name: &str, peer: &str, mode: NetkitMode) -> Self {
        LinkMessageBuilder::<LinkNetkit>::new_with_info_kind(InfoKind::Netkit)
            .name(name.to_string())
            .mode(mode)
            .peer(peer)
    }

    /// Set the peer interface name
    pub fn peer(self, peer: &str) -> Self {
        let peer_msg = LinkMessageBuilder::<LinkUnspec>::new()
            .name(peer.to_string())
            .build();

        self.append_info_data(InfoNetkit::Peer(peer_msg))
    }

    /// Set the netkit mode (L2 or L3)
    pub fn mode(self, mode: NetkitMode) -> Self {
        self.append_info_data(InfoNetkit::Mode(mode))
    }

    /// Set the primary interface flag
    pub fn primary(self, primary: bool) -> Self {
        self.append_info_data(InfoNetkit::Primary(primary))
    }

    /// Set the policy for the primary interface
    pub fn policy(self, policy: NetkitPolicy) -> Self {
        self.append_info_data(InfoNetkit::Policy(policy))
    }

    /// Set the policy for the peer interface
    pub fn peer_policy(self, policy: NetkitPolicy) -> Self {
        self.append_info_data(InfoNetkit::PeerPolicy(policy))
    }

    /// Set the scrub settings for the primary interface
    pub fn scrub(self, scrub: NetkitScrub) -> Self {
        self.append_info_data(InfoNetkit::Scrub(scrub))
    }

    /// Set the scrub settings for the peer interface
    pub fn peer_scrub(self, scrub: NetkitScrub) -> Self {
        self.append_info_data(InfoNetkit::PeerScrub(scrub))
    }

    /// Set the desired headroom
    pub fn headroom(self, headroom: u16) -> Self {
        self.append_info_data(InfoNetkit::Headroom(headroom))
    }

    /// Set the desired tailroom
    pub fn tailroom(self, tailroom: u16) -> Self {
        self.append_info_data(InfoNetkit::Tailroom(tailroom))
    }

    /// Helper to append netkit-specific info data
    fn append_info_data(mut self, info: InfoNetkit) -> Self {
        if let InfoData::Netkit(ref mut infos) = self
            .info_data
            .get_or_insert_with(|| InfoData::Netkit(Vec::new()))
        {
            infos.push(info);
        }
        self
    }
}
