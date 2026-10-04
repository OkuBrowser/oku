use super::tcp::{TcpStream, TcpStreamInner};
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

impl futures::io::AsyncRead for TcpStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buffer: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        poll_read(&mut self.get_mut().inner, cx, buffer)
    }
}

impl futures::io::AsyncWrite for TcpStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buffer: &[u8],
    ) -> Poll<io::Result<usize>> {
        poll_write(&mut self.get_mut().inner, cx, buffer)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        poll_flush(&mut self.get_mut().inner, cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        poll_close(&mut self.get_mut().inner, cx)
    }
}

#[cfg(feature = "tokio")]
impl ::tokio::io::AsyncRead for TcpStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buffer: &mut ::tokio::io::ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let destination = buffer.initialize_unfilled();
        match poll_read(&mut self.get_mut().inner, cx, destination) {
            Poll::Ready(Ok(read)) => {
                buffer.advance(read);
                Poll::Ready(Ok(()))
            }
            Poll::Ready(Err(error)) => Poll::Ready(Err(error)),
            Poll::Pending => Poll::Pending,
        }
    }
}

#[cfg(feature = "tokio")]
impl ::tokio::io::AsyncWrite for TcpStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buffer: &[u8],
    ) -> Poll<io::Result<usize>> {
        poll_write(&mut self.get_mut().inner, cx, buffer)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        poll_flush(&mut self.get_mut().inner, cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        poll_close(&mut self.get_mut().inner, cx)
    }
}

fn poll_read(
    inner: &mut TcpStreamInner,
    cx: &mut Context<'_>,
    buffer: &mut [u8],
) -> Poll<io::Result<usize>> {
    match inner {
        #[cfg(feature = "tokio")]
        TcpStreamInner::Tokio(stream) => {
            let mut read_buffer = ::tokio::io::ReadBuf::new(buffer);
            match ::tokio::io::AsyncRead::poll_read(Pin::new(stream), cx, &mut read_buffer) {
                Poll::Ready(Ok(())) => Poll::Ready(Ok(read_buffer.filled().len())),
                Poll::Ready(Err(error)) => Poll::Ready(Err(error)),
                Poll::Pending => Poll::Pending,
            }
        }
        #[cfg(feature = "smol")]
        TcpStreamInner::Smol(stream) => {
            futures::io::AsyncRead::poll_read(Pin::new(stream), cx, buffer)
        }
        #[cfg(feature = "compio")]
        TcpStreamInner::Compio(stream) => {
            futures::io::AsyncRead::poll_read(stream.as_mut(), cx, buffer)
        }
        #[cfg(not(any(feature = "tokio", feature = "smol", feature = "compio")))]
        _ => unreachable!(),
    }
}

fn poll_write(
    inner: &mut TcpStreamInner,
    cx: &mut Context<'_>,
    buffer: &[u8],
) -> Poll<io::Result<usize>> {
    match inner {
        #[cfg(feature = "tokio")]
        TcpStreamInner::Tokio(stream) => {
            ::tokio::io::AsyncWrite::poll_write(Pin::new(stream), cx, buffer)
        }
        #[cfg(feature = "smol")]
        TcpStreamInner::Smol(stream) => {
            futures::io::AsyncWrite::poll_write(Pin::new(stream), cx, buffer)
        }
        #[cfg(feature = "compio")]
        TcpStreamInner::Compio(stream) => {
            futures::io::AsyncWrite::poll_write(stream.as_mut(), cx, buffer)
        }
        #[cfg(not(any(feature = "tokio", feature = "smol", feature = "compio")))]
        _ => unreachable!(),
    }
}

fn poll_flush(inner: &mut TcpStreamInner, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
    match inner {
        #[cfg(feature = "tokio")]
        TcpStreamInner::Tokio(stream) => ::tokio::io::AsyncWrite::poll_flush(Pin::new(stream), cx),
        #[cfg(feature = "smol")]
        TcpStreamInner::Smol(stream) => futures::io::AsyncWrite::poll_flush(Pin::new(stream), cx),
        #[cfg(feature = "compio")]
        TcpStreamInner::Compio(stream) => futures::io::AsyncWrite::poll_flush(stream.as_mut(), cx),
        #[cfg(not(any(feature = "tokio", feature = "smol", feature = "compio")))]
        _ => unreachable!(),
    }
}

fn poll_close(inner: &mut TcpStreamInner, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
    match inner {
        #[cfg(feature = "tokio")]
        TcpStreamInner::Tokio(stream) => {
            ::tokio::io::AsyncWrite::poll_shutdown(Pin::new(stream), cx)
        }
        #[cfg(feature = "smol")]
        TcpStreamInner::Smol(stream) => futures::io::AsyncWrite::poll_close(Pin::new(stream), cx),
        #[cfg(feature = "compio")]
        TcpStreamInner::Compio(stream) => futures::io::AsyncWrite::poll_close(stream.as_mut(), cx),
        #[cfg(not(any(feature = "tokio", feature = "smol", feature = "compio")))]
        _ => unreachable!(),
    }
}
