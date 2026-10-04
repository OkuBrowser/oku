// SPDX-License-Identifier: MIT

#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
#[non_exhaustive]
// We are using not using #[repr(u8)] here as we have duplicate(e.g. AF_ROUTE vs
// AF_NETLINK) here
pub enum AddressFamily {
    #[default]
    Unspec,
    Local,
    Unix,
    Inet,
    Inet6,
    Other(u8),
}

impl From<u8> for AddressFamily {
    fn from(d: u8) -> Self {
        match d {
            d if d == libc::AF_UNSPEC as u8 => Self::Unspec,
            d if d == libc::AF_LOCAL as u8 => Self::Local,
            d if d == libc::AF_UNIX as u8 => Self::Unix,
            d if d == libc::AF_INET as u8 => Self::Inet,
            d if d == libc::AF_INET6 as u8 => Self::Inet6,
            _ => Self::Other(d),
        }
    }
}

impl From<AddressFamily> for u8 {
    fn from(v: AddressFamily) -> u8 {
        match v {
            AddressFamily::Unspec => libc::AF_UNSPEC as u8,
            AddressFamily::Local => libc::AF_LOCAL as u8,
            AddressFamily::Unix => libc::AF_UNIX as u8,
            AddressFamily::Inet => libc::AF_INET as u8,
            AddressFamily::Inet6 => libc::AF_INET6 as u8,
            AddressFamily::Other(d) => d,
        }
    }
}

impl std::fmt::Display for AddressFamily {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unspec => f.write_str("unspec"),
            Self::Local => f.write_str("local"),
            Self::Unix => f.write_str("unix"),
            Self::Inet => f.write_str("inet"),
            Self::Inet6 => f.write_str("inet6"),
            Self::Other(d) => write!(f, "{d}"),
        }
    }
}
