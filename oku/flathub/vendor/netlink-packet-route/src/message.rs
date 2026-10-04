// SPDX-License-Identifier: MIT

use netlink_packet_core::{
    DecodeError, Emitable, ErrorContext, NetlinkDeserializable, NetlinkHeader,
    NetlinkPayload, NetlinkSerializable, Parseable, ParseableParametrized,
};

use crate::{
    address::{AddressHeader, AddressMessage},
    link::LinkMessage,
    neighbour::NeighbourMessage,
    neighbour_table::NeighbourTableMessage,
    nsid::NsidMessage,
    prefix::PrefixMessage,
    route::{RouteHeader, RouteMessage},
    rule::RuleMessage,
    stats::StatsMessage,
    tc::{TcActionMessage, TcMessage},
};

const RTM_NEWLINK: u16 = 16;
const RTM_DELLINK: u16 = 17;
const RTM_GETLINK: u16 = 18;
const RTM_SETLINK: u16 = 19;
const RTM_NEWADDR: u16 = 20;
const RTM_DELADDR: u16 = 21;
const RTM_GETADDR: u16 = 22;
const RTM_NEWROUTE: u16 = 24;
const RTM_DELROUTE: u16 = 25;
const RTM_GETROUTE: u16 = 26;
const RTM_NEWNEIGH: u16 = 28;
const RTM_DELNEIGH: u16 = 29;
const RTM_GETNEIGH: u16 = 30;
const RTM_NEWRULE: u16 = 32;
const RTM_DELRULE: u16 = 33;
const RTM_GETRULE: u16 = 34;
const RTM_NEWQDISC: u16 = 36;
const RTM_DELQDISC: u16 = 37;
const RTM_GETQDISC: u16 = 38;
const RTM_NEWTCLASS: u16 = 40;
const RTM_DELTCLASS: u16 = 41;
const RTM_GETTCLASS: u16 = 42;
const RTM_NEWTFILTER: u16 = 44;
const RTM_DELTFILTER: u16 = 45;
const RTM_GETTFILTER: u16 = 46;
const RTM_NEWACTION: u16 = 48;
const RTM_DELACTION: u16 = 49;
const RTM_GETACTION: u16 = 50;
const RTM_NEWPREFIX: u16 = 52;
// const RTM_GETMULTICAST: u16 = 58;
// const RTM_GETANYCAST: u16 = 62;
const RTM_NEWNEIGHTBL: u16 = 64;
const RTM_GETNEIGHTBL: u16 = 66;
const RTM_SETNEIGHTBL: u16 = 67;
// const RTM_NEWNDUSEROPT: u16 = 68;
// const RTM_NEWADDRLABEL: u16 = 72;
// const RTM_DELADDRLABEL: u16 = 73;
// const RTM_GETADDRLABEL: u16 = 74;
// const RTM_GETDCB: u16 = 78;
// const RTM_SETDCB: u16 = 79;
// const RTM_NEWNETCONF: u16 = 80;
// const RTM_DELNETCONF: u16 = 81;
// const RTM_GETNETCONF: u16 = 82;
// const RTM_NEWMDB: u16 = 84;
// const RTM_DELMDB: u16 = 85;
// const RTM_GETMDB: u16 = 86;
const RTM_NEWNSID: u16 = 88;
const RTM_DELNSID: u16 = 89;
const RTM_GETNSID: u16 = 90;
const RTM_NEWSTATS: u16 = 92;
const RTM_GETSTATS: u16 = 94;
// const RTM_NEWCACHEREPORT: u16 = 96;
const RTM_NEWCHAIN: u16 = 100;
const RTM_DELCHAIN: u16 = 101;
const RTM_GETCHAIN: u16 = 102;
const RTM_NEWLINKPROP: u16 = 108;
const RTM_DELLINKPROP: u16 = 109;

impl ParseableParametrized<[u8], u16> for RouteNetlinkMessage {
    fn parse_with_param(
        buf: &[u8],
        message_type: u16,
    ) -> Result<Self, DecodeError> {
        let message = match message_type {
            // Link messages
            RTM_NEWLINK | RTM_GETLINK | RTM_DELLINK | RTM_SETLINK => {
                // HACK: iproute2 sends invalid RTM_GETLINK message, where
                // the header is limited to the interface family (1 byte) and
                // 3 bytes of padding.
                let msg = if buf.len() == 4 && message_type == RTM_GETLINK {
                    let mut msg = LinkMessage::default();
                    msg.header.interface_family = buf[0].into();
                    msg
                } else {
                    LinkMessage::parse(buf).context("invalid link message")?
                };
                match message_type {
                    RTM_NEWLINK => RouteNetlinkMessage::NewLink(msg),
                    RTM_GETLINK => RouteNetlinkMessage::GetLink(msg),
                    RTM_DELLINK => RouteNetlinkMessage::DelLink(msg),
                    RTM_SETLINK => RouteNetlinkMessage::SetLink(msg),
                    _ => unreachable!(),
                }
            }

            // Address messages
            RTM_NEWADDR | RTM_GETADDR | RTM_DELADDR => {
                // HACK: iproute2 sends invalid RTM_GETADDR message, where
                // the header is limited to the interface family (1 byte) and
                // 3 bytes of padding.
                let msg = if buf.len() == 4 && message_type == RTM_GETADDR {
                    let mut msg = AddressMessage {
                        header: AddressHeader::default(),
                        attributes: vec![],
                    };
                    msg.header.family = buf[0].into();
                    msg
                } else {
                    AddressMessage::parse(buf)
                        .context("invalid address message")?
                };
                match message_type {
                    RTM_NEWADDR => RouteNetlinkMessage::NewAddress(msg),
                    RTM_GETADDR => RouteNetlinkMessage::GetAddress(msg),
                    RTM_DELADDR => RouteNetlinkMessage::DelAddress(msg),
                    _ => unreachable!(),
                }
            }

            // Neighbour messages
            RTM_NEWNEIGH | RTM_GETNEIGH | RTM_DELNEIGH => {
                let err = "invalid neighbour message";
                let msg = NeighbourMessage::parse(buf).context(err)?;
                match message_type {
                    RTM_GETNEIGH => RouteNetlinkMessage::GetNeighbour(msg),
                    RTM_NEWNEIGH => RouteNetlinkMessage::NewNeighbour(msg),
                    RTM_DELNEIGH => RouteNetlinkMessage::DelNeighbour(msg),
                    _ => unreachable!(),
                }
            }

            // Neighbour table messages
            RTM_NEWNEIGHTBL | RTM_GETNEIGHTBL | RTM_SETNEIGHTBL => {
                let err = "invalid neighbour table message";
                let msg = NeighbourTableMessage::parse(buf).context(err)?;
                match message_type {
                    RTM_GETNEIGHTBL => {
                        RouteNetlinkMessage::GetNeighbourTable(msg)
                    }
                    RTM_NEWNEIGHTBL => {
                        RouteNetlinkMessage::NewNeighbourTable(msg)
                    }
                    RTM_SETNEIGHTBL => {
                        RouteNetlinkMessage::SetNeighbourTable(msg)
                    }
                    _ => unreachable!(),
                }
            }

            // Route messages
            RTM_NEWROUTE | RTM_GETROUTE | RTM_DELROUTE => {
                // HACK: iproute2 sends invalid RTM_GETROUTE message, where
                // the header is limited to the interface family (1 byte) and
                // 3 bytes of padding.
                //
                // Not only does iproute2 sends invalid messages, it's also
                // inconsistent in doing so: for link and address messages, the
                // length advertised in the netlink header includes the 3 bytes
                // of padding but it does not seem to be the case for the route
                // message, hence the buf.length() == 1 check.
                let msg = if (buf.len() == 4 || buf.len() == 1)
                    && message_type == RTM_GETROUTE
                {
                    let mut msg = RouteMessage {
                        header: RouteHeader::default(),
                        attributes: vec![],
                    };
                    msg.header.address_family = buf[0].into();
                    msg
                } else {
                    RouteMessage::parse(buf).context("invalid route message")?
                };
                match message_type {
                    RTM_NEWROUTE => RouteNetlinkMessage::NewRoute(msg),
                    RTM_GETROUTE => RouteNetlinkMessage::GetRoute(msg),
                    RTM_DELROUTE => RouteNetlinkMessage::DelRoute(msg),
                    _ => unreachable!(),
                }
            }

            // Prefix messages
            RTM_NEWPREFIX => {
                let err = "invalid prefix message";
                RouteNetlinkMessage::NewPrefix(
                    PrefixMessage::parse(buf).context(err)?,
                )
            }

            RTM_NEWRULE | RTM_GETRULE | RTM_DELRULE => {
                let err = "invalid fib rule message";
                let msg = RuleMessage::parse(buf).context(err)?;
                match message_type {
                    RTM_NEWRULE => RouteNetlinkMessage::NewRule(msg),
                    RTM_DELRULE => RouteNetlinkMessage::DelRule(msg),
                    RTM_GETRULE => RouteNetlinkMessage::GetRule(msg),
                    _ => unreachable!(),
                }
            }
            // TC Messages
            RTM_NEWQDISC | RTM_DELQDISC | RTM_GETQDISC | RTM_NEWTCLASS
            | RTM_DELTCLASS | RTM_GETTCLASS | RTM_NEWTFILTER
            | RTM_DELTFILTER | RTM_GETTFILTER | RTM_NEWCHAIN | RTM_DELCHAIN
            | RTM_GETCHAIN => {
                let err = "invalid tc message";
                let msg = TcMessage::parse(buf).context(err)?;
                match message_type {
                    RTM_NEWQDISC => {
                        RouteNetlinkMessage::NewQueueDiscipline(msg)
                    }
                    RTM_DELQDISC => {
                        RouteNetlinkMessage::DelQueueDiscipline(msg)
                    }
                    RTM_GETQDISC => {
                        RouteNetlinkMessage::GetQueueDiscipline(msg)
                    }
                    RTM_NEWTCLASS => RouteNetlinkMessage::NewTrafficClass(msg),
                    RTM_DELTCLASS => RouteNetlinkMessage::DelTrafficClass(msg),
                    RTM_GETTCLASS => RouteNetlinkMessage::GetTrafficClass(msg),
                    RTM_NEWTFILTER => {
                        RouteNetlinkMessage::NewTrafficFilter(msg)
                    }
                    RTM_DELTFILTER => {
                        RouteNetlinkMessage::DelTrafficFilter(msg)
                    }
                    RTM_GETTFILTER => {
                        RouteNetlinkMessage::GetTrafficFilter(msg)
                    }
                    RTM_NEWCHAIN => RouteNetlinkMessage::NewTrafficChain(msg),
                    RTM_DELCHAIN => RouteNetlinkMessage::DelTrafficChain(msg),
                    RTM_GETCHAIN => RouteNetlinkMessage::GetTrafficChain(msg),
                    _ => unreachable!(),
                }
            }

            RTM_NEWACTION | RTM_DELACTION | RTM_GETACTION => {
                let err = "invalid tc action message";
                let msg = TcActionMessage::parse(buf).context(err)?;
                match message_type {
                    RTM_NEWACTION => RouteNetlinkMessage::NewTrafficAction(msg),
                    RTM_DELACTION => RouteNetlinkMessage::DelTrafficAction(msg),
                    RTM_GETACTION => RouteNetlinkMessage::GetTrafficAction(msg),
                    _ => unreachable!(),
                }
            }

            // Stats messages
            RTM_NEWSTATS | RTM_GETSTATS => {
                let err = "invalid stats message";
                let msg = StatsMessage::parse(buf).context(err)?;
                match message_type {
                    RTM_NEWSTATS => RouteNetlinkMessage::NewStats(msg),
                    RTM_GETSTATS => RouteNetlinkMessage::GetStats(msg),
                    _ => unreachable!(),
                }
            }

            // ND ID Messages
            RTM_NEWNSID | RTM_GETNSID | RTM_DELNSID => {
                let err = "invalid nsid message";
                let msg = NsidMessage::parse(buf).context(err)?;
                match message_type {
                    RTM_NEWNSID => RouteNetlinkMessage::NewNsId(msg),
                    RTM_DELNSID => RouteNetlinkMessage::DelNsId(msg),
                    RTM_GETNSID => RouteNetlinkMessage::GetNsId(msg),
                    _ => unreachable!(),
                }
            }

            _ => {
                return Err(
                    format!("Unknown message type: {message_type}").into()
                )
            }
        };
        Ok(message)
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[non_exhaustive]
pub enum RouteNetlinkMessage {
    NewLink(LinkMessage),
    DelLink(LinkMessage),
    GetLink(LinkMessage),
    SetLink(LinkMessage),
    NewLinkProp(LinkMessage),
    DelLinkProp(LinkMessage),
    NewAddress(AddressMessage),
    DelAddress(AddressMessage),
    GetAddress(AddressMessage),
    NewNeighbour(NeighbourMessage),
    GetNeighbour(NeighbourMessage),
    DelNeighbour(NeighbourMessage),
    NewNeighbourTable(NeighbourTableMessage),
    GetNeighbourTable(NeighbourTableMessage),
    SetNeighbourTable(NeighbourTableMessage),
    NewRoute(RouteMessage),
    DelRoute(RouteMessage),
    GetRoute(RouteMessage),
    NewPrefix(PrefixMessage),
    NewQueueDiscipline(TcMessage),
    DelQueueDiscipline(TcMessage),
    GetQueueDiscipline(TcMessage),
    NewTrafficClass(TcMessage),
    DelTrafficClass(TcMessage),
    GetTrafficClass(TcMessage),
    NewTrafficFilter(TcMessage),
    DelTrafficFilter(TcMessage),
    GetTrafficFilter(TcMessage),
    NewTrafficAction(TcActionMessage),
    DelTrafficAction(TcActionMessage),
    GetTrafficAction(TcActionMessage),
    NewTrafficChain(TcMessage),
    DelTrafficChain(TcMessage),
    GetTrafficChain(TcMessage),
    NewStats(StatsMessage),
    GetStats(StatsMessage),
    NewNsId(NsidMessage),
    DelNsId(NsidMessage),
    GetNsId(NsidMessage),
    NewRule(RuleMessage),
    DelRule(RuleMessage),
    GetRule(RuleMessage),
}

impl RouteNetlinkMessage {
    pub fn is_new_link(&self) -> bool {
        matches!(self, RouteNetlinkMessage::NewLink(_))
    }

    pub fn is_del_link(&self) -> bool {
        matches!(self, RouteNetlinkMessage::DelLink(_))
    }

    pub fn is_get_link(&self) -> bool {
        matches!(self, RouteNetlinkMessage::GetLink(_))
    }

    pub fn is_set_link(&self) -> bool {
        matches!(self, RouteNetlinkMessage::SetLink(_))
    }

    pub fn is_new_address(&self) -> bool {
        matches!(self, RouteNetlinkMessage::NewAddress(_))
    }

    pub fn is_del_address(&self) -> bool {
        matches!(self, RouteNetlinkMessage::DelAddress(_))
    }

    pub fn is_get_address(&self) -> bool {
        matches!(self, RouteNetlinkMessage::GetAddress(_))
    }

    pub fn is_get_neighbour(&self) -> bool {
        matches!(self, RouteNetlinkMessage::GetNeighbour(_))
    }

    pub fn is_new_route(&self) -> bool {
        matches!(self, RouteNetlinkMessage::NewRoute(_))
    }

    pub fn is_new_neighbour(&self) -> bool {
        matches!(self, RouteNetlinkMessage::NewNeighbour(_))
    }

    pub fn is_get_route(&self) -> bool {
        matches!(self, RouteNetlinkMessage::GetRoute(_))
    }

    pub fn is_del_neighbour(&self) -> bool {
        matches!(self, RouteNetlinkMessage::DelNeighbour(_))
    }

    pub fn is_new_neighbour_table(&self) -> bool {
        matches!(self, RouteNetlinkMessage::NewNeighbourTable(_))
    }

    pub fn is_get_neighbour_table(&self) -> bool {
        matches!(self, RouteNetlinkMessage::GetNeighbourTable(_))
    }

    pub fn is_set_neighbour_table(&self) -> bool {
        matches!(self, RouteNetlinkMessage::SetNeighbourTable(_))
    }

    pub fn is_del_route(&self) -> bool {
        matches!(self, RouteNetlinkMessage::DelRoute(_))
    }

    pub fn is_new_qdisc(&self) -> bool {
        matches!(self, RouteNetlinkMessage::NewQueueDiscipline(_))
    }

    pub fn is_del_qdisc(&self) -> bool {
        matches!(self, RouteNetlinkMessage::DelQueueDiscipline(_))
    }

    pub fn is_get_qdisc(&self) -> bool {
        matches!(self, RouteNetlinkMessage::GetQueueDiscipline(_))
    }

    pub fn is_new_class(&self) -> bool {
        matches!(self, RouteNetlinkMessage::NewTrafficClass(_))
    }

    pub fn is_del_class(&self) -> bool {
        matches!(self, RouteNetlinkMessage::DelTrafficClass(_))
    }

    pub fn is_get_class(&self) -> bool {
        matches!(self, RouteNetlinkMessage::GetTrafficClass(_))
    }

    pub fn is_new_filter(&self) -> bool {
        matches!(self, RouteNetlinkMessage::NewTrafficFilter(_))
    }

    pub fn is_del_filter(&self) -> bool {
        matches!(self, RouteNetlinkMessage::DelTrafficFilter(_))
    }

    pub fn is_get_filter(&self) -> bool {
        matches!(self, RouteNetlinkMessage::GetTrafficFilter(_))
    }

    pub fn is_new_action(&self) -> bool {
        matches!(self, RouteNetlinkMessage::NewTrafficAction(_))
    }

    pub fn is_del_action(&self) -> bool {
        matches!(self, RouteNetlinkMessage::DelTrafficAction(_))
    }

    pub fn is_get_action(&self) -> bool {
        matches!(self, RouteNetlinkMessage::GetTrafficAction(_))
    }

    pub fn is_new_chain(&self) -> bool {
        matches!(self, RouteNetlinkMessage::NewTrafficChain(_))
    }

    pub fn is_del_chain(&self) -> bool {
        matches!(self, RouteNetlinkMessage::DelTrafficChain(_))
    }

    pub fn is_get_chain(&self) -> bool {
        matches!(self, RouteNetlinkMessage::GetTrafficChain(_))
    }

    pub fn is_new_nsid(&self) -> bool {
        matches!(self, RouteNetlinkMessage::NewNsId(_))
    }

    pub fn is_get_nsid(&self) -> bool {
        matches!(self, RouteNetlinkMessage::GetNsId(_))
    }

    pub fn is_del_nsid(&self) -> bool {
        matches!(self, RouteNetlinkMessage::DelNsId(_))
    }

    pub fn is_get_rule(&self) -> bool {
        matches!(self, RouteNetlinkMessage::GetRule(_))
    }

    pub fn is_new_rule(&self) -> bool {
        matches!(self, RouteNetlinkMessage::NewRule(_))
    }

    pub fn is_del_rule(&self) -> bool {
        matches!(self, RouteNetlinkMessage::DelRule(_))
    }

    pub fn is_new_stats(&self) -> bool {
        matches!(self, RouteNetlinkMessage::NewStats(_))
    }

    pub fn is_get_stats(&self) -> bool {
        matches!(self, RouteNetlinkMessage::GetStats(_))
    }

    pub fn message_type(&self) -> u16 {
        match self {
            Self::NewLink(_) => RTM_NEWLINK,
            Self::DelLink(_) => RTM_DELLINK,
            Self::GetLink(_) => RTM_GETLINK,
            Self::SetLink(_) => RTM_SETLINK,
            Self::NewLinkProp(_) => RTM_NEWLINKPROP,
            Self::DelLinkProp(_) => RTM_DELLINKPROP,
            Self::NewAddress(_) => RTM_NEWADDR,
            Self::DelAddress(_) => RTM_DELADDR,
            Self::GetAddress(_) => RTM_GETADDR,
            Self::GetNeighbour(_) => RTM_GETNEIGH,
            Self::NewNeighbour(_) => RTM_NEWNEIGH,
            Self::DelNeighbour(_) => RTM_DELNEIGH,
            Self::GetNeighbourTable(_) => RTM_GETNEIGHTBL,
            Self::NewNeighbourTable(_) => RTM_NEWNEIGHTBL,
            Self::SetNeighbourTable(_) => RTM_SETNEIGHTBL,
            Self::NewRoute(_) => RTM_NEWROUTE,
            Self::DelRoute(_) => RTM_DELROUTE,
            Self::GetRoute(_) => RTM_GETROUTE,
            Self::NewPrefix(_) => RTM_NEWPREFIX,
            Self::NewQueueDiscipline(_) => RTM_NEWQDISC,
            Self::DelQueueDiscipline(_) => RTM_DELQDISC,
            Self::GetQueueDiscipline(_) => RTM_GETQDISC,
            Self::NewTrafficClass(_) => RTM_NEWTCLASS,
            Self::DelTrafficClass(_) => RTM_DELTCLASS,
            Self::GetTrafficClass(_) => RTM_GETTCLASS,
            Self::NewTrafficFilter(_) => RTM_NEWTFILTER,
            Self::DelTrafficFilter(_) => RTM_DELTFILTER,
            Self::GetTrafficFilter(_) => RTM_GETTFILTER,
            Self::NewTrafficAction(_) => RTM_NEWACTION,
            Self::DelTrafficAction(_) => RTM_DELACTION,
            Self::GetTrafficAction(_) => RTM_GETACTION,
            Self::NewTrafficChain(_) => RTM_NEWCHAIN,
            Self::DelTrafficChain(_) => RTM_DELCHAIN,
            Self::GetTrafficChain(_) => RTM_GETCHAIN,
            Self::GetNsId(_) => RTM_GETNSID,
            Self::NewNsId(_) => RTM_NEWNSID,
            Self::DelNsId(_) => RTM_DELNSID,
            Self::NewStats(_) => RTM_NEWSTATS,
            Self::GetStats(_) => RTM_GETSTATS,
            Self::GetRule(_) => RTM_GETRULE,
            Self::NewRule(_) => RTM_NEWRULE,
            Self::DelRule(_) => RTM_DELRULE,
        }
    }
}

impl Emitable for RouteNetlinkMessage {
    fn buffer_len(&self) -> usize {
        match self {
            Self::NewLink(ref msg)
            | Self::DelLink(ref msg)
            | Self::GetLink(ref msg)
            | Self::SetLink(ref msg)
            | Self::NewLinkProp(ref msg)
            | Self::DelLinkProp(ref msg) => msg.buffer_len(),

            Self::NewAddress(ref msg)
            | Self::DelAddress(ref msg)
            | Self::GetAddress(ref msg) => msg.buffer_len(),

            Self::NewNeighbour(ref msg)
            | Self::GetNeighbour(ref msg)
            | Self::DelNeighbour(ref msg) => msg.buffer_len(),

            Self::NewNeighbourTable(ref msg)
            | Self::GetNeighbourTable(ref msg)
            | Self::SetNeighbourTable(ref msg) => msg.buffer_len(),

            Self::NewRoute(ref msg)
            | Self::DelRoute(ref msg)
            | Self::GetRoute(ref msg) => msg.buffer_len(),

            Self::NewPrefix(ref msg) => msg.buffer_len(),

            Self::NewQueueDiscipline(ref msg)
            | Self::DelQueueDiscipline(ref msg)
            | Self::GetQueueDiscipline(ref msg)
            | Self::NewTrafficClass(ref msg)
            | Self::DelTrafficClass(ref msg)
            | Self::GetTrafficClass(ref msg)
            | Self::NewTrafficFilter(ref msg)
            | Self::DelTrafficFilter(ref msg)
            | Self::GetTrafficFilter(ref msg)
            | Self::NewTrafficChain(ref msg)
            | Self::DelTrafficChain(ref msg)
            | Self::GetTrafficChain(ref msg) => msg.buffer_len(),

            Self::NewNsId(ref msg)
            | Self::DelNsId(ref msg)
            | Self::GetNsId(ref msg) => msg.buffer_len(),

            Self::NewStats(ref msg) | Self::GetStats(ref msg) => {
                msg.buffer_len()
            }

            Self::NewRule(ref msg)
            | Self::DelRule(ref msg)
            | Self::GetRule(ref msg) => msg.buffer_len(),

            Self::NewTrafficAction(ref msg)
            | Self::DelTrafficAction(ref msg)
            | Self::GetTrafficAction(ref msg) => msg.buffer_len(),
        }
    }

    fn emit(&self, buffer: &mut [u8]) {
        match self {
            Self::NewLink(ref msg)
            | Self::DelLink(ref msg)
            | Self::GetLink(ref msg)
            | Self::SetLink(ref msg)
            | Self::NewLinkProp(ref msg)
            | Self::DelLinkProp(ref msg) => msg.emit(buffer),

            Self::NewAddress(ref msg)
            | Self::DelAddress(ref msg)
            | Self::GetAddress(ref msg) => msg.emit(buffer),

            Self::GetNeighbour(ref msg)
            | Self::NewNeighbour(ref msg)
            | Self::DelNeighbour(ref msg) => msg.emit(buffer),

            Self::GetNeighbourTable(ref msg)
            | Self::NewNeighbourTable(ref msg)
            | Self::SetNeighbourTable(ref msg) => msg.emit(buffer),

            Self::NewRoute(ref msg)
            | Self::DelRoute(ref msg)
            | Self::GetRoute(ref msg) => msg.emit(buffer),

            Self::NewPrefix(ref msg) => msg.emit(buffer),

            Self::NewQueueDiscipline(ref msg)
            | Self::DelQueueDiscipline(ref msg)
            | Self::GetQueueDiscipline(ref msg)
            | Self::NewTrafficClass(ref msg)
            | Self::DelTrafficClass(ref msg)
            | Self::GetTrafficClass(ref msg)
            | Self::NewTrafficFilter(ref msg)
            | Self::DelTrafficFilter(ref msg)
            | Self::GetTrafficFilter(ref msg)
            | Self::NewTrafficChain(ref msg)
            | Self::DelTrafficChain(ref msg)
            | Self::GetTrafficChain(ref msg) => msg.emit(buffer),

            Self::NewNsId(ref msg)
            | Self::DelNsId(ref msg)
            | Self::GetNsId(ref msg) => msg.emit(buffer),

            Self::NewStats(ref msg) | Self::GetStats(ref msg) => {
                msg.emit(buffer)
            }

            Self::NewRule(ref msg)
            | Self::DelRule(ref msg)
            | Self::GetRule(ref msg) => msg.emit(buffer),

            Self::NewTrafficAction(ref msg)
            | Self::DelTrafficAction(ref msg)
            | Self::GetTrafficAction(ref msg) => msg.emit(buffer),
        }
    }
}

impl NetlinkSerializable for RouteNetlinkMessage {
    fn message_type(&self) -> u16 {
        self.message_type()
    }

    fn buffer_len(&self) -> usize {
        <Self as Emitable>::buffer_len(self)
    }

    fn serialize(&self, buffer: &mut [u8]) {
        self.emit(buffer)
    }
}

impl NetlinkDeserializable for RouteNetlinkMessage {
    type Error = DecodeError;
    fn deserialize(
        header: &NetlinkHeader,
        payload: &[u8],
    ) -> Result<Self, Self::Error> {
        RouteNetlinkMessage::parse_with_param(payload, header.message_type)
    }
}

impl From<RouteNetlinkMessage> for NetlinkPayload<RouteNetlinkMessage> {
    fn from(message: RouteNetlinkMessage) -> Self {
        NetlinkPayload::InnerMessage(message)
    }
}
