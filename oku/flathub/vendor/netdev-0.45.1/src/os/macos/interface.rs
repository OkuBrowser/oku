use crate::os::darwin::types::{get_functional_type, interface_type_by_name};
use crate::os::macos::sc::{get_sc_interface_map, read_sc_plist_interface_map};
use crate::{
    interface::interface::Interface,
    os::{
        macos::sc::SCInterface, macos::wifi::get_wifi_transmit_rate,
        unix::interface::unix_interfaces,
    },
    prelude::InterfaceType,
};

use std::collections::HashMap;

pub fn interfaces() -> Vec<Interface> {
    let mut ifaces: Vec<Interface> = unix_interfaces();

    let if_extra_map: HashMap<String, SCInterface> =
        read_sc_plist_interface_map().unwrap_or_else(|_| {
            // Fallback to SCNetworkInterfaceCopyAll ...
            get_sc_interface_map()
        });

    #[cfg(feature = "gateway")]
    let gateway_map = crate::os::darwin::route::get_gateway_map();

    for iface in &mut ifaces {
        // If interface type is Ethernet, try to get a more accurate type
        if iface.if_type == InterfaceType::Ethernet {
            let ft: InterfaceType = get_functional_type(&iface.name);
            if ft != InterfaceType::Unknown {
                iface.if_type = ft;
            }
        }
        if let Some(name_type) = interface_type_by_name(&iface.name) {
            iface.if_type = name_type;
        }

        if let Some(sc_inface) = if_extra_map.get(&iface.name) {
            if let Some(sc_type) = sc_inface.if_type() {
                iface.if_type = sc_type;
            }
            iface.friendly_name = sc_inface.friendly_name.clone();
            iface.dhcp_v4_enabled = sc_inface.dhcp_v4_enabled;
            iface.dhcp_v6_enabled = sc_inface.dhcp_v6_enabled;
        }

        if iface.if_type == InterfaceType::Wireless80211 {
            iface.transmit_speed = get_wifi_transmit_rate(&iface.name);
        }

        #[cfg(feature = "gateway")]
        {
            if let Some(gateway) = gateway_map.get(&iface.index) {
                iface.gateway = Some(gateway.clone());
            }
        }
    }

    #[cfg(feature = "gateway")]
    {
        if let Some(local_ip) = crate::net::ip::get_local_ipaddr() {
            if let Some(idx) = crate::interface::pick_default_iface_index(&ifaces, local_ip) {
                if let Some(iface) = ifaces.iter_mut().find(|it| it.index == idx) {
                    iface.default = true;
                    iface.dns_servers = crate::os::unix::dns::get_system_dns_conf();
                }
            }
        }
    }

    ifaces
}
