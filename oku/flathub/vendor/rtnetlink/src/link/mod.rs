// SPDX-License-Identifier: MIT

mod add;
mod bond;
mod bond_port;
mod bridge;
mod bridge_port;
#[cfg(any(target_os = "linux", target_os = "fuchsia", target_os = "android"))]
mod bridge_vlan;
mod builder;
mod del;
mod dummy;
mod get;
mod handle;
mod mac_vlan;
mod mac_vtap;
mod macsec;
mod netkit;
mod property_add;
mod property_del;
mod set;
mod veth;
mod vlan;
mod vrf;
mod vxlan;
mod wireguard;
mod xfrm;

pub use self::{
    add::LinkAddRequest,
    bond::LinkBond,
    bond_port::LinkBondPort,
    bridge::LinkBridge,
    bridge_port::LinkBridgePort,
    builder::{LinkMessageBuilder, LinkUnspec},
    del::LinkDelRequest,
    dummy::LinkDummy,
    get::LinkGetRequest,
    handle::LinkHandle,
    mac_vlan::LinkMacVlan,
    mac_vtap::LinkMacVtap,
    macsec::LinkMacSec,
    netkit::LinkNetkit,
    property_add::LinkNewPropRequest,
    property_del::LinkDelPropRequest,
    set::LinkSetRequest,
    veth::LinkVeth,
    vlan::{LinkVlan, QosMapping},
    vrf::LinkVrf,
    vxlan::LinkVxlan,
    wireguard::LinkWireguard,
    xfrm::LinkXfrm,
};

#[cfg(test)]
mod test;

#[cfg(any(
    target_os = "linux",
    target_os = "fuchsia",
    target_os = "android"
))]
pub use self::bridge_vlan::LinkBridgeVlan;
