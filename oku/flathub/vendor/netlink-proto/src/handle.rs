// SPDX-License-Identifier: MIT

use futures_channel::mpsc::{unbounded, UnboundedSender};
use futures_util::Stream;
use netlink_packet_core::NetlinkMessage;
use std::fmt::Debug;

use crate::{errors::Error, sys::SocketAddr, Request};

/// A handle to pass requests to a [`Connection`](struct.Connection.html).
#[derive(Clone, Debug)]
pub struct ConnectionHandle<T>
where
    T: Debug,
{
    requests_tx: UnboundedSender<Request<T>>,
}

impl<T> ConnectionHandle<T>
where
    T: Debug,
{
    pub(crate) fn new(requests_tx: UnboundedSender<Request<T>>) -> Self {
        ConnectionHandle { requests_tx }
    }

    /// Send a new request and get the response as a stream of messages. Note
    /// that some messages are not part of the response stream:
    ///
    /// - **acknowledgments**: when an acknowledgment is received, the stream is
    ///   closed
    /// - **end of dump messages**: similarly, upon receiving an "end of dump"
    ///   message, the stream is closed
    pub fn request(
        &self,
        message: NetlinkMessage<T>,
        destination: SocketAddr,
    ) -> Result<impl Stream<Item = NetlinkMessage<T>>, Error<T>> {
        self.request_batch([message], destination)
    }

    /// Send a multiple messages as a _single_ request and get the response as
    /// a stream of messages.
    ///
    /// The stream is only closed when every message received an acknowledgment.
    /// This method can be used to send NfNetlink messages in batches.
    pub fn request_batch(
        &self,
        messages: impl IntoIterator<Item = NetlinkMessage<T>>,
        destination: SocketAddr,
    ) -> Result<impl Stream<Item = NetlinkMessage<T>>, Error<T>> {
        let (tx, rx) = unbounded::<NetlinkMessage<T>>();
        let request = Request {
            metadata: tx,
            messages: messages.into_iter().collect(),
            destination,
        };

        trace!("handle: forwarding new request to connection");
        UnboundedSender::unbounded_send(&self.requests_tx, request).map_err(
            |e| {
                // the channel is unbounded, so it can't be full. If this
                // failed, it means the Connection shut down.
                if e.is_full() {
                    panic!("internal error: unbounded channel full?!");
                } else if e.is_disconnected() {
                    Error::ConnectionClosed
                } else {
                    panic!("unknown error: {:?}", e);
                }
            },
        )?;
        Ok(rx)
    }

    pub fn notify(
        &self,
        message: NetlinkMessage<T>,
        destination: SocketAddr,
    ) -> Result<(), Error<T>> {
        let (tx, _rx) = unbounded::<NetlinkMessage<T>>();
        let request = Request::from((message, destination, tx));
        trace!("handle: forwarding new request to connection");
        UnboundedSender::unbounded_send(&self.requests_tx, request)
            .map_err(|_| Error::ConnectionClosed)
    }
}
