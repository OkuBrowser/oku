// SPDX-License-Identifier: MIT

use netlink_packet_core::{
    emit_u32, parse_u32, parse_u8, DecodeError, DefaultNla, ErrorContext, Nla,
    NlaBuffer, NlasIterator, Parseable,
};

const IFLA_MCTP_NET: u16 = 1;
const IFLA_MCTP_PHYS_BINDING: u16 = 2;

#[derive(Clone, Eq, PartialEq, Debug)]
#[non_exhaustive]
pub enum AfSpecMctp {
    Net(u32),
    PhysBinding(MctpPhysBinding),
    Other(DefaultNla),
}

// MCTP IDs and Codes from DMTF specification
// "DSP0239 Management Component Transport Protocol (MCTP) IDs and Codes"
// https://www.dmtf.org/sites/default/files/standards/documents/DSP0239_1.11.1.pdf
const MCTP_PHYS_BINDING_SMBUS: u8 = 0x01;
const MCTP_PHYS_BINDING_PCIE_VDM: u8 = 0x02;
const MCTP_PHYS_BINDING_USB: u8 = 0x03;
const MCTP_PHYS_BINDING_KCS: u8 = 0x04;
const MCTP_PHYS_BINDING_SERIAL: u8 = 0x05;
const MCTP_PHYS_BINDING_I3C: u8 = 0x06;
const MCTP_PHYS_BINDING_MMBI: u8 = 0x07;
const MCTP_PHYS_BINDING_PCC: u8 = 0x08;
const MCTP_PHYS_BINDING_UCIE: u8 = 0x09;
const MCTP_PHYS_BINDING_VENDOR: u8 = 0xFF;

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[non_exhaustive]
#[repr(u8)]
pub enum MctpPhysBinding {
    Smbus,
    PcieVdm,
    Usb,
    Kcs,
    Serial,
    I3c,
    Mmbi,
    Pcc,
    Ucie,
    Vendor,
    Other(u8),
}

// Not construted on non-Linux targets
#[allow(unused)]
pub(crate) struct VecAfSpecMctp(pub(crate) Vec<AfSpecMctp>);

impl<'a, T: AsRef<[u8]> + ?Sized> Parseable<NlaBuffer<&'a T>>
    for VecAfSpecMctp
{
    fn parse(buf: &NlaBuffer<&'a T>) -> Result<Self, DecodeError> {
        let mut nlas = vec![];
        let err = "Invalid AF_MCTP NLA for IFLA_AF_SPEC(AF_UNSPEC)";
        for nla in NlasIterator::new(buf.into_inner()) {
            let nla = nla.context(err)?;
            nlas.push(AfSpecMctp::parse(&nla).context(err)?);
        }
        Ok(Self(nlas))
    }
}

impl Nla for AfSpecMctp {
    fn value_len(&self) -> usize {
        match *self {
            Self::Net(_) => 4,
            Self::PhysBinding(_) => 1,
            Self::Other(ref nla) => nla.value_len(),
        }
    }

    fn emit_value(&self, buffer: &mut [u8]) {
        match *self {
            Self::Net(ref value) => emit_u32(buffer, *value).unwrap(),
            Self::PhysBinding(ref b) => buffer[0] = b.into(),
            Self::Other(ref nla) => nla.emit_value(buffer),
        }
    }

    fn kind(&self) -> u16 {
        match *self {
            Self::Net(_) => IFLA_MCTP_NET,
            Self::PhysBinding(_) => IFLA_MCTP_PHYS_BINDING,
            Self::Other(ref nla) => nla.kind(),
        }
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> Parseable<NlaBuffer<&'a T>> for AfSpecMctp {
    fn parse(buf: &NlaBuffer<&'a T>) -> Result<Self, DecodeError> {
        let payload = buf.value();
        Ok(match buf.kind() {
            IFLA_MCTP_NET => Self::Net(
                parse_u32(payload).context("invalid IFLA_MCTP_NET value")?,
            ),
            IFLA_MCTP_PHYS_BINDING => {
                let b = parse_u8(payload)
                    .context("invalid IFLA_MCTP_PHYS_BINDING value")?;
                Self::PhysBinding(MctpPhysBinding::from(b))
            }
            kind => Self::Other(DefaultNla::parse(buf).context(format!(
                "unknown AF_MCTP NLA type {kind} for IFLA_AF_SPEC(AF_UNSPEC)"
            ))?),
        })
    }
}

impl From<u8> for MctpPhysBinding {
    fn from(d: u8) -> Self {
        match d {
            MCTP_PHYS_BINDING_SMBUS => Self::Smbus,
            MCTP_PHYS_BINDING_PCIE_VDM => Self::PcieVdm,
            MCTP_PHYS_BINDING_USB => Self::Usb,
            MCTP_PHYS_BINDING_KCS => Self::Kcs,
            MCTP_PHYS_BINDING_SERIAL => Self::Serial,
            MCTP_PHYS_BINDING_I3C => Self::I3c,
            MCTP_PHYS_BINDING_MMBI => Self::Mmbi,
            MCTP_PHYS_BINDING_PCC => Self::Pcc,
            MCTP_PHYS_BINDING_UCIE => Self::Ucie,
            MCTP_PHYS_BINDING_VENDOR => Self::Vendor,
            _ => Self::Other(d),
        }
    }
}

impl From<&MctpPhysBinding> for u8 {
    fn from(v: &MctpPhysBinding) -> Self {
        match v {
            MctpPhysBinding::Smbus => MCTP_PHYS_BINDING_SMBUS,
            MctpPhysBinding::PcieVdm => MCTP_PHYS_BINDING_PCIE_VDM,
            MctpPhysBinding::Usb => MCTP_PHYS_BINDING_USB,
            MctpPhysBinding::Kcs => MCTP_PHYS_BINDING_KCS,
            MctpPhysBinding::Serial => MCTP_PHYS_BINDING_SERIAL,
            MctpPhysBinding::I3c => MCTP_PHYS_BINDING_I3C,
            MctpPhysBinding::Mmbi => MCTP_PHYS_BINDING_MMBI,
            MctpPhysBinding::Pcc => MCTP_PHYS_BINDING_PCC,
            MctpPhysBinding::Ucie => MCTP_PHYS_BINDING_UCIE,
            MctpPhysBinding::Vendor => MCTP_PHYS_BINDING_VENDOR,
            MctpPhysBinding::Other(b) => *b,
        }
    }
}

impl std::fmt::Display for MctpPhysBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Smbus => write!(f, "SMBus"),
            Self::PcieVdm => write!(f, "PCIe VDM"),
            Self::Usb => write!(f, "USB"),
            Self::Kcs => write!(f, "KCS"),
            Self::Serial => write!(f, "Serial"),
            Self::I3c => write!(f, "I3C"),
            Self::Mmbi => write!(f, "MMBI"),
            Self::Pcc => write!(f, "PCC"),
            Self::Ucie => write!(f, "UCIE"),
            Self::Vendor => write!(f, "Vendor"),
            Self::Other(d) => write!(f, "other({d})"),
        }
    }
}
