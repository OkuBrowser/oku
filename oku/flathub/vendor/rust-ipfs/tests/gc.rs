use ipld_core::cid::Cid;
use multihash_codetable::{Code, MultihashDigest};
use rust_ipfs::block::BlockCodec;
use rust_ipfs::{Block, Node};

fn create_block() -> Block {
    let data = b"hello block\n".to_vec();
    let cid = Cid::new_v1(BlockCodec::Raw.into(), Code::Sha2_256.digest(&data));

    Block::new_unchecked(cid, data)
}

#[tokio::test]
async fn cleanup_unpinned_blocks() -> anyhow::Result<()> {
    let node = Node::new("gc_test_node").await;
    let block = create_block();
    let cid = node.put_block(&block).await?;

    let removed = node.gc().await?;

    assert_eq!(removed[0], cid);

    Ok(())
}

#[tokio::test]
async fn gc_cleanup_attempt_of_pinned_blocks() -> anyhow::Result<()> {
    let node = Node::new("gc_test_node").await;
    let block = create_block();
    let cid = node.put_block(&block).await?;
    node.insert_pin(cid).await?;
    let removed = node.gc().await?;
    assert!(removed.is_empty());

    Ok(())
}
