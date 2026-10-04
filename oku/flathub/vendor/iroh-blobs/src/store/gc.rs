use std::{collections::HashSet, pin::Pin, sync::Arc};

use bao_tree::ChunkRanges;
use genawaiter::sync::{Co, Gen};
use n0_future::{time::Duration, Stream, StreamExt};
use tracing::{debug, error, info, warn};

use crate::{api::Store, Hash, HashAndFormat};

/// An event related to GC
#[derive(Debug)]
pub enum GcMarkEvent {
    /// A custom event (info)
    CustomDebug(String),
    /// A custom non critical error
    CustomWarning(String, Option<crate::api::Error>),
    /// An unrecoverable error during GC
    Error(crate::api::Error),
}

/// An event related to GC
#[derive(Debug)]
pub enum GcSweepEvent {
    /// A custom event (debug)
    CustomDebug(String),
    /// A custom non critical error
    #[allow(dead_code)]
    CustomWarning(String, Option<crate::api::Error>),
    /// An unrecoverable error during GC
    Error(crate::api::Error),
}

/// Compute the set of live hashes
pub(super) async fn gc_mark_task(
    store: &Store,
    live: &mut HashSet<Hash>,
    co: &Co<GcMarkEvent>,
) -> crate::api::Result<()> {
    macro_rules! trace {
        ($($arg:tt)*) => {
            co.yield_(GcMarkEvent::CustomDebug(format!($($arg)*))).await;
        };
    }
    macro_rules! warn {
        ($($arg:tt)*) => {
            co.yield_(GcMarkEvent::CustomWarning(format!($($arg)*), None)).await;
        };
    }
    let mut roots = HashSet::new();
    trace!("traversing tags");
    let mut tags = store.tags().list().await?;
    while let Some(tag) = tags.next().await {
        let info = tag?;
        trace!("adding root {:?} {:?}", info.name, info.hash_and_format());
        roots.insert(info.hash_and_format());
    }
    trace!("traversing temp roots");
    let mut tts = store.tags().list_temp_tags().await?;
    while let Some(tt) = tts.next().await {
        trace!("adding temp root {:?}", tt);
        roots.insert(tt);
    }
    for HashAndFormat { hash, format } in roots {
        // we need to do this for all formats except raw
        if live.insert(hash) && !format.is_raw() {
            let mut stream = store.export_bao(hash, ChunkRanges::all()).hashes();
            while let Some(hash) = stream.next().await {
                match hash {
                    Ok(hash) => {
                        live.insert(hash);
                    }
                    Err(e) => {
                        warn!("error while traversing hashseq: {e:?}");
                    }
                }
            }
        }
    }
    trace!("gc mark done. found {} live blobs", live.len());
    Ok(())
}

async fn gc_sweep_task(
    store: &Store,
    live: &HashSet<Hash>,
    co: &Co<GcSweepEvent>,
) -> crate::api::Result<()> {
    let mut blobs = store.blobs().list().stream().await?;
    let mut count = 0;
    let mut batch = Vec::new();
    while let Some(hash) = blobs.next().await {
        let hash = hash?;
        if !live.contains(&hash) {
            batch.push(hash);
            count += 1;
        }
        if batch.len() >= 100 {
            store.blobs().delete(batch.clone()).await?;
            batch.clear();
        }
    }
    if !batch.is_empty() {
        store.blobs().delete(batch).await?;
    }
    store.sync_db().await?;
    co.yield_(GcSweepEvent::CustomDebug(format!("deleted {count} blobs")))
        .await;
    Ok(())
}

fn gc_mark<'a>(
    store: &'a Store,
    live: &'a mut HashSet<Hash>,
) -> impl Stream<Item = GcMarkEvent> + 'a {
    Gen::new(|co| async move {
        if let Err(e) = gc_mark_task(store, live, &co).await {
            co.yield_(GcMarkEvent::Error(e)).await;
        }
    })
}

fn gc_sweep<'a>(
    store: &'a Store,
    live: &'a HashSet<Hash>,
) -> impl Stream<Item = GcSweepEvent> + 'a {
    Gen::new(|co| async move {
        if let Err(e) = gc_sweep_task(store, live, &co).await {
            co.yield_(GcSweepEvent::Error(e)).await;
        }
    })
}

/// Configuration for garbage collection.
///
/// To protect blobs during long-running writes without pausing the GC
/// schedule, use [`crate::api::blobs::Batch::temp_tag`].
#[derive(derive_more::Debug, Clone)]
pub struct GcConfig {
    /// Interval in which to run garbage collection.
    pub interval: Duration,
    /// Optional callback to manually add protected blobs.
    ///
    /// The callback is called before each garbage collection run. It gets a `&mut HashSet<Hash>`
    /// and returns a future that returns [`ProtectOutcome`]. All hashes that are added to the
    /// [`HashSet`] will be protected from garbage collection during this run.
    ///
    /// In normal operation, return [`ProtectOutcome::Continue`] from the callback. If you return
    /// [`ProtectOutcome::Abort`], the garbage collection run will be aborted.Use this if your
    /// source of hashes to protect returned an error, and thus garbage collection should be skipped
    /// completely to not unintentionally delete blobs that should be protected.
    #[debug("ProtectCallback")]
    pub add_protected: Option<ProtectCb>,
}

/// Returned from [`ProtectCb`].
///
/// See [`GcConfig::add_protected] for details.
#[derive(Debug)]
pub enum ProtectOutcome {
    /// Continue with the garbage collection run.
    Continue,
    /// Abort the garbage collection run.
    Abort,
}

/// The type of the garbage collection callback.
///
/// See [`GcConfig::add_protected] for details.
pub type ProtectCb = Arc<
    dyn for<'a> Fn(
            &'a mut HashSet<Hash>,
        )
            -> Pin<Box<dyn std::future::Future<Output = ProtectOutcome> + Send + Sync + 'a>>
        + Send
        + Sync
        + 'static,
>;

pub async fn gc_run_once(store: &Store, live: &mut HashSet<Hash>) -> crate::api::Result<()> {
    debug!(externally_protected = live.len(), "gc: start");
    {
        store.clear_protected().await?;
        let mut stream = gc_mark(store, live);
        while let Some(ev) = stream.next().await {
            match ev {
                GcMarkEvent::CustomDebug(msg) => {
                    debug!("{}", msg);
                }
                GcMarkEvent::CustomWarning(msg, err) => {
                    warn!("{}: {:?}", msg, err);
                }
                GcMarkEvent::Error(err) => {
                    error!("error during gc mark: {:?}", err);
                    return Err(err);
                }
            }
        }
    }
    debug!(total_protected = live.len(), "gc: sweep");
    {
        let mut stream = gc_sweep(store, live);
        while let Some(ev) = stream.next().await {
            match ev {
                GcSweepEvent::CustomDebug(msg) => {
                    debug!("{}", msg);
                }
                GcSweepEvent::CustomWarning(msg, err) => {
                    warn!("{}: {:?}", msg, err);
                }
                GcSweepEvent::Error(err) => {
                    error!("error during gc sweep: {:?}", err);
                    return Err(err);
                }
            }
        }
    }
    debug!("gc: done");

    Ok(())
}

pub async fn run_gc(store: Store, config: GcConfig) {
    debug!("gc enabled with interval {:?}", config.interval);
    let mut live = HashSet::new();
    loop {
        live.clear();
        n0_future::time::sleep(config.interval).await;
        if let Some(ref cb) = config.add_protected {
            match (cb)(&mut live).await {
                ProtectOutcome::Continue => {}
                ProtectOutcome::Abort => {
                    info!("abort gc run: protect callback indicated abort");
                    continue;
                }
            }
        }
        if let Err(e) = gc_run_once(&store, &mut live).await {
            error!("error during gc run: {e}");
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::{self};

    use bao_tree::io::EncodeError;
    use range_collections::RangeSet2;
    use testresult::TestResult;

    use super::*;
    use crate::{
        api::{blobs::AddBytesOptions, ExportBaoError, RequestError, Store},
        hashseq::HashSeq,
        BlobFormat,
    };

    async fn gc_smoke(store: &Store) -> TestResult<()> {
        let blobs = store.blobs();
        let at = blobs.add_slice("a").temp_tag().await?;
        let bt = blobs.add_slice("b").temp_tag().await?;
        let ct = blobs.add_slice("c").temp_tag().await?;
        let dt = blobs.add_slice("d").temp_tag().await?;
        let et = blobs.add_slice("e").temp_tag().await?;
        let ft = blobs.add_slice("f").temp_tag().await?;
        let gt = blobs.add_slice("g").temp_tag().await?;
        let ht = blobs.add_slice("h").with_named_tag("h").await?;
        let a = at.hash();
        let b = bt.hash();
        let c = ct.hash();
        let d = dt.hash();
        let e = et.hash();
        let f = ft.hash();
        let g = gt.hash();
        let h = ht.hash;
        store.tags().set("c", ct.hash_and_format()).await?;
        let dehs = [d, e].into_iter().collect::<HashSeq>();
        let hehs = blobs
            .add_bytes_with_opts(AddBytesOptions {
                data: dehs.into(),
                format: BlobFormat::HashSeq,
            })
            .await?;
        let fghs = [f, g].into_iter().collect::<HashSeq>();
        let fghs = blobs
            .add_bytes_with_opts(AddBytesOptions {
                data: fghs.into(),
                format: BlobFormat::HashSeq,
            })
            .temp_tag()
            .await?;
        store.tags().set("fg", fghs.hash_and_format()).await?;
        drop(fghs);
        drop(bt);
        store.tags().delete("h").await?;
        let mut live = HashSet::new();
        gc_run_once(store, &mut live).await?;
        // a is protected because we keep the temp tag
        assert!(live.contains(&a));
        assert!(store.has(a).await?);
        // b is not protected because we drop the temp tag
        assert!(!live.contains(&b));
        assert!(!store.has(b).await?);
        // c is protected because we set an explicit tag
        assert!(live.contains(&c));
        assert!(store.has(c).await?);
        // d and e are protected because they are part of a hashseq protected by a temp tag
        assert!(live.contains(&d));
        assert!(store.has(d).await?);
        assert!(live.contains(&e));
        assert!(store.has(e).await?);
        // f and g are protected because they are part of a hashseq protected by a tag
        assert!(live.contains(&f));
        assert!(store.has(f).await?);
        assert!(live.contains(&g));
        assert!(store.has(g).await?);
        // h is not protected because we deleted the tag before gc ran
        assert!(!live.contains(&h));
        assert!(!store.has(h).await?);
        drop(at);
        drop(hehs);
        Ok(())
    }

    #[cfg(feature = "fs-store")]
    async fn gc_file_delete(path: &std::path::Path, store: &Store) -> TestResult<()> {
        use bao_tree::ChunkNum;

        use crate::store::{fs::options::PathOptions, util::tests::create_n0_bao};
        let mut live = HashSet::new();
        let options = PathOptions::new(&path.join("db"));
        // create a large complete file and check that the data and outboard files are deleted by gc
        {
            let a = store
                .blobs()
                .add_slice(vec![0u8; 8000000])
                .temp_tag()
                .await?;
            let ah = a.hash();
            let data_path = options.data_path(&ah);
            let outboard_path = options.outboard_path(&ah);
            assert!(data_path.exists());
            assert!(outboard_path.exists());
            assert!(store.has(ah).await?);
            drop(a);
            gc_run_once(store, &mut live).await?;
            assert!(!data_path.exists());
            assert!(!outboard_path.exists());
        }
        live.clear();
        // create a large partial file and check that the data and outboard file as well as
        // the sizes and bitfield files are deleted by gc
        {
            let data = vec![1u8; 8000000];
            let ranges = ChunkRanges::from(..ChunkNum(19));
            let (bh, b_bao) = create_n0_bao(&data, &ranges)?;
            store.import_bao_bytes(bh, ranges, b_bao).await?;
            let data_path = options.data_path(&bh);
            let outboard_path = options.outboard_path(&bh);
            let sizes_path = options.sizes_path(&bh);
            let bitfield_path = options.bitfield_path(&bh);
            store.wait_idle().await?;
            assert!(data_path.exists());
            assert!(outboard_path.exists());
            assert!(sizes_path.exists());
            assert!(bitfield_path.exists());
            gc_run_once(store, &mut live).await?;
            assert!(!data_path.exists());
            assert!(!outboard_path.exists());
            assert!(!sizes_path.exists());
            assert!(!bitfield_path.exists());
        }
        Ok(())
    }

    #[tokio::test]
    #[cfg(feature = "fs-store")]
    async fn gc_smoke_fs() -> TestResult {
        tracing_subscriber::fmt::try_init().ok();
        let testdir = tempfile::tempdir()?;
        let db_path = testdir.path().join("db");
        let store = crate::store::fs::FsStore::load(&db_path).await?;
        gc_smoke(&store).await?;
        gc_file_delete(testdir.path(), &store).await?;
        Ok(())
    }

    #[tokio::test]
    async fn gc_smoke_mem() -> TestResult {
        tracing_subscriber::fmt::try_init().ok();
        let store = crate::store::mem::MemStore::new();
        gc_smoke(&store).await?;
        Ok(())
    }

    #[tokio::test]
    #[cfg(feature = "fs-store")]
    async fn gc_check_deletion_fs() -> TestResult {
        tracing_subscriber::fmt::try_init().ok();
        let testdir = tempfile::tempdir()?;
        let db_path = testdir.path().join("db");
        let store = crate::store::fs::FsStore::load(&db_path).await?;
        gc_check_deletion(&store).await
    }

    #[tokio::test]
    async fn gc_check_deletion_mem() -> TestResult {
        tracing_subscriber::fmt::try_init().ok();
        let store = crate::store::mem::MemStore::default();
        gc_check_deletion(&store).await
    }

    async fn gc_check_deletion(store: &Store) -> TestResult {
        let temp_tag = store.add_bytes(b"foo".to_vec()).temp_tag().await?;
        let hash = temp_tag.hash();
        assert_eq!(store.get_bytes(hash).await?.as_ref(), b"foo");
        drop(temp_tag);
        let mut live = HashSet::new();
        gc_run_once(store, &mut live).await?;

        // check that `get_bytes` returns an error.
        let res = store.get_bytes(hash).await;
        assert!(res.is_err());
        assert!(matches!(
            res,
            Err(ExportBaoError::ExportBaoInner {
                source: EncodeError::Io(cause),
                ..
            }) if cause.kind() == io::ErrorKind::NotFound
        ));

        // check that `export_ranges` returns an error.
        let res = store
            .export_ranges(hash, RangeSet2::all())
            .concatenate()
            .await;
        assert!(res.is_err());
        assert!(matches!(
            res,
            Err(RequestError::Inner{
                source: crate::api::Error::Io(cause),
                ..
            }) if cause.kind() == io::ErrorKind::NotFound
        ));

        // check that `export_bao` returns an error.
        let res = store
            .export_bao(hash, ChunkRanges::all())
            .bao_to_vec()
            .await;
        assert!(res.is_err());
        println!("export_bao res {res:?}");
        assert!(matches!(
            res,
            Err(RequestError::Inner{
                source: crate::api::Error::Io(cause),
                ..
            }) if cause.kind() == io::ErrorKind::NotFound
        ));

        // check that `export` returns an error.
        let target = tempfile::NamedTempFile::new()?;
        let path = target.path();
        let res = store.export(hash, path).await;
        assert!(res.is_err());
        assert!(matches!(
            res,
            Err(RequestError::Inner{
                source: crate::api::Error::Io(cause),
                ..
            }) if cause.kind() == io::ErrorKind::NotFound
        ));
        Ok(())
    }
}
