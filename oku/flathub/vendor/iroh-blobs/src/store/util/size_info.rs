use std::io;

use bao_tree::io::sync::WriteAt;

use crate::store::IROH_BLOCK_SIZE;

/// Keep track of the most precise size we know of.
///
/// When in memory, we don't have to write the size for every chunk to a separate
/// slot, but can just keep the best one.
#[derive(Debug, Default)]
pub struct SizeInfo {
    pub offset: u64,
    pub size: u64,
}

#[allow(dead_code)]
impl SizeInfo {
    /// Create a new size info for a complete file of size `size`.
    pub(crate) fn complete(size: u64) -> Self {
        let mask = (1 << IROH_BLOCK_SIZE.chunk_log()) - 1;
        // offset of the last bao chunk in a file of size `size`
        let last_chunk_offset = size & mask;
        Self {
            offset: last_chunk_offset,
            size,
        }
    }

    /// Write a size at the given offset. The size at the highest offset is going to be kept.
    pub fn write(&mut self, offset: u64, size: u64) {
        // >= instead of > because we want to be able to update size 0, the initial value.
        if offset >= self.offset {
            self.offset = offset;
            self.size = size;
        }
    }

    /// The current size, representing the most correct size we know.
    pub fn current_size(&self) -> u64 {
        self.size
    }

    /// Persist into a file where each chunk has its own slot.
    pub fn persist(&self, mut target: impl WriteAt) -> io::Result<()> {
        let size_offset = (self.offset >> IROH_BLOCK_SIZE.chunk_log()) << 3;
        target.write_all_at(size_offset, self.size.to_le_bytes().as_slice())?;
        Ok(())
    }

    /// Convert to a vec in slot format.
    #[allow(dead_code)]
    pub fn to_vec(&self) -> Vec<u8> {
        let mut res = Vec::new();
        self.persist(&mut res).expect("io error writing to vec");
        res
    }
}
