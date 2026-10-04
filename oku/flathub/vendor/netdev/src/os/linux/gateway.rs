use std::collections::HashMap;

#[derive(Default)]
struct MacCandidates {
    ipv4: Option<[u8; 6]>,
    ipv6: Option<[u8; 6]>,
}

#[derive(Default)]
pub(crate) struct GatewayMacCandidates {
    interfaces: HashMap<u32, MacCandidates>,
}

impl GatewayMacCandidates {
    pub(crate) fn record_ipv4(&mut self, ifindex: u32, mac: [u8; 6]) {
        self.interfaces
            .entry(ifindex)
            .or_default()
            .ipv4
            .get_or_insert(mac);
    }

    pub(crate) fn record_ipv6(&mut self, ifindex: u32, mac: [u8; 6]) {
        self.interfaces
            .entry(ifindex)
            .or_default()
            .ipv6
            .get_or_insert(mac);
    }

    pub(crate) fn get(&self, ifindex: u32) -> Option<[u8; 6]> {
        let candidates = self.interfaces.get(&ifindex)?;
        candidates.ipv4.or(candidates.ipv6)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefers_ipv4_independently_of_neighbor_order() {
        let ipv4_mac = [0, 1, 2, 3, 4, 5];
        let ipv6_mac = [6, 7, 8, 9, 10, 11];

        let mut ipv6_first = GatewayMacCandidates::default();
        ipv6_first.record_ipv6(2, ipv6_mac);
        ipv6_first.record_ipv4(2, ipv4_mac);

        let mut ipv4_first = GatewayMacCandidates::default();
        ipv4_first.record_ipv4(2, ipv4_mac);
        ipv4_first.record_ipv6(2, ipv6_mac);

        assert_eq!(ipv6_first.get(2), Some(ipv4_mac));
        assert_eq!(ipv4_first.get(2), Some(ipv4_mac));
    }

    #[test]
    fn falls_back_to_ipv6_and_keeps_interfaces_separate() {
        let ipv6_mac = [6, 7, 8, 9, 10, 11];
        let other_mac = [12, 13, 14, 15, 16, 17];
        let mut candidates = GatewayMacCandidates::default();
        candidates.record_ipv6(2, ipv6_mac);
        candidates.record_ipv4(3, other_mac);

        assert_eq!(candidates.get(2), Some(ipv6_mac));
        assert_eq!(candidates.get(3), Some(other_mac));
        assert_eq!(candidates.get(4), None);
    }
}
