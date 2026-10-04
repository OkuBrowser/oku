// SPDX-License-Identifier: MIT

use std::{
    marker::PhantomData,
    net::{Ipv4Addr, Ipv6Addr},
};

use netlink_packet_route::{
    address::{AddressAttribute, AddressMessage},
    AddressFamily,
};

#[derive(Debug)]
/// Helper struct for building [AddressMessage].
pub struct AddressMessageBuilder<T> {
    message: AddressMessage,
    _phantom: PhantomData<T>,
}

impl<T> AddressMessageBuilder<T> {
    /// Create a new [AddressMessageBuilder] without specifying the address
    /// family.
    fn new_no_address_family() -> Self {
        AddressMessageBuilder {
            message: AddressMessage::default(),
            _phantom: PhantomData,
        }
    }

    /// Sets the interface index.
    pub fn index(mut self, index: u32) -> Self {
        self.message.header.index = index;
        self
    }

    /// Builds [AddressMessage].
    pub fn build(self) -> AddressMessage {
        self.message
    }
}

impl Default for AddressMessageBuilder<Ipv4Addr> {
    fn default() -> Self {
        Self::new()
    }
}

impl AddressMessageBuilder<Ipv4Addr> {
    /// Create a new [AddressMessageBuilder] for IPv4 addresses.
    pub fn new() -> Self {
        let mut builder = Self::new_no_address_family();
        builder.message.header.family = AddressFamily::Inet;
        builder
    }

    /// Sets the address and prefix length.
    pub fn address(mut self, address: Ipv4Addr, prefix_len: u8) -> Self {
        self.message.header.prefix_len = prefix_len;

        if !address.is_multicast() {
            self.message
                .attributes
                .push(AddressAttribute::Address(address.into()));

            // The IFA_LOCAL address can be set to the same value as
            // IFA_ADDRESS.
            self.message
                .attributes
                .push(AddressAttribute::Local(address.into()));

            // Set the IFA_BROADCAST address as well.
            if prefix_len == 32 {
                self.message
                    .attributes
                    .push(AddressAttribute::Broadcast(address));
            } else {
                let ip_addr = u32::from(address);
                let brd = Ipv4Addr::from(
                    (0xffff_ffff_u32) >> u32::from(prefix_len) | ip_addr,
                );
                self.message
                    .attributes
                    .push(AddressAttribute::Broadcast(brd));
            };
        }

        self
    }
}

impl Default for AddressMessageBuilder<Ipv6Addr> {
    fn default() -> Self {
        Self::new()
    }
}

impl AddressMessageBuilder<Ipv6Addr> {
    /// Create a new [AddressMessageBuilder] for IPv6 addresses.
    pub fn new() -> Self {
        let mut builder = Self::new_no_address_family();
        builder.message.header.family = AddressFamily::Inet6;
        builder
    }

    /// Sets the address and prefix length.
    pub fn address(mut self, address: Ipv6Addr, prefix_len: u8) -> Self {
        self.message.header.prefix_len = prefix_len;

        if address.is_multicast() {
            self.message
                .attributes
                .push(AddressAttribute::Multicast(address));
        } else {
            self.message
                .attributes
                .push(AddressAttribute::Address(address.into()));

            // The IFA_LOCAL address can be set to the same value as
            // IFA_ADDRESS.
            self.message
                .attributes
                .push(AddressAttribute::Local(address.into()));
        }

        self
    }
}
