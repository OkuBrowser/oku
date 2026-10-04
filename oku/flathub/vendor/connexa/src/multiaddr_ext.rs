use crate::prelude::PeerId;
use libp2p::Multiaddr;
use libp2p::multiaddr::Protocol;

pub trait MultiaddrExt {
    fn is_relayed(&self) -> bool;

    fn is_public(&self) -> bool;

    fn is_loopback(&self) -> bool;

    fn is_private(&self) -> bool;

    fn is_unspecified(&self) -> bool;
    fn relay_peer_id(&self) -> Option<PeerId>;
}

impl MultiaddrExt for Multiaddr {
    fn is_relayed(&self) -> bool {
        self.iter().any(|protocol| protocol == Protocol::P2pCircuit)
    }

    fn is_public(&self) -> bool {
        !self.is_private() && !self.is_loopback() && !self.is_unspecified()
    }

    fn is_loopback(&self) -> bool {
        self.iter().any(|proto| match proto {
            Protocol::Ip4(ip) => ip.is_loopback(),
            Protocol::Ip6(ip) => ip.is_loopback(),
            _ => false,
        })
    }

    fn is_private(&self) -> bool {
        self.iter().any(|proto| match proto {
            Protocol::Ip4(ip) => ip.is_private(),
            Protocol::Ip6(ip) => {
                (ip.segments()[0] & 0xffc0) == 0xfe80 || (ip.segments()[0] & 0xfe00) == 0xfc00
            }
            _ => false,
        })
    }

    fn is_unspecified(&self) -> bool {
        self.iter().any(|proto| match proto {
            Protocol::Ip4(ip) => ip.is_unspecified(),
            Protocol::Ip6(ip) => ip.is_unspecified(),
            _ => false,
        })
    }

    fn relay_peer_id(&self) -> Option<PeerId> {
        let mut last_p2p = None;
        for proto in self.iter() {
            match proto {
                Protocol::P2p(peer) => last_p2p = Some(peer),
                Protocol::P2pCircuit => return last_p2p,
                _ => {}
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(s: &str) -> Multiaddr {
        s.parse().unwrap()
    }

    // is_relayed
    #[test]
    fn relayed_address() {
        let a = addr(
            "/ip4/1.2.3.4/tcp/4001/p2p/12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN/p2p-circuit",
        );
        assert!(a.is_relayed());
    }

    #[test]
    fn non_relayed_address() {
        let a = addr("/ip4/1.2.3.4/tcp/4001");
        assert!(!a.is_relayed());
    }

    #[test]
    fn loopback_ip() {
        assert!(addr("/ip4/127.0.0.1/tcp/8080").is_loopback());
        assert!(addr("/ip6/::1/tcp/8080").is_loopback());
    }

    #[test]
    fn not_loopback() {
        assert!(!addr("/ip4/192.168.1.1/tcp/8080").is_loopback());
    }

    #[test]
    fn private_ipv4() {
        assert!(addr("/ip4/10.0.0.1/tcp/8080").is_private());
        assert!(addr("/ip4/172.16.0.1/tcp/8080").is_private());
        assert!(addr("/ip4/192.168.0.1/tcp/8080").is_private());
    }

    #[test]
    fn private_ipv6() {
        assert!(addr("/ip6/fe80::1/tcp/8080").is_private());
        assert!(addr("/ip6/fd00::1/tcp/8080").is_private());
    }

    #[test]
    fn not_private_public_ip() {
        assert!(!addr("/ip4/8.8.8.8/tcp/8080").is_private());
        assert!(!addr("/ip6/2001:db8::1/tcp/8080").is_private());
    }

    #[test]
    fn unspecified_ipv4() {
        assert!(addr("/ip4/0.0.0.0/tcp/8080").is_unspecified());
    }

    #[test]
    fn unspecified_ipv6() {
        assert!(addr("/ip6/::/tcp/8080").is_unspecified());
    }

    #[test]
    fn not_unspecified() {
        assert!(!addr("/ip4/1.2.3.4/tcp/8080").is_unspecified());
    }

    #[test]
    fn public_ip() {
        assert!(addr("/ip4/8.8.8.8/tcp/8080").is_public());
        assert!(addr("/ip6/2001:db8::1/tcp/8080").is_public());
    }

    #[test]
    fn not_public_private() {
        assert!(!addr("/ip4/192.168.0.1/tcp/8080").is_public());
    }

    #[test]
    fn not_public_loopback() {
        assert!(!addr("/ip4/127.0.0.1/tcp/8080").is_public());
    }

    #[test]
    fn not_public_unspecified() {
        assert!(!addr("/ip4/0.0.0.0/tcp/8080").is_public());
    }

    #[test]
    fn relayed_public_address_is_public_and_relayed() {
        let a = addr(
            "/ip4/1.2.3.4/tcp/4001/p2p/12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN/p2p-circuit",
        );
        assert!(a.is_relayed());
        assert!(a.is_public());
    }
}
