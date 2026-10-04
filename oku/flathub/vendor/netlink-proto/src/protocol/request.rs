// SPDX-License-Identifier: MIT

use std::fmt::Debug;

use netlink_packet_core::NetlinkMessage;

use crate::sys::SocketAddr;

#[derive(Debug)]
pub(crate) struct Request<T, M> {
    pub metadata: M,
    pub messages: Box<[NetlinkMessage<T>]>,
    pub destination: SocketAddr,
}

impl<T, M> From<(NetlinkMessage<T>, SocketAddr, M)> for Request<T, M>
where
    T: Debug,
    M: Debug,
{
    fn from(
        (message, destination, metadata): (NetlinkMessage<T>, SocketAddr, M),
    ) -> Self {
        Request {
            metadata,
            messages: Box::new([message]),
            destination,
        }
    }
}
