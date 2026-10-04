use std::net::SocketAddrV4;

use tracing::info;

use crate::RequestSpecific;
use crate::common::messages::MessageType;
use crate::core::Core;
use crate::core::supports_signed_peers;

use crate::common::{Id, Node, RequestTypeSpecific};

impl Core {
    pub fn handle_request(
        &mut self,
        from: SocketAddrV4,
        request_from_read_only_node: bool,
        version: Option<[u8; 4]>,
        request_specific: RequestSpecific,
    ) -> (Option<MessageType>, bool) {
        self.maybe_add_node_from_request(
            from,
            version,
            request_from_read_only_node,
            &request_specific,
        );

        let should_repopulate_routing_tables =
            self.does_verify_our_new_public_address_with_self_ping(from, &request_specific);

        let response = if self.server_mode {
            let server = &mut self.server;
            let response = server.handle_request(
                &self.routing_table,
                &self.signed_peers_routing_table,
                from,
                request_specific,
            );

            match response {
                Some(MessageType::Error(_)) | Some(MessageType::Response(_)) => response,
                _ => None,
            }
        } else {
            None
        };

        (response, should_repopulate_routing_tables)
    }

    /// In client mode, we never add to our table any node contacting us
    /// without querying it first, to avoid eclipse attacks.
    ///
    /// While bootstrapping though, we can't be that picky, we have two
    /// reasons to except that rule:
    ///
    /// 1. first node creating the DHT;
    ///    without this exception, the bootstrapping node's routing table will never be populated.
    /// 2. Bootstrapping signed_peers_routing_table requires that we latch to any node
    ///    that claims to support it.
    ///    In `periodic_node_maintaenance` fake unresponsive nodes will be removed.
    ///    And either way we prioritize secure nodes, so making up nodes from same
    //    machine won't have much effect.
    fn maybe_add_node_from_request(
        &mut self,
        from: SocketAddrV4,
        version: Option<[u8; 4]>,
        request_from_read_only_node: bool,
        request_specific: &RequestSpecific,
    ) {
        if self.server_mode
            && !request_from_read_only_node
            && let RequestTypeSpecific::FindNode(ref param) = request_specific.request_type
        {
            let node = Node::new(param.target, from);
            let supports_signed_peers = supports_signed_peers(version);

            if self.bootstrap.is_empty() {
                self.routing_table.add(node.clone());

                if supports_signed_peers {
                    self.signed_peers_routing_table.add(node);
                }
            } else if supports_signed_peers {
                self.signed_peers_routing_table.add(node);
            }
        }
    }

    /// Check an incoming request, and if it is a PING request, with our own
    /// id and from our own address, then we assume that the ip address we are being
    /// told is valid.
    // TODO: can this be spoofed? should we check the `tid` as well to be sure?
    fn does_verify_our_new_public_address_with_self_ping(
        &mut self,
        from: SocketAddrV4,
        request_specific: &RequestSpecific,
    ) -> bool {
        if let Some(our_address) = self.public_address {
            let is_ping = matches!(request_specific.request_type, RequestTypeSpecific::Ping);

            if from == our_address && is_ping {
                self.firewalled = false;

                let ipv4 = our_address.ip();

                // Restarting our routing table with new secure Id if necessary.
                if !self.id().is_valid_for_ip(*ipv4) {
                    let new_id = Id::from_ipv4(*ipv4);

                    info!(
                        "Our current id {} is not valid for adrsess {}. Using new id {}",
                        self.id(),
                        our_address,
                        new_id
                    );

                    self.routing_table.reset_id(new_id);
                    self.signed_peers_routing_table.reset_id(new_id);

                    return true;
                }
            }
        }

        false
    }
}
