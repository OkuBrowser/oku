use crate::interface::types::InterfaceType;

pub fn guess_type_by_name(name: &str) -> Option<InterfaceType> {
    let n = name.as_bytes();

    if n == b"lo" {
        return Some(InterfaceType::Loopback);
    }
    if n.starts_with(b"wlan") || n.starts_with(b"wifi") {
        return Some(InterfaceType::Wireless80211);
    }
    if n.starts_with(b"p2p") || n.starts_with(b"swlan") {
        return Some(InterfaceType::PeerToPeerWireless);
    }
    if n.starts_with(b"rmnet")
        || n.starts_with(b"rmnet_data")
        || n.starts_with(b"ccmni")
        || n.starts_with(b"pdp")
    {
        return Some(InterfaceType::Wwanpp);
    }
    if n.starts_with(b"tun")
        || n.starts_with(b"tap")
        || n.starts_with(b"ipsec")
        || n.starts_with(b"clat")
        || n.starts_with(b"v4-")
    {
        return Some(InterfaceType::Tunnel);
    }
    if n.starts_with(b"br-") || n.starts_with(b"bridge") {
        return Some(InterfaceType::Bridge);
    }
    if n.starts_with(b"veth") {
        return Some(InterfaceType::ProprietaryVirtual);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::guess_type_by_name;
    use crate::interface::types::InterfaceType;

    #[test]
    fn test_name_guess_lo() {
        assert_eq!(guess_type_by_name("lo"), Some(InterfaceType::Loopback));
    }

    #[test]
    fn test_name_guess_wlan() {
        assert_eq!(
            guess_type_by_name("wlan0"),
            Some(InterfaceType::Wireless80211)
        );
        assert_eq!(
            guess_type_by_name("wifi-aware0"),
            Some(InterfaceType::Wireless80211)
        );
    }

    #[test]
    fn test_name_guess_peer_to_peer_wireless() {
        assert_eq!(
            guess_type_by_name("p2p0"),
            Some(InterfaceType::PeerToPeerWireless)
        );
        assert_eq!(
            guess_type_by_name("swlan0"),
            Some(InterfaceType::PeerToPeerWireless)
        );
    }

    #[test]
    fn test_name_guess_cellular() {
        assert_eq!(
            guess_type_by_name("rmnet_data0"),
            Some(InterfaceType::Wwanpp)
        );
        assert_eq!(guess_type_by_name("ccmni0"), Some(InterfaceType::Wwanpp));
        assert_eq!(guess_type_by_name("pdp0"), Some(InterfaceType::Wwanpp));
    }

    #[test]
    fn test_name_guess_tunnel() {
        assert_eq!(guess_type_by_name("tun0"), Some(InterfaceType::Tunnel));
        assert_eq!(guess_type_by_name("tap0"), Some(InterfaceType::Tunnel));
        assert_eq!(guess_type_by_name("ipsec0"), Some(InterfaceType::Tunnel));
        assert_eq!(guess_type_by_name("clat4"), Some(InterfaceType::Tunnel));
        assert_eq!(guess_type_by_name("v4-wlan0"), Some(InterfaceType::Tunnel));
    }

    #[test]
    fn test_name_guess_bridge_and_virtual() {
        assert_eq!(guess_type_by_name("br-lan"), Some(InterfaceType::Bridge));
        assert_eq!(guess_type_by_name("bridge0"), Some(InterfaceType::Bridge));
        assert_eq!(
            guess_type_by_name("veth1234"),
            Some(InterfaceType::ProprietaryVirtual)
        );
    }

    #[test]
    fn test_name_guess_ambiguous_usb() {
        assert_eq!(guess_type_by_name("usb0"), None);
    }
}
