use super::IpldDag;
use crate::{Block, Error};
use anyhow::anyhow;
use bytes::{Bytes, BytesMut};
use futures::future::BoxFuture;
use futures::io::{AsyncRead, AsyncReadExt, BufReader};
use futures::stream::BoxStream;
use futures::{FutureExt, Stream, StreamExt, TryStreamExt};
use ipld_core::cid::Cid;
use ipld_core::codec::Codec;
use ipld_core::ipld::Ipld;
use std::collections::{BTreeMap, HashSet};
use std::future::IntoFuture;
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
#[cfg(not(target_arch = "wasm32"))]
use tokio::io::AsyncWriteExt;

/// The fixed 11-byte CARv2 pragma: `varint(10)` followed by the dag-cbor `{"version": 2}`.
const CAR_V2_PRAGMA: [u8; 11] = [
    0x0a, 0xa1, 0x67, 0x76, 0x65, 0x72, 0x73, 0x69, 0x6f, 0x6e, 0x02,
];
const V2_HEADER_LEN: usize = 40;
const IMPORT_BATCH: usize = 256;
const FETCH_TIMEOUT: Duration = Duration::from_secs(60);
/// Caps on the length a single CAR varint may declare before we allocate, so a crafted archive
/// cannot make us reserve gigabytes.
const MAX_HEADER_LEN: u64 = 32 << 20;
const MAX_SECTION_LEN: u64 = 32 << 20;

impl IpldDag {
    /// Exports the DAG rooted at `root` as a CARv1 archive.
    pub fn export(&self, root: Cid) -> CarExport {
        self.export_many([root])
    }

    /// Exports the DAGs rooted at `roots` as a single CARv1 archive sharing one block stream.
    pub fn export_many(&self, roots: impl IntoIterator<Item = Cid>) -> CarExport {
        CarExport {
            dag: self.clone(),
            roots: roots.into_iter().collect(),
            fetch: false,
            inner: None,
        }
    }

    /// Imports a CAR archive from `reader`, storing every block and returning the archive roots.
    pub fn import<R>(&self, reader: R) -> CarImport<R>
    where
        R: AsyncRead + Unpin + Send + 'static,
    {
        CarImport {
            dag: self.clone(),
            reader: Some(reader),
            pin: false,
            inner: None,
        }
    }
}

/// A CARv1 export
pub struct CarExport {
    dag: IpldDag,
    roots: Vec<Cid>,
    fetch: bool,
    inner: Option<BoxStream<'static, Result<Bytes, Error>>>,
}

impl CarExport {
    /// Fetches any block missing from the local store over the network.
    pub fn fetch(mut self) -> Self {
        self.fetch = true;
        self
    }

    /// Writes the archive to `path`, streaming without buffering the whole thing.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn to_file(self, path: impl AsRef<Path>) -> Result<(), Error> {
        let mut file = tokio::fs::File::create(path.as_ref()).await?;
        let mut stream = export_stream(self.dag, self.roots, self.fetch);
        while let Some(chunk) = stream.next().await {
            file.write_all(&chunk?).await?;
        }
        file.flush().await?;
        Ok(())
    }

    fn stream(&mut self) -> &mut BoxStream<'static, Result<Bytes, Error>> {
        if self.inner.is_none() {
            self.inner = Some(export_stream(
                self.dag.clone(),
                self.roots.clone(),
                self.fetch,
            ));
        }
        self.inner.as_mut().expect("just initialized")
    }
}

impl Stream for CarExport {
    type Item = Result<Bytes, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.get_mut().stream().poll_next_unpin(cx)
    }
}

impl IntoFuture for CarExport {
    type Output = Result<Bytes, Error>;
    type IntoFuture = BoxFuture<'static, Result<Bytes, Error>>;

    fn into_future(self) -> Self::IntoFuture {
        async move {
            let mut stream = export_stream(self.dag, self.roots, self.fetch);
            let mut out = BytesMut::new();
            while let Some(chunk) = stream.try_next().await? {
                out.extend_from_slice(&chunk);
            }
            Ok(out.freeze())
        }
        .boxed()
    }
}

fn export_stream(
    dag: IpldDag,
    roots: Vec<Cid>,
    fetch: bool,
) -> BoxStream<'static, Result<Bytes, Error>> {
    async_stream::try_stream! {
        if fetch && dag.ipfs.is_none() {
            Err::<(), _>(anyhow!("export .fetch() requires an Ipfs-backed dag; this dag is repo-only"))?;
        }

        yield frame(&encode_v1_header(&roots)?);

        let mut seen = HashSet::new();
        let mut stack: Vec<Cid> = roots.iter().rev().copied().collect();
        while let Some(cid) = stack.pop() {
            if !seen.insert(cid) {
                continue;
            }
            let block = export_block(&dag, cid, fetch).await?;

            let mut payload = block.cid().to_bytes();
            payload.extend_from_slice(block.data());
            yield frame(&payload);

            let mut refs = Vec::new();
            // an unknown codec has no links we can read; archive the block as an opaque leaf
            // rather than aborting the whole export
            if let Err(e) = block.references(&mut refs)
                && e.kind() != std::io::ErrorKind::Unsupported
            {
                Err::<(), Error>(e.into())?;
            }
            for child in refs.into_iter().rev() {
                if !seen.contains(&child) {
                    stack.push(child);
                }
            }
        }
    }
    .boxed()
}

async fn export_block(dag: &IpldDag, cid: Cid, fetch: bool) -> Result<Block, Error> {
    if fetch {
        dag.repo
            .get_block(cid)
            .set_local(false)
            .timeout(Some(FETCH_TIMEOUT))
            .await
    } else {
        dag.repo
            .get_block_now(cid)
            .await?
            .ok_or_else(|| anyhow!("missing block {cid}; not in the local store"))
    }
}

#[derive(Debug)]
pub enum ImportStatus {
    Progress { blocks: usize, bytes: u64 },
    Completed { roots: Vec<Cid>, blocks: usize },
    Failed { error: Error },
}

pub struct CarImport<R> {
    dag: IpldDag,
    reader: Option<R>,
    pin: bool,
    inner: Option<BoxStream<'static, ImportStatus>>,
}

impl<R> CarImport<R>
where
    R: AsyncRead + Unpin + Send + 'static,
{
    /// Recursively pins each root after a successful import.
    pub fn pin_roots(mut self) -> Self {
        self.pin = true;
        self
    }

    fn stream(&mut self) -> &mut BoxStream<'static, ImportStatus> {
        if self.inner.is_none() {
            let reader = self.reader.take().expect("CarImport already consumed");
            self.inner = Some(import_status_stream(self.dag.clone(), reader, self.pin).boxed());
        }
        self.inner.as_mut().expect("just initialized")
    }
}

impl<R> Stream for CarImport<R>
where
    R: AsyncRead + Unpin + Send + 'static,
{
    type Item = ImportStatus;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.get_mut().stream().poll_next_unpin(cx)
    }
}

impl<R> IntoFuture for CarImport<R>
where
    R: AsyncRead + Unpin + Send + 'static,
{
    type Output = Result<Vec<Cid>, Error>;
    type IntoFuture = BoxFuture<'static, Result<Vec<Cid>, Error>>;

    fn into_future(self) -> Self::IntoFuture {
        let CarImport {
            dag, reader, pin, ..
        } = self;
        let reader = reader.expect("CarImport already consumed");
        async move {
            let stream = import_status_stream(dag, reader, pin);
            futures::pin_mut!(stream);
            let mut roots = None;
            while let Some(status) = stream.next().await {
                match status {
                    ImportStatus::Completed { roots: done, .. } => roots = Some(done),
                    ImportStatus::Failed { error } => return Err(error),
                    ImportStatus::Progress { .. } => {}
                }
            }
            roots.ok_or_else(|| anyhow!("import produced no result"))
        }
        .boxed()
    }
}

fn import_status_stream<R>(dag: IpldDag, reader: R, pin: bool) -> impl Stream<Item = ImportStatus>
where
    R: AsyncRead + Unpin,
{
    async_stream::stream! {
        let mut reader = BufReader::new(reader);
        let (roots, limit) = match read_car_header(&mut reader).await {
            Ok(value) => value,
            Err(error) => {
                yield ImportStatus::Failed { error };
                return;
            }
        };

        let mut block_reader = (&mut reader).take(limit);
        let mut batch = Vec::with_capacity(IMPORT_BATCH);
        let mut blocks = 0usize;
        let mut bytes = 0u64;

        loop {
            let body = match read_frame(&mut block_reader, MAX_SECTION_LEN).await {
                Ok(Some(body)) => body,
                Ok(None) => break,
                Err(error) => {
                    yield ImportStatus::Failed { error };
                    return;
                }
            };
            let mut cursor = std::io::Cursor::new(&body[..]);
            let cid = match Cid::read_bytes(&mut cursor) {
                Ok(cid) => cid,
                Err(e) => {
                    yield ImportStatus::Failed { error: anyhow!("malformed cid in CAR: {e}") };
                    return;
                }
            };
            let data = body[cursor.position() as usize..].to_vec();
            let block = match make_block(cid, data) {
                Ok(block) => block,
                Err(error) => {
                    yield ImportStatus::Failed { error };
                    return;
                }
            };
            blocks += 1;
            bytes += body.len() as u64;
            batch.push(block);
            if batch.len() >= IMPORT_BATCH {
                if let Err(error) = dag.repo.put_blocks(std::mem::take(&mut batch)).await {
                    yield ImportStatus::Failed { error };
                    return;
                }
                yield ImportStatus::Progress { blocks, bytes };
            }
        }

        if !batch.is_empty()
            && let Err(error) = dag.repo.put_blocks(batch).await
        {
            yield ImportStatus::Failed { error };
            return;
        }

        if limit != u64::MAX && block_reader.limit() != 0 {
            yield ImportStatus::Failed {
                error: anyhow!("CARv2: inner archive shorter than its declared data size"),
            };
            return;
        }

        if pin {
            for root in &roots {
                match dag.repo.is_pinned(root).await {
                    Ok(true) => {}
                    Ok(false) => {
                        if let Err(error) = dag.repo.pin(*root).recursive().local().await {
                            yield ImportStatus::Failed { error };
                            return;
                        }
                    }
                    Err(error) => {
                        yield ImportStatus::Failed { error };
                        return;
                    }
                }
            }
        }

        yield ImportStatus::Progress { blocks, bytes };
        yield ImportStatus::Completed { roots, blocks };
    }
}

async fn read_car_header<R>(reader: &mut BufReader<R>) -> Result<(Vec<Cid>, u64), Error>
where
    R: AsyncRead + Unpin,
{
    let header = read_frame(reader, MAX_HEADER_LEN)
        .await?
        .ok_or_else(|| anyhow!("empty CAR: no header"))?;

    match parse_header(&header)? {
        Header::V1 { roots } => Ok((roots, u64::MAX)),
        Header::V2Pragma => {
            if header != CAR_V2_PRAGMA[1..] {
                return Err(anyhow!("non-canonical CARv2 pragma"));
            }
            let v2 = read_exact_vec(reader, V2_HEADER_LEN).await?;
            let data_offset = u64::from_le_bytes(v2[16..24].try_into().expect("8 bytes"));
            let data_size = u64::from_le_bytes(v2[24..32].try_into().expect("8 bytes"));
            let consumed = (CAR_V2_PRAGMA.len() + V2_HEADER_LEN) as u64;
            if data_offset < consumed {
                return Err(anyhow!(
                    "malformed CARv2: data offset {data_offset} precedes the header"
                ));
            }
            skip(reader, data_offset - consumed).await?;
            let mut inner = (&mut *reader).take(data_size);
            let inner_header = read_frame(&mut inner, MAX_HEADER_LEN)
                .await?
                .ok_or_else(|| anyhow!("CARv2: empty inner archive"))?;
            let roots = match parse_header(&inner_header)? {
                Header::V1 { roots } => roots,
                Header::V2Pragma => return Err(anyhow!("nested CARv2 is not supported")),
            };
            Ok((roots, inner.limit()))
        }
    }
}

/// Builds a block from a CAR section. Blocks whose multihash this build can recompute are verified
/// (rejecting a corrupt archive), an identity (0x00) block is checked against its data directly, and
/// any other hash function is trusted as the `(cid, bytes)` pairing.
fn make_block(cid: Cid, data: Vec<u8>) -> Result<Block, Error> {
    let code = cid.hash().code();
    if code == 0x00 {
        if cid.hash().digest() != data.as_slice() {
            return Err(anyhow!("identity block {cid} does not match its data"));
        }
        return Ok(Block::new_unchecked(cid, data));
    }
    if multihash_codetable::Code::try_from(code).is_err() {
        return Ok(Block::new_unchecked(cid, data));
    }
    Block::new(cid, data).map_err(|e| anyhow!("invalid block {cid} in CAR: {e}"))
}

enum Header {
    V1 { roots: Vec<Cid> },
    V2Pragma,
}

fn parse_header(bytes: &[u8]) -> Result<Header, Error> {
    let ipld: Ipld = serde_ipld_dagcbor::codec::DagCborCodec::decode_from_slice(bytes)
        .map_err(|e| anyhow!("invalid CAR header: {e}"))?;
    let Ipld::Map(map) = ipld else {
        return Err(anyhow!("CAR header is not a map"));
    };
    let version = match map.get("version") {
        Some(Ipld::Integer(v)) => *v,
        _ => return Err(anyhow!("CAR header has no version")),
    };
    match version {
        1 => {
            let roots = match map.get("roots") {
                Some(Ipld::List(list)) => list
                    .iter()
                    .map(|item| match item {
                        Ipld::Link(cid) => Ok(*cid),
                        _ => Err(anyhow!("CAR root is not a cid")),
                    })
                    .collect::<Result<Vec<_>, _>>()?,
                Some(_) => return Err(anyhow!("CAR roots is not a list")),
                None => Vec::new(),
            };
            Ok(Header::V1 { roots })
        }
        2 => Ok(Header::V2Pragma),
        other => Err(anyhow!("unsupported CAR version {other}")),
    }
}

fn encode_v1_header(roots: &[Cid]) -> Result<Vec<u8>, Error> {
    let mut map = BTreeMap::new();
    map.insert(
        "roots".to_string(),
        Ipld::List(roots.iter().map(|c| Ipld::Link(*c)).collect()),
    );
    map.insert("version".to_string(), Ipld::Integer(1));
    serde_ipld_dagcbor::codec::DagCborCodec::encode_to_vec(&Ipld::Map(map))
        .map_err(|e| anyhow!("encoding CAR header failed: {e}"))
}

fn frame(payload: &[u8]) -> Bytes {
    let mut buf = Vec::with_capacity(payload.len() + 10);
    let mut varint = unsigned_varint::encode::u64_buffer();
    buf.extend_from_slice(unsigned_varint::encode::u64(
        payload.len() as u64,
        &mut varint,
    ));
    buf.extend_from_slice(payload);
    Bytes::from(buf)
}

async fn read_uvarint<R>(reader: &mut R) -> Result<Option<u64>, Error>
where
    R: AsyncRead + Unpin,
{
    let mut value = 0u64;
    let mut shift = 0u32;
    loop {
        let mut byte = [0u8; 1];
        if reader.read(&mut byte).await? == 0 {
            if shift == 0 {
                return Ok(None);
            }
            return Err(anyhow!("unexpected end of CAR in a varint"));
        }
        // the 10th byte (shift 63) may only carry the single top bit, with no continuation
        if shift == 63 && byte[0] > 0x01 {
            return Err(anyhow!("CAR varint overflows u64"));
        }
        value |= u64::from(byte[0] & 0x7f) << shift;
        if byte[0] & 0x80 == 0 {
            return Ok(Some(value));
        }
        shift += 7;
        if shift >= 64 {
            return Err(anyhow!("CAR varint overflows u64"));
        }
    }
}

async fn read_exact_vec<R>(reader: &mut R, n: usize) -> Result<Vec<u8>, Error>
where
    R: AsyncRead + Unpin,
{
    let mut buf = vec![0u8; n];
    reader.read_exact(&mut buf).await?;
    Ok(buf)
}

/// Reads one length-prefixed frame, capping the declared length at `max` before allocating.
/// Returns `None` at a clean end-of-stream boundary.
async fn read_frame<R>(reader: &mut R, max: u64) -> Result<Option<Vec<u8>>, Error>
where
    R: AsyncRead + Unpin,
{
    let Some(len) = read_uvarint(reader).await? else {
        return Ok(None);
    };
    if len > max {
        return Err(anyhow!(
            "CAR frame length {len} exceeds the {max}-byte limit"
        ));
    }
    Ok(Some(read_exact_vec(reader, len as usize).await?))
}

async fn skip<R>(reader: &mut R, mut n: u64) -> Result<(), Error>
where
    R: AsyncRead + Unpin,
{
    let mut scratch = [0u8; 4096];
    while n > 0 {
        let take = n.min(scratch.len() as u64) as usize;
        reader.read_exact(&mut scratch[..take]).await?;
        n -= take as u64;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repo::Repo;

    async fn new_dag() -> (Repo<crate::repo::DefaultStorage>, IpldDag) {
        let repo = Repo::new_memory();
        repo.init().await.unwrap();
        let dag = IpldDag::from(repo.clone());
        (repo, dag)
    }

    async fn sample_dag(dag: &IpldDag) -> (Cid, Cid) {
        let leaf = dag.put_dag("hello car").await.unwrap();
        let mut map = BTreeMap::new();
        map.insert("child".to_string(), Ipld::Link(leaf));
        let root = dag.put_dag(Ipld::Map(map)).await.unwrap();
        (root, leaf)
    }

    #[tokio::test]
    async fn export_import_roundtrip() {
        let (_repo, dag) = new_dag().await;
        let (root, leaf) = sample_dag(&dag).await;

        let car = dag.export(root).await.unwrap();
        assert!(!car.is_empty());

        let (dest_repo, dest) = new_dag().await;
        let roots = dest
            .import(futures::io::Cursor::new(car.clone()))
            .await
            .unwrap();
        assert_eq!(roots, vec![root]);

        assert!(dest_repo.contains(&root).await.unwrap());
        assert!(dest_repo.contains(&leaf).await.unwrap());
        assert_eq!(
            dest.get_dag(leaf).await.unwrap(),
            Ipld::String("hello car".into())
        );
    }

    #[tokio::test]
    async fn streamed_export_matches_collected() {
        let (_repo, dag) = new_dag().await;
        let (root, _) = sample_dag(&dag).await;

        let collected = dag.export(root).await.unwrap();

        let mut streamed = Vec::new();
        let export = dag.export(root);
        futures::pin_mut!(export);
        while let Some(chunk) = export.next().await {
            streamed.extend_from_slice(&chunk.unwrap());
        }
        assert_eq!(streamed, collected);
    }

    #[tokio::test]
    async fn export_missing_block_errors() {
        let (_repo, dag) = new_dag().await;
        let absent =
            Cid::try_from("bafyreib2rxk3rybk3aobmv5cjuql3bm2twh4jo5uxgt5e7ym74n5lvrxle").unwrap();
        assert!(dag.export(absent).await.is_err());
    }

    #[tokio::test]
    async fn import_pins_roots_when_requested() {
        let (_repo, dag) = new_dag().await;
        let (root, leaf) = sample_dag(&dag).await;
        let car = dag.export(root).await.unwrap();

        let (dest_repo, dest) = new_dag().await;
        dest.import(futures::io::Cursor::new(car.clone()))
            .pin_roots()
            .await
            .unwrap();
        assert!(dest_repo.is_pinned(&root).await.unwrap());
        assert!(
            dest_repo.is_pinned(&leaf).await.unwrap(),
            "recursive pin reaches children"
        );
    }

    #[tokio::test]
    async fn imports_carv2_wrapping_a_v1_payload() {
        let (_repo, dag) = new_dag().await;
        let (root, leaf) = sample_dag(&dag).await;
        let v1 = dag.export(root).await.unwrap();

        // wrap the v1 payload in a minimal CARv2: pragma, 40-byte header, then the payload (no index)
        let mut v2 = Vec::new();
        v2.extend_from_slice(&CAR_V2_PRAGMA);
        v2.extend_from_slice(&[0u8; 16]); // characteristics
        let data_offset = (CAR_V2_PRAGMA.len() + V2_HEADER_LEN) as u64;
        v2.extend_from_slice(&data_offset.to_le_bytes());
        v2.extend_from_slice(&(v1.len() as u64).to_le_bytes());
        v2.extend_from_slice(&0u64.to_le_bytes()); // index offset (none)
        v2.extend_from_slice(&v1);

        let (dest_repo, dest) = new_dag().await;
        let roots = dest
            .import(futures::io::Cursor::new(v2.clone()))
            .await
            .unwrap();
        assert_eq!(roots, vec![root]);
        assert!(dest_repo.contains(&leaf).await.unwrap());
    }

    #[tokio::test]
    async fn import_rejects_a_corrupt_block() {
        let (_repo, dag) = new_dag().await;
        let (root, _) = sample_dag(&dag).await;
        let mut car = dag.export(root).await.unwrap().to_vec();
        // flip a byte in the last block's payload so its cid no longer matches its data
        *car.last_mut().unwrap() ^= 0xff;

        let (_dest_repo, dest) = new_dag().await;
        assert!(
            dest.import(futures::io::Cursor::new(car.clone()))
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn exports_unknown_codec_block_as_leaf() {
        use multihash_codetable::{Code, MultihashDigest};
        let (repo, dag) = new_dag().await;
        let opaque = b"bytes under a codec we cannot decode";
        let unk = Cid::new_v1(0x0200, Code::Sha2_256.digest(opaque));
        repo.put_block(&Block::new(unk, opaque.to_vec()).unwrap())
            .await
            .unwrap();

        let mut map = BTreeMap::new();
        map.insert("opaque".to_string(), Ipld::Link(unk));
        let root = dag.put_dag(Ipld::Map(map)).await.unwrap();

        // export must archive the undecodable child as a leaf, not abort
        let car = dag.export(root).await.unwrap();
        let (dest_repo, dest) = new_dag().await;
        let roots = dest.import(futures::io::Cursor::new(car)).await.unwrap();
        assert_eq!(roots, vec![root]);
        assert!(dest_repo.contains(&unk).await.unwrap());
    }

    #[tokio::test]
    async fn imports_identity_hashed_block() {
        use ipld_core::cid::multihash::Multihash;
        let data = b"inline identity payload";
        let cid = Cid::new_v1(0x55, Multihash::wrap(0x00, data).unwrap());

        let mut payload = cid.to_bytes();
        payload.extend_from_slice(data);
        let mut car = Vec::new();
        car.extend_from_slice(&frame(&encode_v1_header(&[cid]).unwrap()));
        car.extend_from_slice(&frame(&payload));

        let (dest_repo, dest) = new_dag().await;
        let roots = dest.import(futures::io::Cursor::new(car)).await.unwrap();
        assert_eq!(roots, vec![cid]);
        assert!(dest_repo.contains(&cid).await.unwrap());
    }

    #[tokio::test]
    async fn import_rejects_oversized_frame() {
        let mut car = Vec::new();
        car.extend_from_slice(&frame(&encode_v1_header(&[]).unwrap()));
        let mut vbuf = unsigned_varint::encode::u64_buffer();
        car.extend_from_slice(unsigned_varint::encode::u64(MAX_SECTION_LEN + 1, &mut vbuf));

        let (_r, dest) = new_dag().await;
        assert!(dest.import(futures::io::Cursor::new(car)).await.is_err());
    }

    #[tokio::test]
    async fn pin_roots_is_idempotent_across_imports() {
        let (_repo, dag) = new_dag().await;
        let (root, _) = sample_dag(&dag).await;
        let car = dag.export(root).await.unwrap();

        let (dest_repo, dest) = new_dag().await;
        dest.import(futures::io::Cursor::new(car.clone()))
            .pin_roots()
            .await
            .unwrap();
        dest.import(futures::io::Cursor::new(car.clone()))
            .pin_roots()
            .await
            .unwrap();
        assert!(dest_repo.is_pinned(&root).await.unwrap());
    }

    #[tokio::test]
    async fn import_reports_progress() {
        let (_repo, dag) = new_dag().await;
        let mut links = BTreeMap::new();
        for i in 0..300u32 {
            let leaf = dag.put_dag(format!("leaf {i}")).await.unwrap();
            links.insert(format!("l{i:03}"), Ipld::Link(leaf));
        }
        let root = dag.put_dag(Ipld::Map(links)).await.unwrap();
        let car = dag.export(root).await.unwrap();

        let (dest_repo, dest) = new_dag().await;
        let mut import = dest.import(futures::io::Cursor::new(car));
        let mut last = 0usize;
        let mut progress_events = 0usize;
        let mut completed = None;
        while let Some(status) = import.next().await {
            match status {
                ImportStatus::Progress { blocks, bytes } => {
                    assert!(blocks >= last);
                    assert!(bytes > 0);
                    last = blocks;
                    progress_events += 1;
                }
                ImportStatus::Completed { roots, blocks } => completed = Some((roots, blocks)),
                ImportStatus::Failed { error } => panic!("import failed: {error}"),
            }
        }

        let (roots, blocks) = completed.unwrap();
        assert_eq!(roots, vec![root]);
        assert_eq!(blocks, 301);
        assert_eq!(last, 301);
        assert!(
            progress_events >= 2,
            "batched flush plus a final progress, got {progress_events}"
        );
        assert!(dest_repo.contains(&root).await.unwrap());
    }
}
