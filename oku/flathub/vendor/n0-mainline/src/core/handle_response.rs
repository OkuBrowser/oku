use std::net::SocketAddrV4;

use tracing::{debug, trace};

use crate::{
    MutableItem, Node,
    common::{
        GetImmutableResponseArguments, GetMutableResponseArguments, GetPeersResponseArguments,
        GetSignedPeersResponseArguments, Id, Message, NoMoreRecentValueResponseArguments,
        NoValuesResponseArguments, RequestTypeSpecific, ResponseSpecific, SignedAnnounce,
        messages::MessageType, validate_immutable,
    },
    core::{Response, supports_signed_peers},
};

use super::Core;

impl Core {
    pub(crate) fn handle_response(
        &mut self,
        from: SocketAddrV4,
        message: Message,
    ) -> Option<(Id, Response)> {
        // If someone claims to be readonly, then let's not store anything even if they respond.
        if message.read_only {
            return None;
        };

        // If the response looks like a Ping response, check StoreQueries for the transaction_id.
        if let Some(query) = self
            .put_queries
            .values_mut()
            .find(|query| query.inflight(message.transaction_id))
        {
            match message.message_type {
                MessageType::Response(ResponseSpecific::Ping(_)) => {
                    // Mark storage at that node as a success.
                    query.success();
                }
                MessageType::Error(error) => query.error(error),
                _ => {}
            };

            return None;
        }

        let mut should_add_node = false;
        let author_id = message.get_author_id();
        let from_version = message.version.to_owned();

        // Get corresponding query for message.transaction_id
        if let Some(query) = self
            .iterative_queries
            .values_mut()
            .find(|query| query.inflight(message.transaction_id))
        {
            // KrpcSocket would not give us a response from the wrong address for the transaction_id
            should_add_node = true;

            if let Some(nodes) = message.get_closer_nodes() {
                for node in nodes {
                    query.add_candidate(node.clone());
                }
            }

            if let Some((responder_id, token)) = message.get_token() {
                query.add_responding_node(Node::new_with_token(responder_id, from, token.into()));
            }

            if let Some(proposed_ip) = message.requester_ip {
                query.add_address_vote(proposed_ip);
            }

            let target = query.target();

            match message.message_type {
                MessageType::Response(ResponseSpecific::GetPeers(GetPeersResponseArguments {
                    values,
                    ..
                })) => {
                    let response = Response::Peers(values);
                    query.response(from, response.clone());

                    return Some((target, response));
                }
                MessageType::Response(ResponseSpecific::GetSignedPeers(
                    GetSignedPeersResponseArguments {
                        responder_id,
                        peers,
                        ..
                    },
                )) => {
                    let mut verified_peers = vec![];
                    let mut malicious = false;

                    for (k, t, sig) in peers {
                        if let Ok(peer) = SignedAnnounce::from_dht_response(&target, &k, t, &sig) {
                            verified_peers.push(peer);
                        } else {
                            malicious = true;
                            break;
                        }
                    }

                    if malicious {
                        debug!(
                            ?from,
                            ?responder_id,
                            ?from_version,
                            "Invalid signed announce record"
                        );

                        should_add_node = false;
                    } else {
                        let response = Response::SignedPeers(verified_peers);
                        query.response(from, response.clone());

                        return Some((target, response));
                    }
                }
                MessageType::Response(ResponseSpecific::GetImmutable(
                    GetImmutableResponseArguments {
                        v, responder_id, ..
                    },
                )) => {
                    if validate_immutable(&v, query.target()) {
                        let response = Response::Immutable(v);
                        query.response(from, response.clone());

                        return Some((target, response));
                    }

                    let target = query.target();
                    debug!(
                        ?v,
                        ?target,
                        ?responder_id,
                        ?from,
                        ?from_version,
                        "Invalid immutable value"
                    );
                }
                MessageType::Response(ResponseSpecific::GetMutable(
                    GetMutableResponseArguments {
                        v,
                        seq,
                        sig,
                        k,
                        responder_id,
                        ..
                    },
                )) => {
                    let salt = match query.request.request_type.clone() {
                        RequestTypeSpecific::GetValue(args) => args.salt,
                        _ => None,
                    };
                    let target = query.target();

                    match MutableItem::from_dht_message(query.target(), &k, v, seq, &sig, salt) {
                        Ok(item) => {
                            let response = Response::Mutable(item);
                            query.response(from, response.clone());

                            return Some((target, response));
                        }
                        Err(error) => {
                            debug!(
                                ?error,
                                ?from,
                                ?responder_id,
                                ?from_version,
                                "Invalid mutable record"
                            );
                        }
                    }
                }
                MessageType::Response(ResponseSpecific::NoMoreRecentValue(
                    NoMoreRecentValueResponseArguments {
                        seq, responder_id, ..
                    },
                )) => {
                    trace!(
                        target= ?query.target(),
                        salt= ?match query.request.request_type.clone() {
                            RequestTypeSpecific::GetValue(args) => args.salt,
                            _ => None,
                        },
                        ?seq,
                        ?from,
                        ?responder_id,
                        ?from_version,
                        "No more recent value"
                    );
                }
                MessageType::Response(ResponseSpecific::NoValues(NoValuesResponseArguments {
                    responder_id,
                    ..
                })) => {
                    trace!(
                        target= ?query.target(),
                        salt= ?match query.request.request_type.clone() {
                            RequestTypeSpecific::GetValue(args) => args.salt,
                            _ => None,
                        },
                        ?from,
                        ?responder_id,
                        ?from_version ,
                        "No values"
                    );
                }
                MessageType::Error(error) => {
                    debug!(?error, ?from_version, "Get query got error response");
                }
                // Ping response is already handled in add_node()
                // FindNode response is already handled in query.add_candidate()
                // Requests are handled elsewhere
                MessageType::Response(ResponseSpecific::Ping(_))
                | MessageType::Response(ResponseSpecific::FindNode(_))
                | MessageType::Request(_) => {}
            };
        };

        if should_add_node {
            // Add a node to our routing table on any expected incoming response.

            if let Some(id) = author_id {
                self.routing_table.add(Node::new(id, from));

                if supports_signed_peers(message.version) {
                    self.signed_peers_routing_table.add(Node::new(id, from));
                }
            }
        }

        None
    }
}
