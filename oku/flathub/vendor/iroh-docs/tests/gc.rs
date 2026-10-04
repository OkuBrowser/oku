#![allow(unused)]

use std::path::PathBuf;

use anyhow::Result;
use bytes::Bytes;
use iroh_blobs::api::blobs::ImportMode;
use n0_future::{time::Duration, StreamExt};
use rand::Rng;
use testdir::testdir;
use util::Node;

mod util;

pub fn create_test_data(size: usize) -> Bytes {
    let mut rand = rand::rng();
    let mut res = vec![0u8; size];
    rand.fill_bytes(&mut res);
    res.into()
}

/// Wrap a bao store in a node that has gc enabled.
#[cfg(feature = "fs-store")]
async fn persistent_node(
    path: PathBuf,
    gc_period: Duration,
) -> (Node, async_channel::Receiver<()>) {
    use iroh::endpoint::presets;

    let (gc_send, gc_recv) = async_channel::unbounded();
    let ep = iroh::Endpoint::bind(presets::Minimal).await.unwrap();
    let node = Node::persistent(path, ep)
        .gc_interval(Some(gc_period))
        .register_gc_done_cb(Box::new(move || {
            gc_send.send_blocking(()).ok();
        }))
        .spawn()
        .await
        .unwrap();
    (node, gc_recv)
}

#[tokio::test]
#[cfg(feature = "fs-store")]
async fn redb_doc_import_stress() -> Result<()> {
    let _ = tracing_subscriber::fmt::try_init();
    let dir = testdir!();
    let (node, _) = persistent_node(dir.join("store"), Duration::from_secs(10)).await;
    let blobs = node.blobs().clone();
    let client = node.client();
    let doc = client.docs().create().await?;
    let author = client.docs().author_create().await?;
    let temp_path = dir.join("temp");
    tokio::fs::create_dir_all(&temp_path).await?;
    let mut to_import = Vec::new();
    for i in 0..100 {
        let data = create_test_data(16 * 1024 * 3 + 1);
        let path = temp_path.join(format!("file{i}"));
        tokio::fs::write(&path, &data).await?;
        let key = Bytes::from(format!("{}", path.display()));
        to_import.push((key, path, data));
    }
    for (key, path, _) in to_import.iter() {
        let mut progress = doc
            .import_file(&blobs, author, key.clone(), path, ImportMode::TryReference)
            .await?;
        while let Some(msg) = progress.next().await {
            tracing::info!("import progress {:?}", msg);
        }
    }
    for (i, (key, _, expected)) in to_import.iter().enumerate() {
        let Some(entry) = doc.get_exact(author, key.clone(), true).await? else {
            anyhow::bail!("doc entry not found {}", i);
        };
        let hash = entry.content_hash();
        if !blobs.has(hash).await? {
            anyhow::bail!("content not found {} {}", i, &hash.to_hex()[..8]);
        };
        let data = blobs.get_bytes(hash).await?;
        assert_eq!(data, expected);
    }
    Ok(())
}
