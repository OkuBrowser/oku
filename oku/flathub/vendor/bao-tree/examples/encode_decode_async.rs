use std::io;

use bao_tree::{
    io::{
        fsm::{decode_ranges, encode_ranges_validated, valid_ranges, CreateOutboard},
        outboard::PreOrderOutboard,
        round_up_to_chunks,
    },
    BlockSize, ByteRanges, ChunkRanges,
};
use bytes::BytesMut;
use futures_lite::StreamExt;

/// Use a block size of 16 KiB, a good default for most cases
const BLOCK_SIZE: BlockSize = BlockSize::from_chunk_log(4);

#[tokio::main]
async fn main() -> io::Result<()> {
    // The file we want to serve
    let mut file = iroh_io::File::open("video.mp4".into()).await?;
    // Create an outboard for the file, using the current size
    let mut ob = PreOrderOutboard::<BytesMut>::create(&mut file, BLOCK_SIZE).await?;
    // Encode the first 100000 bytes of the file
    let ranges = ByteRanges::from(0..100000);
    let ranges = round_up_to_chunks(&ranges);
    // Stream of data to client. Needs to implement `io::Write`. We just use a vec here.
    let mut to_client = Vec::new();
    encode_ranges_validated(file, &mut ob, &ranges, &mut to_client).await?;

    // Stream of data from client. Needs to implement `io::Read`. We just wrap the vec in a cursor.
    let from_server = io::Cursor::new(to_client.as_slice());
    let root = ob.root;
    let tree = ob.tree;

    // Decode the encoded data into a file
    let mut decoded = iroh_io::File::open("copy.mp4".into()).await?;
    let mut ob = PreOrderOutboard {
        tree,
        root,
        data: BytesMut::new(),
    };
    decode_ranges(from_server, ranges, &mut decoded, &mut ob).await?;

    // the first 100000 bytes of the file should now be in `decoded`
    // in addition, the required part of the tree to validate that the data is
    // correct are in `ob.data`

    // Print the valid ranges of the file
    let ranges = ChunkRanges::all();
    let mut stream = valid_ranges(&mut ob, &mut decoded, &ranges);
    while let Some(range) = stream.next().await {
        println!("{range:?}");
    }
    Ok(())
}
