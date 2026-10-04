use std::{
    fmt::Debug,
    fs::File,
    io::{self, Read, Seek},
};

use bao_tree::io::{
    mixed::ReadBytesAt,
    sync::{ReadAt, Size},
};
use bytes::Bytes;

use super::SliceInfoExt;

/// A wrapper for a file with a fixed size.
#[derive(Debug)]
pub struct FixedSize<T> {
    file: T,
    pub size: u64,
}

impl<T> FixedSize<T> {
    pub fn new(file: T, size: u64) -> Self {
        Self { file, size }
    }
}

impl FixedSize<File> {
    pub fn try_clone(&self) -> io::Result<Self> {
        Ok(Self::new(self.file.try_clone()?, self.size))
    }
}

impl<T: ReadAt> ReadAt for FixedSize<T> {
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> io::Result<usize> {
        self.file.read_at(offset, buf)
    }
}

impl<T: ReadBytesAt> ReadBytesAt for FixedSize<T> {
    fn read_bytes_at(&self, offset: u64, size: usize) -> io::Result<Bytes> {
        self.file.read_bytes_at(offset, size)
    }
}

impl<T> Size for FixedSize<T> {
    fn size(&self) -> io::Result<Option<u64>> {
        Ok(Some(self.size))
    }
}

/// This is a general purpose Either, just like Result, except that the two cases
/// are Mem for something that is in memory, and File for something that is somewhere
/// external and only available via io.
#[derive(Debug)]
pub enum MemOrFile<M, F> {
    /// We got it all in memory
    Mem(M),
    /// A file
    File(F),
}

impl<M: AsRef<[u8]>, F: Debug> MemOrFile<M, F> {
    pub fn fmt_short(&self) -> String {
        match self {
            Self::Mem(mem) => format!("Mem(size={},addr={})", mem.as_ref().len(), mem.addr_short()),
            Self::File(_) => "File".to_string(),
        }
    }
}

impl<T> MemOrFile<Bytes, FixedSize<T>> {
    pub fn size(&self) -> u64 {
        match self {
            MemOrFile::Mem(mem) => mem.len() as u64,
            MemOrFile::File(file) => file.size,
        }
    }
}

impl<A: Read, B: Read> Read for MemOrFile<A, B> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            MemOrFile::Mem(mem) => mem.read(buf),
            MemOrFile::File(file) => file.read(buf),
        }
    }
}

impl<A: Seek, B: Seek> Seek for MemOrFile<A, B> {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        match self {
            MemOrFile::Mem(mem) => mem.seek(pos),
            MemOrFile::File(file) => file.seek(pos),
        }
    }
}

impl<A: AsRef<[u8]>, B: ReadAt> ReadAt for MemOrFile<A, B> {
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            MemOrFile::Mem(mem) => mem.as_ref().read_at(offset, buf),
            MemOrFile::File(file) => file.read_at(offset, buf),
        }
    }
}

impl<A: ReadBytesAt, B: ReadBytesAt> ReadBytesAt for MemOrFile<A, B> {
    fn read_bytes_at(&self, offset: u64, size: usize) -> io::Result<Bytes> {
        match self {
            MemOrFile::Mem(mem) => mem.read_bytes_at(offset, size),
            MemOrFile::File(file) => file.read_bytes_at(offset, size),
        }
    }
}

impl<A: Size, B: Size> Size for MemOrFile<A, B> {
    fn size(&self) -> io::Result<Option<u64>> {
        match self {
            MemOrFile::Mem(mem) => mem.size(),
            MemOrFile::File(file) => file.size(),
        }
    }
}

impl<M: Default, F> Default for MemOrFile<M, F> {
    fn default() -> Self {
        MemOrFile::Mem(Default::default())
    }
}

impl<F> MemOrFile<Bytes, F> {
    /// Create an empty MemOrFile, using a Bytes for the Mem part
    pub fn empty() -> Self {
        MemOrFile::default()
    }
}

impl<M, F> MemOrFile<M, F> {
    /// True if this is a Mem
    pub fn is_mem(&self) -> bool {
        matches!(self, MemOrFile::Mem(_))
    }
}
