//! Adaptation for `ipfs-unixfs` crate functionality on top of [`Ipfs`].
//!
//! Adding files and directory structures is supported but not exposed via an API. See examples and
//! `ipfs-http`.

#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

use anyhow::Error;
use bytes::Bytes;
use futures::{
    StreamExt,
    stream::{self, BoxStream},
};
use ipld_core::cid::Cid;
use ll::file::FileReadFailed;
pub use rust_unixfs as ll;

mod add;
mod cat;
mod get;
mod ls;
mod prefetch;
pub use add::UnixfsAdd;
pub use cat::{StartingPoint, UnixfsCat};
pub use get::UnixfsGet;
pub use ls::{Entry, UnixfsLs};

use crate::{
    Ipfs, IpfsPath,
    dag::{ResolveError, UnexpectedResolved},
};

pub struct IpfsUnixfs {
    ipfs: Ipfs,
}

pub enum AddOpt {
    #[cfg(not(target_arch = "wasm32"))]
    Path(PathBuf),
    Stream(BoxStream<'static, std::io::Result<Bytes>>),
    StreamWithName(String, BoxStream<'static, std::io::Result<Bytes>>),
}

#[cfg(not(target_arch = "wasm32"))]
impl From<&str> for AddOpt {
    fn from(value: &str) -> Self {
        AddOpt::Path(PathBuf::from(value))
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<String> for AddOpt {
    fn from(value: String) -> Self {
        AddOpt::Path(PathBuf::from(value))
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<&std::path::Path> for AddOpt {
    fn from(path: &std::path::Path) -> Self {
        AddOpt::Path(path.to_path_buf())
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<PathBuf> for AddOpt {
    fn from(path: PathBuf) -> Self {
        AddOpt::Path(path)
    }
}

impl From<Vec<u8>> for AddOpt {
    fn from(bytes: Vec<u8>) -> Self {
        let bytes: Bytes = bytes.into();
        Self::from(bytes)
    }
}

impl From<&'static [u8]> for AddOpt {
    fn from(bytes: &'static [u8]) -> Self {
        let bytes: Bytes = bytes.into();
        Self::from(bytes)
    }
}

impl From<(String, Vec<u8>)> for AddOpt {
    fn from((name, bytes): (String, Vec<u8>)) -> Self {
        let bytes: Bytes = bytes.into();
        Self::from((name, bytes))
    }
}

impl From<(String, &'static [u8])> for AddOpt {
    fn from((name, bytes): (String, &'static [u8])) -> Self {
        let bytes: Bytes = bytes.into();
        Self::from((name, bytes))
    }
}

impl From<Bytes> for AddOpt {
    fn from(bytes: Bytes) -> Self {
        let stream = stream::once(async { Ok::<_, std::io::Error>(bytes) }).boxed();
        AddOpt::Stream(stream)
    }
}

impl From<(String, Bytes)> for AddOpt {
    fn from((name, bytes): (String, Bytes)) -> Self {
        let stream = stream::once(async { Ok::<_, std::io::Error>(bytes) }).boxed();
        Self::from((name, stream))
    }
}

impl From<BoxStream<'static, std::io::Result<Bytes>>> for AddOpt {
    fn from(stream: BoxStream<'static, std::io::Result<Bytes>>) -> Self {
        AddOpt::Stream(stream)
    }
}

impl From<(String, BoxStream<'static, std::io::Result<Bytes>>)> for AddOpt {
    fn from((name, stream): (String, BoxStream<'static, std::io::Result<Bytes>>)) -> Self {
        AddOpt::StreamWithName(name, stream)
    }
}

impl From<BoxStream<'static, std::io::Result<Vec<u8>>>> for AddOpt {
    fn from(stream: BoxStream<'static, std::io::Result<Vec<u8>>>) -> Self {
        AddOpt::Stream(stream.map(|result| result.map(|data| data.into())).boxed())
    }
}

impl From<(String, BoxStream<'static, std::io::Result<Vec<u8>>>)> for AddOpt {
    fn from((name, stream): (String, BoxStream<'static, std::io::Result<Vec<u8>>>)) -> Self {
        let stream = stream.map(|result| result.map(|data| data.into())).boxed();
        AddOpt::StreamWithName(name, stream)
    }
}

impl IpfsUnixfs {
    pub fn new(ipfs: Ipfs) -> Self {
        Self { ipfs }
    }

    /// Creates a stream which will yield the bytes of an UnixFS file from the root Cid, with the
    /// optional file byte range. If the range is specified and is outside of the file, the stream
    /// will end without producing any bytes.
    pub fn cat(&self, starting_point: impl Into<StartingPoint>) -> UnixfsCat {
        UnixfsCat::with_ipfs(&self.ipfs, starting_point)
    }

    /// Add a file from either a file or stream
    pub fn add(&self, item: impl Into<AddOpt>) -> UnixfsAdd {
        let item = item.into();
        match item {
            #[cfg(not(target_arch = "wasm32"))]
            AddOpt::Path(path) => UnixfsAdd::with_ipfs(&self.ipfs, path),
            AddOpt::Stream(stream) => UnixfsAdd::with_ipfs(
                &self.ipfs,
                add::AddOpt::Stream {
                    name: None,
                    total: None,
                    stream,
                },
            ),
            AddOpt::StreamWithName(name, stream) => UnixfsAdd::with_ipfs(
                &self.ipfs,
                add::AddOpt::Stream {
                    name: Some(name),
                    total: None,
                    stream,
                },
            ),
        }
    }

    /// Retreive a file and saving it to a local path.
    ///
    /// To create an owned version of the stream, please use `ipfs::unixfs::get` directly.
    pub fn get(&self, path: impl Into<IpfsPath>, dest: impl AsRef<std::path::Path>) -> UnixfsGet {
        UnixfsGet::with_ipfs(&self.ipfs, path, dest)
    }

    /// List directory contents
    pub fn ls(&self, path: impl Into<IpfsPath>) -> UnixfsLs {
        UnixfsLs::with_ipfs(&self.ipfs, path)
    }
}

#[derive(Debug)]
pub enum UnixfsStatus {
    ProgressStatus {
        written: usize,
        total_size: Option<usize>,
    },
    CompletedStatus {
        path: IpfsPath,
        written: usize,
        total_size: Option<usize>,
    },
    FailedStatus {
        written: usize,
        total_size: Option<usize>,
        error: Error,
    },
}

/// Types of failures which can occur while walking the UnixFS graph.
#[derive(Debug, thiserror::Error)]
pub enum TraversalFailed {
    /// Failure to resolve the given path; does not happen when given a block.
    #[error("path resolving failed")]
    Resolving(#[source] ResolveError),

    /// The given path was resolved to non dag-pb block, does not happen when starting the walk
    /// from a block.
    #[error("path resolved to unexpected")]
    Path(#[source] UnexpectedResolved),

    /// Loading of a block during walk failed
    #[error("loading of {} failed", .0)]
    Loading(Cid, #[source] Error),

    #[error("data exceeded max length")]
    MaxLengthExceeded { size: usize, length: usize },
    #[error("Timeout while resolving {path}")]
    Timeout { path: IpfsPath },

    /// Processing of the block failed
    #[error("walk failed on {}", .0)]
    Walking(Cid, #[source] FileReadFailed),

    #[error(transparent)]
    Io(std::io::Error),
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_file_cid() {
        // note: old versions of `ipfs::unixfs::File` was an interface where user would provide the
        // unixfs encoded data. this test case has been migrated to put the "content" as the the
        // file data instead of the unixfs encoding. the previous way used to produce
        // QmSy5pnHk1EnvE5dmJSyFKG5unXLGjPpBuJJCBQkBTvBaW.
        let content = "\u{8}\u{2}\u{12}\u{12}Here is some data\n\u{18}\u{12}";

        // legacy go-ipfs 0.6.0 vector is CIDv0 while the default is now CIDv1.
        let mut adder = rust_unixfs::file::adder::FileAdder::builder()
            .with_cid_version(ipld_core::cid::Version::V0)
            .build();
        let (mut blocks, consumed) = adder.push(content.as_bytes());
        assert_eq!(consumed, content.len(), "should had consumed all content");
        assert_eq!(
            blocks.next(),
            None,
            "should not had produced any blocks yet"
        );

        let mut blocks = adder.finish();

        let (cid, _block) = blocks.next().unwrap();
        assert_eq!(blocks.next(), None, "should had been the last");

        assert_eq!(
            "QmQZE72h2Vdm3F5gWr9RLuzSw3rUJEkKedWEa8t8XVygT5",
            cid.to_string(),
            "matches cid from go-ipfs 0.6.0"
        );
    }
}
