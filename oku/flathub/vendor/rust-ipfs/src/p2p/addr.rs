use connexa::prelude::{Multiaddr, PeerId, Protocol};

pub trait MultiaddrExt {
    /// Peer id
    fn peer_id(&self) -> Option<PeerId>;

    fn extract_peer_id(&mut self) -> Option<PeerId>;

    /// Relay peer id
    fn relay_peer_id(&self) -> Option<PeerId>;

    /// Address that only doesnt include peer protocols
    fn address(&self) -> Multiaddr;

    /// Determine if the address is a relay circuit
    fn is_relay(&self) -> bool;

    /// Determine if the address is being relayed to a peer
    fn is_relayed(&self) -> bool;

    /// Determine if address is loopback or local address
    fn is_loopback(&self) -> bool;

    /// Determine if address is private address
    fn is_private(&self) -> bool;
}

impl MultiaddrExt for Multiaddr {
    fn peer_id(&self) -> Option<PeerId> {
        if let Some(Protocol::P2p(peer)) = self.iter().last() {
            return Some(peer);
        }

        None
    }

    fn extract_peer_id(&mut self) -> Option<PeerId> {
        self.peer_id()?;

        match self.pop() {
            Some(Protocol::P2p(peer)) => Some(peer),
            _ => None,
        }
    }

    fn relay_peer_id(&self) -> Option<PeerId> {
        if !self.is_relay() {
            return None;
        }

        while let Some(protocol) = self.iter().next() {
            //Find the first peer id and return it
            if let Protocol::P2p(peer) = protocol {
                return Some(peer);
            }
        }

        None
    }

    fn address(&self) -> Multiaddr {
        let mut addr = Multiaddr::empty();

        for proto in self.iter() {
            if matches!(
                proto,
                Protocol::Ip4(_)
                    | Protocol::Ip6(_)
                    | Protocol::Tcp(_)
                    | Protocol::Udp(_)
                    | Protocol::Quic
                    | Protocol::QuicV1
                    | Protocol::Dnsaddr(_)
            ) {
                addr.push(proto);
            }
        }

        addr
    }

    fn is_relay(&self) -> bool {
        self.iter()
            .any(|proto| matches!(proto, Protocol::P2pCircuit))
    }

    fn is_relayed(&self) -> bool {
        if !self.is_relay() {
            return false;
        }

        if self.peer_id().is_none() {
            return false;
        }

        true
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
                (ip.segments()[0] & 0xffc0) != 0xfe80 && (ip.segments()[0] & 0xfe00) != 0xfc00
            }
            _ => false,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn connection_targets() {
        let peer_id = PeerId::from_str("QmaCpDMGvV2BGHeYERUEnRQAwe3N8SzbUtfsmvsqQLuvuJ")
            .expect("Valid peer id");
        let multiaddr_wo_peer =
            Multiaddr::from_str("/ip4/104.131.131.82/tcp/4001").expect("Valid multiaddr");
        let multiaddr_with_peer =
            Multiaddr::from_str(&format!("{multiaddr_wo_peer}/p2p/{peer_id}"))
                .expect("valid multiaddr");
        let p2p_peer = Multiaddr::from_str(&format!("/p2p/{peer_id}")).expect("Valid multiaddr");

        assert!(multiaddr_wo_peer.peer_id().is_none());
        assert!(multiaddr_with_peer.peer_id().is_some());
        assert!(p2p_peer.peer_id().is_some());

        let peer_id_target = multiaddr_with_peer.peer_id().expect("Valid peer id");

        assert_eq!(peer_id_target, peer_id);
    }
}
