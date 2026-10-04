use std::{
    collections::HashSet,
    ffi::{OsStr, OsString},
    fmt::Debug,
    hash::Hash,
    path::PathBuf,
    sync::{Arc, atomic::Ordering},
};

use std::sync::{RwLock, atomic::AtomicU64};


use crate::{inode_mapper, types::*};
use crate::{inode_mapper::InodeMapper, inode_multi_mapper::*};

/// Trait to allow a FileIdType to be mapped to use a converter
pub trait InodeResolvable {
    type Resolver: FileIdResolver<ResolvedType = Self>;

    fn create_resolver() -> Self::Resolver;
}

impl InodeResolvable for PathBuf {
    type Resolver = PathResolver;

    fn create_resolver() -> Self::Resolver {
        PathResolver::new()
    }
}

impl InodeResolvable for Inode {
    type Resolver = InodeResolver;

    fn create_resolver() -> Self::Resolver {
        InodeResolver::new()
    }
}

impl InodeResolvable for Vec<OsString> {
    type Resolver = ComponentsResolver;

    fn create_resolver() -> Self::Resolver {
        ComponentsResolver::new()
    }
}

impl<BackingId> InodeResolvable for HybridId<BackingId>
where
    BackingId: Clone + Eq + Hash + Send + Sync + Debug + 'static,
{
    type Resolver = HybridResolver<BackingId>;

    fn create_resolver() -> Self::Resolver {
        HybridResolver::new()
    }
}

/// FileIdResolver
/// FileIdResolver handles its data behind Locks if needed and should not be nested inside a Mutex
pub trait FileIdResolver: Send + Sync + 'static {
    type ResolvedType: FileIdType;

    fn new() -> Self;
    fn resolve_id(&self, ino: Inode) -> Self::ResolvedType;
    fn lookup(
        &self,
        parent: Inode,
        child: &OsStr,
        id: <Self::ResolvedType as FileIdType>::_Id,
        increment: bool,
    ) -> Inode;
    fn add_children(
        &self,
        parent: Inode,
        children: Vec<(OsString, <Self::ResolvedType as FileIdType>::_Id)>,
        increment: bool,
    ) -> Vec<(OsString, Inode)>;
    fn forget(&self, ino: Inode, nlookup: u64);
    fn prune(&self, keep: &HashSet<Self::ResolvedType>);
    fn rename(&self, parent: Inode, name: &OsStr, newparent: Inode, newname: &OsStr);
}

pub struct InodeResolver {}

impl FileIdResolver for InodeResolver {
    type ResolvedType = Inode;

    fn new() -> Self {
        Self {}
    }

    fn resolve_id(&self, ino: Inode) -> Self::ResolvedType {
        ino
    }

    fn lookup(&self, _parent: Inode, _child: &OsStr, id: Inode, _increment: bool) -> Inode {
        id
    }

    fn add_children(
        &self,
        _parent: Inode,
        children: Vec<(OsString, Inode)>,
        _increment: bool,
    ) -> Vec<(OsString, Inode)> {
        children
    }

    fn forget(&self, _ino: Inode, _nlookup: u64) {}

    fn prune(&self, _keep: &HashSet<Self::ResolvedType>) {}

    fn rename(&self, _parent: Inode, _name: &OsStr, _newparent: Inode, _newname: &OsStr) {}
}

pub struct ComponentsResolver {
    mapper: RwLock<InodeMapper<AtomicU64>>,
}

impl FileIdResolver for ComponentsResolver {
    type ResolvedType = Vec<OsString>;

    fn new() -> Self {
        ComponentsResolver {
            mapper: RwLock::new(InodeMapper::new(AtomicU64::new(0))),
        }
    }

    fn resolve_id(&self, ino: Inode) -> Self::ResolvedType {
        self.mapper
            .read()
            .unwrap()
            .resolve(&ino)
            .expect("Failed to resolve inode")
            .iter()
            .map(|inode_info| (**inode_info.name).clone())
            .collect()
    }

    fn lookup(&self, parent: Inode, child: &OsStr, _id: (), increment: bool) -> Inode {
        {
            // Optimistically assume the child exists
            if let Some(lookup_result) = self.mapper.read().unwrap().lookup(&parent, child) {
                if increment {
                    lookup_result.data.fetch_add(1, Ordering::SeqCst);
                }
                return lookup_result.inode.clone();
            }
        }
        // This scenario happens if the child node does not exist or the backing ID does not match
        self.mapper
            .write()
            .expect("Failed to acquire write lock")
            .insert_child(&parent, child.to_os_string(), |_| {
                // If the child node already exists, use the existing reference count
                AtomicU64::new(if increment { 1 } else { 0 })
            })
            .expect("Failed to insert child")
    }

    fn add_children(
        &self,
        parent: Inode,
        children: Vec<(OsString, ())>,
        increment: bool,
    ) -> Vec<(OsString, Inode)> {
        let value_creator = |value_creator: inode_mapper::ValueCreatorParams<AtomicU64>| {
            if let Some(nlookup) = value_creator.existing_data {
                let count = nlookup.load(Ordering::Relaxed);
                AtomicU64::new(if increment { count + 1 } else { count })
            } else {
                AtomicU64::new(if increment { 1 } else { 0 })
            }
        };
        let children_with_creator: Vec<_> = children
            .iter()
            .map(|(name, _)| (name.clone(), value_creator))
            .collect();
        let inserted_children = self
            .mapper
            .write()
            .expect("Failed to acquire write lock")
            .insert_children(&parent, children_with_creator)
            .expect("Failed to insert children");
        inserted_children
            .into_iter()
            .zip(children)
            .map(|(inode, (name, _))| (name, inode))
            .collect()
    }

    fn forget(&self, ino: Inode, nlookup: u64) {
        {
            // Optimistically assume we don't have to remove yet
            let guard = self.mapper.read().expect("Failed to acquire read lock");
            let inode_info = guard.get(&ino).expect("Failed to find inode");
            if inode_info.data.fetch_sub(nlookup, Ordering::SeqCst) > 0 {
                return;
            }
        }
        self.mapper.write().unwrap().remove(&ino).unwrap();
    }

    fn prune(&self, keep: &HashSet<Self::ResolvedType>) {
        self.mapper
            .write()
            .expect("Failed to acquire write lock")
            .prune(keep);
    }

    fn rename(&self, parent: Inode, name: &OsStr, newparent: Inode, newname: &OsStr) {
        self.mapper
            .write()
            .expect("Failed to acquire write lock")
            .rename(
                &parent,
                name,
                &newparent,
                newname.to_os_string(),
            )
            .expect("Failed to rename inode");
    }
}

pub struct PathResolver {
    resolver: ComponentsResolver,
}

impl FileIdResolver for PathResolver {
    type ResolvedType = PathBuf;

    fn new() -> Self {
        PathResolver {
            resolver: ComponentsResolver::new(),
        }
    }

    fn resolve_id(&self, ino: Inode) -> Self::ResolvedType {
        self.resolver
            .resolve_id(ino)
            .iter()
            .rev()
            .collect::<PathBuf>()
    }

    fn lookup(
        &self,
        parent: Inode,
        child: &OsStr,
        id: <Self::ResolvedType as FileIdType>::_Id,
        increment: bool,
    ) -> Inode {
        self.resolver.lookup(parent, child, id, increment)
    }

    fn add_children(
        &self,
        parent: Inode,
        children: Vec<(OsString, <Self::ResolvedType as FileIdType>::_Id)>,
        increment: bool,
    ) -> Vec<(OsString, Inode)> {
        self.resolver.add_children(parent, children, increment)
    }

    fn forget(&self, ino: Inode, nlookup: u64) {
        self.resolver.forget(ino, nlookup);
    }

    fn prune(&self, keep: &HashSet<Self::ResolvedType>) {
        let resolver_keep: HashSet<Vec<OsString>> = keep
            .iter()
            .map(|path| path.iter().map(|s| s.to_os_string()).collect())
            .collect();
        self.resolver.prune(&resolver_keep);
    }

    fn rename(&self, parent: Inode, name: &OsStr, newparent: Inode, newname: &OsStr) {
        self.resolver.rename(parent, name, newparent, newname);
    }
}

pub struct HybridResolver<BackingId>
where
    BackingId: Clone + Eq + Hash,
{
    mapper: Arc<RwLock<InodeMultiMapper<AtomicU64, BackingId>>>,
}

impl<BackingId> FileIdResolver for HybridResolver<BackingId>
where
    BackingId: Clone + Eq + Hash + Send + Sync + std::fmt::Debug + 'static,
{
    type ResolvedType = HybridId<BackingId>;

    fn new() -> Self {
        let instance = Arc::new(RwLock::new(InodeMultiMapper::new(AtomicU64::new(0))));
        HybridResolver { mapper: instance }
    }

    fn resolve_id(&self, ino: Inode) -> Self::ResolvedType {
        HybridId::new(ino, self.mapper.clone())
    }

    fn lookup(
        &self,
        parent: Inode,
        child: &OsStr,
        id: <Self::ResolvedType as FileIdType>::_Id,
        increment: bool,
    ) -> Inode {
        {
            // Optimistically assume the child exists
            if let Some(lookup_result) = self
                .mapper
                .read()
                .expect("cannot acquire read lock")
                .lookup(&parent, child)
            {
                // Backing ID must match to use the hot path
                if lookup_result.backing_id.cloned() == id {
                    if increment {
                        lookup_result.data.fetch_add(1, Ordering::SeqCst);
                    }
                    return lookup_result.inode.clone();
                }
            }
        }
        // This scenario happens if the child node does not exist or the backing ID does not match
        self.mapper
            .write()
            .expect("Failed to acquire write lock")
            .insert_child(&parent, child.to_os_string(), id, |params| {
                // If the child node already exists, use the existing reference count
                let mut new_value = params
                    .existing_data
                    .map(|d| d.load(Ordering::SeqCst))
                    .unwrap_or(0);
                if increment {
                    new_value += 1;
                }
                AtomicU64::new(new_value)
            })
            .expect("Failed to insert child")
    }

    fn add_children(
        &self,
        parent: Inode,
        children: Vec<(OsString, <Self::ResolvedType as FileIdType>::_Id)>,
        increment: bool,
    ) -> Vec<(OsString, Inode)> {
        let value_creator = |value_creator: ValueCreatorParams<AtomicU64>| {
            if let Some(nlookup) = value_creator.existing_data {
                let count = nlookup.load(Ordering::Relaxed);
                AtomicU64::new(if increment { count + 1 } else { count })
            } else {
                AtomicU64::new(if increment { 1 } else { 0 })
            }
        };
        let children_with_creator: Vec<_> = children
            .iter()
            .map(|(name, id)| (name.clone(), id.clone(), value_creator))
            .collect();
        let inserted_children = self
            .mapper
            .write()
            .expect("Failed to acquire write lock")
            .insert_children(&parent, children_with_creator)
            .expect("Failed to insert children");
        inserted_children
            .into_iter()
            .zip(children)
            .map(|(inode, (name, _))| (name, inode))
            .collect()
    }

    fn forget(&self, ino: Inode, nlookup: u64) {
        {
            // Optimistically assume we don't have to remove yet
            let guard = self.mapper.read().expect("Failed to acquire read lock");
            let inode_info = guard.get(&ino).expect("Failed to find inode");
            if inode_info.data.fetch_sub(nlookup, Ordering::SeqCst) > 0 {
                return;
            }
        }
        self.mapper
            .write()
            .expect("Failed to acquire write lock")
            .remove(&ino)
            .unwrap();
    }

    fn prune(&self, _keep: &HashSet<Self::ResolvedType>) {
        // TODO
    }

    fn rename(&self, parent: Inode, name: &OsStr, newparent: Inode, newname: &OsStr) {
        self.mapper
            .write()
            .expect("Failed to acquire write lock")
            .rename(
                &parent,
                name,
                &newparent,
                newname.to_os_string(),
            )
            .expect("Failed to rename inode");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;
    use std::path::PathBuf;

    #[test]
    fn test_components_resolver() {
        let resolver = ComponentsResolver::new();

        // Test lookup and resolve_id
        let parent_ino = ROOT_INODE;
        let child_ino = resolver.lookup(parent_ino, OsStr::new("child"), (), true);
        let resolved_path = resolver.resolve_id(child_ino);

        assert_eq!(resolved_path, vec![OsString::from("child")]);

        // Test add_children
        let grandchildren = vec![
            (OsString::from("grandchild1"), ()),
            (OsString::from("grandchild2"), ()),
        ];
        let added_children = resolver.add_children(child_ino, grandchildren, true);

        assert_eq!(added_children.len(), 2);

        // Test forget
        resolver.forget(child_ino, 1);

        // Test rename
        resolver.rename(
            parent_ino,
            OsStr::new("child"),
            parent_ino,
            OsStr::new("renamed_child"),
        );

        let renamed_path = resolver.resolve_id(child_ino);
        assert_eq!(renamed_path, vec![OsString::from("renamed_child")]);

        // Test prune
        let keep = HashSet::new();
        resolver.prune(&keep);
    }

    #[test]
    #[should_panic(expected = "Failed to resolve inode")]
    fn test_components_resolver_prune_panics_on_resolved_deleted() {
        let resolver = ComponentsResolver::new();
        let parent_ino = ROOT_INODE;
        let child_ino = resolver.lookup(parent_ino, OsStr::new("child"), (), true);
        resolver.forget(child_ino, 1);
        resolver.prune(&HashSet::new());
        resolver.resolve_id(child_ino);
    }

    #[test]
    fn test_path_resolver() {
        let resolver = PathResolver::new();

        // Test lookup and resolve_id for root
        let root_ino = ROOT_INODE;
        let root_path = resolver.resolve_id(root_ino);
        assert_eq!(root_path, PathBuf::from(""));

        // Create a nested structure: /dir1/dir2/file.txt
        let dir1_ino = resolver.lookup(root_ino, OsStr::new("dir1"), (), true);
        let dir2_ino = resolver.lookup(dir1_ino, OsStr::new("dir2"), (), true);
        let file_ino = resolver.lookup(dir2_ino, OsStr::new("file.txt"), (), true);

        // Test resolve_id for nested structure
        let file_path = resolver.resolve_id(file_ino);
        assert_eq!(file_path, PathBuf::from("dir1/dir2/file.txt"));

        // Test add_children
        let dir2_children = vec![
            (OsString::from("child1.txt"), ()),
            (OsString::from("child2.txt"), ()),
        ];
        let added_children = resolver.add_children(dir2_ino, dir2_children, true);
        assert_eq!(added_children.len(), 2);

        // Verify added children
        for (name, ino) in added_children {
            let child_path = resolver.resolve_id(ino);
            assert_eq!(
                child_path,
                PathBuf::from(format!("dir1/dir2/{}", name.to_str().unwrap()))
            );
        }

        // Test forget
        resolver.forget(file_ino, 1);

        // Test rename within the same directory
        resolver.rename(
            dir2_ino,
            OsStr::new("file.txt"),
            dir2_ino,
            OsStr::new("renamed_file.txt"),
        );

        let renamed_file_path = resolver.resolve_id(file_ino);
        assert_eq!(
            renamed_file_path,
            PathBuf::from("dir1/dir2/renamed_file.txt")
        );

        // Test rename to a different directory
        let dir3_ino = resolver.lookup(root_ino, OsStr::new("dir3"), (), true);
        resolver.rename(
            dir2_ino,
            OsStr::new("renamed_file.txt"),
            dir3_ino,
            OsStr::new("moved_file.txt"),
        );

        let moved_file_path = resolver.resolve_id(file_ino);
        assert_eq!(moved_file_path, PathBuf::from("dir3/moved_file.txt"));

        // Test lookup for non-existent file
        let non_existent_ino = resolver.lookup(root_ino, OsStr::new("non_existent"), (), false);
        assert_ne!(non_existent_ino.0, 0);
        let non_existent_path = resolver.resolve_id(non_existent_ino);
        assert_eq!(non_existent_path, PathBuf::from("non_existent"));
    }

    #[test]
    fn test_hybrid_resolver() {
        let resolver = HybridResolver::<u64>::new();

        // Test lookup and resolve_id for root
        let root_ino = ROOT_INODE;
        let root_id = resolver.resolve_id(root_ino);
        assert_eq!(root_id.first_path(), Some(PathBuf::from("")));

        // Test lookup and resolve_id for child and create nested structures
        let dir1_ino = resolver.lookup(root_ino, OsStr::new("dir1"), Some(1), true);
        let dir1_id = resolver.resolve_id(dir1_ino);
        assert_eq!(dir1_id.first_path(), Some(PathBuf::from("dir1")));

        let dir2_ino = resolver.lookup(dir1_ino, OsStr::new("dir2"), Some(2), true);
        let dir2_id = resolver.resolve_id(dir2_ino);
        assert_eq!(dir2_id.first_path(), Some(PathBuf::from("dir1/dir2")));

        // Test add_children
        let grandchildren = vec![
            (OsString::from("grandchild1"), Some(3)),
            (OsString::from("grandchild2"), Some(4)),
        ];
        let added_grandchildren = resolver.add_children(dir2_ino, grandchildren, true);
        assert_eq!(added_grandchildren.len(), 2);
        for (name, ino) in added_grandchildren.iter() {
            let child_path = resolver.resolve_id(*ino);
            assert_eq!(
                child_path.first_path(),
                Some(PathBuf::from("dir1/dir2").join(name))
            );
        }

        // Test forget
        resolver.forget(added_grandchildren[0].1, 1);

        // Test rename within the same directory
        resolver.rename(
            dir2_ino,
            OsStr::new("grandchild2"),
            dir2_ino,
            OsStr::new("grandchild2_renamed"),
        );
        let renamed_grandchild_path = resolver.resolve_id(added_grandchildren[1].1);
        assert_eq!(
            renamed_grandchild_path.first_path(),
            Some(PathBuf::from("dir1/dir2/grandchild2_renamed"))
        );

        // Test rename to a different directory
        let dir3_ino = resolver.lookup(root_ino, OsStr::new("dir3"), Some(5), true);
        resolver.rename(
            dir2_ino,
            OsStr::new("grandchild2_renamed"),
            dir3_ino,
            OsStr::new("grandchild2_renamed"),
        );
        let renamed_grandchild_path = resolver.resolve_id(added_grandchildren[1].1);
        assert_eq!(
            renamed_grandchild_path.first_path(),
            Some(PathBuf::from("dir3/grandchild2_renamed"))
        );

        // Test lookup for non-existent file
        let non_existent_ino =
            resolver.lookup(root_ino, OsStr::new("non_existent"), Some(6), false);
        assert_ne!(non_existent_ino.0, 0);
        let non_existent_path = resolver.resolve_id(non_existent_ino);
        assert_eq!(
            non_existent_path.first_path(),
            Some(PathBuf::from("non_existent"))
        );

        // Test lookup for a file with existing backing ID
        let hard_link_ino = resolver.lookup(root_ino, OsStr::new("hard_link"), Some(7), true);
        let hard_link_id = resolver.resolve_id(hard_link_ino);
        assert_eq!(hard_link_id.first_path(), Some(PathBuf::from("hard_link")));

        let hard_link_ino_2 = resolver.lookup(dir2_ino, OsStr::new("hard_linked"), Some(7), true);
        let hard_link_id_2 = resolver.resolve_id(hard_link_ino_2);
        assert_eq!(
            hard_link_ino_2, hard_link_ino,
            "hard link should be the same if callers supply the same ID"
        );

        resolver.lookup(dir1_ino, OsStr::new("hard_linked_2"), Some(7), true);
        let paths = hard_link_id_2.all_paths(Some(100));
        assert!(paths.contains(&PathBuf::from("dir1/dir2/hard_linked")));
        assert!(paths.contains(&PathBuf::from("hard_link")));
        assert!(paths.contains(&PathBuf::from("dir1/hard_linked_2")));

        // Overriding a location with a new backing ID should always create a new inode
        let overridden_hard_link_ino =
            resolver.lookup(dir2_ino, OsStr::new("hard_linked"), Some(8), true);
        assert_ne!(
            overridden_hard_link_ino, hard_link_ino,
            "overridden location's inode should change upon encountering a new ID"
        );

        // Test path resolution after overriding a location with a new backing ID
        let paths = hard_link_id_2.all_paths(Some(100));
        assert!(
            !paths.contains(&PathBuf::from("dir1/dir2/hard_linked")),
            "the path list should no longer contain the overridden location"
        );
        assert!(paths.contains(&PathBuf::from("hard_link")));
        assert!(paths.contains(&PathBuf::from("dir1/hard_linked_2")));
    }

    #[test]
    fn test_path_resolver_back_and_forth_rename() {
        let resolver = PathResolver::new();

        // Test lookup and resolve_id for root
        let root_ino = ROOT_INODE;
        let root_path = resolver.resolve_id(root_ino);
        assert_eq!(root_path, PathBuf::from(""));

        // Add directories
        let dir1_ino = resolver.lookup(root_ino, OsStr::new("dir1"), (), true);
        let dir2_ino = resolver.lookup(dir1_ino, OsStr::new("dir2"), (), true);
        let file_ino = resolver.lookup(root_ino, OsStr::new("file.txt"), (), true);

        // Rename file to a different directory
        resolver.rename(
            root_ino,
            OsStr::new("file.txt"),
            dir2_ino,
            OsStr::new("file.txt"),
        );
        let renamed_file_path = resolver.resolve_id(file_ino);
        assert_eq!(renamed_file_path, PathBuf::from("dir1/dir2/file.txt"));

        // Rename file back to original directory
        resolver.rename(
            dir2_ino,
            OsStr::new("file.txt"),
            root_ino,
            OsStr::new("file.txt"),
        );
        let renamed_file_path = resolver.resolve_id(file_ino);
        assert_eq!(renamed_file_path, PathBuf::from("file.txt"));
    }
}
