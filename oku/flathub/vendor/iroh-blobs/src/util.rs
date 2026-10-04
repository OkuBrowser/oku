//! Utilities
pub(crate) mod channel;
mod stream;
pub(crate) mod temp_tag;
pub use iroh_util::connection_pool;
pub use stream::{
    AsyncReadRecvStream, AsyncReadRecvStreamExtra, AsyncWriteSendStream, AsyncWriteSendStreamExtra,
    RecvStream, RecvStreamAsyncStreamReader, SendStream,
};
pub(crate) use stream::{RecvStreamExt, SendStreamExt};

pub(crate) mod serde {
    // Module that handles io::Error serialization/deserialization
    pub mod io_error_serde {
        use std::{fmt, io};

        use serde::{
            de::{self, SeqAccess, Visitor},
            ser::SerializeTuple,
            Deserializer, Serializer,
        };

        fn error_kind_to_u8(kind: io::ErrorKind) -> u8 {
            match kind {
                io::ErrorKind::AddrInUse => 0,
                io::ErrorKind::AddrNotAvailable => 1,
                io::ErrorKind::AlreadyExists => 2,
                io::ErrorKind::ArgumentListTooLong => 3,
                io::ErrorKind::BrokenPipe => 4,
                io::ErrorKind::ConnectionAborted => 5,
                io::ErrorKind::ConnectionRefused => 6,
                io::ErrorKind::ConnectionReset => 7,
                io::ErrorKind::CrossesDevices => 8,
                io::ErrorKind::Deadlock => 9,
                io::ErrorKind::DirectoryNotEmpty => 10,
                io::ErrorKind::ExecutableFileBusy => 11,
                io::ErrorKind::FileTooLarge => 12,
                io::ErrorKind::HostUnreachable => 13,
                io::ErrorKind::Interrupted => 14,
                io::ErrorKind::InvalidData => 15,
                io::ErrorKind::InvalidInput => 17,
                io::ErrorKind::IsADirectory => 18,
                io::ErrorKind::NetworkDown => 19,
                io::ErrorKind::NetworkUnreachable => 20,
                io::ErrorKind::NotADirectory => 21,
                io::ErrorKind::NotConnected => 22,
                io::ErrorKind::NotFound => 23,
                io::ErrorKind::NotSeekable => 24,
                io::ErrorKind::Other => 25,
                io::ErrorKind::OutOfMemory => 26,
                io::ErrorKind::PermissionDenied => 27,
                io::ErrorKind::QuotaExceeded => 28,
                io::ErrorKind::ReadOnlyFilesystem => 29,
                io::ErrorKind::ResourceBusy => 30,
                io::ErrorKind::StaleNetworkFileHandle => 31,
                io::ErrorKind::StorageFull => 32,
                io::ErrorKind::TimedOut => 33,
                io::ErrorKind::TooManyLinks => 34,
                io::ErrorKind::UnexpectedEof => 35,
                io::ErrorKind::Unsupported => 36,
                io::ErrorKind::WouldBlock => 37,
                io::ErrorKind::WriteZero => 38,
                _ => 25,
            }
        }

        fn u8_to_error_kind(num: u8) -> io::ErrorKind {
            match num {
                0 => io::ErrorKind::AddrInUse,
                1 => io::ErrorKind::AddrNotAvailable,
                2 => io::ErrorKind::AlreadyExists,
                3 => io::ErrorKind::ArgumentListTooLong,
                4 => io::ErrorKind::BrokenPipe,
                5 => io::ErrorKind::ConnectionAborted,
                6 => io::ErrorKind::ConnectionRefused,
                7 => io::ErrorKind::ConnectionReset,
                8 => io::ErrorKind::CrossesDevices,
                9 => io::ErrorKind::Deadlock,
                10 => io::ErrorKind::DirectoryNotEmpty,
                11 => io::ErrorKind::ExecutableFileBusy,
                12 => io::ErrorKind::FileTooLarge,
                13 => io::ErrorKind::HostUnreachable,
                14 => io::ErrorKind::Interrupted,
                15 => io::ErrorKind::InvalidData,
                // 16 => io::ErrorKind::InvalidFilename,
                17 => io::ErrorKind::InvalidInput,
                18 => io::ErrorKind::IsADirectory,
                19 => io::ErrorKind::NetworkDown,
                20 => io::ErrorKind::NetworkUnreachable,
                21 => io::ErrorKind::NotADirectory,
                22 => io::ErrorKind::NotConnected,
                23 => io::ErrorKind::NotFound,
                24 => io::ErrorKind::NotSeekable,
                25 => io::ErrorKind::Other,
                26 => io::ErrorKind::OutOfMemory,
                27 => io::ErrorKind::PermissionDenied,
                28 => io::ErrorKind::QuotaExceeded,
                29 => io::ErrorKind::ReadOnlyFilesystem,
                30 => io::ErrorKind::ResourceBusy,
                31 => io::ErrorKind::StaleNetworkFileHandle,
                32 => io::ErrorKind::StorageFull,
                33 => io::ErrorKind::TimedOut,
                34 => io::ErrorKind::TooManyLinks,
                35 => io::ErrorKind::UnexpectedEof,
                36 => io::ErrorKind::Unsupported,
                37 => io::ErrorKind::WouldBlock,
                38 => io::ErrorKind::WriteZero,
                _ => io::ErrorKind::Other,
            }
        }

        pub fn serialize<S>(error: &io::Error, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let mut tup = serializer.serialize_tuple(2)?;
            tup.serialize_element(&error_kind_to_u8(error.kind()))?;
            tup.serialize_element(&error.to_string())?;
            tup.end()
        }

        pub fn deserialize<'de, D>(deserializer: D) -> Result<io::Error, D::Error>
        where
            D: Deserializer<'de>,
        {
            struct IoErrorVisitor;

            impl<'de> Visitor<'de> for IoErrorVisitor {
                type Value = io::Error;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("a tuple of (u32, String) representing io::Error")
                }

                fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
                where
                    A: SeqAccess<'de>,
                {
                    let num: u8 = seq
                        .next_element()?
                        .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                    let message: String = seq
                        .next_element()?
                        .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                    let kind = u8_to_error_kind(num);
                    Ok(io::Error::new(kind, message))
                }
            }

            deserializer.deserialize_tuple(2, IoErrorVisitor)
        }
    }

    #[cfg(test)]
    mod tests {
        use std::io::{self, ErrorKind};

        use postcard;
        use serde::{Deserialize, Serialize};

        use super::io_error_serde;

        #[derive(Serialize, Deserialize)]
        struct TestError(#[serde(with = "io_error_serde")] io::Error);

        #[test]
        fn test_roundtrip_error_kinds() {
            let message = "test error";
            let kinds = [
                ErrorKind::AddrInUse,
                ErrorKind::AddrNotAvailable,
                ErrorKind::AlreadyExists,
                ErrorKind::ArgumentListTooLong,
                ErrorKind::BrokenPipe,
                ErrorKind::ConnectionAborted,
                ErrorKind::ConnectionRefused,
                ErrorKind::ConnectionReset,
                ErrorKind::CrossesDevices,
                ErrorKind::Deadlock,
                ErrorKind::DirectoryNotEmpty,
                ErrorKind::ExecutableFileBusy,
                ErrorKind::FileTooLarge,
                ErrorKind::HostUnreachable,
                ErrorKind::Interrupted,
                ErrorKind::InvalidData,
                // ErrorKind::InvalidFilename,
                ErrorKind::InvalidInput,
                ErrorKind::IsADirectory,
                ErrorKind::NetworkDown,
                ErrorKind::NetworkUnreachable,
                ErrorKind::NotADirectory,
                ErrorKind::NotConnected,
                ErrorKind::NotFound,
                ErrorKind::NotSeekable,
                ErrorKind::Other,
                ErrorKind::OutOfMemory,
                ErrorKind::PermissionDenied,
                ErrorKind::QuotaExceeded,
                ErrorKind::ReadOnlyFilesystem,
                ErrorKind::ResourceBusy,
                ErrorKind::StaleNetworkFileHandle,
                ErrorKind::StorageFull,
                ErrorKind::TimedOut,
                ErrorKind::TooManyLinks,
                ErrorKind::UnexpectedEof,
                ErrorKind::Unsupported,
                ErrorKind::WouldBlock,
                ErrorKind::WriteZero,
            ];

            for kind in kinds {
                let err = TestError(io::Error::new(kind, message));
                let serialized = postcard::to_allocvec(&err).unwrap();
                let deserialized: TestError = postcard::from_bytes(&serialized).unwrap();

                assert_eq!(err.0.kind(), deserialized.0.kind());
                assert_eq!(err.0.to_string(), deserialized.0.to_string());
            }
        }
    }
}

#[cfg(feature = "fs-store")]
pub(crate) mod outboard_with_progress {
    use std::io::{self, BufReader, Read};

    use bao_tree::{
        blake3,
        io::{
            outboard::PreOrderOutboard,
            sync::{OutboardMut, WriteAt},
        },
        iter::BaoChunk,
        BaoTree, ChunkNum,
    };
    use smallvec::SmallVec;

    use super::sink::Sink;

    fn hash_subtree(start_chunk: u64, data: &[u8], is_root: bool) -> blake3::Hash {
        use blake3::hazmat::{ChainingValue, HasherExt};
        if is_root {
            debug_assert!(start_chunk == 0);
            blake3::hash(data)
        } else {
            let mut hasher = blake3::Hasher::new();
            hasher.set_input_offset(start_chunk * 1024);
            hasher.update(data);
            let non_root_hash: ChainingValue = hasher.finalize_non_root();
            blake3::Hash::from(non_root_hash)
        }
    }

    fn parent_cv(
        left_child: &blake3::Hash,
        right_child: &blake3::Hash,
        is_root: bool,
    ) -> blake3::Hash {
        use blake3::hazmat::{merge_subtrees_non_root, merge_subtrees_root, ChainingValue, Mode};
        let left_child: ChainingValue = *left_child.as_bytes();
        let right_child: ChainingValue = *right_child.as_bytes();
        if is_root {
            merge_subtrees_root(&left_child, &right_child, Mode::Hash)
        } else {
            blake3::Hash::from(merge_subtrees_non_root(
                &left_child,
                &right_child,
                Mode::Hash,
            ))
        }
    }

    pub async fn init_outboard<R, W, P>(
        data: R,
        outboard: &mut PreOrderOutboard<W>,
        progress: &mut P,
    ) -> std::io::Result<std::result::Result<(), P::Error>>
    where
        W: WriteAt,
        R: Read,
        P: Sink<ChunkNum>,
    {
        // wrap the reader in a buffered reader, so we read in large chunks
        // this reduces the number of io ops
        let size = usize::try_from(outboard.tree.size()).unwrap_or(usize::MAX);
        let read_buf_size = size.min(1024 * 1024);
        let chunk_buf_size = size.min(outboard.tree.block_size().bytes());
        let reader = BufReader::with_capacity(read_buf_size, data);
        let mut buffer = SmallVec::<[u8; 128]>::from_elem(0u8, chunk_buf_size);
        let res = init_impl(outboard.tree, reader, outboard, &mut buffer, progress).await?;
        Ok(res)
    }

    async fn init_impl<W, P>(
        tree: BaoTree,
        mut data: impl Read,
        outboard: &mut PreOrderOutboard<W>,
        buffer: &mut [u8],
        progress: &mut P,
    ) -> io::Result<std::result::Result<(), P::Error>>
    where
        W: WriteAt,
        P: Sink<ChunkNum>,
    {
        // do not allocate for small trees
        let mut stack = SmallVec::<[blake3::Hash; 10]>::new();
        // debug_assert!(buffer.len() == tree.chunk_group_bytes());
        for item in tree.post_order_chunks_iter() {
            match item {
                BaoChunk::Parent { is_root, node, .. } => {
                    let right_hash = stack.pop().unwrap();
                    let left_hash = stack.pop().unwrap();
                    outboard.save(node, &(left_hash, right_hash))?;
                    let parent = parent_cv(&left_hash, &right_hash, is_root);
                    stack.push(parent);
                }
                BaoChunk::Leaf {
                    size,
                    is_root,
                    start_chunk,
                    ..
                } => {
                    if let Err(err) = progress.send(start_chunk).await {
                        return Ok(Err(err));
                    }
                    let buf = &mut buffer[..size];
                    data.read_exact(buf)?;
                    let hash = hash_subtree(start_chunk.0, buf, is_root);
                    stack.push(hash);
                }
            }
        }
        debug_assert_eq!(stack.len(), 1);
        outboard.root = stack.pop().unwrap();
        Ok(Ok(()))
    }

    #[cfg(test)]
    mod tests {
        use bao_tree::{
            blake3,
            io::{outboard::PreOrderOutboard, sync::CreateOutboard},
            BaoTree,
        };
        use testresult::TestResult;

        use crate::{
            store::{fs::tests::test_data, IROH_BLOCK_SIZE},
            util::{outboard_with_progress::init_outboard, sink::Drain},
        };

        #[tokio::test]
        async fn init_outboard_with_progress() -> TestResult<()> {
            for size in [1024 * 18 + 1] {
                let data = test_data(size);
                let mut o1 = PreOrderOutboard::<Vec<u8>> {
                    tree: BaoTree::new(data.len() as u64, IROH_BLOCK_SIZE),
                    ..Default::default()
                };
                let mut o2 = o1.clone();
                o1.init_from(data.as_ref())?;
                init_outboard(data.as_ref(), &mut o2, &mut Drain).await??;
                assert_eq!(o1.root, blake3::hash(&data));
                assert_eq!(o1.root, o2.root);
                assert_eq!(o1.data, o2.data);
            }
            Ok(())
        }
    }
}

pub(crate) mod sink {
    use std::future::Future;

    use irpc::RpcMessage;

    /// Our version of a sink, that can be mapped etc.
    pub trait Sink<Item> {
        type Error;
        fn send(
            &mut self,
            value: Item,
        ) -> impl Future<Output = std::result::Result<(), Self::Error>>;

        fn with_map_err<F, U>(self, f: F) -> WithMapErr<Self, F>
        where
            Self: Sized,
            F: Fn(Self::Error) -> U + Send + 'static,
        {
            WithMapErr { inner: self, f }
        }

        fn with_map<F, U>(self, f: F) -> WithMap<Self, F>
        where
            Self: Sized,
            F: Fn(U) -> Item + Send + 'static,
        {
            WithMap { inner: self, f }
        }
    }

    impl<I, T> Sink<T> for &mut I
    where
        I: Sink<T>,
    {
        type Error = I::Error;

        async fn send(&mut self, value: T) -> std::result::Result<(), Self::Error> {
            (*self).send(value).await
        }
    }

    #[allow(dead_code)]
    pub struct IrpcSenderSink<T>(pub irpc::channel::mpsc::Sender<T>);

    impl<T> Sink<T> for IrpcSenderSink<T>
    where
        T: RpcMessage,
    {
        type Error = irpc::channel::SendError;

        async fn send(&mut self, value: T) -> std::result::Result<(), Self::Error> {
            self.0.send(value).await
        }
    }

    pub struct IrpcSenderRefSink<'a, T>(pub &'a mut irpc::channel::mpsc::Sender<T>);

    impl<'a, T> Sink<T> for IrpcSenderRefSink<'a, T>
    where
        T: RpcMessage,
    {
        type Error = irpc::channel::SendError;

        async fn send(&mut self, value: T) -> std::result::Result<(), Self::Error> {
            self.0.send(value).await
        }
    }

    pub struct TokioMpscSenderSink<T>(pub tokio::sync::mpsc::Sender<T>);

    impl<T> Sink<T> for TokioMpscSenderSink<T> {
        type Error = irpc::channel::SendError;

        async fn send(&mut self, value: T) -> std::result::Result<(), Self::Error> {
            self.0
                .send(value)
                .await
                .map_err(|_| n0_error::e!(irpc::channel::SendError::ReceiverClosed))
        }
    }

    pub struct WithMapErr<P, F> {
        inner: P,
        f: F,
    }

    impl<P, F, E, U> Sink<U> for WithMapErr<P, F>
    where
        P: Sink<U>,
        F: Fn(P::Error) -> E + Send + 'static,
    {
        type Error = E;

        async fn send(&mut self, value: U) -> std::result::Result<(), Self::Error> {
            match self.inner.send(value).await {
                Ok(()) => Ok(()),
                Err(err) => {
                    let err = (self.f)(err);
                    Err(err)
                }
            }
        }
    }

    pub struct WithMap<P, F> {
        inner: P,
        f: F,
    }

    impl<P, F, T, U> Sink<T> for WithMap<P, F>
    where
        P: Sink<U>,
        F: Fn(T) -> U + Send + 'static,
    {
        type Error = P::Error;

        async fn send(&mut self, value: T) -> std::result::Result<(), Self::Error> {
            self.inner.send((self.f)(value)).await
        }
    }

    pub struct Drain;

    impl<T> Sink<T> for Drain {
        type Error = irpc::channel::SendError;

        async fn send(&mut self, _offset: T) -> std::result::Result<(), Self::Error> {
            Ok(())
        }
    }
}
