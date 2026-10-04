//! `refs` or the references of dag-pb and other supported IPLD formats functionality.

use crate::block::BlockCodec;
use crate::repo::{GCGuard, Repo, RepoTypes};
use async_stream::stream;
use connexa::prelude::identity::PeerId;
use futures::stream::Stream;
use ipld_core::{cid::Cid, ipld::Ipld};
use std::borrow::Borrow;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::fmt;
use std::time::Duration;

/// Represents a single link in an IPLD tree encountered during a `refs` walk.
#[derive(Clone, PartialEq, Eq)]
pub struct Edge {
    /// Source document which links to [`Edge::destination`]
    pub source: Cid,
    /// The destination document
    pub destination: Cid,
    /// The name of the link, in case of dag-pb
    pub name: Option<String>,
}

impl fmt::Debug for Edge {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            fmt,
            "Edge {{ source: {}, destination: {}, name: {:?} }}",
            self.source, self.destination, self.name
        )
    }
}

#[derive(Debug, thiserror::Error)]
pub enum IpldRefsError {
    #[error("loading failed")]
    Loading(#[from] crate::Error),
    #[error("block not found locally: {}", .0)]
    BlockNotFound(Cid),
}

pub(crate) struct IpldRefs {
    max_depth: Option<u64>,
    unique: bool,
    download_blocks: bool,
    exit_on_error: bool,
    providers: Vec<PeerId>,
    timeout: Option<Duration>,
    gc_guard: Option<GCGuard>,
}

impl Default for IpldRefs {
    fn default() -> Self {
        IpldRefs {
            max_depth: None, // unlimited
            unique: false,
            download_blocks: true,
            exit_on_error: false,
            providers: vec![],
            timeout: None,
            gc_guard: None,
        }
    }
}

impl IpldRefs {
    pub(crate) fn with_gc_guard(mut self, guard: GCGuard) -> Self {
        self.gc_guard = Some(guard);
        self
    }

    /// Overrides the default maximum depth of "unlimited" with the given maximum depth. Zero is
    /// allowed and will result in an empty stream.
    #[allow(dead_code)]
    pub fn with_max_depth(mut self, depth: u64) -> IpldRefs {
        self.max_depth = Some(depth);
        self
    }

    /// Overrides the default of returning all links by supressing the links which have already
    /// been reported once.
    pub fn with_only_unique(mut self) -> IpldRefs {
        self.unique = true;
        self
    }

    /// Overrides the default of allowing the refs operation to fetch blocks. Useful at least
    /// internally in rust-ipfs to implement pinning recursively. This changes the stream's
    /// behaviour to stop on first block which is not found locally.
    pub fn with_existing_blocks(mut self) -> IpldRefs {
        self.download_blocks = false;
        self
    }

    /// Duration to fetch the block from the network before
    /// timing out
    pub fn with_timeout(mut self, duration: Duration) -> IpldRefs {
        self.timeout = Some(duration);
        self
    }

    /// List of peers that may contain the block
    pub fn providers(mut self, providers: &[PeerId]) -> Self {
        self.providers = providers.into();
        self
    }

    /// Yield an error to exit the stream than to continue
    #[allow(dead_code)]
    pub fn with_exit_on_error(mut self) -> IpldRefs {
        self.exit_on_error = true;
        self
    }

    pub fn refs_of_resolved<'a, S, MaybeOwned, Iter>(
        self,
        repo: MaybeOwned,
        iplds: Iter,
    ) -> impl Stream<Item = Result<Edge, IpldRefsError>> + Send + 'a
    where
        S: RepoTypes,
        MaybeOwned: Borrow<Repo<S>> + Send + 'a,
        Iter: IntoIterator<Item = (Cid, Ipld)> + Send + 'a,
    {
        iplds_refs_inner(repo, iplds, self)
    }
}

/// Gather links as edges between two documents from all of the `iplds` which represent the
/// document and it's original `Cid`, as the `Ipld` can be a subtree of the document.
///
/// This stream does not stop on **error**.
///
/// # Differences from other implementations
///
/// `js-ipfs` does seem to do a recursive descent on all links. Looking at the tests it would
/// appear that `go-ipfs` implements this in similar fashion. This implementation is breadth-first
/// to be simpler at least.
///
/// Related: https://github.com/ipfs/js-ipfs/pull/2982
///
/// # Lifetime of returned stream
///
/// Depending on how this function is called, the lifetime will be tied to the lifetime of given
/// `&Ipfs` or `'static` when given ownership of `Ipfs`.
pub fn iplds_refs<'a, S, MaybeOwned, Iter>(
    repo: MaybeOwned,
    iplds: Iter,
    max_depth: Option<u64>,
    unique: bool,
) -> impl Stream<Item = Result<Edge, anyhow::Error>> + Send + 'a
where
    S: RepoTypes,
    MaybeOwned: Borrow<Repo<S>> + Send + 'a,
    Iter: IntoIterator<Item = (Cid, Ipld)> + Send + 'a,
{
    use futures::stream::TryStreamExt;
    let opts = IpldRefs {
        max_depth,
        unique,
        download_blocks: true,
        timeout: None,
        providers: vec![],
        exit_on_error: true,
        gc_guard: None,
    };
    iplds_refs_inner(repo, iplds, opts).map_err(|e| match e {
        IpldRefsError::Loading(e) => e,
        x => unreachable!(
            "iplds_refs_inner should not return other errors for download_blocks: false; {}",
            x
        ),
    })
}

fn iplds_refs_inner<'a, S, MaybeOwned, Iter>(
    repo: MaybeOwned,
    iplds: Iter,
    opts: IpldRefs,
) -> impl Stream<Item = Result<Edge, IpldRefsError>> + Send + 'a
where
    S: RepoTypes,
    MaybeOwned: Borrow<Repo<S>> + Send + 'a,
    Iter: IntoIterator<Item = (Cid, Ipld)>,
{
    let mut work = VecDeque::new();
    let mut queued_or_visited = HashSet::new();

    let IpldRefs {
        max_depth,
        unique,
        download_blocks,
        timeout,
        exit_on_error,
        providers,
        gc_guard,
    } = opts;

    let empty_stream = max_depth.map(|n| n == 0).unwrap_or(false);

    // double check the max_depth before filling the work and queued_or_visited up just in case we
    // are going to be returning an empty stream
    if !empty_stream {
        // not building these before moving the work and hashset into the stream would impose
        // apparently impossible bounds on `Iter`, in addition to `Send + 'a`.
        for (origin, ipld) in iplds {
            for (link_name, next_cid) in ipld_links(&origin, ipld) {
                if unique && !queued_or_visited.insert(next_cid) {
                    trace!("skipping already queued {}", next_cid);
                    continue;
                }
                work.push_back((0, next_cid, origin, link_name));
            }
        }
    }

    stream! {
        if empty_stream || work.is_empty() {
            return;
        }

        let borrowed = repo.borrow();
        let guard = match gc_guard {
            Some(guard) => guard,
            None => borrowed.gc_guard().await,
        };

        while let Some((depth, cid, source, link_name)) = work.pop_front() {
            let traverse_links = match max_depth {
                Some(d) if d <= depth => {
                    // important to continue instead of stopping
                    continue;
                },
                // no need to list links which would be filtered out
                Some(d) if d + 1 == depth => false,
                _ => true
            };

            let block = if download_blocks {
                match borrowed.get_block(cid).with_gc_guard(guard.clone()).providers(&providers).set_local(!download_blocks).timeout(timeout).await {
                    Ok(block) => block,
                    Err(e) => {
                        warn!("failed to load {}, linked from {}: {}", cid, source, e);
                        if exit_on_error {
                            yield Err(IpldRefsError::from(e));
                            return;
                        }
                        continue;
                    }
                }
            } else {
                match borrowed.get_block_now(&cid).await {
                    Ok(Some(block)) => block,
                    Ok(None) => {
                        if exit_on_error {
                            yield Err(IpldRefsError::BlockNotFound(cid.to_owned()));
                            return;
                        }
                        continue;
                    }
                    Err(e) => {
                        if exit_on_error {
                            yield Err(IpldRefsError::from(e));
                            return;
                        }
                        continue;
                    }
                }
            };

            trace!(cid = %cid, "loaded next");

            let ipld = match block.to_ipld() {
                Ok(ipld) => ipld,
                Err(e) => {
                    warn!(cid = %cid, source = %cid, "failed to parse: {}", e);
                    // go-ipfs on raw Qm hash:
                    // > failed to decode Protocol Buffers: incorrectly formatted merkledag node: unmarshal failed. proto: illegal wireType 6
                    yield Err(anyhow::Error::from(e).into());
                    continue;
                }
            };

            if traverse_links {
                for (link_name, next_cid) in ipld_links(&cid, ipld) {
                    if unique && !queued_or_visited.insert(next_cid) {
                        trace!(queued = %next_cid, "skipping already queued");
                        continue;
                    }

                    work.push_back((depth + 1, next_cid, cid, link_name));
                }
            }

            yield Ok(Edge { source, destination: cid, name: link_name });
        }
    }
}

fn ipld_links(
    cid: &Cid,
    ipld: Ipld,
) -> impl Iterator<Item = (Option<String>, Cid)> + Send + 'static {
    // a wrapping iterator without there being a libipld_base::IpldIntoIter might not be doable
    // with safe code
    let items = if cid.codec() == <BlockCodec as Into<u64>>::into(BlockCodec::DagPb) {
        dagpb_links(ipld)
    } else {
        ipld.iter()
            .filter_map(|val| match val {
                Ipld::Link(cid) => Some(cid),
                _ => None,
            })
            .cloned()
            // only dag-pb ever has any link names, probably because in cbor the "name" on the LHS
            // might have a different meaning from a "link name" in dag-pb ... Doesn't seem
            // immediatedly obvious why this is done.
            .map(|cid| (None, cid))
            .collect::<Vec<(Option<String>, Cid)>>()
    };

    items.into_iter()
}

/// Special handling for the structure created while loading dag-pb as ipld.
///
/// # Panics
///
/// If the dag-pb ipld tree doesn't conform to expectations, as in, we are out of sync with the
/// libipld crate. This is on purpose.
fn dagpb_links(ipld: Ipld) -> Vec<(Option<String>, Cid)> {
    let links = match ipld {
        Ipld::Map(mut m) => m.remove("Links"),
        // lets assume this means "no links"
        _ => return Vec::new(),
    };

    let links = match links {
        Some(Ipld::List(v)) => v,
        x => panic!("Expected dag-pb2ipld \"Links\" to be a list, got: {x:?}"),
    };

    links
        .into_iter()
        .enumerate()
        .filter_map(|(i, ipld)| {
            match ipld {
                Ipld::Map(mut m) => {
                    let link = match m.remove("Hash") {
                        Some(Ipld::Link(cid)) => cid,
                        Some(x) => panic!(
                            "Expected dag-pb2ipld \"Links[{i}]/Hash\" to be a link, got: {x:?}"
                        ),
                        None => return None,
                    };
                    let name = match m.remove("Name") {
                        // not sure of this, not covered by tests, though these are only
                        // present for multi-block files so maybe it's better to panic
                        Some(Ipld::String(s)) if s == "/" => {
                            unimplemented!("Slashes as the name of link")
                        }
                        Some(Ipld::String(s)) => Some(s),
                        Some(x) => panic!(
                            "Expected dag-pb2ipld \"Links[{i}]/Name\" to be a string, got: {x:?}"
                        ),
                        // not too sure of this, this could be the index as string as well?
                        None => unimplemented!(
                            "Default name for dag-pb2ipld links, should it be index?"
                        ),
                    };

                    Some((name, link))
                }
                x => panic!("Expected dag-pb2ipld \"Links[{i}]\" to be a map, got: {x:?}"),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{Edge, ipld_links, iplds_refs};
    use crate::{Block, Node};
    use futures::stream::TryStreamExt;
    use hex_literal::hex;
    use ipld_core::cid::Cid;
    use std::collections::HashSet;
    use std::convert::TryFrom;

    #[test]
    fn dagpb_links() {
        // this is the same as in ipfs-http::v0::refs::path::tests::walk_dagpb_links
        let payload = hex!(
            "12330a2212206aad27d7e2fc815cd15bf679535062565dc927a831547281
            fc0af9e5d7e67c74120b6166726963616e2e747874180812340a221220fd
            36ac5279964db0cba8f7fa45f8c4c44ef5e2ff55da85936a378c96c9c632
            04120c616d6572696361732e747874180812360a2212207564c20415869d
            77a8a40ca68a9158e397dd48bdff1325cdb23c5bcd181acd17120e617573
            7472616c69616e2e7478741808"
        );

        let cid = Cid::try_from("QmbrFTo4s6H23W6wmoZKQC2vSogGeQ4dYiceSqJddzrKVa").unwrap();

        let decoded = Block::new(cid, payload.to_vec())
            .unwrap()
            .to_ipld()
            .unwrap();

        let links = ipld_links(&cid, decoded)
            .map(|(name, _)| name.unwrap())
            .collect::<Vec<_>>();

        assert_eq!(links, ["african.txt", "americas.txt", "australian.txt",]);
    }

    #[tokio::test]
    async fn all_refs_from_root() {
        let Node { ipfs, .. } = preloaded_testing_ipfs().await;

        let (root, dag0, unixfs0, dag1, unixfs1) = (
            // this is the dag with content: [dag0, unixfs0, dag1, unixfs1]
            "bafyreihpc3vupfos5yqnlakgpjxtyx3smkg26ft7e2jnqf3qkyhromhb64",
            // {foo: dag1, bar: unixfs0}
            "bafyreidquig3arts3bmee53rutt463hdyu6ff4zeas2etf2h2oh4dfms44",
            "QmPJ4A6Su27ABvvduX78x2qdWMzkdAYxqeH5TVrHeo3xyy",
            // {foo: unixfs1}
            "bafyreibvjvcv745gig4mvqs4hctx4zfkono4rjejm2ta6gtyzkqxfjeily",
            "QmRgutAxd8t7oGkSm4wmeuByG6M51wcTso6cubDdQtuEfL",
        );

        let root_block = ipfs.get_block(Cid::try_from(root).unwrap()).await.unwrap();
        let ipld = root_block.to_ipld().unwrap();

        let all_edges: Vec<_> =
            iplds_refs(ipfs.repo(), vec![(*root_block.cid(), ipld)], None, false)
                .map_ok(
                    |Edge {
                         source,
                         destination,
                         ..
                     }| (source.to_string(), destination.to_string()),
                )
                .try_collect()
                .await
                .unwrap();

        // not sure why go-ipfs outputs this order, this is more like dfs?
        let expected = [
            (root, dag0),
            (dag0, unixfs0),
            (dag0, dag1),
            (dag1, unixfs1),
            (root, unixfs0),
            (root, dag1),
            (dag1, unixfs1),
            (root, unixfs1),
        ];

        println!("found edges:\n{all_edges:#?}");

        assert_edges(&expected, all_edges.as_slice());
    }

    #[tokio::test]
    async fn all_unique_refs_from_root() {
        let Node { ipfs, .. } = preloaded_testing_ipfs().await;

        let (root, dag0, unixfs0, dag1, unixfs1) = (
            // this is the dag with content: [dag0, unixfs0, dag1, unixfs1]
            "bafyreihpc3vupfos5yqnlakgpjxtyx3smkg26ft7e2jnqf3qkyhromhb64",
            // {foo: dag1, bar: unixfs0}
            "bafyreidquig3arts3bmee53rutt463hdyu6ff4zeas2etf2h2oh4dfms44",
            "QmPJ4A6Su27ABvvduX78x2qdWMzkdAYxqeH5TVrHeo3xyy",
            // {foo: unixfs1}
            "bafyreibvjvcv745gig4mvqs4hctx4zfkono4rjejm2ta6gtyzkqxfjeily",
            "QmRgutAxd8t7oGkSm4wmeuByG6M51wcTso6cubDdQtuEfL",
        );

        let root_block = ipfs.get_block(Cid::try_from(root).unwrap()).await.unwrap();
        let ipld = root_block.to_ipld().unwrap();

        let destinations: HashSet<_> =
            iplds_refs(ipfs.repo(), vec![(*root_block.cid(), ipld)], None, true)
                .map_ok(|Edge { destination, .. }| destination.to_string())
                .try_collect()
                .await
                .unwrap();

        // go-ipfs output:
        // bafyreihpc3vupfos5yqnlakgpjxtyx3smkg26ft7e2jnqf3qkyhromhb64 -> bafyreidquig3arts3bmee53rutt463hdyu6ff4zeas2etf2h2oh4dfms44
        // bafyreihpc3vupfos5yqnlakgpjxtyx3smkg26ft7e2jnqf3qkyhromhb64 -> QmPJ4A6Su27ABvvduX78x2qdWMzkdAYxqeH5TVrHeo3xyy
        // bafyreihpc3vupfos5yqnlakgpjxtyx3smkg26ft7e2jnqf3qkyhromhb64 -> bafyreibvjvcv745gig4mvqs4hctx4zfkono4rjejm2ta6gtyzkqxfjeily
        // bafyreihpc3vupfos5yqnlakgpjxtyx3smkg26ft7e2jnqf3qkyhromhb64 -> QmRgutAxd8t7oGkSm4wmeuByG6M51wcTso6cubDdQtuEfL

        let expected = [dag0, unixfs0, dag1, unixfs1]
            .iter()
            .map(|&s| String::from(s))
            .collect::<HashSet<_>>();

        let diff = destinations
            .symmetric_difference(&expected)
            .map(|s| s.as_str())
            .collect::<Vec<&str>>();

        assert!(diff.is_empty(), "{diff:?}");
    }

    fn assert_edges(expected: &[(&str, &str)], actual: &[(String, String)]) {
        let expected: HashSet<_> = expected.iter().map(|&(a, b)| (a, b)).collect();

        let actual: HashSet<_> = actual
            .iter()
            .map(|(a, b)| (a.as_str(), b.as_str()))
            .collect();

        let diff: Vec<_> = expected.symmetric_difference(&actual).collect();

        assert!(diff.is_empty(), "{diff:#?}");
    }

    async fn preloaded_testing_ipfs() -> Node {
        let ipfs = Node::new("test_node").await;

        let blocks = [
            (
                // echo -n '{ "foo": { "/": "bafyreibvjvcv745gig4mvqs4hctx4zfkono4rjejm2ta6gtyzkqxfjeily" }, "bar": { "/": "QmPJ4A6Su27ABvvduX78x2qdWMzkdAYxqeH5TVrHeo3xyy" } }' | /ipfs dag put
                "bafyreidquig3arts3bmee53rutt463hdyu6ff4zeas2etf2h2oh4dfms44",
                &hex!(
                    "a263626172d82a58230012200e317512b6f9f86e015a154cb97a9ddcdc7e372cccceb3947921634953c6537463666f6fd82a58250001711220354d455ff3a641b8cac25c38a77e64aa735dc8a48966a60f1a78caa172a4885e"
                )[..],
            ),
            (
                // echo barfoo > file2 && ipfs add file2
                "QmPJ4A6Su27ABvvduX78x2qdWMzkdAYxqeH5TVrHeo3xyy",
                &hex!("0a0d08021207626172666f6f0a1807")[..],
            ),
            (
                // echo -n '{ "foo": { "/": "QmRgutAxd8t7oGkSm4wmeuByG6M51wcTso6cubDdQtuEfL" } }' | ipfs dag put
                "bafyreibvjvcv745gig4mvqs4hctx4zfkono4rjejm2ta6gtyzkqxfjeily",
                &hex!(
                    "a163666f6fd82a582300122031c3d57080d8463a3c63b2923df5a1d40ad7a73eae5a14af584213e5f504ac33"
                )[..],
            ),
            (
                // echo foobar > file1 && ipfs add file1
                "QmRgutAxd8t7oGkSm4wmeuByG6M51wcTso6cubDdQtuEfL",
                &hex!("0a0d08021207666f6f6261720a1807")[..],
            ),
            (
                // echo -e '[{"/":"bafyreidquig3arts3bmee53rutt463hdyu6ff4zeas2etf2h2oh4dfms44"},{"/":"QmPJ4A6Su27ABvvduX78x2qdWMzkdAYxqeH5TVrHeo3xyy"},{"/":"bafyreibvjvcv745gig4mvqs4hctx4zfkono4rjejm2ta6gtyzkqxfjeily"},{"/":"QmRgutAxd8t7oGkSm4wmeuByG6M51wcTso6cubDdQtuEfL"}]' | ./ipfs dag put
                "bafyreihpc3vupfos5yqnlakgpjxtyx3smkg26ft7e2jnqf3qkyhromhb64",
                &hex!(
                    "84d82a5825000171122070a20db04672d858427771a4e7cf6ce3c53c52f32404b4499747d38fc19592e7d82a58230012200e317512b6f9f86e015a154cb97a9ddcdc7e372cccceb3947921634953c65374d82a58250001711220354d455ff3a641b8cac25c38a77e64aa735dc8a48966a60f1a78caa172a4885ed82a582300122031c3d57080d8463a3c63b2923df5a1d40ad7a73eae5a14af584213e5f504ac33"
                )[..],
            ),
        ];

        for (cid_str, data) in blocks.iter() {
            let cid = Cid::try_from(*cid_str).unwrap();
            let block = Block::new(cid, data.to_vec()).unwrap();
            block.to_ipld().unwrap();
            ipfs.put_block(&block).await.unwrap();
        }

        ipfs
    }
}
