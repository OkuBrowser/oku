//! This example shows how to create tags that expire after a certain time.
//!
//! We use a prefix so we can distinguish between expiring and normal tags, and
//! then encode the expiry date in the tag name after the prefix, in a format
//! that sorts in the same order as the expiry date.
//!
//! The example creates a number of blobs and protects them directly or indirectly
//! with expiring tags. Watch as the expired tags are deleted and the blobs
//! are removed from the store.
use std::{
    ops::Deref,
    time::{Duration, SystemTime},
};

use chrono::Utc;
use iroh_blobs::{
    api::{blobs::AddBytesOptions, Store, Tag},
    hashseq::HashSeq,
    store::{
        fs::options::{BatchOptions, InlineOptions, Options, PathOptions},
        GcConfig,
    },
    BlobFormat, Hash,
};
use n0_future::StreamExt;
use tokio::signal::ctrl_c;

/// Using an iroh rpc client, create a tag that is marked to expire at `expiry` for all the given hashes.
///
/// The tag name will be `prefix`- followed by the expiry date in iso8601 format (e.g. `expiry-2025-01-01T12:00:00Z`).
async fn create_expiring_tag(
    store: &Store,
    hashes: &[Hash],
    prefix: &str,
    expiry: SystemTime,
) -> anyhow::Result<()> {
    let expiry = chrono::DateTime::<chrono::Utc>::from(expiry);
    let expiry = expiry.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let tagname = format!("{prefix}-{expiry}");
    if hashes.is_empty() {
        return Ok(());
    } else if hashes.len() == 1 {
        let hash = hashes[0];
        store.tags().set(&tagname, hash).await?;
    } else {
        let hs = hashes.iter().copied().collect::<HashSeq>();
        store
            .add_bytes_with_opts(AddBytesOptions {
                data: hs.into(),
                format: BlobFormat::HashSeq,
            })
            .with_named_tag(&tagname)
            .await?;
    };
    println!("Created tag {tagname}");
    Ok(())
}

async fn delete_expired_tags(blobs: &Store, prefix: &str, bulk: bool) -> anyhow::Result<()> {
    let prefix = format!("{prefix}-");
    let now = chrono::Utc::now();
    let end = format!(
        "{}-{}",
        prefix,
        now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
    );
    if bulk {
        // delete all tags with the prefix and an expiry date before now
        //
        // this should be very efficient, since it is just a single database operation
        blobs
            .tags()
            .delete_range(Tag::from(prefix.clone())..Tag::from(end))
            .await?;
    } else {
        // find tags to delete one by one and then delete them
        //
        // this allows us to print the tags before deleting them
        let mut tags = blobs.tags().list().await?;
        let mut to_delete = Vec::new();
        while let Some(tag) = tags.next().await {
            let tag = tag?.name;
            if let Some(rest) = tag.0.strip_prefix(prefix.as_bytes()) {
                let Ok(expiry) = std::str::from_utf8(rest) else {
                    tracing::warn!("Tag {} does have non utf8 expiry", tag);
                    continue;
                };
                let Ok(expiry) = chrono::DateTime::parse_from_rfc3339(expiry) else {
                    tracing::warn!("Tag {} does have invalid expiry date", tag);
                    continue;
                };
                let expiry = expiry.with_timezone(&Utc);
                if expiry < now {
                    to_delete.push(tag);
                }
            }
        }
        for tag in to_delete {
            println!("Deleting expired tag {tag}\n");
            blobs.tags().delete(tag).await?;
        }
    }
    Ok(())
}

async fn print_store_info(store: &Store) -> anyhow::Result<()> {
    let now = chrono::Utc::now();
    let mut tags = store.tags().list().await?;
    println!(
        "Current time: {}",
        now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
    );
    println!("Tags:");
    while let Some(tag) = tags.next().await {
        let tag = tag?;
        println!("  {tag:?}");
    }
    let mut blobs = store.list().stream().await?;
    println!("Blobs:");
    while let Some(item) = blobs.next().await {
        println!("  {}", item?);
    }
    println!();
    Ok(())
}

async fn info_task(store: Store) -> anyhow::Result<()> {
    n0_future::time::sleep(Duration::from_secs(1)).await;
    loop {
        print_store_info(&store).await?;
        n0_future::time::sleep(Duration::from_secs(5)).await;
    }
}

async fn delete_expired_tags_task(store: Store, prefix: &str) -> anyhow::Result<()> {
    loop {
        delete_expired_tags(&store, prefix, false).await?;
        n0_future::time::sleep(Duration::from_secs(5)).await;
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let path = std::env::current_dir()?.join("blobs");
    let options = Options {
        path: PathOptions::new(&path),
        gc: Some(GcConfig {
            add_protected: None,
            interval: Duration::from_secs(10),
        }),
        inline: InlineOptions::default(),
        batch: BatchOptions::default(),
    };
    let store =
        iroh_blobs::store::fs::FsStore::load_with_opts(path.join("blobs.db"), options).await?;

    // setup: add some data and tag it
    {
        // add several blobs and tag them with an expiry date 10 seconds in the future
        let batch = store.batch().await?;
        let a = batch.add_bytes("blob 1".as_bytes()).await?;
        let b = batch.add_bytes("blob 2".as_bytes()).await?;

        let expires_at = SystemTime::now()
            .checked_add(Duration::from_secs(10))
            .unwrap();
        create_expiring_tag(&store, &[a.hash(), b.hash()], "expiring", expires_at).await?;

        // add a single blob and tag it with an expiry date 60 seconds in the future
        let c = batch.add_bytes("blob 3".as_bytes()).await?;
        let expires_at = SystemTime::now()
            .checked_add(Duration::from_secs(60))
            .unwrap();
        create_expiring_tag(&store, &[c.hash()], "expiring", expires_at).await?;
        // batch goes out of scope, so data is only protected by the tags we created
    }

    // delete expired tags every 5 seconds
    let delete_task = tokio::spawn(delete_expired_tags_task(store.deref().clone(), "expiring"));
    // print all tags and blobs every 5 seconds
    let info_task = tokio::spawn(info_task(store.deref().clone()));

    ctrl_c().await?;
    delete_task.abort();
    info_task.abort();
    store.shutdown().await?;
    Ok(())
}
