use crate::interface::types::InterfaceType;
use crate::os::darwin::types::{get_functional_type, interface_type_by_name};
#[cfg(feature = "gateway")]
use crate::os::ios::network::NWPathStatus;
use crate::{interface::interface::Interface, os::unix::interface::unix_interfaces};
#[cfg(feature = "gateway")]
use crate::{net::device::NetworkDevice, net::mac::MacAddr};

#[cfg(feature = "gateway")]
fn merge_gateway(base: &mut NetworkDevice, supplement: &NetworkDevice) {
    if base.mac_addr == MacAddr::zero() && supplement.mac_addr != MacAddr::zero() {
        base.mac_addr = supplement.mac_addr;
    }

    for ip in &supplement.ipv4 {
        if !base.ipv4.contains(ip) {
            base.ipv4.push(*ip);
        }
    }

    for ip in &supplement.ipv6 {
        if !base.ipv6.contains(ip) {
            base.ipv6.push(*ip);
        }
    }
}

pub fn interfaces() -> Vec<Interface> {
    let mut ifaces: Vec<Interface> = unix_interfaces();

    let nw_path_snapshot = super::network::current_path_snapshot();
    let nw_iface_map = nw_path_snapshot
        .as_ref()
        .map(|snapshot| snapshot.interface_map())
        .unwrap_or_default();
    #[cfg(feature = "apple-system-configuration-extra")]
    let sc_iface_map = super::sc::get_sc_interface_map();

    #[cfg(feature = "gateway")]
    let gateway_map = crate::os::darwin::route::get_gateway_map();

    #[cfg(feature = "gateway")]
    let default_idx = crate::net::ip::get_local_ipaddr()
        .and_then(|local_ip| crate::interface::pick_default_iface_index(&ifaces, local_ip))
        .or_else(|| {
            nw_path_snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.first_non_loopback_interface_index())
        });

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

        if let Some(nw_iface) = nw_iface_map.get(&iface.name) {
            iface.if_type = nw_iface.if_type;
        }

        #[cfg(feature = "apple-system-configuration-extra")]
        if let Some(sc_iface) = sc_iface_map.get(&iface.name) {
            if let Some(sc_type) = sc_iface.if_type() {
                iface.if_type = sc_type;
            }
            iface.friendly_name = sc_iface.friendly_name.clone();
            iface.dhcp_v4_enabled = sc_iface.dhcp_v4_enabled;
            iface.dhcp_v6_enabled = sc_iface.dhcp_v6_enabled;
        }

        #[cfg(feature = "gateway")]
        {
            if let Some(gateway) = gateway_map.get(&iface.index) {
                iface.gateway = Some(gateway.clone());
            }

            if Some(iface.index) == default_idx {
                iface.default = true;

                if let Some(snapshot) = nw_path_snapshot.as_ref() {
                    if snapshot.status == NWPathStatus::Satisfied
                        && (!snapshot.gateways.ipv4.is_empty()
                            || !snapshot.gateways.ipv6.is_empty())
                    {
                        let gateway = iface.gateway.get_or_insert_with(NetworkDevice::new);
                        merge_gateway(gateway, &snapshot.gateways);
                    }
                }
            }
        }
    }

    #[cfg(feature = "gateway")]
    {
        if let Some(idx) = default_idx {
            if let Some(iface) = ifaces.iter_mut().find(|it| it.index == idx) {
                iface.default = true;

                #[cfg(feature = "apple-system-configuration-extra")]
                {
                    iface.dns_servers = crate::os::ios::dns::get_system_dns_conf();
                }
                #[cfg(not(feature = "apple-system-configuration-extra"))]
                {
                    iface.dns_servers = crate::os::unix::dns::get_system_dns_conf();
                }
            }
        }
    }

    ifaces
}
