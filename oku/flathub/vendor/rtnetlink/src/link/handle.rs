// SPDX-License-Identifier: MIT

use super::{
    LinkAddRequest, LinkDelPropRequest, LinkDelRequest, LinkGetRequest,
    LinkNewPropRequest, LinkSetRequest,
};
use crate::{
    packet_core::{NLM_F_ACK, NLM_F_REQUEST},
    packet_route::link::LinkMessage,
    Handle,
};

pub struct LinkHandle(Handle);

impl LinkHandle {
    pub fn new(handle: Handle) -> Self {
        LinkHandle(handle)
    }

    /// Using `RTM_SETLINK`. Currently this is only used for bridge commands
    /// like `bridge vlan` and `bridge vlan`.
    /// For changing existing network interface, please use
    /// [LinkHandle::add()]
    pub fn set(&self, message: LinkMessage) -> LinkSetRequest {
        LinkSetRequest::new(self.0.clone(), message)
    }

    pub fn add(&self, message: LinkMessage) -> LinkAddRequest {
        LinkAddRequest::new(self.0.clone(), message)
    }

    /// Equal to `ip link set` command.
    pub fn change(&self, message: LinkMessage) -> LinkAddRequest {
        LinkAddRequest::new(self.0.clone(), message)
            .set_flags(NLM_F_REQUEST | NLM_F_ACK)
    }

    pub fn property_add(&self, index: u32) -> LinkNewPropRequest {
        LinkNewPropRequest::new(self.0.clone(), index)
    }

    pub fn property_del(&self, index: u32) -> LinkDelPropRequest {
        LinkDelPropRequest::new(self.0.clone(), index)
    }

    pub fn del(&self, index: u32) -> LinkDelRequest {
        LinkDelRequest::new(self.0.clone(), index)
    }

    /// Delete specified information from interface.
    /// For example: To delete bridge VLANs, it is required to include
    /// LinkMessage of VLAN information to delete.
    pub fn del_with_message(&self, message: LinkMessage) -> LinkDelRequest {
        LinkDelRequest::new_with_message(self.0.clone(), message)
    }

    /// Retrieve the list of links (equivalent to `ip link show`)
    pub fn get(&self) -> LinkGetRequest {
        LinkGetRequest::new(self.0.clone())
    }

    /// The `LinkHandle::set()` cannot be used for setting bond or bridge port
    /// configuration, `RTM_NEWLINK` and `NLM_F_REQUEST|NLM_F_ACK` are required,
    /// Equal to `LinkAddRequest::new().set_flags(NLM_F_REQUEST | NLM_F_ACK)`
    pub fn set_port(&self, message: LinkMessage) -> LinkAddRequest {
        LinkAddRequest::new(self.0.clone(), message)
            .set_flags(NLM_F_REQUEST | NLM_F_ACK)
    }
}
