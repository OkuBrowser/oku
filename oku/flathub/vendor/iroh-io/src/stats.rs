//! Utilities for measuring time for io ops.
//!
//! This is useful for debugging performance issues and for io bookkeeping.
//! These wrapper do not distinguish between successful and failed operations.
//! The expectation is that failed operations will be counted separately.
//!
//! Statistics are always added using saturating arithmetic, so there can't be
//! a panic even in unlikely scenarios.
use std::{
    future::Future,
    io,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use bytes::Bytes;
use pin_project::pin_project;

use crate::{AsyncSliceReader, AsyncSliceWriter, AsyncStreamReader, AsyncStreamWriter};

/// Statistics about a tracked operation.
#[derive(Debug, Clone, Copy, Default)]
pub struct Stats {
    /// The number of times the operation was called.
    pub count: u64,
    /// The total time spent in the operation.
    pub duration: Duration,
}

impl std::ops::Add<Stats> for Stats {
    type Output = Stats;

    fn add(self, rhs: Stats) -> Self::Output {
        Self {
            count: self.count.saturating_add(rhs.count),
            duration: self.duration.saturating_add(rhs.duration),
        }
    }
}

impl std::ops::AddAssign<Stats> for Stats {
    fn add_assign(&mut self, rhs: Stats) {
        *self = *self + rhs;
    }
}

/// Statistics about a tracked operation.
#[derive(Debug, Clone, Copy, Default)]
pub struct SizeAndStats {
    /// The number of bytes read or written.
    pub size: u64,
    /// Statistics about the operation.
    pub stats: Stats,
}

impl From<Stats> for SizeAndStats {
    fn from(stats: Stats) -> Self {
        Self { size: 0, stats }
    }
}

impl std::ops::Add<SizeAndStats> for SizeAndStats {
    type Output = SizeAndStats;

    fn add(self, rhs: SizeAndStats) -> Self::Output {
        Self {
            size: self.size.saturating_add(rhs.size),
            stats: self.stats + rhs.stats,
        }
    }
}

impl std::ops::AddAssign<SizeAndStats> for SizeAndStats {
    fn add_assign(&mut self, rhs: SizeAndStats) {
        *self = *self + rhs;
    }
}

/// Statistics about a stream writer.
#[derive(Debug, Clone, Copy, Default)]
pub struct StreamWriterStats {
    /// Statistics about the `write` operation.
    pub write: SizeAndStats,
    /// Statistics about the `write_bytes` operation.
    pub write_bytes: SizeAndStats,
    /// Statistics about the `sync` operation.
    pub sync: Stats,
}

impl StreamWriterStats {
    /// Gives the total stats for this writer.
    ///
    /// This adds the count and duration from all three operations,
    /// and the total number of bytes written from write and wite_bytes.
    ///
    /// It is important to also add the sync stats, because some buffered
    /// writers will do most actual io in sync.
    pub fn total(&self) -> SizeAndStats {
        self.write + self.write_bytes + self.sync.into()
    }
}

impl std::ops::Add<StreamWriterStats> for StreamWriterStats {
    type Output = StreamWriterStats;

    fn add(self, rhs: StreamWriterStats) -> Self::Output {
        Self {
            write: self.write + rhs.write,
            write_bytes: self.write_bytes + rhs.write_bytes,
            sync: self.sync + rhs.sync,
        }
    }
}

impl std::ops::AddAssign<StreamWriterStats> for StreamWriterStats {
    fn add_assign(&mut self, rhs: StreamWriterStats) {
        *self = *self + rhs;
    }
}

/// A stream writer that tracks the time spent in write operations.
#[derive(Debug, Clone)]
pub struct TrackingStreamWriter<W> {
    inner: W,
    /// Statistics about the write operations.
    stats: StreamWriterStats,
}

impl<W> TrackingStreamWriter<W> {
    /// Create a new `TrackingStreamWriter`.
    pub fn new(inner: W) -> Self {
        Self {
            inner,
            stats: Default::default(),
        }
    }

    /// Get the statistics about the write operations.
    pub fn stats(&self) -> StreamWriterStats {
        self.stats
    }
}

impl<W: AsyncStreamWriter> AsyncStreamWriter for TrackingStreamWriter<W> {
    async fn write(&mut self, data: &[u8]) -> io::Result<()> {
        // increase the size by the length of the data, even if the write fails
        self.stats.write.size = self.stats.write.size.saturating_add(data.len() as u64);
        AggregateStats::new(self.inner.write(data), &mut self.stats.write.stats).await
    }

    async fn write_bytes(&mut self, data: bytes::Bytes) -> io::Result<()> {
        // increase the size by the length of the data, even if the write fails
        self.stats.write_bytes.size = self
            .stats
            .write_bytes
            .size
            .saturating_add(data.len() as u64);
        AggregateStats::new(
            self.inner.write_bytes(data),
            &mut self.stats.write_bytes.stats,
        )
        .await
    }

    async fn sync(&mut self) -> io::Result<()> {
        AggregateStats::new(self.inner.sync(), &mut self.stats.sync).await
    }
}

/// Statistics about a stream writer.
#[derive(Debug, Clone, Copy, Default)]
pub struct StreamReaderStats {
    /// Statistics about the `read` operation.
    pub read: SizeAndStats,
}

impl StreamReaderStats {
    /// Gives the total stats for this reader.
    pub fn total(&self) -> SizeAndStats {
        self.read
    }
}

impl std::ops::Add<StreamReaderStats> for StreamReaderStats {
    type Output = StreamReaderStats;

    fn add(self, rhs: StreamReaderStats) -> Self::Output {
        Self {
            read: self.read + rhs.read,
        }
    }
}

impl std::ops::AddAssign<StreamReaderStats> for StreamReaderStats {
    fn add_assign(&mut self, rhs: StreamReaderStats) {
        *self = *self + rhs;
    }
}

/// A stream writer that tracks the time spent in write operations.
#[derive(Debug, Clone)]
pub struct TrackingStreamReader<W> {
    inner: W,
    /// Statistics about the write operations.
    stats: StreamReaderStats,
}

impl<W> TrackingStreamReader<W> {
    /// Create a new `TrackingStreamWriter`.
    pub fn new(inner: W) -> Self {
        Self {
            inner,
            stats: Default::default(),
        }
    }

    /// Get the statistics about the write operations.
    pub fn stats(&self) -> StreamReaderStats {
        self.stats
    }
}

impl<W: AsyncStreamReader> AsyncStreamReader for TrackingStreamReader<W> {
    async fn read_bytes(&mut self, len: usize) -> io::Result<Bytes> {
        AggregateSizeAndStats::new(self.inner.read_bytes(len), &mut self.stats.read).await
    }

    async fn read<const L: usize>(&mut self) -> io::Result<[u8; L]> {
        AggregateSizeAndStats::new(self.inner.read(), &mut self.stats.read).await
    }
}

/// Statistics about a slice reader.
#[derive(Debug, Clone, Copy, Default)]
pub struct SliceReaderStats {
    /// Statistics about the `read_at` operation.
    pub read_at: SizeAndStats,
    /// Statistics about the `len` operation.
    pub len: Stats,
}

impl SliceReaderStats {
    /// Gives the total stats for this reader.
    pub fn total(&self) -> SizeAndStats {
        self.read_at + self.len.into()
    }
}

impl std::ops::Add<SliceReaderStats> for SliceReaderStats {
    type Output = SliceReaderStats;

    fn add(self, rhs: SliceReaderStats) -> Self::Output {
        Self {
            read_at: self.read_at + rhs.read_at,
            len: self.len + rhs.len,
        }
    }
}

impl std::ops::AddAssign<SliceReaderStats> for SliceReaderStats {
    fn add_assign(&mut self, rhs: SliceReaderStats) {
        *self = *self + rhs;
    }
}

/// A slice reader that tracks the time spent in read operations.
#[derive(Debug, Clone)]
pub struct TrackingSliceReader<R> {
    inner: R,
    /// Statistics about the read operations.
    stats: SliceReaderStats,
}

impl<R: AsyncSliceReader> TrackingSliceReader<R> {
    /// Create a new `TrackingSliceReader`.
    pub fn new(inner: R) -> Self {
        Self {
            inner,
            stats: Default::default(),
        }
    }

    /// Get the statistics about the read operations.
    pub fn stats(&self) -> SliceReaderStats {
        self.stats
    }
}

impl<R: AsyncSliceReader> AsyncSliceReader for TrackingSliceReader<R> {
    async fn read_at(&mut self, offset: u64, len: usize) -> io::Result<Bytes> {
        AggregateSizeAndStats::new(self.inner.read_at(offset, len), &mut self.stats.read_at).await
    }

    async fn size(&mut self) -> io::Result<u64> {
        AggregateStats::new(self.inner.size(), &mut self.stats.len).await
    }
}

/// Statistics about a slice writer.
#[derive(Debug, Clone, Copy, Default)]
pub struct SliceWriterStats {
    /// Statistics about the `write_at` operation.
    pub write_at: SizeAndStats,
    /// Statistics about the `write_bytes_at` operation.
    pub write_bytes_at: SizeAndStats,
    /// Statistics about the `set_len` operation.
    pub set_len: Stats,
    /// Statistics about the `sync` operation.
    pub sync: Stats,
}

impl SliceWriterStats {
    /// Gives the total stats for this writer.
    pub fn total(&self) -> SizeAndStats {
        self.write_at + self.write_bytes_at + self.set_len.into() + self.sync.into()
    }
}

impl std::ops::Add<SliceWriterStats> for SliceWriterStats {
    type Output = SliceWriterStats;

    fn add(self, rhs: SliceWriterStats) -> Self::Output {
        Self {
            write_at: self.write_at + rhs.write_at,
            write_bytes_at: self.write_bytes_at + rhs.write_bytes_at,
            set_len: self.set_len + rhs.set_len,
            sync: self.sync + rhs.sync,
        }
    }
}

impl std::ops::AddAssign<SliceWriterStats> for SliceWriterStats {
    fn add_assign(&mut self, rhs: SliceWriterStats) {
        *self = *self + rhs;
    }
}

/// A slice writer that tracks the time spent in write operations.
#[derive(Debug, Clone)]
pub struct TrackingSliceWriter<W> {
    inner: W,
    /// Statistics about the write operations.
    stats: SliceWriterStats,
}

impl<W> TrackingSliceWriter<W> {
    /// Create a new `TrackingSliceWriter`.
    pub fn new(inner: W) -> Self {
        Self {
            inner,
            stats: Default::default(),
        }
    }

    /// Get the statistics about the write operations.
    pub fn stats(&self) -> SliceWriterStats {
        self.stats
    }
}

impl<W: AsyncSliceWriter> AsyncSliceWriter for TrackingSliceWriter<W> {
    async fn write_at(&mut self, offset: u64, data: &[u8]) -> io::Result<()> {
        // increase the size by the length of the data, even if the write fails
        self.stats.write_at.size = self.stats.write_at.size.saturating_add(data.len() as u64);
        AggregateStats::new(
            self.inner.write_at(offset, data),
            &mut self.stats.write_at.stats,
        )
        .await
    }

    async fn write_bytes_at(&mut self, offset: u64, data: bytes::Bytes) -> io::Result<()> {
        // increase the size by the length of the data, even if the write fails
        self.stats.write_bytes_at.size = self
            .stats
            .write_bytes_at
            .size
            .saturating_add(data.len() as u64);
        AggregateStats::new(
            self.inner.write_bytes_at(offset, data),
            &mut self.stats.write_bytes_at.stats,
        )
        .await
    }

    async fn set_len(&mut self, len: u64) -> io::Result<()> {
        AggregateStats::new(self.inner.set_len(len), &mut self.stats.set_len).await
    }

    async fn sync(&mut self) -> io::Result<()> {
        AggregateStats::new(self.inner.sync(), &mut self.stats.sync).await
    }
}

/// A future that measures the time spent until it is ready.
#[pin_project]
#[derive(Debug)]
pub struct AggregateStats<'a, F> {
    #[pin]
    inner: F,
    start: std::time::Instant,
    target: &'a mut Stats,
}

impl<'a, F: Future> AggregateStats<'a, F> {
    /// Create a new `WriteTiming` future.
    pub fn new(inner: F, target: &'a mut Stats) -> Self {
        Self {
            inner,
            target,
            start: std::time::Instant::now(),
        }
    }
}

impl<F: Future> Future for AggregateStats<'_, F> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let p = self.project();
        match p.inner.poll(cx) {
            Poll::Ready(x) => {
                p.target.duration = p.target.duration.saturating_add(p.start.elapsed());
                p.target.count = p.target.count.saturating_add(1);
                Poll::Ready(x)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

/// A future that measures the time spent until it is ready, and the size of the result.
#[pin_project]
#[derive(Debug)]
pub struct AggregateSizeAndStats<'a, F> {
    #[pin]
    inner: F,
    start: std::time::Instant,
    target: &'a mut SizeAndStats,
}

impl<'a, F: Future> AggregateSizeAndStats<'a, F> {
    /// Create a new `WriteTiming` future.
    pub fn new(inner: F, target: &'a mut SizeAndStats) -> Self {
        Self {
            inner,
            target,
            start: std::time::Instant::now(),
        }
    }
}

/// A trait for types that may have a size.
pub trait ReadResult {
    /// Get the size of the value, if known.
    fn size(&self) -> Option<u64>;
}

impl<T: AsRef<[u8]>> ReadResult for std::io::Result<T> {
    fn size(&self) -> Option<u64> {
        match self {
            Ok(x) => Some(x.as_ref().len() as u64),
            Err(_) => None,
        }
    }
}

impl<F: Future> Future for AggregateSizeAndStats<'_, F>
where
    F::Output: ReadResult,
{
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let p = self.project();
        match p.inner.poll(cx) {
            Poll::Ready(x) => {
                p.target.stats.duration = p.target.stats.duration.saturating_add(p.start.elapsed());
                p.target.stats.count = p.target.stats.count.saturating_add(1);
                if let Some(size) = x.size() {
                    // increase the size by the length of the data, if known
                    // e.g. an error will not have a size
                    p.target.size = p.target.size.saturating_add(size);
                }
                Poll::Ready(x)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn tracking_stream_writer() {
        let mut writer = TrackingStreamWriter::new(Vec::<u8>::new());
        writer.write(&[1, 2, 3]).await.unwrap();
        writer.write(&[1, 2, 3]).await.unwrap();
        writer.write_bytes(vec![1, 2, 3].into()).await.unwrap();
        writer.sync().await.unwrap();
        assert_eq!(writer.stats().write.size, 6);
        assert_eq!(writer.stats().write.stats.count, 2);
        assert_eq!(writer.stats().write_bytes.size, 3);
        assert_eq!(writer.stats().write_bytes.stats.count, 1);
        assert_eq!(writer.stats().sync.count, 1);
    }

    #[tokio::test]
    async fn tracking_stream_reader() {
        let mut writer = TrackingStreamReader::new(Bytes::from(vec![0, 1, 2, 3]));
        writer.read_bytes(2).await.unwrap();
        writer.read_bytes(3).await.unwrap();
        assert_eq!(writer.stats().read.size, 4); // not 5, because the last read was only 2 bytes
        assert_eq!(writer.stats().read.stats.count, 2);
    }

    #[tokio::test]
    async fn tracking_slice_writer() {
        let mut writer = TrackingSliceWriter::new(Vec::new());
        writer.write_at(0, &[1, 2, 3]).await.unwrap();
        writer.write_at(10, &[1, 2, 3]).await.unwrap();
        writer
            .write_bytes_at(20, vec![1, 2, 3].into())
            .await
            .unwrap();
        writer.sync().await.unwrap();
        writer.set_len(0).await.unwrap();
        assert_eq!(writer.stats().write_at.size, 6);
        assert_eq!(writer.stats().write_at.stats.count, 2);
        assert_eq!(writer.stats().write_bytes_at.size, 3);
        assert_eq!(writer.stats().write_bytes_at.stats.count, 1);
        assert_eq!(writer.stats().set_len.count, 1);
        assert_eq!(writer.stats().sync.count, 1);
    }

    #[tokio::test]
    async fn tracking_slice_reader() {
        let mut reader = TrackingSliceReader::new(Bytes::from(vec![1u8, 2, 3]));
        let _ = reader.read_at(0, 1).await.unwrap();
        let _ = reader.read_at(10, 1).await.unwrap();
        let _ = reader.size().await.unwrap();
        assert_eq!(reader.stats().read_at.size, 1);
        assert_eq!(reader.stats().read_at.stats.count, 2);
        assert_eq!(reader.stats().len.count, 1);
    }
}
