// SPDX-License-Identifier: MIT

use crate::{
    packet_route::{
        link::{
            AfSpecBridge, BridgeFlag, BridgeVlanInfo, BridgeVlanInfoFlags,
            LinkAttribute,
        },
        AddressFamily,
    },
    LinkMessageBuilder,
};

#[derive(Debug)]
pub struct LinkBridgeVlan;

impl LinkBridgeVlan {
    pub fn new(port_index: u32) -> LinkMessageBuilder<Self> {
        LinkMessageBuilder::<LinkBridgeVlan>::default()
            .index(port_index)
            .interface_family(AddressFamily::Bridge)
    }
}

impl LinkMessageBuilder<LinkBridgeVlan> {
    /// Append arbitrary [LinkAttribute::AfSpecBridge]
    pub fn append_af_spec(self, spec: AfSpecBridge) -> Self {
        let mut ret = self;

        for attr in ret.extra_attriutes.iter_mut() {
            if let LinkAttribute::AfSpecBridge(specs) = attr {
                specs.push(spec);
                return ret;
            }
        }

        ret.append_extra_attribute(LinkAttribute::AfSpecBridge(vec![spec]))
    }

    pub fn vlan(self, vid: u16, flags: BridgeVlanInfoFlags) -> Self {
        self.append_af_spec(AfSpecBridge::VlanInfo(BridgeVlanInfo {
            vid,
            flags,
        }))
    }

    /// Helper function by adding [BridgeVlanInfoFlags::RangeBegin]
    /// automatically to `flags`
    pub fn vlan_range_start(
        self,
        vid: u16,
        flags: BridgeVlanInfoFlags,
    ) -> Self {
        self.vlan(vid, flags | BridgeVlanInfoFlags::RangeBegin)
    }

    /// Helper function by adding [BridgeVlanInfoFlags::RangeEnd]
    /// automatically to `flags`
    pub fn vlan_range_end(self, vid: u16, flags: BridgeVlanInfoFlags) -> Self {
        self.vlan(vid, flags | BridgeVlanInfoFlags::RangeEnd)
    }

    /// Change VLAN of bridge itself
    /// Equal to the `self` argument in `bridge vlan add dev br0 vid 11 self`
    pub fn bridge_self(self) -> Self {
        self.append_af_spec(AfSpecBridge::Flags(BridgeFlag::LowerDev))
    }
}
