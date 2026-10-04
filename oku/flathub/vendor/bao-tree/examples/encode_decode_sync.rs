use std::io;

use bao_tree::{
    io::{
        outboard::PreOrderOutboard,
        round_up_to_chunks,
        sync::{decode_ranges, encode_ranges_validated, valid_ranges, CreateOutboard},
    },
    BlockSize, ByteRanges, ChunkRanges,
};

/// Use a block size of 16 KiB, a good default for most cases
const BLOCK_SIZE: BlockSize = BlockSize::from_chunk_log(4);

fn main() -> io::Result<()> {
    // The file we want to serve
    let file = std::fs::File::open("video.mp4")?;
    // Create an outboard for the file, using the current size
    let ob = PreOrderOutboard::<Vec<u8>>::create(&file, BLOCK_SIZE)?;
    // Encode the first 100000 bytes of the file
    let ranges = ByteRanges::from(0..100000);
    let ranges = round_up_to_chunks(&ranges);
    // Stream of data to client. Needs to implement `io::Write`. We just use a vec here.
    let mut to_client = vec![];
    encode_ranges_validated(&file, &ob, &ranges, &mut to_client)?;

    // Stream of data from client. Needs to implement `io::Read`. We just wrap the vec in a cursor.
    let from_server = io::Cursor::new(to_client);
    let root = ob.root;
    let tree = ob.tree;

    // Decode the encoded data into a file
    let mut decoded = std::fs::File::create("copy.mp4")?;
    let mut ob = PreOrderOutboard {
        tree,
        root,
        data: vec![],
    };
    decode_ranges(from_server, &ranges, &mut decoded, &mut ob)?;

    // the first 100000 bytes of the file should now be in `decoded`
    // in addition, the required part of the tree to validate that the data is
    // correct are in `ob.data`

    // Print the valid ranges of the file
    for range in valid_ranges(&ob, &decoded, &ChunkRanges::all()) {
        println!("{range:?}");
    }
    Ok(())
}
