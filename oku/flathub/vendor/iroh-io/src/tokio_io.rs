//! Blocking io for [std::fs::File], using the tokio blocking task pool.
use std::{
    future::Future,
    io::{self, Read, Seek, SeekFrom},
    path::PathBuf,
    pin::Pin,
    task::{Context, Poll},
};

use bytes::Bytes;
use pin_project::pin_project;
use tokio::{
    io::{AsyncReadExt, AsyncWrite},
    task::{spawn_blocking, JoinHandle},
};

use super::{make_io_error, AsyncSliceReader, AsyncSliceWriter, AsyncStreamWriter};
use crate::AsyncStreamReader;

const MAX_PREALLOC: usize = 1024 * 16;

/// A wrapper around a [std::fs::File] that implements [AsyncSliceReader] and [AsyncSliceWriter]
#[derive(Debug)]
pub struct File(Option<FileAdapterFsm>);

impl File {
    /// Create a new [File] from a function that creates a [std::fs::File]
    pub async fn create(
        create_file: impl FnOnce() -> io::Result<std::fs::File> + Send + 'static,
    ) -> io::Result<Self> {
        let inner = spawn_blocking(create_file).await.map_err(make_io_error)??;
        Ok(Self::from_std(inner))
    }

    /// Create a new [File] from a [std::fs::File]
    ///
    /// This is fine if you already have a [std::fs::File] and want to use it with [File],
    /// but opening a file is a blocking op that you probably don't want to do in an async context.
    pub fn from_std(file: std::fs::File) -> Self {
        Self(Some(FileAdapterFsm(file)))
    }

    /// Open a [File] from a path
    pub async fn open(path: PathBuf) -> io::Result<Self> {
        Self::create(move || std::fs::File::open(path)).await
    }

    /// Test helper to read the contents of the file
    #[cfg(test)]
    pub fn read_contents(&self) -> Vec<u8> {
        let mut std_file = &self.0.as_ref().unwrap().0;
        let mut t = Vec::new();
        // this is not needed since at least for POSIX IO "read your own writes"
        // is guaranteed.
        // std_file.sync_all().unwrap();
        std_file.rewind().unwrap();
        std_file.read_to_end(&mut t).unwrap();
        t
    }
}

/// Support for the [File]
pub mod file {
    use super::*;

    impl AsyncSliceReader for File {
        async fn read_at(&mut self, offset: u64, len: usize) -> io::Result<Bytes> {
            Asyncify::from(self.0.take().map(|t| (t.read_at(offset, len), &mut self.0))).await
        }

        async fn size(&mut self) -> io::Result<u64> {
            Asyncify::from(self.0.take().map(|t| (t.len(), &mut self.0))).await
        }
    }

    impl AsyncSliceWriter for File {
        async fn write_bytes_at(&mut self, offset: u64, data: Bytes) -> io::Result<()> {
            Asyncify::from(
                self.0
                    .take()
                    .map(|t| (t.write_bytes_at(offset, data), &mut self.0)),
            )
            .await
        }

        async fn write_at(&mut self, offset: u64, data: &[u8]) -> io::Result<()> {
            Asyncify::from(
                self.0
                    .take()
                    .map(|t| (t.write_at(offset, data), &mut self.0)),
            )
            .await
        }

        async fn sync(&mut self) -> io::Result<()> {
            Asyncify::from(self.0.take().map(|t| (t.sync(), &mut self.0))).await
        }

        async fn set_len(&mut self, len: u64) -> io::Result<()> {
            Asyncify::from(self.0.take().map(|t| (t.set_len(len), &mut self.0))).await
        }
    }
}

/// A future wrapper to unpack the result of a sync computation and store the
/// state on completion, making the io object available again.
#[derive(Debug)]
#[pin_project(project = AsyncifyProj)]
enum Asyncify<'a, R, T> {
    /// we got a future and a handle where we can store the state on completion
    Ok(
        #[pin] tokio::task::JoinHandle<(T, io::Result<R>)>,
        &'a mut Option<T>,
    ),
    /// the handle was busy
    BusyErr,
}

impl<'a, R, T> From<Option<(JoinHandle<(T, io::Result<R>)>, &'a mut Option<T>)>>
    for Asyncify<'a, R, T>
{
    fn from(value: Option<(JoinHandle<(T, io::Result<R>)>, &'a mut Option<T>)>) -> Self {
        match value {
            Some((f, h)) => Self::Ok(f, h),
            None => Self::BusyErr,
        }
    }
}

impl<'a, T: 'a, R> Future for Asyncify<'a, R, T> {
    type Output = io::Result<R>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project() {
            AsyncifyProj::Ok(f, h) => f.poll(cx).map(|x| {
                match x {
                    Ok((state, r)) => {
                        // we got a result, so we can store the state
                        **h = Some(state);
                        r
                    }
                    Err(e) => Err(io::Error::other(e)),
                }
            }),
            AsyncifyProj::BusyErr => Poll::Ready(io::Result::Err(io::Error::other(
                "previous io op not polled to completion",
            ))),
        }
    }
}

/// A wrapper around a [std::fs::File] that defines IO operations that spawn blocking tasks.
///
/// This implements all operations of [AsyncSliceReader] and [AsyncSliceWriter] in state
/// passing style.
#[derive(Debug)]
struct FileAdapterFsm(std::fs::File);

impl FileAdapterFsm {
    fn read_at(mut self, offset: u64, len: usize) -> JoinHandle<(Self, io::Result<Bytes>)> {
        fn inner<R: std::io::Read + std::io::Seek>(
            this: &mut R,
            offset: u64,
            len: usize,
            buf: &mut Vec<u8>,
        ) -> io::Result<()> {
            this.seek(SeekFrom::Start(offset))?;
            this.take(len as u64).read_to_end(buf)?;
            Ok(())
        }
        spawn_blocking(move || {
            // len is just the expected len, so if it is too big, we should not allocate
            // the entire size.
            let mut buf = Vec::with_capacity(len.min(MAX_PREALLOC));
            let res = inner(&mut self.0, offset, len, &mut buf);
            (self, res.map(|_| buf.into()))
        })
    }

    fn len(mut self) -> JoinHandle<(Self, io::Result<u64>)> {
        spawn_blocking(move || {
            let res = self.0.seek(SeekFrom::End(0));
            (self, res)
        })
    }
}

impl FileAdapterFsm {
    fn write_bytes_at(mut self, offset: u64, data: Bytes) -> JoinHandle<(Self, io::Result<()>)> {
        fn inner<W: std::io::Write + std::io::Seek>(
            this: &mut W,
            offset: u64,
            buf: &[u8],
        ) -> io::Result<()> {
            this.seek(SeekFrom::Start(offset))?;
            this.write_all(buf)?;
            Ok(())
        }
        spawn_blocking(move || {
            let res = inner(&mut self.0, offset, &data);
            (self, res)
        })
    }

    fn write_at(mut self, offset: u64, bytes: &[u8]) -> JoinHandle<(Self, io::Result<()>)> {
        fn inner<W: std::io::Write + std::io::Seek>(
            this: &mut W,
            offset: u64,
            buf: smallvec::SmallVec<[u8; 16]>,
        ) -> io::Result<()> {
            this.seek(SeekFrom::Start(offset))?;
            this.write_all(&buf)?;
            Ok(())
        }
        let t: smallvec::SmallVec<[u8; 16]> = bytes.into();
        spawn_blocking(move || {
            let res = inner(&mut self.0, offset, t);
            (self, res)
        })
    }

    fn set_len(self, len: u64) -> JoinHandle<(Self, io::Result<()>)> {
        spawn_blocking(move || {
            let res = self.0.set_len(len);
            (self, res)
        })
    }

    fn sync(self) -> JoinHandle<(Self, io::Result<()>)> {
        spawn_blocking(move || {
            let res = self.0.sync_all();
            (self, res)
        })
    }
}

/// Utility to convert an [AsyncWrite] into an [AsyncSliceWriter] by just ignoring the offsets
#[derive(Debug)]
pub struct ConcatenateSliceWriter<W>(W);

impl<W> ConcatenateSliceWriter<W> {
    /// Create a new `ConcatenateSliceWriter` from an inner writer
    pub fn new(inner: W) -> Self {
        Self(inner)
    }

    /// Return the inner writer
    pub fn into_inner(self) -> W {
        self.0
    }
}

impl<W: AsyncWrite + Unpin + 'static> AsyncSliceWriter for ConcatenateSliceWriter<W> {
    async fn write_bytes_at(&mut self, _offset: u64, data: Bytes) -> io::Result<()> {
        self.0.write_all(&data).await
    }

    async fn write_at(&mut self, _offset: u64, bytes: &[u8]) -> io::Result<()> {
        self.0.write_all(bytes).await
    }

    async fn sync(&mut self) -> io::Result<()> {
        self.0.flush().await
    }

    async fn set_len(&mut self, _len: u64) -> io::Result<()> {
        io::Result::Ok(())
    }
}

/// Utility to convert a [tokio::io::AsyncWrite] into an [AsyncStreamWriter].
#[derive(Debug, Clone)]
pub struct TokioStreamWriter<T>(pub T);

use tokio::io::AsyncWriteExt;

impl<T: tokio::io::AsyncWrite + Unpin> AsyncStreamWriter for TokioStreamWriter<T> {
    async fn write(&mut self, data: &[u8]) -> io::Result<()> {
        self.0.write_all(data).await
    }

    async fn write_bytes(&mut self, data: Bytes) -> io::Result<()> {
        self.0.write_all(&data).await
    }

    async fn sync(&mut self) -> io::Result<()> {
        self.0.flush().await
    }
}

/// Utility to convert a [tokio::io::AsyncRead] into an [AsyncStreamReader].
#[derive(Debug, Clone)]
pub struct TokioStreamReader<T>(pub T);

impl<T> TokioStreamReader<T> {
    /// Create a new `TokioStreamReader` from an inner reader
    pub fn new(inner: T) -> Self {
        Self(inner)
    }

    /// Return the inner reader
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T: tokio::io::AsyncRead + Unpin> AsyncStreamReader for TokioStreamReader<T> {
    async fn read_bytes(&mut self, len: usize) -> io::Result<Bytes> {
        let mut buf = Vec::with_capacity(len.min(MAX_PREALLOC));
        (&mut self.0).take(len as u64).read_to_end(&mut buf).await?;
        Ok(buf.into())
    }

    async fn read<const L: usize>(&mut self) -> io::Result<[u8; L]> {
        let mut buf = [0; L];
        self.0.read_exact(&mut buf).await?;
        Ok(buf)
    }
}
