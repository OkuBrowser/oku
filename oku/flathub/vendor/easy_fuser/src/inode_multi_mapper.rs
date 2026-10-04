use crate::types::{Inode, ROOT_INODE, InodeExt};
use bimap::BiHashMap;
use std::{
    borrow::Borrow,
    collections::{HashMap, HashSet},
    ffi::{OsStr, OsString},
    fmt::Debug,
    hash::{Hash, Hasher},
    ops::Deref,
    sync::Arc,
};

#[derive(Debug)]
pub struct InodeMultiMapper<Data, BackingId>
where
    BackingId: Clone + Eq + Hash,
    Data: Send + Sync + 'static,
{
    data: InodeData<Data, BackingId>,
    root_inode: Inode,
    next_inode: Inode,
}

#[derive(Debug)]
struct InodeData<Data, BackingId>
where
    BackingId: Clone + Eq + Hash,
    Data: Send + Sync + 'static,
{
    /// A map of inodes' internal data
    inodes: HashMap<Inode, InodeValue<Data>>,
    /// A map of inodes' child nodes.
    children: HashMap<Inode, HashMap<OsStringWrapper, Inode>>,
    /// Bidirectional hash map to allow lookups and upserts of inodes by
    /// user-supplied backing IDs (e.g. using a stat and statfs call).
    backing: BiHashMap<Inode, BackingId>,
}

#[derive(Debug)]
struct InodeValue<Data>
where
    Data: Send + Sync + 'static,
{
    links: HashMap<Inode, HashSet<OsStringWrapper>>,
    data: Data,
}

#[derive(Debug)]
pub struct ValueCreatorParams<'a, Data>
where
    Data: Send + Sync + 'static,
{
    pub new_inode: &'a Inode,
    pub parent: &'a Inode,
    pub child_name: &'a OsStr,
    pub existing_data: Option<&'a Data>,
}

#[derive(Debug)]

pub struct LookupResult<'a, Data, BackingId>
where
    Data: Send + Sync + 'static,
    BackingId: Clone + Eq + Hash,
{
    pub inode: &'a Inode,
    pub backing_id: Option<&'a BackingId>,
    pub links: &'a HashMap<Inode, HashSet<OsStringWrapper>>,
    pub data: &'a Data,
}

#[derive(Debug)]
pub struct InodeInfo<'a, Data>
where
    Data: Send + Sync + 'static,
{
    pub links: &'a HashMap<Inode, HashSet<OsStringWrapper>>,
    #[allow(dead_code)]
    pub data: &'a Data,
}

impl<'a, Data> Clone for InodeInfo<'a, Data>
where
    Data: Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        InodeInfo {
            links: self.links,
            data: self.data,
        }
    }
}

#[derive(Debug)]
pub struct InodeResolveItem<'a, Data>
where
    Data: Send + Sync + 'static,
{
    pub parent: &'a Inode,
    pub name: &'a Arc<OsString>,
    pub inode: InodeInfo<'a, Data>,
}

impl<'a, Data> Clone for InodeResolveItem<'a, Data>
where
    Data: Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        InodeResolveItem {
            parent: self.parent,
            name: self.name,
            inode: self.inode.clone(),
        }
    }
}

impl<'a, Data> Deref for InodeResolveItem<'a, Data>
where
    Data: Send + Sync + 'static,
{
    type Target = InodeInfo<'a, Data>;

    fn deref(&self) -> &Self::Target {
        &self.inode
    }
}

#[derive(Debug)]
pub struct InodeInfoMut<'a, Data>
where
    Data: Send + Sync + 'static,
{
    #[allow(dead_code)]
    links: &'a mut HashMap<Inode, HashSet<OsStringWrapper>>,
    #[allow(dead_code)]
    data: &'a mut Data,
}

#[derive(Debug, PartialEq, Eq)]
pub enum InsertError {
    ParentNotFound,
}

#[derive(Debug, PartialEq, Eq)]
pub enum RenameError {
    NotFound,
    ParentNotFound,
    NewParentNotFound,
}

/// A wrapper around `Arc<OsString>` for efficient storage and comparison in hash maps.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct OsStringWrapper(Arc<OsString>);

impl AsRef<Arc<OsString>> for OsStringWrapper {
    fn as_ref(&self) -> &Arc<OsString> {
        &self.0
    }
}

impl AsMut<Arc<OsString>> for OsStringWrapper {
    fn as_mut(&mut self) -> &mut Arc<OsString> {
        &mut self.0
    }
}

impl Borrow<OsStr> for OsStringWrapper {
    fn borrow(&self) -> &OsStr {
        self.0.as_os_str()
    }
}

impl<Data, BackingId> InodeMultiMapper<Data, BackingId>
where
    BackingId: Clone + Eq + Hash + Debug,
    Data: Send + Sync + 'static,
{
    /// Creates a new `InodeMultiMapper` instance with the root inode initialized.
    ///
    /// This function initializes the `InodeMultiMapper` with an empty structure and sets up the root inode
    /// with the provided data. The root inode is assigned an empty name and its parent is set to itself.
    pub fn new(data: Data) -> Self {
        let mut result = InodeMultiMapper {
            data: InodeData {
                inodes: HashMap::new(),
                children: HashMap::new(),
                backing: BiHashMap::new(),
            },
            root_inode: ROOT_INODE.clone(),
            next_inode: ROOT_INODE.add_one(),
        };
        result.data.inodes.insert(
            ROOT_INODE.clone(),
            InodeValue {
                links: HashMap::from([(
                    ROOT_INODE.clone(),
                    HashSet::from([OsStringWrapper(Arc::new(OsString::from("")))]),
                )]),
                data,
            },
        );
        result
    }

    fn reserve_inode_space(&mut self, entries_count: usize) {
        if self.data.inodes.is_empty() {
            self.data.inodes.reserve(entries_count);
        } else if self.data.inodes.capacity() * 2 < self.data.inodes.len() + entries_count {
            self.data
                .inodes
                .reserve(entries_count + self.data.inodes.len() - self.data.inodes.capacity());
        }
    }

    fn reserve_children_space(&mut self, entries_count: usize) {
        if self.data.children.is_empty() {
            self.data.children.reserve(entries_count);
        } else if self.data.children.capacity() * 2 < self.data.children.len() + entries_count {
            self.data
                .children
                .reserve(entries_count + self.data.children.len() - self.data.children.capacity());
        }
    }

    fn reserve_inode_children_space(&mut self, parent: &Inode, entries_count: usize) {
        if let Some(parent_children) = self.data.children.get_mut(parent) {
            if parent_children.is_empty() {
                parent_children.reserve(entries_count);
            } else if parent_children.capacity() * 2 < parent_children.len() + entries_count {
                parent_children
                    .reserve(entries_count + parent_children.len() - parent_children.capacity());
            }
        } else {
            self.data
                .children
                .insert(parent.clone(), HashMap::with_capacity(entries_count));
        }
    }

    fn reserve_backing_space(&mut self, entries_count: usize) {
        if self.data.backing.is_empty() {
            self.data.backing.reserve(entries_count);
        } else if self.data.backing.capacity() * 2 < self.data.backing.len() + entries_count {
            self.data
                .backing
                .reserve(entries_count + self.data.backing.len() - self.data.backing.capacity());
        }
    }

    /// Compute a deterministic inode value based on the backing ID.
    /// If the inode already exists, the algorithm tries to find the next available inode
    /// starting from the hash value.
    /// If the backing ID is not provided, allocate a new inode.
    fn compute_or_allocate_inode(&mut self, backing_id: Option<&BackingId>) -> Inode {
        // Deterministically hash the backing ID to get a stable inode value if possible
        match backing_id {
            Some(backing_id) => {
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                backing_id.hash(&mut hasher);
                let hash = hasher.finish();
                let mut preferred_inode = fuser::INodeNo(hash);
                loop {
                    if !self.data.inodes.contains_key(&preferred_inode) {
                        break preferred_inode;
                    }
                    preferred_inode = preferred_inode.add_one();
                }
            }
            None => loop {
                let new_inode = self.next_inode.clone();
                self.next_inode = new_inode.add_one();
                if !self.data.inodes.contains_key(&new_inode) {
                    break new_inode;
                }
            },
        }
    }

    pub fn get_root_inode(&self) -> Inode {
        self.root_inode.clone()
    }

    /// A private method that inserts a child inode into the InodeMultiMapper,
    /// even if the parent doesn't exist.
    ///
    /// This function creates a new inode or updates an existing one,
    /// associating it with the given parent and child name. It uses a
    /// value_creator function to generate or update the data associated with
    /// the inode.
    ///
    /// Note: This method doesn't check if the parent exists, which can lead to
    /// inconsistencies if used incorrectly. It's primarily intended for internal use or in scenarios where the parent's existence is guaranteed.
    ///
    /// # Behavior:
    /// - If the child doesn't exist:
    ///     - If the backing ID is not specified, or if the backing ID cannot be resolved to a valid inode:
    ///         - A new inode is created with a unique ID.
    ///         - The new inode is bi-directionally associated with the parent as well as to the backing ID.
    ///         - The data is created using the value_creator function.
    ///     - If the backing ID is specified and points to a valid existing inode,
    ///         - That existing inode will be associated with the parent instead.
    /// - If the child already exists:
    ///     - If the backing ID is specified and points to an existing inode:
    ///         - If the inode (A) pointed to by the backing ID is different, the old child inode (B) will be
    ///           unassociated from the parent and be replaced with inode (A).
    ///         - The data is updated using the value_creator function.
    ///     - If the backing ID (I1) is specified and does not point to any existing inode:
    ///         - If the child inode (A) does not have a backing ID, the backing ID (I1) will be associated with the child inode (A).
    ///           The data is then updated using the value_creator function.
    ///         - If the child inode (A) has a backing ID (I2) therefore (I2 != I1), the child inode (A) will be unassociated from the
    ///           parent. A new inode is then created and the value_creator function is then called.
    /// - The value_creator function is called with the inode, parent, child name, and existing data (if any) as arguments.
    ///
    /// # Caveats
    /// - This method may create orphaned inodes if used with non-existent parents. Use with caution.
    fn insert_child_unchecked(
        &mut self,
        parent: &Inode,
        child: OsString,
        backing_id: Option<BackingId>,
        value_creator: impl Fn(ValueCreatorParams<Data>) -> Data,
    ) -> Inode {
        let child_name = OsStringWrapper(Arc::new(child));
        let parent_children = self
            .data
            .children
            .entry(parent.clone())
            .or_default();
        let backing_inode = backing_id
            .clone()
            .and_then(|backing_id| {
                self.data
                    .backing
                    .get_by_right(&backing_id).cloned()
            });
        let target_child_inode = parent_children.get(&child_name).cloned();
        match (backing_inode, target_child_inode) {
            (Some(backing_inode), Some(target_child_inode)) => {
                if backing_inode != target_child_inode {
                    let target_child_inode_data = self
                        .data
                        .inodes
                        .get_mut(&target_child_inode)
                        .expect("target child inode not found");
                    // Deassociate parent from old child inode if possible
                    let links = target_child_inode_data
                        .links
                        .entry(parent.clone())
                        .or_default();
                    links.remove(&child_name.clone());
                    if links.is_empty() {
                        target_child_inode_data.links.remove(&parent.clone());
                    }
                }
                let existing_inode_data = self
                    .data
                    .inodes
                    .get_mut(&backing_inode)
                    .expect("backing inode not found");
                // Associate parent to new child
                existing_inode_data
                    .links
                    .entry(parent.clone())
                    .or_default()
                    .insert(child_name.clone());
                existing_inode_data.data = value_creator(ValueCreatorParams {
                    parent,
                    new_inode: &backing_inode,
                    child_name: child_name.as_ref(),
                    existing_data: Some(&existing_inode_data.data),
                });
                // Associate new child to parent
                parent_children.insert(child_name.clone(), backing_inode.clone());
                backing_inode
            }
            (None, Some(target_child_inode)) => {
                let target_child_inode_backing_id = self
                    .data
                    .backing
                    .get_by_left(&target_child_inode).cloned();
                let target_child_inode_data = self
                    .data
                    .inodes
                    .get_mut(&target_child_inode)
                    .expect("target child inode not found");
                if let Some(desired_backing_id) = backing_id.clone()
                    && let Some(_target_child_inode_backing_id) = target_child_inode_backing_id
                {
                    #[cfg(debug_assertions)]
                    assert_ne!(
                        desired_backing_id, _target_child_inode_backing_id,
                        "the desired backing ID should not match because it is not yet recognized"
                    );
                    // Deassociate parent from target child
                    let links = target_child_inode_data
                        .links
                        .entry(parent.clone())
                        .or_default();
                    links.remove(&child_name.clone());
                    if links.is_empty() {
                        target_child_inode_data.links.remove(&parent.clone());
                    }
                    // Create new inode
                    let new_inode = self.compute_or_allocate_inode(Some(&desired_backing_id));
                    // Associate parent to new child and initialize data
                    self.data.inodes.insert(
                        new_inode.clone(),
                        InodeValue {
                            links: HashMap::from([(
                                parent.clone(),
                                HashSet::from([child_name.clone()]),
                            )]),
                            data: value_creator(ValueCreatorParams {
                                parent,
                                new_inode: &new_inode,
                                child_name: child_name.as_ref(),
                                existing_data: None,
                            }),
                        },
                    );
                    // Associate new child to parent
                    self.data
                        .backing
                        .insert(new_inode.clone(), desired_backing_id);
                    new_inode
                } else {
                    // Associate parent to target child
                    target_child_inode_data
                        .links
                        .entry(parent.clone())
                        .or_default()
                        .insert(child_name.clone());
                    // Associate target child to parent
                    parent_children.insert(child_name.clone(), target_child_inode.clone());
                    // Update data
                    target_child_inode_data.data = value_creator(ValueCreatorParams {
                        parent,
                        new_inode: &target_child_inode,
                        child_name: child_name.as_ref(),
                        existing_data: Some(&target_child_inode_data.data),
                    });
                    // Associate target child to backing ID
                    if let Some(backing_id) = backing_id {
                        self.data
                            .backing
                            .insert(target_child_inode.clone(), backing_id);
                    }
                    target_child_inode
                }
            }
            (Some(backing_inode), None) => {
                let backing_inode_data = self
                    .data
                    .inodes
                    .get_mut(&backing_inode)
                    .expect("backing inode not found");
                // Associate parent to child
                backing_inode_data
                    .links
                    .entry(parent.clone())
                    .or_default()
                    .insert(child_name.clone());
                // Associate child to parent
                parent_children.insert(child_name.clone(), backing_inode.clone());
                // Update data
                backing_inode_data.data = value_creator(ValueCreatorParams {
                    parent,
                    new_inode: &backing_inode,
                    child_name: child_name.as_ref(),
                    existing_data: Some(&backing_inode_data.data),
                });
                // Backing inode is already associated with the backing ID, so this step is skipped
                backing_inode
            }
            (None, None) => {
                let new_inode = self.compute_or_allocate_inode(backing_id.as_ref());
                // Associate parent to child and initialize data
                self.data.inodes.insert(
                    new_inode.clone(),
                    InodeValue {
                        links: HashMap::from([(
                            parent.clone(),
                            HashSet::from([child_name.clone()]),
                        )]),
                        data: value_creator(ValueCreatorParams {
                            parent,
                            new_inode: &new_inode,
                            child_name: child_name.as_ref(),
                            existing_data: None,
                        }),
                    },
                );
                // Associate child to parent
                let parent_children = self
                    .data
                    .children
                    .entry(parent.clone())
                    .or_default();
                parent_children.insert(child_name.clone(), new_inode.clone());
                if let Some(backing_id) = backing_id {
                    self.data.backing.insert(new_inode.clone(), backing_id);
                }
                new_inode
            }
        }
    }

    /// Safely inserts a child inode into the InodeMultiMapper.
    ///
    /// This method checks if the parent exists before inserting the child. It uses a value_creator
    /// function to generate the data associated with the new inode. If the backing ID is specified,
    /// the new child will be associated with the backing ID.
    ///
    /// # Behavior
    /// - Returns Err(InsertError::ParentNotFound) if the parent doesn't exist.
    /// - If successful, returns Ok(Inode) with the newly created or existing child inode.
    ///
    /// The value_creator function is called with the new inode, parent inode, child name, and existing data (if any) as arguments.
    pub fn insert_child(
        &mut self,
        parent: &Inode,
        child: OsString,
        backing_id: Option<BackingId>,
        value_creator: impl Fn(ValueCreatorParams<Data>) -> Data,
    ) -> Result<Inode, InsertError> {
        if !self.data.inodes.contains_key(parent) {
            return Err(InsertError::ParentNotFound);
        }
        Ok(self.insert_child_unchecked(parent, child, backing_id, value_creator))
    }

    /// Inserts multiple children into the InodeMultiMapper for a given parent inode.
    ///
    /// This method efficiently inserts multiple children at once, optimizing memory allocation
    /// for the parent's children HashMap. It checks if the parent exists before insertion.
    ///
    /// # Behavior
    /// - Returns Err(InsertError::ParentNotFound) if the parent doesn't exist.
    /// - If successful, returns `Ok(Vec<Inode>)` with the newly created or existing child inodes.
    ///
    /// The value_creator function is called with the new inode, parent inode, child name, and
    /// existing data (if any) as arguments.
    pub fn insert_children(
        &mut self,
        parent: &Inode,
        children: Vec<(
            OsString,
            Option<BackingId>,
            impl Fn(ValueCreatorParams<Data>) -> Data,
        )>,
    ) -> Result<Vec<Inode>, InsertError> {
        if !self.data.inodes.contains_key(parent) {
            return Err(InsertError::ParentNotFound);
        }

        self.reserve_inode_space(children.len());
        self.reserve_inode_children_space(parent, children.len());
        self.reserve_backing_space(children.len());

        Ok(children
            .into_iter()
            .map(|(child, backing_id, value_creator)| {
                self.insert_child_unchecked(parent, child, backing_id, value_creator)
            })
            .collect())
    }

    /// Batch inserts multiple entries into the InodeMultiMapper, creating missing parent directories as needed.
    ///
    /// This method efficiently handles the insertion of multiple entries, potentially with nested paths.
    /// It sorts entries by path length to ensure parent directories are created before their children.
    ///
    /// # Behavior
    /// - Creates missing parent directories using the default_parent_creator function. (data field will always be null)
    /// - Inserts entries using the provided value_creator function.
    /// - Returns Err(InsertError::ParentNotFound) if the initial parent inode doesn't exist.
    ///
    /// # Note
    /// Expects each entry's path to include the entry name as the last element.
    ///
    /// # Caveats
    /// If the closures are not defined in same scope, there might be a compiler error concerning lifetimes (eg: implementation of `Fn` is not general enough)
    /// To resolve this problem, always fully qualify the argumentsof the closure (eg: `|my_data: ValueCreatorParams<MyType>| {}` and not `|my_data| {}`)
    pub fn batch_insert(
        &mut self,
        parent: &Inode,
        entries: Vec<(
            Vec<OsString>,
            Option<BackingId>,
            impl Fn(ValueCreatorParams<Data>) -> Data,
        )>,
        default_parent_creator: impl Fn(ValueCreatorParams<Data>) -> Data,
    ) -> Result<(), InsertError> {
        if !self.data.inodes.contains_key(parent) {
            return Err(InsertError::ParentNotFound);
        }

        // Sort entries by path length to ensure parents are created first
        let mut sorted_entries = entries;
        sorted_entries.sort_by_key(|f| f.0.len());

        let mut path_cache: HashMap<Vec<OsString>, Inode> = HashMap::new();
        path_cache.insert(vec![], parent.clone());

        self.reserve_inode_space(sorted_entries.len());
        self.reserve_children_space(sorted_entries.len());
        self.reserve_inode_children_space(parent, sorted_entries.len());
        self.reserve_backing_space(sorted_entries.len());

        for (mut path, backing_id, value_creator) in sorted_entries {
            let name = path.pop().expect("Name should be provided");
            let parent_inode =
                self.ensure_path_exists(&mut path_cache, &path, &default_parent_creator);
            self.insert_child_unchecked(&parent_inode, name, backing_id, value_creator);
        }
        Ok(())
    }

    fn ensure_path_exists(
        &mut self,
        path_cache: &mut HashMap<Vec<OsString>, Inode>,
        path: &[OsString],
        default_parent_creator: &impl Fn(ValueCreatorParams<Data>) -> Data,
    ) -> Inode {
        let mut current_inode = path_cache[&vec![]].clone();
        for (i, component) in path.iter().enumerate() {
            let current_path = &path[..=i];
            if let Some(inode) = path_cache.get(current_path) {
                current_inode = inode.clone();
            } else {
                let child_inode = self
                    .data
                    .children
                    .get_mut(&current_inode)
                    .and_then(|children| children.get(component.as_os_str()));
                let new_inode = if let Some(child_inode) = child_inode {
                    child_inode.clone()
                } else {
                    // Since backing_id and child_inode is both None, there will always be a new inode
                    self.insert_child_unchecked(
                        &current_inode,
                        component.clone(),
                        None,
                        |mut value_creator_params| {
                            value_creator_params.existing_data = None;
                            default_parent_creator(value_creator_params)
                        },
                    )
                };
                path_cache.insert(current_path.to_vec(), new_inode.clone());
                current_inode = new_inode;
            }
        }
        current_inode
    }

    /// Resolves an inode to one combination of its full path components
    ///
    /// # Notes
    /// - Due to the nature of an inode being able to have multiple links, there can be multiple combinations of path components
    ///   that resolve to the same inode. This method only returns the first combination of path components that
    ///   resolves to the inode.
    /// - Returns `None` if any inode in the path is not found, indicating an incomplete or invalid path, or
    ///   there is an infinite loop (eg: if the inode is linked to itself and there is no way to trace back to the
    ///   root inode).
    /// - The root inode is identified when its parent is equal to itself and is never returned
    pub fn resolve(&self, inode: &Inode) -> Option<Vec<InodeResolveItem<'_, Data>>> {
        let mut visited = HashSet::new();
        let mut result: Vec<InodeResolveItem<Data>> = Vec::new();
        let mut current_info = self.get(inode)?;
        let mut current_inode = inode.clone();

        'resolution_loop: loop {
            let is_root_inode = current_inode == ROOT_INODE;
            if is_root_inode {
                break 'resolution_loop;
            }
            for (parent, names) in current_info.links.iter() {
                if visited.contains(parent) {
                    // The parent inode has already been visited, do not follow, try another link
                    continue;
                }
                // There must be at least one name, orphaned inodes cannot be resolved
                if names.is_empty() {
                    continue;
                }
                visited.insert(current_inode.clone());
                current_inode = parent.clone();
                result.push(InodeResolveItem {
                    parent,
                    name: names.iter().next().unwrap().as_ref(),
                    inode: current_info,
                });
                current_info = self.get(&current_inode)?;
                continue 'resolution_loop;
            }
            return None;
        }
        Some(result)
    }

    /// Recursively resolve all possible combinations of path components that resolve to the given inode, up to a given limit.
    pub fn resolve_all<'a>(
        &'a self,
        inode: &Inode,
        limit: Option<usize>,
    ) -> Vec<Vec<InodeResolveItem<'a, Data>>> {
        let mut result = vec![];

        fn scoped_resolve<'a, Data, BackingId>(
            mapper: &'a InodeMultiMapper<Data, BackingId>,
            result: &mut Vec<Vec<InodeResolveItem<'a, Data>>>,
            limit: Option<usize>,
            current_inode: &Inode,
            resolve_item_stack: &mut Vec<InodeResolveItem<'a, Data>>,
            visited_stack: &mut HashSet<Inode>,
        )
        where
            BackingId: Clone + Eq + Hash + Debug,
            Data: Send + Sync,
        {
            let is_root_inode = *current_inode == ROOT_INODE;
            if is_root_inode {
                if limit.is_none_or(|limit| result.len() < limit) {
                    // Freeze the result
                    result.push(resolve_item_stack.to_vec());
                }
                // All resolved paths must not go beyond the root inode
                return;
            }
            let current_info = match mapper.get(current_inode) {
                Some(info) => info,
                None => return,
            };
            visited_stack.insert(current_inode.clone());
            'scan_loop: for (parent, names) in current_info.links.iter() {
                if limit.is_some_and(|limit| result.len() >= limit) {
                    break 'scan_loop;
                }
                if visited_stack.contains(parent) {
                    continue;
                }
                if names.is_empty() {
                    continue;
                }
                for name in names.iter() {
                    if limit.is_some_and(|limit| result.len() >= limit) {
                        break 'scan_loop;
                    }
                    resolve_item_stack.push(InodeResolveItem {
                        parent,
                        name: name.as_ref(),
                        inode: current_info.clone(),
                    });
                    scoped_resolve(
                        mapper,
                        result,
                        limit,
                        parent,
                        resolve_item_stack,
                        visited_stack,
                    );
                    resolve_item_stack.pop();
                }
            }
            // Pop the inserted item from the visited stack
            visited_stack.remove(current_inode);
        }
        scoped_resolve(
            self,
            &mut result,
            limit,
            inode,
            &mut Vec::new(),
            &mut HashSet::new(),
        );
        result
    }

    pub fn get(&self, inode: &Inode) -> Option<InodeInfo<'_, Data>> {
        self.data.inodes.get(inode).map(|inode_value| InodeInfo {
            links: &inode_value.links,
            data: &inode_value.data,
        })
    }

    pub fn get_mut(&mut self, inode: &Inode) -> Option<InodeInfoMut<'_, Data>> {
        self.data
            .inodes
            .get_mut(inode)
            .map(|inode_value| InodeInfoMut {
                links: &mut inode_value.links,
                data: &mut inode_value.data,
            })
    }

    /// Retrieves the backing ID of a given inode
    pub fn get_backing_id(&self, inode: &Inode) -> Option<&BackingId> {
        self.data.backing.get_by_left(inode)
    }

    /// Retrieves all children of a given parent inode.
    ///
    /// # Note
    /// - Does not check if the parent inode exists.
    /// - Returns an empty vector if the parent has no children or doesn't exist.
    pub fn get_children(&self, parent: &Inode) -> Vec<(&Arc<OsString>, &Inode)> {
        self.data
            .children
            .get(parent)
            .map(|children| {
                children
                    .iter()
                    .map(|(name, inode)| (name.as_ref(), inode))
                    .collect()
            })
            .unwrap_or(vec![])
    }

    /// Looks up a child inode by its parent inode and name
    pub fn lookup(
        &self,
        parent: &Inode,
        name: &OsStr,
    ) -> Option<LookupResult<'_, Data, BackingId>> {
        self.data
            .children
            .get(parent)
            .and_then(|children| children.get(name))
            .map(|inode| {
                let inode_value = self.data.inodes.get(inode).unwrap();
                LookupResult {
                    inode,
                    backing_id: self.data.backing.get_by_left(inode),
                    links: &inode_value.links,
                    data: &inode_value.data,
                }
            })
    }

    /// Renames a child inode from one parent to another
    pub fn rename(
        &mut self,
        parent: &Inode,
        oldname: &OsStr,
        newparent: &Inode,
        newname: OsString,
    ) -> Result<Option<(Inode, Data)>, RenameError> {
        let newname = OsStringWrapper(Arc::new(newname));

        // Check if the new parent exists
        if !self.data.inodes.contains_key(parent) {
            return Err(RenameError::ParentNotFound);
        }
        if !self.data.inodes.contains_key(newparent) {
            return Err(RenameError::NewParentNotFound);
        }

        // Remove the child from the old parent
        let mut is_parent_empty = false;
        let child_inode = self
            .data
            .children
            .get_mut(parent)
            .ok_or(RenameError::NotFound)
            .and_then(|parent_children| {
                let child_inode = parent_children
                    .remove(oldname)
                    .ok_or(RenameError::NotFound)?;
                if parent_children.is_empty() {
                    is_parent_empty = true;
                }
                Ok(child_inode)
            })?;

        // Remove the old parent if it's now empty
        if is_parent_empty {
            self.data.children.remove(parent);
        }

        // Update the inode value, remove an association to old parent and add association to new parent
        self.data.inodes.get_mut(&child_inode).map(|inode_value| {
            // Remove an association to old parent, and remove the set
            let old_parent_associations = inode_value
                .links
                .entry(parent.clone())
                .or_insert_with(HashSet::new);
            old_parent_associations.remove(oldname);
            if old_parent_associations.is_empty() {
                inode_value.links.remove(parent);
            }

            // Add an association to new parent
            inode_value
                .links
                .entry(newparent.clone())
                .or_insert_with(HashSet::new)
                .insert(newname.clone());
        });

        // Insert the child into the new parent's children map
        self.data
            .children
            .entry(newparent.clone())
            .or_default()
            .insert(newname, child_inode);

        Ok(None)
    }

    /// Removes an inode and its associated data from the `InodeMapper`.
    ///
    /// This function removes the specified inode from the `inodes`, `children` and `backing` maps.
    /// It also cleans up empty parent entries in the `children` map.
    ///
    /// # Note
    /// This operation will no longer cascade to child inodes since an inode may be
    /// owned by multiple parents, and any inode can now have ROOT_INODE as a child.
    ///
    /// # Behavior
    /// - Panics if we intend to remove ROOT in debug build
    /// - If the inode doesn't exist, the function does nothing.
    /// - If the parent's children map becomes empty after removal, the parent entry
    ///   is also removed from the `children` map to conserve memory.
    pub fn remove(&mut self, inode: &Inode) -> Option<Data> {
        #[cfg(debug_assertions)]
        if *inode == ROOT_INODE {
            panic!("Cannot remove ROOT");
        }
        if let Some(inode_value) = self.data.inodes.remove(inode) {
            // Remove this inode from its parent's children
            for (parent, names) in inode_value.links.iter() {
                if let Some(parent_children) = self.data.children.get_mut(parent) {
                    for name in names.iter() {
                        parent_children.remove(name);
                    }
                    if parent_children.is_empty() {
                        self.data.children.remove(parent);
                    }
                }
            }

            // Remove links to children, but don't cascade
            self.data.children.remove(inode);

            // Remove links to backing ID
            self.data.backing.remove_by_left(inode);
            Some(inode_value.data)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::HashSet;
    use std::ffi::OsString;

    use crate::types::ROOT_INODE;

    #[test]
    fn test_insert_child_returns_old_inode() {
        let mut mapper = InodeMultiMapper::<u64, u64>::new(0);
        let root = mapper.get_root_inode();
        let child_name = OsString::from("child");

        // Insert the first child
        let first_child_inode = fuser::INodeNo(2);
        assert_eq!(
            mapper.insert_child(&root, child_name.clone(), None, |value_creator_params| {
                assert!(value_creator_params.existing_data.is_none());
                42
            }),
            Ok(first_child_inode.clone())
        );

        // Insert a child with the same name
        assert_eq!(
            mapper.insert_child(&root, child_name.clone(), None, |value_creator_params| {
                assert_eq!(value_creator_params.existing_data, Some(&42));
                84
            }),
            Ok(first_child_inode.clone())
        );

        // Verify that the child was indeed replaced
        let lookup_result = mapper.lookup(&root, child_name.as_os_str());
        assert!(lookup_result.is_some());
        assert_eq!(*lookup_result.unwrap().data, 84);
    }

    #[test]
    fn test_insert_multiple_children() {
        let mut mapper = InodeMultiMapper::<u64, u64>::new(0);
        let children: Vec<(
            OsString,
            Option<u64>,
            Box<dyn Fn(ValueCreatorParams<u64>) -> u64>,
        )> = vec![
            (OsString::from("child1"), None, Box::new(|_| 10)),
            (OsString::from("child2"), None, Box::new(|_| 20)),
            (OsString::from("child3"), None, Box::new(|_| 30)),
        ];

        let result = mapper.insert_children(&ROOT_INODE, children);

        assert!(result.is_ok());
        let inserted_inodes = result.unwrap();
        assert_eq!(inserted_inodes.len(), 3);

        for (i, inode) in inserted_inodes.iter().enumerate() {
            let child_name = OsString::from(format!("child{}", i + 1));
            let child_value = mapper.lookup(&ROOT_INODE, &child_name).unwrap();
            assert_eq!(child_value.inode, inode);
            assert_eq!(
                child_value.links.get(&ROOT_INODE),
                Some(&HashSet::from([OsStringWrapper(Arc::new(
                    child_name.clone()
                ))]))
            );
            assert_eq!(*child_value.data, (i as u64 + 1) * 10);
        }
    }

    #[test]
    fn test_batch_insert_large_entries_varying_depths() {
        let mut mapper = InodeMultiMapper::<u64, u64>::new(0);
        let mut entries = Vec::new();
        let mut expected_inodes = HashSet::new();

        const FILE_COUNT: usize = 50;
        // Create a large number of entries with varying depths
        for i in 0..FILE_COUNT as u64 {
            let depth = i % 5; // Vary depth from 0 to 4
            let mut path = Vec::new();
            for j in 0..depth {
                path.push(OsString::from(format!("dir_{}", j)));
            }
            path.push(OsString::from(format!("file_{}", i)));
            entries.push((path, None, move |_: ValueCreatorParams<u64>| i));
            expected_inodes.insert(fuser::INodeNo(i + 2)); // Start from 2 to avoid conflict with root_inode
        }

        // Perform batch insert
        let result = mapper.batch_insert(&ROOT_INODE, entries, |_: ValueCreatorParams<u64>| 0);

        // Verify results
        assert!(result.is_ok(), "Batch insert should succeed");

        // Check if all inserted inodes exist
        for i in 2..=(FILE_COUNT as u64 + 1) {
            let inode = fuser::INodeNo(i);
            assert!(mapper.get(&inode).is_some(), "{:?} should exist", inode);
        }

        // Verify the structure for a few sample paths
        let sample_paths = vec![
            vec!["file_0"],
            vec!["dir_0", "file_1"],
            vec!["dir_0", "dir_1", "file_2"],
            vec!["dir_0", "dir_1", "dir_2", "file_3"],
            vec!["dir_0", "dir_1", "dir_2", "dir_3", "file_4"],
        ];

        for (i, path) in sample_paths.iter().enumerate() {
            let mut current_inode = ROOT_INODE.clone();
            for (j, component) in path.iter().enumerate() {
                let lookup_result = mapper.lookup(&current_inode, OsStr::new(component));
                assert!(
                    lookup_result.is_some(),
                    "Failed to find {} in path {:?}",
                    component,
                    path
                );
                let lookup_result_unwraped = lookup_result.unwrap();
                if j == path.len() - 1 {
                    assert_eq!(
                        *lookup_result_unwraped.data, i as u64,
                        "Incorrect data for file {}",
                        i
                    );
                }
                current_inode = lookup_result_unwraped.inode.clone();
            }
        }
    }

    #[test]
    fn test_resolve_inode_to_full_path() {
        let mut mapper = InodeMultiMapper::<(), u64>::new(());

        let dir_inode = mapper
            .insert_child(
                &mapper.get_root_inode(),
                OsString::from("dir"),
                None,
                |_| (),
            )
            .unwrap();
        let file_inode = mapper
            .insert_child(&dir_inode, OsString::from("file.txt"), None, |_| ())
            .unwrap();

        // Resolve the file inode
        let path = mapper.resolve(&file_inode).unwrap();

        // Check the resolved path (it should be in reverse order)
        assert_eq!(path.len(), 2);
        assert_eq!(
            path[0]
                .links
                .values()
                .next()
                .unwrap()
                .iter()
                .next()
                .unwrap()
                .as_ref()
                .to_str()
                .unwrap(),
            "file.txt"
        );
        assert_eq!(
            path[1]
                .links
                .values()
                .next()
                .unwrap()
                .iter()
                .next()
                .unwrap()
                .as_ref()
                .to_str()
                .unwrap(),
            "dir"
        );

        // Resolve the root inode (should be empty)
        let root_path = mapper.resolve(&ROOT_INODE).unwrap();
        assert!(root_path.is_empty());

        // Try to resolve a non-existent inode
        assert!(mapper.resolve(&fuser::INodeNo(999)).is_none());
    }

    #[test]
    fn test_resolve_invalid_inode() {
        let mapper = InodeMultiMapper::<u64, u64>::new(0);
        let invalid_inode = fuser::INodeNo(999);

        // Attempt to resolve an invalid inode
        let result = mapper.resolve(&invalid_inode);

        // Assert that the result is None
        assert!(
            result.is_none(),
            "Resolving an invalid inode should return None"
        );
    }

    #[test]
    fn test_rename_child_inode() {
        let mut mapper = InodeMultiMapper::<(), u64>::new(());
        let root = mapper.get_root_inode();

        // Insert initial structure
        let parent1 = mapper
            .insert_child(&root, OsString::from("parent1"), None, |_| ())
            .unwrap();
        let parent2 = mapper
            .insert_child(&root, OsString::from("parent2"), None, |_| ())
            .unwrap();
        let child = mapper
            .insert_child(&parent1, OsString::from("old_name"), None, |_| ())
            .unwrap();
        mapper
            .insert_child(&parent2, OsString::from("dummy"), None, |_| ())
            .unwrap();

        // Perform rename
        let result = mapper.rename(
            &parent1,
            OsStr::new("old_name"),
            &parent2,
            OsString::from("new_name"),
        );

        // Assert successful rename
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);

        // Verify new location
        let renamed_child = mapper.lookup(&parent2, OsStr::new("new_name"));
        assert!(renamed_child.is_some());
        assert_eq!(renamed_child.unwrap().inode, &child);

        // Verify old location is empty
        assert!(mapper.lookup(&parent1, OsStr::new("old_name")).is_none());

        // Verify inode data is updated
        let inode_value = mapper.get(&child).unwrap();
        assert_eq!(inode_value.links.get(&parent1), None);
        assert_eq!(
            inode_value.links.get(&parent2),
            Some(&HashSet::from([OsStringWrapper(Arc::new(OsString::from(
                "new_name"
            )))]))
        );
    }

    #[test]
    fn test_should_not_prematurely_purge_old_inode_after_renaming() {
        // Data fields of all inodes in this test are 1 to simulate reflection of the FUSE inode refcount
        let mut mapper = InodeMultiMapper::<u64, u64>::new(1u64);
        let root = mapper.get_root_inode();

        let parent1 = mapper
            .insert_child(&root, OsString::from("parent1"), None, |_| 1)
            .unwrap();
        let parent2 = mapper
            .insert_child(&root, OsString::from("parent2"), None, |_| 1)
            .unwrap();
        let child1 = mapper
            .insert_child(&parent1, OsString::from("child1"), None, |_| 1)
            .unwrap();
        let child2 = mapper
            .insert_child(&parent2, OsString::from("child2"), None, |_| 1)
            .unwrap();

        // Rename child1 to child2
        mapper
            .rename(
                &parent1,
                OsStr::new("child1"),
                &parent2,
                OsString::from("child2"),
            )
            .expect("should be able to insert inode");
        assert!(
            mapper.get(&child1).is_some(),
            "first inode should be present"
        );
        assert!(
            mapper.get(&child1).unwrap().links.contains_key(&parent2),
            "first inode should point to parent2 as parent"
        );
        assert!(
            mapper
                .get_children(&parent2)
                .contains(&(&Arc::new(OsString::from("child2")), &child1)),
            "first inode should be in parent2's child node list"
        );
        assert!(
            !mapper
                .get_children(&parent2)
                .contains(&(&Arc::new(OsString::from("child2")), &child2)),
            "second inode should no longer be in parent2's child node list"
        );
        assert!(
            mapper.get(&child2).is_some(),
            "second inode must be present as an orphaned inode but not removed immediately"
        );
    }

    #[test]
    fn test_rename_child_inode_into_empty_dir_inode() {
        let mut mapper = InodeMultiMapper::<(), u64>::new(());
        let root = mapper.get_root_inode();

        // Insert initial structure
        let parent1 = mapper
            .insert_child(&root, OsString::from("parent1"), None, |_| ())
            .unwrap();
        let parent2 = mapper
            .insert_child(&parent1, OsString::from("parent2"), None, |_| ())
            .unwrap();
        let child = mapper
            .insert_child(&root, OsString::from("test_name"), None, |_| ())
            .unwrap();

        // Perform rename
        let result = mapper.rename(
            &root,
            OsStr::new("test_name"),
            &parent2,
            OsString::from("test_name"),
        );

        // Assert successful rename
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);

        // Verify new location
        let renamed_child = mapper.lookup(&parent2, OsStr::new("test_name"));
        assert!(renamed_child.is_some());
        assert_eq!(renamed_child.unwrap().inode, &child);

        // Verify old location is empty
        assert!(mapper.lookup(&root, OsStr::new("test_name")).is_none());

        // Verify inode data is updated
        let inode_value = mapper.get(&child).unwrap();
        assert_eq!(
            inode_value.links.get(&parent2),
            Some(&HashSet::from([OsStringWrapper(Arc::new(OsString::from(
                "test_name"
            )))]))
        );

        // Perform rename back to original path
        let result = mapper.rename(
            &parent2,
            OsStr::new("test_name"),
            &root,
            OsString::from("test_name"),
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);

        // Verify new location
        let renamed_child = mapper.lookup(&root, OsStr::new("test_name"));
        assert!(renamed_child.is_some());
        assert_eq!(renamed_child.unwrap().inode, &child);

        // Verify old location is empty
        assert!(mapper.lookup(&parent2, OsStr::new("test_name")).is_none());

        // Verify inode data is updated
        let inode_value = mapper.get(&child).unwrap();
        assert_eq!(
            inode_value.links.get(&root),
            Some(&HashSet::from([OsStringWrapper(Arc::new(OsString::from(
                "test_name"
            )))]))
        );
    }

    #[test]
    fn test_rename_non_existent_child() {
        let mut mapper = InodeMultiMapper::<u64, u64>::new(0);

        // Insert parent inodes
        let root = mapper.get_root_inode();
        let parent = mapper
            .insert_child(&root, OsString::from("parent"), None, |_| 1)
            .unwrap();
        let newparent = mapper
            .insert_child(&root, OsString::from("newparent"), None, |_| 2)
            .unwrap();

        // Attempt to rename a non-existent child
        let result = mapper.rename(
            &parent,
            OsStr::new("non_existent"),
            &newparent,
            OsString::from("new_name"),
        );

        assert!(matches!(result, Err(RenameError::NotFound)));
    }
}
