use std::future::Future;
use std::io;
use std::io::SeekFrom;
use std::pin::Pin;
#[cfg(any(
    target_arch = "wasm32",
    all(feature = "tokio", not(target_arch = "wasm32")),
    all(
        not(target_arch = "wasm32"),
        any(feature = "compio", feature = "threadpool", feature = "lite")
    )
))]
use std::task::ready;
use std::task::{Context, Poll};

#[cfg(all(
    not(target_arch = "wasm32"),
    not(feature = "compio"),
    any(feature = "threadpool", feature = "lite")
))]
type IoFuture = Pin<Box<dyn Future<Output = IoOutput> + Send + Sync + 'static>>;
#[cfg(any(target_arch = "wasm32", feature = "compio"))]
type IoFuture = Pin<Box<dyn Future<Output = IoOutput> + 'static>>;

#[derive(Default)]
pub(super) struct FileIo {
    #[cfg(any(
        target_arch = "wasm32",
        all(
            not(target_arch = "wasm32"),
            any(feature = "compio", feature = "threadpool", feature = "lite")
        )
    ))]
    operation: Option<IoFuture>,
    #[cfg(any(
        target_arch = "wasm32",
        all(
            not(target_arch = "wasm32"),
            any(feature = "compio", feature = "threadpool", feature = "lite")
        )
    ))]
    buffered_read: Vec<u8>,
    #[cfg(any(
        target_arch = "wasm32",
        all(
            not(target_arch = "wasm32"),
            any(feature = "compio", feature = "threadpool", feature = "lite")
        )
    ))]
    buffered_read_position: usize,
    #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
    futures_seek: Option<SeekFrom>,
    #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
    tokio_seek: Option<SeekFrom>,
}

#[cfg(any(
    target_arch = "wasm32",
    all(
        not(target_arch = "wasm32"),
        any(feature = "compio", feature = "threadpool", feature = "lite")
    )
))]
enum IoOutput {
    Read {
        result: io::Result<usize>,
        buffer: Vec<u8>,
        position: Option<u64>,
    },
    Write {
        result: io::Result<usize>,
        buffer: Vec<u8>,
        position: Option<u64>,
    },
    Flush(io::Result<()>),
    Seek {
        requested: SeekFrom,
        result: io::Result<u64>,
    },
}

/// An open file that supports reading.
pub trait FileRead {
    /// Reads bytes into a buffer and returns how many bytes were read.
    fn read<'a>(&'a mut self, buffer: &'a mut [u8])
    -> impl Future<Output = io::Result<usize>> + 'a;

    /// Reads enough bytes to fill a buffer.
    fn read_exact<'a>(
        &'a mut self,
        mut buffer: &'a mut [u8],
    ) -> impl Future<Output = io::Result<()>> + 'a {
        async move {
            while !buffer.is_empty() {
                match self.read(buffer).await {
                    Ok(0) => {
                        return Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "failed to fill the whole buffer",
                        ));
                    }
                    Ok(read) => buffer = &mut buffer[read..],
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                    Err(error) => return Err(error),
                }
            }
            Ok(())
        }
    }

    /// Reads all remaining bytes and appends them to a vector.
    fn read_to_end<'a>(
        &'a mut self,
        output: &'a mut Vec<u8>,
    ) -> impl Future<Output = io::Result<usize>> + 'a {
        async move {
            let start = output.len();
            let mut buffer = [0; 8 * 1024];
            loop {
                match self.read(&mut buffer).await {
                    Ok(0) => return Ok(output.len() - start),
                    Ok(read) => output.extend_from_slice(&buffer[..read]),
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                    Err(error) => return Err(error),
                }
            }
        }
    }

    /// Reads all remaining bytes and appends them to a string.
    fn read_to_string<'a>(
        &'a mut self,
        output: &'a mut String,
    ) -> impl Future<Output = io::Result<usize>> + 'a {
        async move {
            let mut bytes = Vec::new();
            let read = self.read_to_end(&mut bytes).await?;
            let string = String::from_utf8(bytes)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.utf8_error()))?;
            output.push_str(&string);
            Ok(read)
        }
    }
}

/// An open file that supports writing.
pub trait FileWrite {
    /// Writes bytes and returns how many bytes were written.
    fn write<'a>(&'a mut self, buffer: &'a [u8]) -> impl Future<Output = io::Result<usize>> + 'a;

    /// Writes an entire buffer.
    fn write_all<'a>(
        &'a mut self,
        mut buffer: &'a [u8],
    ) -> impl Future<Output = io::Result<()>> + 'a {
        async move {
            while !buffer.is_empty() {
                match self.write(buffer).await {
                    Ok(0) => {
                        return Err(io::Error::new(
                            io::ErrorKind::WriteZero,
                            "failed to write the whole buffer",
                        ));
                    }
                    Ok(written) => buffer = &buffer[written..],
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                    Err(error) => return Err(error),
                }
            }
            Ok(())
        }
    }

    /// Flushes buffered data to the file.
    fn flush(&mut self) -> impl Future<Output = io::Result<()>>;
}

/// An open file that supports seeking.
pub trait FileSeek {
    /// Moves the file cursor and returns its new position.
    fn seek(&mut self, position: SeekFrom) -> impl Future<Output = io::Result<u64>>;

    /// Moves the file cursor to the start of the file.
    fn rewind(&mut self) -> impl Future<Output = io::Result<()>> {
        async move {
            self.seek(SeekFrom::Start(0)).await?;
            Ok(())
        }
    }

    /// Returns the current file cursor position.
    fn stream_position(&mut self) -> impl Future<Output = io::Result<u64>> {
        self.seek(SeekFrom::Current(0))
    }
}

#[cfg(any(target_arch = "wasm32", feature = "compio"))]
pub(super) fn seek_position(current: u64, end: u64, position: SeekFrom) -> io::Result<u64> {
    let position = match position {
        SeekFrom::Start(position) => return Ok(position),
        SeekFrom::End(offset) => i128::from(end) + i128::from(offset),
        SeekFrom::Current(offset) => i128::from(current) + i128::from(offset),
    };
    u64::try_from(position).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid seek to a negative or overflowing position",
        )
    })
}

impl futures::io::AsyncRead for super::File {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buffer: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        poll_read(&mut this.inner, &mut this.io, cx, buffer)
    }
}

impl futures::io::AsyncWrite for super::File {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buffer: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        poll_write(&mut this.inner, &mut this.io, cx, buffer)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        poll_flush(&mut this.inner, &mut this.io, cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        poll_flush(&mut this.inner, &mut this.io, cx)
    }
}

impl futures::io::AsyncSeek for super::File {
    fn poll_seek(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        position: SeekFrom,
    ) -> Poll<io::Result<u64>> {
        let this = self.get_mut();
        poll_seek(&mut this.inner, &mut this.io, cx, position)
    }
}

#[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
impl ::tokio::io::AsyncRead for super::File {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buffer: &mut ::tokio::io::ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        let destination = buffer.initialize_unfilled();
        let read = ready!(poll_read(&mut this.inner, &mut this.io, cx, destination))?;
        buffer.advance(read);
        Poll::Ready(Ok(()))
    }
}

#[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
impl ::tokio::io::AsyncWrite for super::File {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buffer: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        poll_write(&mut this.inner, &mut this.io, cx, buffer)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        poll_flush(&mut this.inner, &mut this.io, cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        poll_flush(&mut this.inner, &mut this.io, cx)
    }
}

#[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
impl ::tokio::io::AsyncSeek for super::File {
    fn start_seek(self: Pin<&mut Self>, position: SeekFrom) -> io::Result<()> {
        let this = self.get_mut();
        if this.io.tokio_seek.is_some() {
            return Err(io::Error::other("another seek operation is in progress"));
        }
        this.io.tokio_seek = Some(position);
        Ok(())
    }

    fn poll_complete(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<u64>> {
        let this = self.get_mut();
        let position = *this.io.tokio_seek.get_or_insert(SeekFrom::Current(0));
        let result = ready!(poll_seek(&mut this.inner, &mut this.io, cx, position));
        this.io.tokio_seek = None;
        Poll::Ready(result)
    }
}

fn poll_read(
    inner: &mut super::FileInner,
    state: &mut FileIo,
    cx: &mut Context<'_>,
    buffer: &mut [u8],
) -> Poll<io::Result<usize>> {
    let _ = &state;
    let _ = cx;
    if buffer.is_empty() {
        return Poll::Ready(Ok(0));
    }

    #[cfg(any(
        target_arch = "wasm32",
        all(
            not(target_arch = "wasm32"),
            any(feature = "compio", feature = "threadpool", feature = "lite")
        )
    ))]
    if state.buffered_read_position < state.buffered_read.len() {
        let available = &state.buffered_read[state.buffered_read_position..];
        let read = buffer.len().min(available.len());
        buffer[..read].copy_from_slice(&available[..read]);
        state.buffered_read_position += read;
        if state.buffered_read_position == state.buffered_read.len() {
            state.buffered_read.clear();
            state.buffered_read_position = 0;
        }
        return Poll::Ready(Ok(read));
    }

    match inner {
        #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
        super::FileInner::Tokio(file) => {
            let mut read_buffer = ::tokio::io::ReadBuf::new(buffer);
            ready!(::tokio::io::AsyncRead::poll_read(
                Pin::new(file),
                cx,
                &mut read_buffer
            ))?;
            Poll::Ready(Ok(read_buffer.filled().len()))
        }
        #[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
        super::FileInner::Smol(file) => {
            futures::io::AsyncRead::poll_read(Pin::new(file), cx, buffer)
        }
        #[cfg(any(
            target_arch = "wasm32",
            all(
                not(target_arch = "wasm32"),
                any(feature = "compio", feature = "threadpool", feature = "lite")
            )
        ))]
        _ => poll_internal_read(inner, state, cx, buffer),
        #[cfg(all(
            not(target_arch = "wasm32"),
            not(any(
                feature = "tokio",
                feature = "smol",
                feature = "compio",
                feature = "threadpool",
                feature = "lite"
            ))
        ))]
        _ => unreachable!(),
    }
}

fn poll_write(
    inner: &mut super::FileInner,
    state: &mut FileIo,
    cx: &mut Context<'_>,
    buffer: &[u8],
) -> Poll<io::Result<usize>> {
    let _ = &state;
    let _ = cx;
    if buffer.is_empty() {
        return Poll::Ready(Ok(0));
    }

    #[cfg(any(
        target_arch = "wasm32",
        all(
            not(target_arch = "wasm32"),
            any(feature = "compio", feature = "threadpool", feature = "lite")
        )
    ))]
    {
        state.buffered_read.clear();
        state.buffered_read_position = 0;
    }

    match inner {
        #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
        super::FileInner::Tokio(file) => {
            ::tokio::io::AsyncWrite::poll_write(Pin::new(file), cx, buffer)
        }
        #[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
        super::FileInner::Smol(file) => {
            futures::io::AsyncWrite::poll_write(Pin::new(file), cx, buffer)
        }
        #[cfg(any(
            target_arch = "wasm32",
            all(
                not(target_arch = "wasm32"),
                any(feature = "compio", feature = "threadpool", feature = "lite")
            )
        ))]
        _ => poll_internal_write(inner, state, cx, buffer),
        #[cfg(all(
            not(target_arch = "wasm32"),
            not(any(
                feature = "tokio",
                feature = "smol",
                feature = "compio",
                feature = "threadpool",
                feature = "lite"
            ))
        ))]
        _ => unreachable!(),
    }
}

fn poll_flush(
    inner: &mut super::FileInner,
    state: &mut FileIo,
    cx: &mut Context<'_>,
) -> Poll<io::Result<()>> {
    let _ = state;
    let _ = cx;
    match inner {
        #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
        super::FileInner::Tokio(file) => ::tokio::io::AsyncWrite::poll_flush(Pin::new(file), cx),
        #[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
        super::FileInner::Smol(file) => futures::io::AsyncWrite::poll_flush(Pin::new(file), cx),
        #[cfg(any(
            target_arch = "wasm32",
            all(
                not(target_arch = "wasm32"),
                any(feature = "compio", feature = "threadpool", feature = "lite")
            )
        ))]
        _ => poll_internal_flush(inner, state, cx),
        #[cfg(all(
            not(target_arch = "wasm32"),
            not(any(
                feature = "tokio",
                feature = "smol",
                feature = "compio",
                feature = "threadpool",
                feature = "lite"
            ))
        ))]
        _ => unreachable!(),
    }
}

fn poll_seek(
    inner: &mut super::FileInner,
    state: &mut FileIo,
    cx: &mut Context<'_>,
    position: SeekFrom,
) -> Poll<io::Result<u64>> {
    let _ = &state;
    let _ = cx;
    let _ = position;
    #[cfg(any(
        target_arch = "wasm32",
        all(
            not(target_arch = "wasm32"),
            any(feature = "compio", feature = "threadpool", feature = "lite")
        )
    ))]
    {
        state.buffered_read.clear();
        state.buffered_read_position = 0;
    }

    match inner {
        #[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
        super::FileInner::Tokio(file) => loop {
            if let Some(pending) = state.futures_seek {
                let result = ready!(::tokio::io::AsyncSeek::poll_complete(
                    Pin::new(&mut *file),
                    cx
                ));
                state.futures_seek = None;
                if pending == position {
                    return Poll::Ready(result);
                }
                result?;
            }

            ::tokio::io::AsyncSeek::start_seek(Pin::new(&mut *file), position)?;
            state.futures_seek = Some(position);
        },
        #[cfg(all(feature = "smol", not(target_arch = "wasm32")))]
        super::FileInner::Smol(file) => {
            futures::io::AsyncSeek::poll_seek(Pin::new(file), cx, position)
        }
        #[cfg(any(
            target_arch = "wasm32",
            all(
                not(target_arch = "wasm32"),
                any(feature = "compio", feature = "threadpool", feature = "lite")
            )
        ))]
        _ => poll_internal_seek(inner, state, cx, position),
        #[cfg(all(
            not(target_arch = "wasm32"),
            not(any(
                feature = "tokio",
                feature = "smol",
                feature = "compio",
                feature = "threadpool",
                feature = "lite"
            ))
        ))]
        _ => unreachable!(),
    }
}

#[cfg(any(
    target_arch = "wasm32",
    all(
        not(target_arch = "wasm32"),
        any(feature = "compio", feature = "threadpool", feature = "lite")
    )
))]
fn poll_internal_read(
    inner: &mut super::FileInner,
    state: &mut FileIo,
    cx: &mut Context<'_>,
    buffer: &mut [u8],
) -> Poll<io::Result<usize>> {
    loop {
        if state.operation.is_none() {
            state.operation = Some(read_operation(inner, buffer.len()));
        }

        match ready!(poll_operation(state, cx)) {
            IoOutput::Read {
                result,
                buffer: mut owned,
                position,
            } => {
                apply_position(inner, position);
                let read = result?;
                if read > owned.len() {
                    return Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "the filesystem returned an invalid read length",
                    )));
                }
                owned.truncate(read);
                let copied = buffer.len().min(read);
                buffer[..copied].copy_from_slice(&owned[..copied]);
                if copied < read {
                    state.buffered_read = owned;
                    state.buffered_read_position = copied;
                }
                return Poll::Ready(Ok(copied));
            }
            output => finish_discarded(inner, output)?,
        }
    }
}

#[cfg(any(
    target_arch = "wasm32",
    all(
        not(target_arch = "wasm32"),
        any(feature = "compio", feature = "threadpool", feature = "lite")
    )
))]
fn poll_internal_write(
    inner: &mut super::FileInner,
    state: &mut FileIo,
    cx: &mut Context<'_>,
    buffer: &[u8],
) -> Poll<io::Result<usize>> {
    loop {
        if state.operation.is_none() {
            state.operation = Some(write_operation(inner, buffer.to_owned()));
        }

        match ready!(poll_operation(state, cx)) {
            IoOutput::Write {
                result,
                buffer: written,
                position,
            } => {
                apply_position(inner, position);
                if written == buffer {
                    return Poll::Ready(result);
                }
                result?;
            }
            output => finish_discarded(inner, output)?,
        }
    }
}

#[cfg(any(
    target_arch = "wasm32",
    all(
        not(target_arch = "wasm32"),
        any(feature = "compio", feature = "threadpool", feature = "lite")
    )
))]
fn poll_internal_flush(
    inner: &mut super::FileInner,
    state: &mut FileIo,
    cx: &mut Context<'_>,
) -> Poll<io::Result<()>> {
    loop {
        if state.operation.is_none() {
            state.operation = Some(flush_operation(inner));
        }

        match ready!(poll_operation(state, cx)) {
            IoOutput::Flush(result) => return Poll::Ready(result),
            output => finish_discarded(inner, output)?,
        }
    }
}

#[cfg(any(
    target_arch = "wasm32",
    all(
        not(target_arch = "wasm32"),
        any(feature = "compio", feature = "threadpool", feature = "lite")
    )
))]
fn poll_internal_seek(
    inner: &mut super::FileInner,
    state: &mut FileIo,
    cx: &mut Context<'_>,
    position: SeekFrom,
) -> Poll<io::Result<u64>> {
    loop {
        if state.operation.is_none() {
            state.operation = Some(seek_operation(inner, position));
        }

        match ready!(poll_operation(state, cx)) {
            IoOutput::Seek { requested, result } => {
                if let Ok(completed_position) = result {
                    apply_position(inner, Some(completed_position));
                    if requested == position {
                        return Poll::Ready(Ok(completed_position));
                    }
                } else {
                    return Poll::Ready(result);
                }
            }
            output => finish_discarded(inner, output)?,
        }
    }
}

#[cfg(any(
    target_arch = "wasm32",
    all(
        not(target_arch = "wasm32"),
        any(feature = "compio", feature = "threadpool", feature = "lite")
    )
))]
fn poll_operation(state: &mut FileIo, cx: &mut Context<'_>) -> Poll<IoOutput> {
    let output = ready!(state.operation.as_mut().unwrap().as_mut().poll(cx));
    state.operation = None;
    Poll::Ready(output)
}

#[cfg(any(
    target_arch = "wasm32",
    all(
        not(target_arch = "wasm32"),
        any(feature = "compio", feature = "threadpool", feature = "lite")
    )
))]
fn finish_discarded(inner: &mut super::FileInner, output: IoOutput) -> io::Result<()> {
    match output {
        IoOutput::Read {
            result, position, ..
        } => {
            apply_position(inner, position);
            result.map(|_| ())
        }
        IoOutput::Write {
            result, position, ..
        } => {
            apply_position(inner, position);
            result.map(|_| ())
        }
        IoOutput::Flush(result) => result,
        IoOutput::Seek { result, .. } => {
            if let Ok(position) = result {
                apply_position(inner, Some(position));
                Ok(())
            } else {
                result.map(|_| ())
            }
        }
    }
}

#[cfg(any(
    target_arch = "wasm32",
    all(
        not(target_arch = "wasm32"),
        any(feature = "compio", feature = "threadpool", feature = "lite")
    )
))]
fn read_operation(inner: &super::FileInner, capacity: usize) -> IoFuture {
    match inner {
        #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
        super::FileInner::Compio(file) => {
            let mut file = super::CompioFile {
                inner: file.inner.clone(),
                position: file.position,
            };
            Box::pin(async move {
                let mut buffer = vec![0; capacity];
                let result = FileRead::read(&mut file, &mut buffer).await;
                let position = result.as_ref().ok().map(|_| file.position);
                IoOutput::Read {
                    result,
                    buffer,
                    position,
                }
            })
        }
        #[cfg(all(
            any(feature = "threadpool", feature = "lite"),
            not(target_arch = "wasm32")
        ))]
        super::FileInner::Blocking(file) => {
            use crate::ExecutorBlocking;

            let executor = file.executor;
            let file = file.inner.clone();
            let task = executor.spawn_blocking(move || {
                let mut buffer = vec![0; capacity];
                let mut file = &*file;
                let result = std::io::Read::read(&mut file, &mut buffer);
                (result, buffer)
            });
            Box::pin(async move {
                match task.await {
                    Ok((result, buffer)) => IoOutput::Read {
                        result,
                        buffer,
                        position: None,
                    },
                    Err(error) => IoOutput::Read {
                        result: Err(io::Error::other(error)),
                        buffer: Vec::new(),
                        position: None,
                    },
                }
            })
        }
        #[cfg(target_arch = "wasm32")]
        super::FileInner::Wasm(file) => {
            let mut file = super::WasmFile {
                handle: file.handle.clone(),
                readable: file.readable,
                writable: file.writable,
                append: file.append,
                position: file.position,
            };
            Box::pin(async move {
                let mut buffer = vec![0; capacity];
                let result = FileRead::read(&mut file, &mut buffer).await;
                let position = result.as_ref().ok().map(|_| file.position);
                IoOutput::Read {
                    result,
                    buffer,
                    position,
                }
            })
        }
        #[cfg(any(
            all(feature = "tokio", not(target_arch = "wasm32")),
            all(feature = "smol", not(target_arch = "wasm32"))
        ))]
        _ => unreachable!(),
    }
}

#[cfg(any(
    target_arch = "wasm32",
    all(
        not(target_arch = "wasm32"),
        any(feature = "compio", feature = "threadpool", feature = "lite")
    )
))]
fn write_operation(inner: &super::FileInner, buffer: Vec<u8>) -> IoFuture {
    match inner {
        #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
        super::FileInner::Compio(file) => {
            let mut file = super::CompioFile {
                inner: file.inner.clone(),
                position: file.position,
            };
            Box::pin(async move {
                let result = FileWrite::write(&mut file, &buffer).await;
                let position = result.as_ref().ok().map(|_| file.position);
                IoOutput::Write {
                    result,
                    buffer,
                    position,
                }
            })
        }
        #[cfg(all(
            any(feature = "threadpool", feature = "lite"),
            not(target_arch = "wasm32")
        ))]
        super::FileInner::Blocking(file) => {
            use crate::ExecutorBlocking;

            let executor = file.executor;
            let file = file.inner.clone();
            let task = executor.spawn_blocking(move || {
                let mut file = &*file;
                let result = std::io::Write::write(&mut file, &buffer);
                (result, buffer)
            });
            Box::pin(async move {
                match task.await {
                    Ok((result, buffer)) => IoOutput::Write {
                        result,
                        buffer,
                        position: None,
                    },
                    Err(error) => IoOutput::Write {
                        result: Err(io::Error::other(error)),
                        buffer: Vec::new(),
                        position: None,
                    },
                }
            })
        }
        #[cfg(target_arch = "wasm32")]
        super::FileInner::Wasm(file) => {
            let mut file = super::WasmFile {
                handle: file.handle.clone(),
                readable: file.readable,
                writable: file.writable,
                append: file.append,
                position: file.position,
            };
            Box::pin(async move {
                let result = FileWrite::write(&mut file, &buffer).await;
                let position = result.as_ref().ok().map(|_| file.position);
                IoOutput::Write {
                    result,
                    buffer,
                    position,
                }
            })
        }
        #[cfg(any(
            all(feature = "tokio", not(target_arch = "wasm32")),
            all(feature = "smol", not(target_arch = "wasm32"))
        ))]
        _ => unreachable!(),
    }
}

#[cfg(any(
    target_arch = "wasm32",
    all(
        not(target_arch = "wasm32"),
        any(feature = "compio", feature = "threadpool", feature = "lite")
    )
))]
fn flush_operation(inner: &super::FileInner) -> IoFuture {
    match inner {
        #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
        super::FileInner::Compio(_) => Box::pin(async { IoOutput::Flush(Ok(())) }),
        #[cfg(all(
            any(feature = "threadpool", feature = "lite"),
            not(target_arch = "wasm32")
        ))]
        super::FileInner::Blocking(file) => {
            use crate::ExecutorBlocking;

            let executor = file.executor;
            let file = file.inner.clone();
            let task = executor.spawn_blocking(move || {
                let mut file = &*file;
                std::io::Write::flush(&mut file)
            });
            Box::pin(async move {
                IoOutput::Flush(
                    task.await
                        .unwrap_or_else(|error| Err(io::Error::other(error))),
                )
            })
        }
        #[cfg(target_arch = "wasm32")]
        super::FileInner::Wasm(_) => Box::pin(async { IoOutput::Flush(Ok(())) }),
        #[cfg(any(
            all(feature = "tokio", not(target_arch = "wasm32")),
            all(feature = "smol", not(target_arch = "wasm32"))
        ))]
        _ => unreachable!(),
    }
}

#[cfg(any(
    target_arch = "wasm32",
    all(
        not(target_arch = "wasm32"),
        any(feature = "compio", feature = "threadpool", feature = "lite")
    )
))]
fn seek_operation(inner: &super::FileInner, position: SeekFrom) -> IoFuture {
    match inner {
        #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
        super::FileInner::Compio(file) => {
            let mut file = super::CompioFile {
                inner: file.inner.clone(),
                position: file.position,
            };
            Box::pin(async move {
                IoOutput::Seek {
                    requested: position,
                    result: FileSeek::seek(&mut file, position).await,
                }
            })
        }
        #[cfg(all(
            any(feature = "threadpool", feature = "lite"),
            not(target_arch = "wasm32")
        ))]
        super::FileInner::Blocking(file) => {
            use crate::ExecutorBlocking;

            let executor = file.executor;
            let file = file.inner.clone();
            let task = executor.spawn_blocking(move || {
                let mut file = &*file;
                std::io::Seek::seek(&mut file, position)
            });
            Box::pin(async move {
                IoOutput::Seek {
                    requested: position,
                    result: task
                        .await
                        .unwrap_or_else(|error| Err(io::Error::other(error))),
                }
            })
        }
        #[cfg(target_arch = "wasm32")]
        super::FileInner::Wasm(file) => {
            let mut file = super::WasmFile {
                handle: file.handle.clone(),
                readable: file.readable,
                writable: file.writable,
                append: file.append,
                position: file.position,
            };
            Box::pin(async move {
                IoOutput::Seek {
                    requested: position,
                    result: FileSeek::seek(&mut file, position).await,
                }
            })
        }
        #[cfg(any(
            all(feature = "tokio", not(target_arch = "wasm32")),
            all(feature = "smol", not(target_arch = "wasm32"))
        ))]
        _ => unreachable!(),
    }
}

#[cfg(any(
    target_arch = "wasm32",
    all(
        not(target_arch = "wasm32"),
        any(feature = "compio", feature = "threadpool", feature = "lite")
    )
))]
fn apply_position(inner: &mut super::FileInner, position: Option<u64>) {
    let Some(position) = position else {
        return;
    };
    let _ = position;
    #[allow(unreachable_patterns)]
    match inner {
        #[cfg(all(feature = "compio", not(target_arch = "wasm32")))]
        super::FileInner::Compio(file) => file.position = position,
        #[cfg(target_arch = "wasm32")]
        super::FileInner::Wasm(file) => file.position = position,
        _ => {}
    }
}
