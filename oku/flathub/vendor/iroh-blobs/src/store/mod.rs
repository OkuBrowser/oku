//! Store implementations
//!
//! Use the [`mem`] store for sharing a small amount of mutable data,
//! the [`readonly_mem`] store for sharing static data, and the [`fs`] store
//! for when you want to efficiently share more than the available memory and
//! have access to a writeable filesystem.
use bao_tree::BlockSize;
#[cfg(feature = "fs-store")]
#[cfg_attr(iroh_blobs_docsrs, doc(cfg(feature = "fs-store")))]
pub mod fs;
mod gc;
pub mod mem;
pub mod readonly_mem;
mod test;
pub(crate) mod util;

/// Block size used by iroh, 2^4*1024 = 16KiB
pub const IROH_BLOCK_SIZE: BlockSize = BlockSize::from_chunk_log(4);

pub use gc::{GcConfig, ProtectCb, ProtectOutcome};
