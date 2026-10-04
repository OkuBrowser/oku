use std::io;

use bytes::{Bytes, BytesMut};

use super::{AsyncSliceReader, AsyncSliceWriter};

impl AsyncSliceReader for bytes::Bytes {
    async fn read_at(&mut self, offset: u64, len: usize) -> io::Result<Bytes> {
        Ok(get_limited_slice(self, offset, len))
    }

    async fn size(&mut self) -> io::Result<u64> {
        Ok(Bytes::len(self) as u64)
    }
}

impl AsyncSliceReader for bytes::BytesMut {
    async fn read_at(&mut self, offset: u64, len: usize) -> io::Result<Bytes> {
        Ok(copy_limited_slice(self, offset, len))
    }

    async fn size(&mut self) -> io::Result<u64> {
        Ok(BytesMut::len(self) as u64)
    }
}

impl AsyncSliceReader for &[u8] {
    async fn read_at(&mut self, offset: u64, len: usize) -> io::Result<Bytes> {
        Ok(copy_limited_slice(self, offset, len))
    }

    async fn size(&mut self) -> io::Result<u64> {
        Ok(self.len() as u64)
    }
}

impl AsyncSliceWriter for bytes::BytesMut {
    async fn write_bytes_at(&mut self, offset: u64, data: Bytes) -> io::Result<()> {
        write_extend(self, offset, &data)
    }

    async fn write_at(&mut self, offset: u64, data: &[u8]) -> io::Result<()> {
        write_extend(self, offset, data)
    }

    async fn set_len(&mut self, len: u64) -> io::Result<()> {
        let len = len.try_into().unwrap_or(usize::MAX);
        self.resize(len, 0);
        Ok(())
    }

    async fn sync(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl AsyncSliceWriter for Vec<u8> {
    async fn write_bytes_at(&mut self, offset: u64, data: Bytes) -> io::Result<()> {
        write_extend_vec(self, offset, &data)
    }

    async fn write_at(&mut self, offset: u64, data: &[u8]) -> io::Result<()> {
        write_extend_vec(self, offset, data)
    }

    async fn set_len(&mut self, len: u64) -> io::Result<()> {
        let len = len.try_into().unwrap_or(usize::MAX);
        self.resize(len, 0);
        Ok(())
    }

    async fn sync(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(crate) fn limited_range(offset: u64, len: usize, buf_len: usize) -> std::ops::Range<usize> {
    if offset < buf_len as u64 {
        let start = offset as usize;
        let end = start.saturating_add(len).min(buf_len);
        start..end
    } else {
        0..0
    }
}

fn get_limited_slice(bytes: &Bytes, offset: u64, len: usize) -> Bytes {
    bytes.slice(limited_range(offset, len, bytes.len()))
}

fn copy_limited_slice(bytes: &[u8], offset: u64, len: usize) -> Bytes {
    bytes[limited_range(offset, len, bytes.len())]
        .to_vec()
        .into()
}

fn write_extend(bytes: &mut BytesMut, offset: u64, data: &[u8]) -> io::Result<()> {
    let start = usize::try_from(offset).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "start is too large to fit in usize",
        )
    })?;
    let end = start.checked_add(data.len()).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "offset + data.len() is too large to fit in usize",
        )
    })?;
    if data.is_empty() {
        return Ok(());
    }
    if end > BytesMut::len(bytes) {
        bytes.resize(start, 0);
        bytes.extend_from_slice(data);
    } else {
        bytes[start..end].copy_from_slice(data);
    }

    Ok(())
}

fn write_extend_vec(bytes: &mut Vec<u8>, offset: u64, data: &[u8]) -> io::Result<()> {
    let start = usize::try_from(offset).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "start is too large to fit in usize",
        )
    })?;
    let end = start.checked_add(data.len()).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "offset + data.len() is too large to fit in usize",
        )
    })?;
    if data.is_empty() {
        return Ok(());
    }
    if end > Vec::len(bytes) {
        bytes.resize(start, 0);
        bytes.extend_from_slice(data);
    } else {
        bytes[start..end].copy_from_slice(data);
    }

    Ok(())
}
