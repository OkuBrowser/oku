// SPDX-License-Identifier: MIT

use bitflags::bitflags;
use netlink_packet_core::{
    emit_i32, emit_u16, emit_u32, parse_i32, parse_u16, parse_u32, parse_u8,
    DecodeError, DefaultNla, ErrorContext, Nla, NlaBuffer, Parseable,
};

const IFLA_BOND_PORT_STATE_ACTIVE: u8 = 0;
const IFLA_BOND_PORT_STATE_BACKUP: u8 = 1;

const IFLA_BOND_PORT_MII_STATUS_UP: u8 = 0;
const IFLA_BOND_PORT_MII_STATUS_GOING_DOWN: u8 = 1;
const IFLA_BOND_PORT_MII_STATUS_DOWN: u8 = 2;
const IFLA_BOND_PORT_MII_STATUS_GOING_BACK: u8 = 3;

const AD_CHURN_MONITOR: u8 = 0;
const AD_CHURN: u8 = 1;
const AD_NO_CHURN: u8 = 2;

// In kernel, they are IFLA_BOND_SLAVE, here we rename them to IFLA_BOND_PORT
const IFLA_BOND_PORT_STATE: u16 = 1;
const IFLA_BOND_PORT_MII_STATUS: u16 = 2;
const IFLA_BOND_PORT_LINK_FAILURE_COUNT: u16 = 3;
const IFLA_BOND_PORT_PERM_HWADDR: u16 = 4;
const IFLA_BOND_PORT_QUEUE_ID: u16 = 5;
const IFLA_BOND_PORT_AD_AGGREGATOR_ID: u16 = 6;
const IFLA_BOND_PORT_AD_ACTOR_OPER_PORT_STATE: u16 = 7;
const IFLA_BOND_PORT_AD_PARTNER_OPER_PORT_STATE: u16 = 8;
const IFLA_BOND_PORT_PRIO: u16 = 9;
const IFLA_BOND_PORT_ACTOR_PORT_PRIO: u16 = 10;
const IFLA_BOND_PORT_AD_CHURN_ACTOR_STATE: u16 = 11;
const IFLA_BOND_PORT_AD_CHURN_PARTNER_STATE: u16 = 12;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[non_exhaustive]
pub enum BondPortState {
    Active,
    Backup,
    Other(u8),
}

impl From<u8> for BondPortState {
    fn from(value: u8) -> Self {
        match value {
            IFLA_BOND_PORT_STATE_ACTIVE => BondPortState::Active,
            IFLA_BOND_PORT_STATE_BACKUP => BondPortState::Backup,
            _ => BondPortState::Other(value),
        }
    }
}

impl From<BondPortState> for u8 {
    fn from(value: BondPortState) -> Self {
        match value {
            BondPortState::Active => IFLA_BOND_PORT_STATE_ACTIVE,
            BondPortState::Backup => IFLA_BOND_PORT_STATE_BACKUP,
            BondPortState::Other(other) => other,
        }
    }
}

impl std::fmt::Display for BondPortState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => f.write_str("active"),
            Self::Backup => f.write_str("backup"),
            Self::Other(v) => write!(f, "{v}"),
        }
    }
}

impl std::str::FromStr for BondPortState {
    type Err = DecodeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            s if s.eq_ignore_ascii_case("active") => Self::Active,
            s if s.eq_ignore_ascii_case("backup") => Self::Backup,
            _ => {
                return Err(DecodeError::from(format!(
                    "unknown bond port state: {s}"
                )))
            }
        })
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[non_exhaustive]
pub enum MiiStatus {
    Up,
    GoingDown,
    Down,
    GoingBack,
    Other(u8),
}

impl From<u8> for MiiStatus {
    fn from(value: u8) -> Self {
        match value {
            IFLA_BOND_PORT_MII_STATUS_UP => MiiStatus::Up,
            IFLA_BOND_PORT_MII_STATUS_GOING_DOWN => MiiStatus::GoingDown,
            IFLA_BOND_PORT_MII_STATUS_DOWN => MiiStatus::Down,
            IFLA_BOND_PORT_MII_STATUS_GOING_BACK => MiiStatus::GoingBack,
            _ => MiiStatus::Other(value),
        }
    }
}

impl From<MiiStatus> for u8 {
    fn from(value: MiiStatus) -> Self {
        match value {
            MiiStatus::Up => IFLA_BOND_PORT_MII_STATUS_UP,
            MiiStatus::GoingDown => IFLA_BOND_PORT_MII_STATUS_GOING_DOWN,
            MiiStatus::Down => IFLA_BOND_PORT_MII_STATUS_DOWN,
            MiiStatus::GoingBack => IFLA_BOND_PORT_MII_STATUS_GOING_BACK,
            MiiStatus::Other(other) => other,
        }
    }
}

impl std::fmt::Display for MiiStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Up => f.write_str("up"),
            Self::GoingDown => f.write_str("going_down"),
            Self::Down => f.write_str("down"),
            Self::GoingBack => f.write_str("going_back"),
            Self::Other(v) => write!(f, "{v}"),
        }
    }
}

impl std::str::FromStr for MiiStatus {
    type Err = DecodeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            s if s.eq_ignore_ascii_case("up") => Self::Up,
            s if s.eq_ignore_ascii_case("going_down") => Self::GoingDown,
            s if s.eq_ignore_ascii_case("down") => Self::Down,
            s if s.eq_ignore_ascii_case("going_back") => Self::GoingBack,
            _ => {
                return Err(DecodeError::from(format!(
                    "unknown MII status: {s}"
                )))
            }
        })
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct LacpState: u8 {
        /// The port is willing to participate in LACP (LACP_ACTIVITY).
        const LACP_ACTIVITY   = 0x01;
        /// Short (fast) LACP timeout is in use (LACP_TIMEOUT).
        const LACP_TIMEOUT    = 0x02;
        /// The port is part of an aggregation (AGGREGATION).
        const AGGREGATION     = 0x04;
        /// The port is synchronized with its partner (SYNCHRONIZATION).
        const SYNCHRONIZATION = 0x08;
        /// The port is collecting frames (COLLECTING).
        const COLLECTING      = 0x10;
        /// The port is distributing frames (DISTRIBUTING).
        const DISTRIBUTING    = 0x20;
        /// The partner is in the defaulted state (DEFAULTED).
        const DEFAULTED       = 0x40;
        /// The partner is in the expired state (EXPIRED).
        const EXPIRED         = 0x80;
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[non_exhaustive]
pub enum ChurnState {
    Monitor,
    Churn,
    NoChurn,
    Other(u8),
}

impl From<u8> for ChurnState {
    fn from(v: u8) -> Self {
        match v {
            AD_CHURN_MONITOR => Self::Monitor,
            AD_CHURN => Self::Churn,
            AD_NO_CHURN => Self::NoChurn,
            _ => Self::Other(v),
        }
    }
}

impl From<ChurnState> for u8 {
    fn from(v: ChurnState) -> u8 {
        match v {
            ChurnState::Monitor => AD_CHURN_MONITOR,
            ChurnState::Churn => AD_CHURN,
            ChurnState::NoChurn => AD_NO_CHURN,
            ChurnState::Other(v) => v,
        }
    }
}

impl std::fmt::Display for ChurnState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Churn => f.write_str("churn"),
            Self::Monitor => f.write_str("monitor"),
            Self::NoChurn => f.write_str("no_churn"),
            Self::Other(v) => write!(f, "{v}"),
        }
    }
}

impl std::str::FromStr for ChurnState {
    type Err = DecodeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            s if s.eq_ignore_ascii_case("monitor") => Self::Monitor,
            s if s.eq_ignore_ascii_case("churn") => Self::Churn,
            s if s.eq_ignore_ascii_case("no_churn") => Self::NoChurn,
            _ => {
                return Err(DecodeError::from(format!(
                    "unknown churn state: {s}"
                )))
            }
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[non_exhaustive]
pub enum InfoBondPort {
    LinkFailureCount(u32),
    MiiStatus(MiiStatus),
    PermHwaddr(Vec<u8>),
    Prio(i32),
    QueueId(u16),
    BondPortState(BondPortState),
    AdAggregatorId(u16),
    AdActorOperPortState(LacpState),
    AdPartnerOperPortState(LacpState),
    ActorPortPrio(u16),
    AdChurnActorState(ChurnState),
    AdChurnPartnerState(ChurnState),
    Other(DefaultNla),
}

impl Nla for InfoBondPort {
    fn value_len(&self) -> usize {
        match self {
            Self::QueueId(_)
            | Self::AdAggregatorId(_)
            | Self::ActorPortPrio(_)
            // The kernel stores `partner_oper.port_state` as u8, but emits
            // it with nla_put_u16().
            | Self::AdPartnerOperPortState(_) => 2,
            Self::LinkFailureCount(_) | Self::Prio(_) => 4,
            Self::PermHwaddr(ref bytes) => bytes.len(),
            Self::MiiStatus(_)
            | Self::BondPortState(_)
            | Self::AdActorOperPortState(_)
            | Self::AdChurnActorState(_)
            | Self::AdChurnPartnerState(_) => 1,
            Self::Other(nla) => nla.value_len(),
        }
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        match self {
            Self::QueueId(ref value) => emit_u16(buffer, *value).unwrap(),
            Self::AdAggregatorId(ref value) => {
                emit_u16(buffer, *value).unwrap()
            }
            Self::ActorPortPrio(ref value) => emit_u16(buffer, *value).unwrap(),
            Self::PermHwaddr(ref bytes) => {
                buffer.copy_from_slice(bytes.as_slice())
            }
            Self::Prio(ref value) => emit_i32(buffer, *value).unwrap(),
            Self::LinkFailureCount(value) => emit_u32(buffer, *value).unwrap(),
            Self::MiiStatus(state) => buffer[0] = (*state).into(),
            Self::BondPortState(state) => buffer[0] = (*state).into(),
            Self::AdActorOperPortState(state) => buffer[0] = state.bits(),
            Self::AdPartnerOperPortState(state) => {
                emit_u16(buffer, u16::from(state.bits())).unwrap()
            }
            Self::AdChurnActorState(state) => buffer[0] = u8::from(*state),
            Self::AdChurnPartnerState(state) => buffer[0] = u8::from(*state),
            Self::Other(nla) => nla.emit_value(buffer),
        }
    }

    fn kind(&self) -> u16 {
        match self {
            Self::LinkFailureCount(_) => IFLA_BOND_PORT_LINK_FAILURE_COUNT,
            Self::MiiStatus(_) => IFLA_BOND_PORT_MII_STATUS,
            Self::PermHwaddr(_) => IFLA_BOND_PORT_PERM_HWADDR,
            Self::Prio(_) => IFLA_BOND_PORT_PRIO,
            Self::QueueId(_) => IFLA_BOND_PORT_QUEUE_ID,
            Self::BondPortState(_) => IFLA_BOND_PORT_STATE,
            Self::AdAggregatorId(_) => IFLA_BOND_PORT_AD_AGGREGATOR_ID,
            Self::AdActorOperPortState(_) => {
                IFLA_BOND_PORT_AD_ACTOR_OPER_PORT_STATE
            }
            Self::AdPartnerOperPortState(_) => {
                IFLA_BOND_PORT_AD_PARTNER_OPER_PORT_STATE
            }
            Self::ActorPortPrio(_) => IFLA_BOND_PORT_ACTOR_PORT_PRIO,
            Self::AdChurnActorState(_) => IFLA_BOND_PORT_AD_CHURN_ACTOR_STATE,
            Self::AdChurnPartnerState(_) => {
                IFLA_BOND_PORT_AD_CHURN_PARTNER_STATE
            }
            Self::Other(nla) => nla.kind(),
        }
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> Parseable<NlaBuffer<&'a T>> for InfoBondPort {
    fn parse(buf: &NlaBuffer<&'a T>) -> Result<Self, DecodeError> {
        let payload = buf.value();
        Ok(match buf.kind() {
            IFLA_BOND_PORT_LINK_FAILURE_COUNT => {
                Self::LinkFailureCount(parse_u32(payload).context(
                    "invalid IFLA_BOND_PORT_LINK_FAILURE_COUNT value",
                )?)
            }
            IFLA_BOND_PORT_MII_STATUS => Self::MiiStatus(
                parse_u8(payload)
                    .context("invalid IFLA_BOND_PORT_MII_STATUS value")?
                    .into(),
            ),
            IFLA_BOND_PORT_PERM_HWADDR => Self::PermHwaddr(payload.to_vec()),
            IFLA_BOND_PORT_PRIO => Self::Prio(
                parse_i32(payload)
                    .context("invalid IFLA_BOND_PORT_PRIO value")?,
            ),
            IFLA_BOND_PORT_QUEUE_ID => Self::QueueId(
                parse_u16(payload)
                    .context("invalid IFLA_BOND_PORT_QUEUE_ID value")?,
            ),
            IFLA_BOND_PORT_STATE => Self::BondPortState(
                parse_u8(payload)
                    .context("invalid IFLA_BOND_PORT_STATE value")?
                    .into(),
            ),
            IFLA_BOND_PORT_AD_AGGREGATOR_ID => Self::AdAggregatorId(
                parse_u16(payload)
                    .context("invalid IFLA_BOND_PORT_AD_AGGREGATOR_ID value")?,
            ),
            IFLA_BOND_PORT_AD_ACTOR_OPER_PORT_STATE => {
                Self::AdActorOperPortState(LacpState::from_bits_retain(
                    parse_u8(payload).context(
                        "invalid IFLA_BOND_PORT_AD_ACTOR_OPER_PORT_STATE value",
                    )?,
                ))
            }
            IFLA_BOND_PORT_AD_PARTNER_OPER_PORT_STATE => {
                Self::AdPartnerOperPortState(LacpState::from_bits_retain(
                    (parse_u16(payload).context(
                        "invalid IFLA_BOND_PORT_AD_PARTNER_OPER_PORT_STATE \
                         value",
                    )? & 0xff) as u8,
                ))
            }
            IFLA_BOND_PORT_ACTOR_PORT_PRIO => Self::ActorPortPrio(
                parse_u16(payload)
                    .context("invalid IFLA_BOND_PORT_ACTOR_PORT_PRIO value")?,
            ),
            IFLA_BOND_PORT_AD_CHURN_ACTOR_STATE => Self::AdChurnActorState(
                ChurnState::from(parse_u8(payload).context(
                    "invalid IFLA_BOND_PORT_AD_CHURN_ACTOR_STATE value",
                )?),
            ),
            IFLA_BOND_PORT_AD_CHURN_PARTNER_STATE => Self::AdChurnPartnerState(
                ChurnState::from(parse_u8(payload).context(
                    "invalid IFLA_BOND_PORT_AD_CHURN_PARTNER_STATE value",
                )?),
            ),
            kind => Self::Other(
                DefaultNla::parse(buf)
                    .context(format!("unknown NLA type {kind}"))?,
            ),
        })
    }
}
