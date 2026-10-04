use std::{
    io::{self, ErrorKind, SeekFrom},
    pin::Pin,
    task::{Context, Poll},
};

use n0_future::StreamExt;

use crate::{
    api::{
        blobs::{Blobs, ReaderOptions},
        proto::ExportRangesItem,
    },
    Hash,
};

/// A reader for blobs that implements `AsyncRead` and `AsyncSeek`.
#[derive(Debug)]
pub struct BlobReader {
    blobs: Blobs,
    options: ReaderOptions,
    state: ReaderState,
}

#[derive(Default, derive_more::Debug)]
enum ReaderState {
    Idle {
        position: u64,
    },
    Seeking {
        position: u64,
    },
    Reading {
        position: u64,
        #[debug(skip)]
        op: n0_future::boxed::BoxStream<ExportRangesItem>,
    },
    #[default]
    Poisoned,
}

impl BlobReader {
    pub(super) fn new(blobs: Blobs, options: ReaderOptions) -> Self {
        Self {
            blobs,
            options,
            state: ReaderState::Idle { position: 0 },
        }
    }

    pub fn hash(&self) -> &Hash {
        &self.options.hash
    }
}

impl tokio::io::AsyncRead for BlobReader {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        let mut position1 = None;
        loop {
            let guard = &mut this.state;
            match std::mem::take(guard) {
                ReaderState::Idle { position } => {
                    // todo: read until next page boundary instead of fixed size
                    let len = buf.remaining() as u64;
                    let end = position.checked_add(len).ok_or_else(|| {
                        io::Error::new(ErrorKind::InvalidInput, "Position overflow when reading")
                    })?;
                    // start the export op for the entire size of the buffer, and convert to a stream
                    let stream = this
                        .blobs
                        .export_ranges(this.options.hash, position..end)
                        .stream();
                    position1 = Some(position);
                    *guard = ReaderState::Reading {
                        position,
                        op: Box::pin(stream),
                    };
                }
                ReaderState::Reading { position, mut op } => {
                    let position1 = position1.get_or_insert(position);
                    match op.poll_next(cx) {
                        Poll::Ready(Some(ExportRangesItem::Size(_))) => {
                            *guard = ReaderState::Reading { position, op };
                        }
                        Poll::Ready(Some(ExportRangesItem::Data(data))) => {
                            if data.offset != *position1 {
                                break Poll::Ready(Err(io::Error::other(
                                    "Data offset does not match expected position",
                                )));
                            }
                            buf.put_slice(&data.data);
                            // update just local position1, not the position in the state.
                            *position1 =
                                position1
                                    .checked_add(data.data.len() as u64)
                                    .ok_or_else(|| {
                                        io::Error::new(ErrorKind::InvalidInput, "Position overflow")
                                    })?;
                            *guard = ReaderState::Reading { position, op };
                        }
                        Poll::Ready(Some(ExportRangesItem::Error(err))) => {
                            *guard = ReaderState::Idle { position };
                            break Poll::Ready(Err(io::Error::other(format!(
                                "Error reading data: {err}"
                            ))));
                        }
                        Poll::Ready(None) => {
                            // done with the stream, go back in idle.
                            *guard = ReaderState::Idle {
                                position: *position1,
                            };
                            break Poll::Ready(Ok(()));
                        }
                        Poll::Pending => {
                            break if position != *position1 {
                                // we read some data so we need to abort the op.
                                //
                                // we can't be sure we won't be called with the same buf size next time.
                                *guard = ReaderState::Idle {
                                    position: *position1,
                                };
                                Poll::Ready(Ok(()))
                            } else {
                                // nothing was read yet, we remain in the reading state
                                //
                                // we make an assumption here that the next call will be with the same buf size.
                                *guard = ReaderState::Reading {
                                    position: *position1,
                                    op,
                                };
                                Poll::Pending
                            };
                        }
                    }
                }
                state @ ReaderState::Seeking { .. } => {
                    // should I try to recover from this or just keep it poisoned?
                    this.state = state;
                    break Poll::Ready(Err(io::Error::other("Can't read while seeking")));
                }
                ReaderState::Poisoned => {
                    break Poll::Ready(Err(io::Error::other("Reader is poisoned")));
                }
            };
        }
    }
}

impl tokio::io::AsyncSeek for BlobReader {
    fn start_seek(
        self: std::pin::Pin<&mut Self>,
        seek_from: tokio::io::SeekFrom,
    ) -> io::Result<()> {
        let this = self.get_mut();
        let guard = &mut this.state;
        match std::mem::take(guard) {
            ReaderState::Idle { position } => {
                let position1 = match seek_from {
                    SeekFrom::Start(pos) => pos,
                    SeekFrom::Current(offset) => {
                        position.checked_add_signed(offset).ok_or_else(|| {
                            io::Error::new(
                                ErrorKind::InvalidInput,
                                "Position overflow when seeking",
                            )
                        })?
                    }
                    SeekFrom::End(_offset) => {
                        // todo: support seeking from end if we know the size
                        return Err(io::Error::new(
                            ErrorKind::InvalidInput,
                            "Seeking from end is not supported yet",
                        ))?;
                    }
                };
                *guard = ReaderState::Seeking {
                    position: position1,
                };
                Ok(())
            }
            ReaderState::Reading { .. } => Err(io::Error::other("Can't seek while reading")),
            ReaderState::Seeking { .. } => Err(io::Error::other("Already seeking")),
            ReaderState::Poisoned => Err(io::Error::other("Reader is poisoned")),
        }
    }

    fn poll_complete(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<u64>> {
        let this = self.get_mut();
        let guard = &mut this.state;
        Poll::Ready(match std::mem::take(guard) {
            ReaderState::Seeking { position } => {
                *guard = ReaderState::Idle { position };
                Ok(position)
            }
            ReaderState::Idle { position } => {
                // seek calls poll_complete just in case, to finish a pending seek operation
                // before the next seek operation. So it is poll_complete/start_seek/poll_complete
                *guard = ReaderState::Idle { position };
                Ok(position)
            }
            state @ ReaderState::Reading { .. } => {
                // should I try to recover from this or just keep it poisoned?
                *guard = state;
                Err(io::Error::other("Can't seek while reading"))
            }
            ReaderState::Poisoned => Err(io::Error::other("Reader is poisoned")),
        })
    }
}

#[cfg(test)]
#[cfg(feature = "fs-store")]
mod tests {
    use bao_tree::ChunkRanges;
    use testresult::TestResult;
    use tokio::io::{AsyncReadExt, AsyncSeekExt};

    use super::*;
    use crate::{
        protocol::ChunkRangesExt,
        store::{
            fs::{
                tests::{test_data, INTERESTING_SIZES},
                FsStore,
            },
            mem::MemStore,
            util::tests::create_n0_bao,
        },
    };

    async fn reader_smoke(blobs: &Blobs) -> TestResult<()> {
        for size in INTERESTING_SIZES {
            let data = test_data(size);
            let tag = blobs.add_bytes(data.clone()).await?;
            // read all
            {
                let mut reader = blobs.reader(tag.hash);
                let mut buf = Vec::new();
                reader.read_to_end(&mut buf).await?;
                assert_eq!(buf, data);
                let pos = reader.stream_position().await?;
                assert_eq!(pos, data.len() as u64);
            }
            // seek to mid and read all
            {
                let mut reader = blobs.reader(tag.hash);
                let mid = size / 2;
                reader.seek(SeekFrom::Start(mid as u64)).await?;
                let mut buf = Vec::new();
                reader.read_to_end(&mut buf).await?;
                assert_eq!(buf, data[mid..].to_vec());
                let pos = reader.stream_position().await?;
                assert_eq!(pos, data.len() as u64);
            }
        }
        Ok(())
    }

    async fn reader_partial(blobs: &Blobs) -> TestResult<()> {
        for size in INTERESTING_SIZES {
            let data = test_data(size);
            let ranges = ChunkRanges::chunk(0);
            let (hash, bao) = create_n0_bao(&data, &ranges)?;
            println!("importing {} bytes", bao.len());
            blobs.import_bao_bytes(hash, ranges.clone(), bao).await?;
            // read the first chunk or the entire blob, whatever is smaller
            // this should work!
            {
                let mut reader = blobs.reader(hash);
                let valid = size.min(1024);
                let mut buf = vec![0u8; valid];
                reader.read_exact(&mut buf).await?;
                assert_eq!(buf, data[..valid]);
                let pos = reader.stream_position().await?;
                assert_eq!(pos, valid as u64);
            }
            if size > 1024 {
                // read the part we don't have - should immediately return an error
                {
                    let mut reader = blobs.reader(hash);
                    let mut rest = vec![0u8; size - 1024];
                    reader.seek(SeekFrom::Start(1024)).await?;
                    let res = reader.read_exact(&mut rest).await;
                    assert!(res.is_err());
                }
                // read crossing the end of the blob - should return an error despite
                // the first bytes being valid.
                // A read that fails should not update the stream position.
                {
                    let mut reader = blobs.reader(hash);
                    let mut buf = vec![0u8; size];
                    let res = reader.read(&mut buf).await;
                    assert!(res.is_err());
                    let pos = reader.stream_position().await?;
                    assert_eq!(pos, 0);
                }
            }
        }
        Ok(())
    }

    #[tokio::test]
    async fn reader_partial_fs() -> TestResult<()> {
        let testdir = tempfile::tempdir()?;
        let store = FsStore::load(testdir.path().to_owned()).await?;
        reader_partial(store.blobs()).await?;
        Ok(())
    }

    #[tokio::test]
    async fn reader_partial_memory() -> TestResult<()> {
        let store = MemStore::new();
        reader_partial(store.blobs()).await?;
        Ok(())
    }

    #[tokio::test]
    async fn reader_smoke_fs() -> TestResult<()> {
        let testdir = tempfile::tempdir()?;
        let store = FsStore::load(testdir.path().to_owned()).await?;
        reader_smoke(store.blobs()).await?;
        Ok(())
    }

    #[tokio::test]
    async fn reader_smoke_memory() -> TestResult<()> {
        let store = MemStore::new();
        reader_smoke(store.blobs()).await?;
        Ok(())
    }
}
