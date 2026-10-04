// SPDX-License-Identifier: MIT

use std::net::IpAddr;

use crate::AddressFamily;

impl From<IpAddr> for AddressFamily {
    fn from(v: IpAddr) -> Self {
        match v {
            IpAddr::V4(_) => AddressFamily::Inet,
            IpAddr::V6(_) => AddressFamily::Inet6,
        }
    }
}
