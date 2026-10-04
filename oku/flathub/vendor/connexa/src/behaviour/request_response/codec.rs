use bytes::Bytes;
use futures::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use libp2p::StreamProtocol;

#[derive(Debug, Copy, Clone)]
pub struct Codec {
    max_request_size: usize,
    max_response_size: usize,
}

impl Codec {
    pub fn new(max_request_size: usize, max_response_size: usize) -> Self {
        Self {
            max_response_size,
            max_request_size,
        }
    }
}

impl libp2p::request_response::Codec for Codec {
    type Protocol = StreamProtocol;
    type Request = Bytes;
    type Response = Bytes;

    async fn read_request<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
    ) -> std::io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        let max = self.max_request_size;
        let mut buffer = Vec::with_capacity(max.min(8 * 1024));
        io.take(max as u64 + 1).read_to_end(&mut buffer).await?;

        if buffer.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "request is empty",
            ));
        }
        if buffer.len() > max {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "request exceeds max size",
            ));
        }
        Ok(Bytes::from(buffer))
    }

    async fn read_response<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
    ) -> std::io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        let max = self.max_response_size;
        let mut buffer = Vec::with_capacity(max.min(8 * 1024));
        io.take(max as u64 + 1).read_to_end(&mut buffer).await?;

        if buffer.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "response is empty",
            ));
        }
        if buffer.len() > max {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "response exceeds max size",
            ));
        }
        Ok(Bytes::from(buffer))
    }

    async fn write_request<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        req: Self::Request,
    ) -> std::io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        if req.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "request is empty",
            ));
        }

        if req.len() > self.max_request_size {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "request exceeds max size",
            ));
        }

        io.write_all(&req).await?;
        Ok(())
    }

    async fn write_response<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        res: Self::Response,
    ) -> std::io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        if res.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "response is empty",
            ));
        }
        if res.len() > self.max_response_size {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "response exceeds max size",
            ));
        }
        io.write_all(&res).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::io::Cursor;
    use libp2p::request_response::Codec as _;

    fn protocol() -> StreamProtocol {
        StreamProtocol::new("/connexa/codec-test/1.0.0")
    }

    #[tokio::test]
    async fn request_round_trip() {
        let mut codec = Codec::new(1024, 1024);
        let mut sink = Cursor::new(Vec::new());
        codec
            .write_request(&protocol(), &mut sink, Bytes::from_static(b"hello"))
            .await
            .expect("write");
        let mut source = Cursor::new(sink.into_inner());
        let out = codec
            .read_request(&protocol(), &mut source)
            .await
            .expect("read");
        assert_eq!(out, Bytes::from_static(b"hello"));
    }

    #[tokio::test]
    async fn response_round_trip() {
        let mut codec = Codec::new(1024, 1024);
        let mut sink = Cursor::new(Vec::new());
        codec
            .write_response(&protocol(), &mut sink, Bytes::from_static(b"world"))
            .await
            .expect("write");
        let mut source = Cursor::new(sink.into_inner());
        let out = codec
            .read_response(&protocol(), &mut source)
            .await
            .expect("read");
        assert_eq!(out, Bytes::from_static(b"world"));
    }

    #[tokio::test]
    async fn read_accepts_request_at_max_size() {
        let max = 8;
        let mut codec = Codec::new(max, 1024);
        let mut source = Cursor::new(vec![0xAB_u8; max]);
        let out = codec
            .read_request(&protocol(), &mut source)
            .await
            .expect("read");
        assert_eq!(out.len(), max);
    }

    #[tokio::test]
    async fn read_rejects_oversized_request() {
        let max = 8;
        let mut codec = Codec::new(max, 1024);
        let mut source = Cursor::new(vec![0xAB_u8; max + 4]);
        let err = codec
            .read_request(&protocol(), &mut source)
            .await
            .expect_err("oversized request must be rejected");
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    }

    #[tokio::test]
    async fn read_rejects_empty_request() {
        let mut codec = Codec::new(1024, 1024);
        let mut source = Cursor::new(Vec::new());
        let err = codec
            .read_request(&protocol(), &mut source)
            .await
            .expect_err("empty request must be rejected");
        assert_eq!(err.kind(), std::io::ErrorKind::UnexpectedEof);
    }

    #[tokio::test]
    async fn write_rejects_oversized_response() {
        let mut codec = Codec::new(1024, 4);
        let mut sink = Cursor::new(Vec::new());
        let err = codec
            .write_response(&protocol(), &mut sink, Bytes::from_static(b"toolong"))
            .await
            .expect_err("oversized response must be rejected");
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    }
}
