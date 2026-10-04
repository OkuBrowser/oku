use crate::{
    guardian,
    socket::{Mode, Sockets},
    updater, Discoverer, Peer,
};
use acto::{AcTokioRuntime, ActoCell, ActoInput, ActoRef};
use hickory_proto::{
    op::{Message, MessageType, OpCode, Query},
    rr::{
        rdata::{self, TXT},
        DNSClass, Name, RData, Record, RecordType,
    },
};
use rand::{rng, RngExt};
use std::{collections::BTreeMap, net::IpAddr, str::FromStr, time::Duration};

const RESPONSE_DELAY: Duration = Duration::from_millis(100);

pub enum MdnsMsg {
    QueryV4,
    QueryV6,
    Response(BTreeMap<String, Peer>),
    Timeout(usize),
    SizeUpdate(usize),
    Update(guardian::Input),
}

pub async fn sender(
    mut ctx: ActoCell<MdnsMsg, AcTokioRuntime>,
    sockets: Sockets,
    updater: ActoRef<updater::Input>,
    mut discoverer: Discoverer,
    service_name: Name,
) {
    let tau = discoverer.tau;
    let phi = discoverer.phi;
    let cutoff = (tau.as_secs_f32() * phi).ceil() as u32;

    let query = make_query(&service_name);
    let mut response = make_response(&discoverer, &service_name);

    let mut timeout_count = 0;

    updater.send(updater::Input::SizeSubscription(
        ctx.me().contramap(MdnsMsg::SizeUpdate),
    ));

    let mut swarm_size = 1;
    let mut extra_delay = Duration::ZERO;
    let mut has_responded = false;

    loop {
        let me = ctx.me();
        let timeout = tokio::spawn(async move {
            // grow the interval from which the randomized part is draw
            // with the swarm size to keep the number of duplicates low
            let interval = tau * swarm_size as u32 / 10;
            let millionth = rng().random_range(0..1_000_000);
            let delay = tau + interval / 1_000_000 * millionth;
            tracing::debug!(?delay, "waiting for query");
            tokio::time::sleep(delay).await;
            me.send(MdnsMsg::Timeout(timeout_count));
        });

        let mode = loop {
            if let ActoInput::Message(msg) = ctx.recv().await {
                match msg {
                    MdnsMsg::QueryV4 => {
                        timeout.abort();
                        break Mode::V4;
                    }
                    MdnsMsg::QueryV6 => {
                        timeout.abort();
                        break Mode::V6;
                    }
                    MdnsMsg::Response(resp) => {
                        updater.send(updater::Input::Peers(resp));
                    }
                    MdnsMsg::Timeout(count) if count == timeout_count => {
                        sockets.send_msg(&query, Mode::Any).await;
                        break Mode::Any;
                    }
                    MdnsMsg::Timeout(_) => {}
                    MdnsMsg::SizeUpdate(size) => {
                        swarm_size = size;
                    }
                    MdnsMsg::Update(msg) => {
                        response = update_response(&mut discoverer, &service_name, msg);
                    }
                }
            } else {
                return;
            }
        };

        timeout_count += 1;

        let me = ctx.me();
        // for fairness: if we have sent and the swarm is large, delay some more
        if has_responded {
            extra_delay = RESPONSE_DELAY * (swarm_size as u32 / cutoff).min(10);
        } else {
            extra_delay = extra_delay.checked_sub(RESPONSE_DELAY).unwrap_or_default();
        }
        let timeout = tokio::spawn(async move {
            // grow the interval from which the randomized part is draw
            // with the swarm size to keep the number of duplicates low
            // goal is "cutoff within 100ms"
            let interval = RESPONSE_DELAY * swarm_size as u32 / cutoff;
            let millionth = rng().random_range(0..1_000_000);
            let mut delay = interval / 1_000_000 * millionth;
            delay += extra_delay;
            tracing::debug!(?delay, "waiting to respond");
            tokio::time::sleep(delay).await;
            me.send(MdnsMsg::Timeout(timeout_count));
        });

        let mut response_count = 0;
        has_responded = false;
        loop {
            if let ActoInput::Message(msg) = ctx.recv().await {
                match msg {
                    MdnsMsg::Response(resp) => {
                        response_count += resp.len() as u32;
                        updater.send(updater::Input::Peers(resp));
                        if response_count >= cutoff {
                            timeout.abort();
                            break;
                        }
                    }
                    MdnsMsg::Timeout(count) if count == timeout_count => {
                        if let Some(response) = &response {
                            sockets.send_msg(response, mode).await;
                            has_responded = true;
                        }
                        break;
                    }
                    MdnsMsg::SizeUpdate(size) => {
                        swarm_size = size;
                    }
                    MdnsMsg::Update(msg) => {
                        response = update_response(&mut discoverer, &service_name, msg);
                    }
                    MdnsMsg::QueryV4 => {}
                    MdnsMsg::QueryV6 => {}
                    MdnsMsg::Timeout(_) => {}
                }
            }
        }

        timeout_count += 1;
    }
}

fn make_query(service_name: &Name) -> Message {
    let mut msg = Message::new(0, MessageType::Query, OpCode::Query);
    let mut query = Query::new();
    query.set_query_class(DNSClass::IN);
    query.set_query_type(RecordType::PTR);
    query.set_name(service_name.clone());
    msg.add_query(query);
    msg
}

fn make_response(discoverer: &Discoverer, service_name: &Name) -> Option<Message> {
    if let Some(peer) = discoverer.peers.get(&discoverer.peer_id) {
        let mut msg = Message::new(0, MessageType::Response, OpCode::Query);
        msg.metadata.authoritative = true;

        let my_srv_name = Name::from_str(&discoverer.peer_id)
            .expect("PeerId was checked in spawn()")
            .append_domain(service_name)
            .expect("was checked in spawn()");

        let mut srv_map = BTreeMap::new();
        for (ip, port) in &peer.addrs {
            srv_map.entry(*port).or_insert_with(Vec::new).push(*ip);
        }

        for (port, addrs) in srv_map {
            let target = Name::from_str(&format!("{}-{}.local.", discoverer.peer_id, port))
                .expect("PeerId was checked in spawn()");
            msg.add_answer(Record::from_rdata(
                my_srv_name.clone(),
                0,
                RData::SRV(rdata::SRV::new(0, 0, port, target.clone())),
            ));
            for addr in addrs {
                match addr {
                    IpAddr::V4(addr) => {
                        msg.add_additional(Record::from_rdata(
                            target.clone(),
                            0,
                            RData::A(rdata::A::from(addr)),
                        ));
                    }
                    IpAddr::V6(addr) => {
                        msg.add_additional(Record::from_rdata(
                            target.clone(),
                            0,
                            RData::AAAA(rdata::AAAA::from(addr)),
                        ));
                    }
                }
            }
        }
        if !peer.txt.is_empty() {
            let parts = peer
                .txt
                .iter()
                .filter_map(|(k, v)| {
                    if k.is_empty() {
                        None
                    } else {
                        Some(match v {
                            None => k.to_string(),
                            Some(v) => format!("{k}={v}"),
                        })
                    }
                })
                .collect();
            let rdata = TXT::new(parts);
            let record = Record::from_rdata(my_srv_name, 0, RData::TXT(rdata));
            msg.add_answer(record);
        }
        Some(msg)
    } else {
        tracing::info!("no addresses for peer, not announcing");
        None
    }
}

fn update_response(
    discoverer: &mut Discoverer,
    service_name: &Name,
    msg: guardian::Input,
) -> Option<Message> {
    match msg {
        guardian::Input::RemoveAll => {
            discoverer.peers.remove(&discoverer.peer_id);
            make_response(discoverer, service_name)
        }
        guardian::Input::RemovePort(port) => {
            if let Some(peers) = discoverer.peers.get_mut(&discoverer.peer_id) {
                peers.addrs.retain(|(_, p)| *p != port);
            }
            make_response(discoverer, service_name)
        }
        guardian::Input::RemoveAddr(addr) => {
            if let Some(peers) = discoverer.peers.get_mut(&discoverer.peer_id) {
                peers.addrs.retain(|(a, _)| *a != addr);
            }
            make_response(discoverer, service_name)
        }
        guardian::Input::AddAddr(port, addrs) => {
            let peer = discoverer
                .peers
                .entry(discoverer.peer_id.clone())
                .or_insert_with(Peer::new);
            for addr in addrs {
                peer.addrs.push((addr, port));
                peer.addrs.sort_unstable();
                peer.addrs.dedup();
            }
            make_response(discoverer, service_name)
        }
        guardian::Input::SetTxt(key, value) => {
            let peer = discoverer
                .peers
                .entry(discoverer.peer_id.clone())
                .or_insert_with(Peer::new);
            peer.txt.insert(key, value);
            make_response(discoverer, service_name)
        }
        guardian::Input::RemoveTxt(key) => {
            if let Some(peer) = discoverer.peers.get_mut(&discoverer.peer_id) {
                let _ = peer.txt.remove(&key);
                make_response(discoverer, service_name)
            } else {
                None
            }
        }
        guardian::Input::AddInterface(_addr) => {
            // Interface changes don't affect the response content
            None
        }
        guardian::Input::RemoveInterface(_addr) => {
            // Interface changes don't affect the response content
            None
        }
    }
}
