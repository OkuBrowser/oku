use netlink_packet_core::{
    NLM_F_DUMP, NLM_F_DUMP_INTR, NLM_F_REQUEST, NetlinkMessage, NetlinkPayload,
};
use netlink_packet_route::RouteNetlinkMessage;
use netlink_sys::{Socket, SocketAddr};
use std::{
    io, thread,
    time::{Duration, Instant},
};

const RECV_BUFSZ: usize = 1 << 20;
const RECV_TIMEOUT: Duration = Duration::from_secs(2);
const NLMSG_ALIGNTO: usize = 4;
const MIN_NLMSG_HEADER_LEN: usize = 16;

#[derive(Debug)]
enum DatagramStatus {
    Continue,
    Done,
}

#[inline]
fn nlmsg_align(n: usize) -> Option<usize> {
    n.checked_add(NLMSG_ALIGNTO - 1)
        .map(|n| n & !(NLMSG_ALIGNTO - 1))
}

pub(crate) fn set_non_blocking(sock: &Socket) -> io::Result<()> {
    sock.set_non_blocking(true)
        .map_err(|e| io::Error::other(format!("netlink nonblocking: {e}")))
}

pub(crate) fn send_dump(sock: &mut Socket, msg: RouteNetlinkMessage, seq: u32) -> io::Result<()> {
    let mut nl = NetlinkMessage::from(msg);
    nl.header.flags = NLM_F_REQUEST | NLM_F_DUMP;
    nl.header.sequence_number = seq;
    nl.header.port_number = 0;
    nl.finalize();

    let blen = nl.buffer_len();
    if blen < MIN_NLMSG_HEADER_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("netlink message too short: buffer_len={blen}"),
        ));
    }

    let mut buf = vec![0; blen];
    nl.serialize(&mut buf);

    let kernel = SocketAddr::new(0, 0);
    let sent = sock
        .send_to(&buf, &kernel, 0)
        .map_err(|e| io::Error::other(format!("netlink send: {e}")))?;
    if sent != buf.len() {
        return Err(io::Error::new(
            io::ErrorKind::WriteZero,
            format!(
                "incomplete netlink send: sent={sent}, expected={}",
                buf.len()
            ),
        ));
    }
    Ok(())
}

fn parse_datagram(
    bytes: &[u8],
    expect_seq: u32,
    out: &mut Vec<NetlinkMessage<RouteNetlinkMessage>>,
) -> io::Result<DatagramStatus> {
    let mut offset = 0usize;

    while offset < bytes.len() {
        let remaining = bytes.len() - offset;
        if remaining < MIN_NLMSG_HEADER_LEN {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("truncated netlink header: remaining={remaining}"),
            ));
        }

        let consumed = u32::from_ne_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]) as usize;
        if consumed < MIN_NLMSG_HEADER_LEN {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid netlink message length: length={consumed}"),
            ));
        }

        let message_end = offset
            .checked_add(consumed)
            .filter(|end| *end <= bytes.len())
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "truncated netlink message: offset={offset}, length={consumed}, datagram_len={}",
                        bytes.len()
                    ),
                )
            })?;

        let msg = NetlinkMessage::<RouteNetlinkMessage>::deserialize(&bytes[offset..message_end])
            .map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("netlink deserialize: {e:?}"),
            )
        })?;

        if msg.header.sequence_number == expect_seq {
            match &msg.payload {
                NetlinkPayload::Done(done) => {
                    if msg.header.flags & NLM_F_DUMP_INTR != 0 {
                        return Err(io::Error::other("netlink dump was interrupted"));
                    }
                    if done.code != 0 {
                        return Err(io::Error::other(format!(
                            "netlink dump failed: code={}",
                            done.code
                        )));
                    }
                    return Ok(DatagramStatus::Done);
                }
                NetlinkPayload::Error(error) => {
                    if let Some(code) = error.code {
                        return Err(io::Error::other(format!("netlink error: code={code}")));
                    }
                }
                NetlinkPayload::Overrun(_) => {
                    return Err(io::Error::other("netlink receive overrun"));
                }
                NetlinkPayload::Noop => {}
                _ => out.push(msg),
            }
        }

        let aligned = nlmsg_align(consumed).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "netlink message length overflow",
            )
        })?;
        let next_offset = offset.checked_add(aligned).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "netlink message offset overflow",
            )
        })?;

        if next_offset > bytes.len() {
            if message_end == bytes.len() {
                offset = bytes.len();
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "truncated netlink message padding",
                ));
            }
        } else {
            offset = next_offset;
        }
    }

    Ok(DatagramStatus::Continue)
}

pub(crate) fn recv_multi(
    sock: &mut Socket,
    expect_seq: u32,
) -> io::Result<Vec<NetlinkMessage<RouteNetlinkMessage>>> {
    let mut out = Vec::new();
    let mut buf = vec![0u8; RECV_BUFSZ];
    let deadline = Instant::now() + RECV_TIMEOUT;

    loop {
        if Instant::now() >= deadline {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "timed out before netlink dump completed",
            ));
        }

        match sock.recv_from(&mut &mut buf[..], libc::MSG_TRUNC) {
            Ok((size, from)) => {
                if from.port_number() != 0 {
                    continue;
                }
                if size > buf.len() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "netlink datagram exceeds receive buffer: size={size}, capacity={}",
                            buf.len()
                        ),
                    ));
                }
                if matches!(
                    parse_datagram(&buf[..size], expect_seq, &mut out)?,
                    DatagramStatus::Done
                ) {
                    return Ok(out);
                }
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(5));
            }
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{DatagramStatus, MIN_NLMSG_HEADER_LEN, parse_datagram};
    use netlink_packet_core::{DoneMessage, NLM_F_DUMP_INTR, NetlinkMessage, NetlinkPayload};
    use netlink_packet_route::{RouteNetlinkMessage, link::LinkMessage};

    const SEQ: u32 = 42;

    fn serialize(mut message: NetlinkMessage<RouteNetlinkMessage>) -> Vec<u8> {
        message.finalize();
        let mut bytes = vec![0; message.buffer_len()];
        message.serialize(&mut bytes);
        bytes
    }

    fn link_message(seq: u32) -> NetlinkMessage<RouteNetlinkMessage> {
        let mut message =
            NetlinkMessage::from(RouteNetlinkMessage::NewLink(LinkMessage::default()));
        message.header.sequence_number = seq;
        message
    }

    fn done_message() -> NetlinkMessage<RouteNetlinkMessage> {
        let mut message = NetlinkMessage::new(
            Default::default(),
            NetlinkPayload::Done(DoneMessage::default()),
        );
        message.header.sequence_number = SEQ;
        message
    }

    #[test]
    fn parses_matching_messages_until_done() {
        let mut bytes = serialize(link_message(SEQ + 1));
        bytes.extend(serialize(link_message(SEQ)));
        bytes.extend(serialize(done_message()));
        let mut messages = Vec::new();

        let status = parse_datagram(&bytes, SEQ, &mut messages).unwrap();

        assert!(matches!(status, DatagramStatus::Done));
        assert_eq!(messages.len(), 1);
    }

    #[test]
    fn rejects_truncated_headers_and_messages() {
        let mut messages = Vec::new();
        let header_error =
            parse_datagram(&[0; MIN_NLMSG_HEADER_LEN - 1], SEQ, &mut messages).unwrap_err();
        assert_eq!(header_error.kind(), std::io::ErrorKind::InvalidData);

        let mut bytes = serialize(link_message(SEQ));
        bytes.truncate(bytes.len() - 1);
        let message_error = parse_datagram(&bytes, SEQ, &mut messages).unwrap_err();
        assert_eq!(message_error.kind(), std::io::ErrorKind::InvalidData);
    }

    #[test]
    fn rejects_interrupted_dumps() {
        let mut done = done_message();
        done.header.flags = NLM_F_DUMP_INTR;
        let mut messages = Vec::new();

        let error = parse_datagram(&serialize(done), SEQ, &mut messages).unwrap_err();

        assert_eq!(error.to_string(), "netlink dump was interrupted");
    }

    #[test]
    fn rejects_failed_dump_completion() {
        let mut done = done_message();
        if let NetlinkPayload::Done(payload) = &mut done.payload {
            payload.code = -libc::ENOBUFS;
        }
        let mut messages = Vec::new();

        let error = parse_datagram(&serialize(done), SEQ, &mut messages).unwrap_err();

        assert_eq!(
            error.to_string(),
            format!("netlink dump failed: code={}", -libc::ENOBUFS)
        );
    }

    #[test]
    fn rejects_receive_overruns() {
        let mut message = NetlinkMessage::new(
            Default::default(),
            NetlinkPayload::<RouteNetlinkMessage>::Overrun(Vec::new()),
        );
        message.header.sequence_number = SEQ;
        let mut messages = Vec::new();

        let error = parse_datagram(&serialize(message), SEQ, &mut messages).unwrap_err();

        assert_eq!(error.to_string(), "netlink receive overrun");
    }
}
