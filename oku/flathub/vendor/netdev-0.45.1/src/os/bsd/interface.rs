use crate::{interface::interface::Interface, os::unix::interface::unix_interfaces};

pub fn interfaces() -> Vec<Interface> {
    #[cfg(not(feature = "gateway"))]
    {
        unix_interfaces()
    }
    #[cfg(feature = "gateway")]
    {
        use crate::os::unix::dns::get_system_dns_conf;

        let mut ifaces: Vec<Interface> = unix_interfaces();

        let gateway_map = super::route::get_gateway_map();

        for iface in &mut ifaces {
            if let Some(gateway) = gateway_map.get(&iface.index) {
                iface.gateway = Some(gateway.clone());
            }
        }
        if let Some(local_ip) = crate::net::ip::get_local_ipaddr() {
            if let Some(idx) = crate::interface::pick_default_iface_index(&ifaces, local_ip) {
                if let Some(iface) = ifaces.iter_mut().find(|it| it.index == idx) {
                    iface.default = true;
                    iface.dns_servers = get_system_dns_conf();
                }
            }
        }
        ifaces
    }
}
