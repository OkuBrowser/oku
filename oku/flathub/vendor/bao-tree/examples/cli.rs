//! A command line interface to encode and decode parts of files.
//!
//! This is a simple example of how to use bao-tree to encode and decode parts of files.
//!
//! Examples:
//!
//! ```
//! cargo run --example cli encode somefile --ranges 1..10000 --out somefile.bao
//! ```
//!
//! will encode bytes 1..10000 of somefile and write the encoded data to somefile.bao.
//!
//! ```
//! cargo run --example cli decode somefile.bao > /dev/null
//! ```
//!
//! will decode the encoded data in somefile.bao and write it to /dev/null, printing
//! information about the decoding process to stderr.
use std::{
    io::{self, Cursor},
    path::PathBuf,
};

use anyhow::Context;
use bao_tree::{
    blake3,
    io::{outboard::PreOrderMemOutboard, round_up_to_chunks, Leaf, Parent},
    BlockSize, ChunkNum, ChunkRanges,
};
use bytes::Bytes;
use clap::{Parser, Subcommand};
use range_collections::RangeSet2;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

#[derive(Parser, Debug)]
struct Args {
    /// Block size log. Actual block size is 1024 * 2^block_size_log.
    ///
    /// Valid values are 0..8. Default is 4. When setting this to 0, the chunk group size
    /// is 1 chunk or 1024 bytes, and the generated encoding is compatible with the original
    /// bao crate.
    #[clap(long, default_value_t = 4)]
    block_size: u8,
    /// Use async io
    #[clap(long)]
    r#async: bool,
    /// Quiet mode, no progress output to stderr
    #[clap(long, short)]
    quiet: bool,
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Encode a part of a file.
    Encode(EncodeArgs),
    /// Decode a previously encoded part of a file.
    Decode(DecodeArgs),
}

#[derive(Parser, Debug)]
struct EncodeArgs {
    /// The file to encode.
    file: PathBuf,
    /// Byte ranges to encode.
    ///
    /// Ranges can be given as full ranges, e.g. 1..10000, or as open ranges, e.g. 1.. or ..10000.
    /// The range of all bytes is given as .. . Multiple ranges can be given, separated by , or ;.
    ///
    /// To produce output compatible with the original bao crate, use a block size of 0 and just a single
    /// range.
    ///
    /// Giving the range of all bytes will just interleave the data with the outboard. Giving a range
    /// outside the file such as 1000000000.. will encode the last chunk.
    #[clap(long)]
    ranges: Vec<String>,
    /// The file to write the encoded data to.
    ///
    /// If not given, the encoded data is written to stdout.
    #[clap(long)]
    out: Option<PathBuf>,
}

#[derive(Parser, Debug)]
struct DecodeArgs {
    /// The file to decode.
    file: PathBuf,
    /// The file to encode into
    ///
    /// If not given, the data is dumped to stdout.
    /// If the message is just a part of the file, only the chunks in the message
    /// are written to the file, and the rest left as is or zeroed out.
    #[clap(long)]
    target: Option<PathBuf>,
}

/// A message that contains a self-contained verifiable encoding of a part of a file.
///
/// The block size needs to be common for sender and receiver.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "MessageWireFormat", into = "MessageWireFormat")]
struct Message {
    hash: blake3::Hash,
    ranges: ChunkRanges,
    encoded: Vec<u8>,
}

/// Helper struct to serialize and deserialize messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MessageWireFormat {
    hash: [u8; 32],
    ranges: Vec<u64>,
    encoded: Vec<u8>,
}

impl From<Message> for MessageWireFormat {
    fn from(msg: Message) -> Self {
        Self {
            hash: msg.hash.into(),
            ranges: msg.ranges.boundaries().iter().map(|b| b.0).collect(),
            encoded: msg.encoded,
        }
    }
}

impl TryFrom<MessageWireFormat> for Message {
    type Error = anyhow::Error;

    fn try_from(msg: MessageWireFormat) -> Result<Self, Self::Error> {
        let hash = blake3::Hash::from(msg.hash);
        let ranges = msg
            .ranges
            .iter()
            .map(|b| ChunkNum(*b))
            .collect::<SmallVec<_>>();
        let ranges = ChunkRanges::new(ranges).context("chunks not sorted")?;
        Ok(Self {
            hash,
            ranges,
            encoded: msg.encoded,
        })
    }
}

macro_rules! log {
    ($verbose:expr, $($arg:tt)*) => {
        if $verbose {
            eprintln!($($arg)*);
        }
    };
}

/// Parse a range string into a range set containing a single range
fn parse_range(range_str: &str) -> anyhow::Result<RangeSet2<u64>> {
    let parts: Vec<&str> = range_str.split("..").collect();
    match parts.as_slice() {
        [start, end] if !start.is_empty() && !end.is_empty() => {
            Ok(RangeSet2::from(start.parse()?..end.parse()?))
        }
        [start, _] if !start.is_empty() => Ok(RangeSet2::from(start.parse()?..)),
        [_, end] if !end.is_empty() => Ok(RangeSet2::from(..end.parse()?)),
        [_, _] => Ok(RangeSet2::from(..)),
        _ => anyhow::bail!("invalid range"),
    }
}

fn parse_ranges(ranges: Vec<String>) -> anyhow::Result<RangeSet2<u64>> {
    let ranges = ranges
        .iter()
        .flat_map(|x| x.split(&[',', ';']))
        .map(parse_range)
        .collect::<anyhow::Result<Vec<_>>>()?;
    let ranges = ranges
        .into_iter()
        .fold(RangeSet2::empty(), |acc, item| acc | item);
    Ok(ranges)
}

mod sync {
    use std::io::{self, Cursor, Read, Write};

    use bao_tree::{
        io::{
            outboard::PreOrderMemOutboard,
            round_up_to_chunks,
            sync::{encode_ranges_validated, DecodeResponseIter, Outboard},
            BaoContentItem, Leaf, Parent,
        },
        BaoTree, BlockSize, ChunkRanges,
    };
    use positioned_io::WriteAt;

    use crate::{parse_ranges, Args, Command, DecodeArgs, EncodeArgs, Message};

    /// Encode a part of a file, given the outboard.
    fn encode(data: &[u8], outboard: impl Outboard, ranges: ChunkRanges) -> Message {
        let mut encoded = Vec::new();
        encode_ranges_validated(data, &outboard, &ranges, &mut encoded).unwrap();
        Message {
            hash: outboard.root(),
            ranges: ranges.clone(),
            encoded,
        }
    }

    fn decode_into_file(
        msg: &Message,
        mut target: std::fs::File,
        block_size: BlockSize,
        v: bool,
    ) -> io::Result<()> {
        let mut reader = Cursor::new(msg.encoded.as_slice());
        let mut size = [0; 8];
        reader.read_exact(&mut size)?;
        let size = u64::from_le_bytes(size);
        let tree = BaoTree::new(size, block_size);
        let iter = DecodeResponseIter::new(msg.hash, tree, reader, &msg.ranges);
        let mut indent = 0;
        target.set_len(size)?;
        for response in iter {
            match response? {
                BaoContentItem::Parent(Parent { node, pair: (l, r) }) => {
                    indent = indent.max(node.level() + 1);
                    let prefix = " ".repeat((indent - node.level()) as usize);
                    log!(
                        v,
                        "{}got parent {:?} level {} and children {} and {}",
                        prefix,
                        node,
                        node.level(),
                        l.to_hex(),
                        r.to_hex()
                    );
                }
                BaoContentItem::Leaf(Leaf { offset, data }) => {
                    let prefix = " ".repeat(indent as usize);
                    log!(
                        v,
                        "{}got data at offset {} and len {}",
                        prefix,
                        offset,
                        data.len()
                    );
                    target.write_at(offset, &data)?;
                }
            }
        }
        Ok(())
    }

    fn decode_to_stdout(msg: &Message, block_size: BlockSize, v: bool) -> io::Result<()> {
        let mut reader = Cursor::new(&msg.encoded);
        let mut size = [0; 8];
        reader.read_exact(&mut size)?;
        let size = u64::from_le_bytes(size);
        let tree = BaoTree::new(size, block_size);
        let iter = DecodeResponseIter::new(msg.hash, tree, Cursor::new(&msg.encoded), &msg.ranges);
        let mut indent = 0;
        for response in iter {
            match response? {
                BaoContentItem::Parent(Parent { node, pair: (l, r) }) => {
                    indent = indent.max(node.level() + 1);
                    let prefix = " ".repeat((indent - node.level()) as usize);
                    log!(
                        v,
                        "{}got parent {:?} level {} and children {} and {}",
                        prefix,
                        node,
                        node.level(),
                        l.to_hex(),
                        r.to_hex()
                    );
                }
                BaoContentItem::Leaf(Leaf { offset, data }) => {
                    let prefix = " ".repeat(indent as usize);
                    log!(
                        v,
                        "{}got data at offset {} and len {}",
                        prefix,
                        offset,
                        data.len()
                    );
                    io::stdout().write_all(&data)?;
                }
            }
        }
        Ok(())
    }

    pub(super) fn main(args: Args) -> anyhow::Result<()> {
        assert!(args.block_size <= 8);
        let block_size = BlockSize::from_chunk_log(args.block_size);
        let v = !args.quiet;
        match args.command {
            Command::Encode(EncodeArgs { file, ranges, out }) => {
                let ranges = parse_ranges(ranges)?;
                log!(v, "byte ranges: {:?}", ranges);
                let ranges = round_up_to_chunks(&ranges);
                log!(v, "chunk ranges: {:?}", ranges);
                log!(v, "reading file");
                let data = std::fs::read(file)?;
                log!(v, "computing outboard");
                let t0 = std::time::Instant::now();
                let outboard = PreOrderMemOutboard::create(&data, block_size);
                log!(v, "done in {:?}.", t0.elapsed());
                log!(v, "encoding message");
                let t0 = std::time::Instant::now();
                let msg = encode(&data, outboard, ranges);
                log!(
                    v,
                    "done in {:?}. {} bytes.",
                    t0.elapsed(),
                    msg.encoded.len()
                );
                let bytes = postcard::to_stdvec(&msg)?;
                log!(v, "serialized message");
                if let Some(out) = out {
                    std::fs::write(out, bytes)?;
                } else {
                    std::io::stdout().write_all(&bytes)?;
                }
            }
            Command::Decode(DecodeArgs { file, target }) => {
                let data = std::fs::read(file)?;
                let msg: Message = postcard::from_bytes(&data)?;
                if let Some(target) = target {
                    let target = std::fs::OpenOptions::new()
                        .write(true)
                        .create(true)
                        .truncate(true)
                        .open(target)?;
                    decode_into_file(&msg, target, block_size, v)?;
                } else {
                    decode_to_stdout(&msg, block_size, v)?;
                }
            }
        }
        Ok(())
    }
}

mod fsm {
    use bao_tree::{
        io::{
            fsm::{encode_ranges_validated, Outboard, ResponseDecoder, ResponseDecoderNext},
            BaoContentItem,
        },
        BaoTree,
    };
    use iroh_io::AsyncSliceWriter;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    use super::*;

    async fn encode(
        data: Bytes,
        outboard: impl Outboard,
        ranges: ChunkRanges,
    ) -> anyhow::Result<Message> {
        let mut encoded = Vec::new();
        let hash = outboard.root();
        encode_ranges_validated(data, outboard, &ranges, &mut encoded).await?;
        Ok(Message {
            hash,
            ranges: ranges.clone(),
            encoded,
        })
    }

    async fn decode_into_file(
        msg: Message,
        mut target: impl AsyncSliceWriter,
        block_size: BlockSize,
        v: bool,
    ) -> io::Result<()> {
        let mut encoded = Cursor::new(msg.encoded.as_slice());
        let size = encoded.read_u64_le().await?;
        let mut reading = ResponseDecoder::new(
            msg.hash,
            msg.ranges,
            BaoTree::new(size, block_size),
            encoded,
        );
        log!(v, "got header claiming a size of {}", size);
        let mut indent = 0;
        while let ResponseDecoderNext::More((reading1, res)) = reading.next().await {
            match res? {
                BaoContentItem::Parent(Parent { node, pair: (l, r) }) => {
                    indent = indent.max(node.level() + 1);
                    let prefix = " ".repeat((indent - node.level()) as usize);
                    log!(
                        v,
                        "{}got parent {:?} level {} and children {} and {}",
                        prefix,
                        node,
                        node.level(),
                        l.to_hex(),
                        r.to_hex()
                    );
                }
                BaoContentItem::Leaf(Leaf { offset, data }) => {
                    let prefix = " ".repeat(indent as usize);
                    log!(
                        v,
                        "{}got data at offset {} and len {}",
                        prefix,
                        offset,
                        data.len()
                    );
                    target.write_at(offset, &data).await?;
                }
            }
            reading = reading1;
        }
        Ok(())
    }

    async fn decode_to_stdout(msg: Message, block_size: BlockSize, v: bool) -> io::Result<()> {
        let mut encoded = Cursor::new(msg.encoded.as_slice());
        let size = encoded.read_u64_le().await?;
        let mut reading = ResponseDecoder::new(
            msg.hash,
            msg.ranges,
            BaoTree::new(size, block_size),
            encoded,
        );
        log!(v, "got header claiming a size of {}", size);
        let mut indent = 0;
        while let ResponseDecoderNext::More((reading1, res)) = reading.next().await {
            match res? {
                BaoContentItem::Parent(Parent { node, pair: (l, r) }) => {
                    indent = indent.max(node.level() + 1);
                    let prefix = " ".repeat((indent - node.level()) as usize);
                    log!(
                        v,
                        "{}got parent {:?} level {} and children {} and {}",
                        prefix,
                        node,
                        node.level(),
                        l.to_hex(),
                        r.to_hex()
                    );
                }
                BaoContentItem::Leaf(Leaf { offset, data }) => {
                    let prefix = " ".repeat(indent as usize);
                    log!(
                        v,
                        "{}got data at offset {} and len {}",
                        prefix,
                        offset,
                        data.len()
                    );
                    tokio::io::stdout().write_all(&data).await?;
                }
            }
            reading = reading1;
        }
        Ok(())
    }

    pub(super) async fn main(args: Args) -> anyhow::Result<()> {
        assert!(args.block_size <= 8);
        let block_size = BlockSize::from_chunk_log(args.block_size);
        let v = !args.quiet;
        match args.command {
            Command::Encode(EncodeArgs { file, ranges, out }) => {
                let ranges = parse_ranges(ranges)?;
                log!(v, "byte ranges: {:?}", ranges);
                let ranges = round_up_to_chunks(&ranges);
                log!(v, "chunk ranges: {:?}", ranges);
                log!(v, "reading file");
                let data = Bytes::from(std::fs::read(file)?);
                log!(v, "computing outboard");
                let t0 = std::time::Instant::now();
                let outboard = PreOrderMemOutboard::create(&data, block_size);
                log!(v, "done in {:?}.", t0.elapsed());
                log!(v, "encoding message");
                let t0 = std::time::Instant::now();
                let msg = encode(data, outboard, ranges).await?;
                log!(
                    v,
                    "done in {:?}. {} bytes.",
                    t0.elapsed(),
                    msg.encoded.len()
                );
                let bytes = postcard::to_stdvec(&msg)?;
                log!(v, "serialized message");
                if let Some(out) = out {
                    tokio::fs::write(out, bytes).await?;
                } else {
                    tokio::io::stdout().write_all(&bytes).await?;
                }
            }
            Command::Decode(DecodeArgs { file, target }) => {
                let data = std::fs::read(file)?;
                let msg: Message = postcard::from_bytes(&data)?;
                if let Some(target) = target {
                    let target = std::fs::OpenOptions::new()
                        .write(true)
                        .create(true)
                        .truncate(true)
                        .open(target)?;
                    let target = iroh_io::File::from_std(target);
                    decode_into_file(msg, target, block_size, v).await?;
                } else {
                    decode_to_stdout(msg, block_size, v).await?;
                }
            }
        }
        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    if args.r#async {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;
        rt.block_on(fsm::main(args))
    } else {
        sync::main(args)
    }
}
