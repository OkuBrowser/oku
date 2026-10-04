#![cfg(feature = "fs-store")]
use std::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    ops::Deref,
    path::Path,
};

use iroh_blobs::{
    api::{
        blobs::{AddProgressItem, Blobs},
        Store,
    },
    store::{fs::FsStore, mem::MemStore},
    Hash,
};
use n0_future::StreamExt;
use testresult::TestResult;

/// Interesting sizes for testing.
pub const INTERESTING_SIZES: [usize; 8] = [
    0,               // annoying corner case - always present, handled by the api
    1,               // less than 1 chunk, data inline, outboard not needed
    1024,            // exactly 1 chunk, data inline, outboard not needed
    1024 * 16 - 1,   // less than 1 chunk group, data inline, outboard not needed
    1024 * 16,       // exactly 1 chunk group, data inline, outboard not needed
    1024 * 16 + 1,   // data file, outboard inline (just 1 hash pair)
    1024 * 1024,     // data file, outboard inline (many hash pairs)
    1024 * 1024 * 8, // data file, outboard file
];

async fn blobs_smoke(path: &Path, blobs: &Blobs) -> TestResult<()> {
    // test importing and exporting bytes
    {
        let expected = b"hello".to_vec();
        let expected_hash = Hash::new(&expected);
        let tt = blobs.add_bytes(expected.clone()).await?;
        let hash = tt.hash;
        assert_eq!(hash, expected_hash);
        let actual = blobs.get_bytes(hash).await?;
        assert_eq!(actual, expected);
    }

    // test importing and exporting a file
    {
        let expected = b"somestuffinafile".to_vec();
        let temp1 = path.join("test1");
        std::fs::write(&temp1, &expected)?;
        let tt = blobs.add_path(temp1).await?;
        let hash = tt.hash;
        let expected_hash = Hash::new(&expected);
        assert_eq!(hash, expected_hash);

        let temp2 = path.join("test2");
        blobs.export(hash, &temp2).await?;
        let actual = std::fs::read(&temp2)?;
        assert_eq!(actual, expected);
    }

    // test importing a large file with progress
    {
        let expected = vec![0u8; 1024 * 1024];
        let temp1 = path.join("test3");
        std::fs::write(&temp1, &expected)?;
        let mut stream = blobs.add_path(temp1).stream().await;
        let mut res = None;
        while let Some(item) = stream.next().await {
            if let AddProgressItem::Done(tt) = item {
                res = Some(tt);
                break;
            }
        }
        let actual_hash = res.as_ref().map(|x| x.hash());
        let expected_hash = Hash::new(&expected);
        assert_eq!(actual_hash, Some(expected_hash));
    }

    {
        let hashes = blobs.list().hashes().await?;
        assert_eq!(hashes.len(), 3);
    }
    Ok(())
}

#[tokio::test]
async fn blobs_smoke_fs() -> TestResult {
    tracing_subscriber::fmt::try_init().ok();
    let td = tempfile::tempdir()?;
    let store = FsStore::load(td.path().join("a")).await?;
    blobs_smoke(td.path(), store.blobs()).await?;
    store.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn blobs_smoke_mem() -> TestResult {
    tracing_subscriber::fmt::try_init().ok();
    let td = tempfile::tempdir()?;
    let store = MemStore::new();
    blobs_smoke(td.path(), store.blobs()).await?;
    store.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn blobs_smoke_fs_rpc() -> TestResult {
    tracing_subscriber::fmt::try_init().ok();
    let unspecified = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0));
    let (server, cert) = irpc::util::make_server_endpoint(unspecified)?;
    let client = irpc::util::make_client_endpoint(unspecified, &[cert.as_ref()])?;
    let td = tempfile::tempdir()?;
    let store = FsStore::load(td.path().join("a")).await?;
    n0_future::task::spawn(store.deref().clone().listen(server.clone()));
    let api = Store::connect(client, server.local_addr()?);
    blobs_smoke(td.path(), api.blobs()).await?;
    api.shutdown().await?;
    Ok(())
}
