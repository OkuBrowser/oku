use std::num::NonZeroUsize;

use crate::to_right;

use super::*;

/// An iterator over the union of two TrieViews that always yields either the exact value or the
/// longest prefix match of both of them.
pub struct Union<'a, P, L, R> {
    table_l: &'a Table<P, L>,
    table_r: &'a Table<P, R>,
    nodes: Vec<Node<'a, P, L, R>>,
}

/// An iterator over the union of two TrieViews that always yields either the exact value or the
/// longest prefix match of both of them.
pub struct UnionMut<'a, P, L, R> {
    // Safety: table_l must be distinct from table_r
    table_l: &'a Table<P, L>,
    // Safety: table_l must be distinct from table_r
    table_r: &'a Table<P, R>,
    nodes: Vec<UnionIndex>,
}

impl<'a, P, L, R> UnionMut<'a, P, L, R> {
    /// Safety:
    /// 1. Table_l must come from a `TrieViewMut` and satisfy the conditions in `TrieViewMut::new`.
    /// 2. Table_r must come from a `TrieViewMut` and satisfy the conditions in `TrieViewMut::new`.
    /// 3. Table_l and Table_r must be distinct. This is implicitly given by the two rules above.
    unsafe fn new(
        table_l: &'a Table<P, L>,
        table_r: &'a Table<P, R>,
        nodes: Vec<UnionIndex>,
    ) -> Self {
        Self {
            table_l,
            table_r,
            nodes,
        }
    }
}

type Lpm<'a, P, T> = Option<(&'a P, &'a T)>;
type Node<'a, P, L, R> = (UnionIndex, Lpm<'a, P, L>, Lpm<'a, P, R>);

/// An item of the [`Union`] iterator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum UnionItem<'a, P, L, R> {
    /// The prefix is only present in the left TrieView (`self`).
    Left {
        /// The prefix of the element.
        prefix: &'a P,
        /// The value of the element in the left TrieView (`self`).
        left: &'a L,
        /// The longest prefix match in the right TrieView (`other`).
        right: Option<(&'a P, &'a R)>,
    },
    /// The prefix is only present in the right TrieView (`other`).
    Right {
        /// The prefix of the element.
        prefix: &'a P,
        /// The longest prefix match in the left TrieView (`self`).
        left: Option<(&'a P, &'a L)>,
        /// The value of the element in the right TrieView (`other`).
        right: &'a R,
    },
    /// The prefix is only present in the right TrieView (`other`).
    Both {
        /// The prefix of the element.
        prefix: &'a P,
        /// The value of the element in the left TrieView (`self`).
        left: &'a L,
        /// The value of the element in the right TrieView (`other`).
        right: &'a R,
    },
}

impl<'a, P, L, R> UnionItem<'a, P, L, R> {
    /// Get the prefix of the current element (in the exact match).
    pub fn prefix(&self) -> &'a P {
        match self {
            UnionItem::Left { prefix, .. }
            | UnionItem::Right { prefix, .. }
            | UnionItem::Both { prefix, .. } => prefix,
        }
    }

    /// Get the element in both the left and right map, but only if they are both present. By
    /// doing `a.union(b).filter_map(|x| x.both)`, you get an iterator that yields only elements
    /// that are present in both tries.
    pub fn both(&self) -> Option<(&'a P, &'a L, &'a R)> {
        match self {
            UnionItem::Left { .. } | UnionItem::Right { .. } => None,
            UnionItem::Both {
                prefix,
                left,
                right,
            } => Some((prefix, left, right)),
        }
    }

    /// Get the value of the left item (`self`). This either returns the exact match or the
    /// longest-prefix match.
    pub fn left(&self) -> Option<(&'a P, &'a L)> {
        match self {
            UnionItem::Right { left, .. } => *left,
            UnionItem::Left { prefix, left, .. } | UnionItem::Both { prefix, left, .. } => {
                Some((prefix, left))
            }
        }
    }

    /// Get the value of the right item (`other`). This either returns the exact match or the
    /// longest-prefix match.
    pub fn right(&self) -> Option<(&'a P, &'a R)> {
        match self {
            UnionItem::Left { right, .. } => *right,
            UnionItem::Right { prefix, right, .. } | UnionItem::Both { prefix, right, .. } => {
                Some((prefix, right))
            }
        }
    }
}

enum UnionIndex {
    Both(usize, usize),
    FirstL(usize, usize),
    FirstR(usize, usize),
    OnlyL(usize),
    OnlyR(usize),
}

impl<'a, P, L> TrieView<'a, P, L>
where
    P: Prefix,
{
    /// Iterate over the union of two views. If a prefix is present in both trees, the iterator
    /// will yield both elements. Otherwise, the iterator will yield the element of one TrieView
    /// together with the longest prefix match in the other TrieView. Elements are of type
    /// [`UnionItem`].
    ///
    /// **Warning**: The iterator will only yield elements of the given TrieViews. If either of the
    /// two TrieViews is pointing to a branching or a virtual node, then the longest prefix match
    /// returned may be `None`, even though it exists in the larger tree.
    ///
    /// ```
    /// # use prefix_trie::*;
    /// # use prefix_trie::trieview::UnionItem;
    /// # #[cfg(feature = "ipnet")]
    /// macro_rules! net { ($x:literal) => {$x.parse::<ipnet::Ipv4Net>().unwrap()}; }
    ///
    /// # #[cfg(feature = "ipnet")]
    /// # {
    /// let mut map_a: PrefixMap<ipnet::Ipv4Net, usize> = PrefixMap::from_iter([
    ///     (net!("192.168.0.0/20"), 1),
    ///     (net!("192.168.0.0/22"), 2),
    ///     (net!("192.168.0.0/24"), 3),
    /// ]);
    /// let mut map_b: PrefixMap<ipnet::Ipv4Net, &'static str> = PrefixMap::from_iter([
    ///     (net!("192.168.0.0/22"), "a"),
    ///     (net!("192.168.0.0/23"), "b"),
    /// ]);
    /// assert_eq!(
    ///     map_a.view().union(&map_b).collect::<Vec<_>>(),
    ///     vec![
    ///         UnionItem::Left{
    ///             prefix: &net!("192.168.0.0/20"),
    ///             left: &1,
    ///             right: None,
    ///         },
    ///         UnionItem::Both{
    ///             prefix: &net!("192.168.0.0/22"),
    ///             left: &2,
    ///             right: &"a",
    ///         },
    ///         UnionItem::Right{
    ///             prefix: &net!("192.168.0.0/23"),
    ///             left: Some((&net!("192.168.0.0/22"), &2)),
    ///             right: &"b",
    ///         },
    ///         UnionItem::Left{
    ///             prefix: &net!("192.168.0.0/24"),
    ///             left: &3,
    ///             right: Some((&net!("192.168.0.0/23"), &"b")),
    ///         },
    ///     ]
    /// );
    /// # }
    /// ```
    pub fn union<R>(&self, other: &'a impl AsView<P = P, T = R>) -> Union<'a, P, L, R> {
        let other = other.view();
        Union {
            table_l: self.table,
            table_r: other.table,
            nodes: extend_lpm(
                self.table,
                other.table,
                self.table[self.loc.idx()].prefix_value(),
                other.table[other.loc.idx()].prefix_value(),
                next_indices(
                    self.table,
                    other.table,
                    Some(self.loc.idx()),
                    Some(other.loc.idx()),
                ),
            )
            .collect(),
        }
    }
}

impl<P, L> TrieViewMut<'_, P, L>
where
    P: Prefix,
{
    /// Iterate over the union of two views. If a prefix is present in both trees, the iterator
    /// will yield mutable references to both elements. Longest prefix matches are not returned.
    ///
    /// ```
    /// # use prefix_trie::*;
    /// # use prefix_trie::trieview::UnionItem;
    /// # #[cfg(feature = "ipnet")]
    /// macro_rules! net { ($x:literal) => {$x.parse::<ipnet::Ipv4Net>().unwrap()}; }
    ///
    /// # #[cfg(feature = "ipnet")]
    /// # {
    /// let mut map_a: PrefixMap<ipnet::Ipv4Net, usize> = PrefixMap::from_iter([
    ///     (net!("192.168.0.0/20"), 1),
    ///     (net!("192.168.0.0/22"), 2),
    ///     (net!("192.168.0.0/24"), 3),
    ///     (net!("192.168.2.0/23"), 4),
    /// ]);
    /// let mut map_b: PrefixMap<ipnet::Ipv4Net, usize> = PrefixMap::from_iter([
    ///     (net!("192.168.0.0/22"), 10),
    ///     (net!("192.168.0.0/23"), 20),
    ///     (net!("192.168.2.0/24"), 30),
    /// ]);
    ///
    /// // Modify the two maps by adding their values for elements of the same prefix.
    /// for (_, l, r) in map_a.view_mut().union_mut(&mut map_b) {
    ///     if let (Some(l), Some(r)) = (l, r) {
    ///         *l += *r;
    ///         *r = *l;
    ///     }
    /// }
    ///
    /// assert_eq!(
    ///     map_a.into_iter().collect::<Vec<_>>(),
    ///     vec![
    ///         (net!("192.168.0.0/20"), 1),
    ///         (net!("192.168.0.0/22"), 12),
    ///         (net!("192.168.0.0/24"), 3),
    ///         (net!("192.168.2.0/23"), 4),
    ///     ],
    /// );
    /// assert_eq!(
    ///     map_b.into_iter().collect::<Vec<_>>(),
    ///     vec![
    ///         (net!("192.168.0.0/22"), 12),
    ///         (net!("192.168.0.0/23"), 20),
    ///         (net!("192.168.2.0/24"), 30),
    ///     ],
    /// );
    /// # }
    /// ```
    pub fn union_mut<'b, R>(
        &'b mut self,
        other: impl AsViewMut<'b, P, R>,
    ) -> UnionMut<'b, P, L, R> {
        let other = other.view_mut();
        let nodes = next_indices(
            self.table,
            other.table,
            Some(self.loc.idx()),
            Some(other.loc.idx()),
        );
        // Safety: We take the reference to the table from two TrieViewMut. Since they both have to
        // be created using TrieViewMut::new, we satisfy the conditions in `UnionMut::new`.
        unsafe { UnionMut::new(self.table, other.table, nodes) }
    }
}

impl<'a, P: Prefix, L, R> Union<'a, P, L, R> {
    fn extend(
        &mut self,
        indices: impl IntoIterator<Item = UnionIndex> + 'static,
        lpm_l: Lpm<'a, P, L>,
        lpm_r: Lpm<'a, P, R>,
    ) {
        self.nodes.extend(extend_lpm(
            self.table_l,
            self.table_r,
            lpm_l,
            lpm_r,
            indices,
        ));
    }

    fn get_next(
        &self,
        prefix: &'a P,
        l: Option<&'a L>,
        r: Option<&'a R>,
        lpm_l: Lpm<'a, P, L>,
        lpm_r: Lpm<'a, P, R>,
    ) -> Option<UnionItem<'a, P, L, R>> {
        match (l, r) {
            (None, None) => None,
            (None, Some(right)) => Some(UnionItem::Right {
                prefix,
                left: lpm_l,
                right,
            }),
            (Some(left), None) => Some(UnionItem::Left {
                prefix,
                left,
                right: lpm_r,
            }),
            (Some(left), Some(right)) => Some(UnionItem::Both {
                prefix,
                left,
                right,
            }),
        }
    }
}

impl<'a, P: Prefix, L, R> Iterator for Union<'a, P, L, R> {
    type Item = UnionItem<'a, P, L, R>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((cur, lpm_l, lpm_r)) = self.nodes.pop() {
            match cur {
                UnionIndex::Both(l, r) => {
                    let node_l = &self.table_l[l];
                    let node_r = &self.table_r[r];
                    self.extend(
                        next_indices(self.table_l, self.table_r, node_l.right, node_r.right),
                        lpm_l,
                        lpm_r,
                    );
                    self.extend(
                        next_indices(self.table_l, self.table_r, node_l.left, node_r.left),
                        lpm_l,
                        lpm_r,
                    );
                    if let Some(x) = self.get_next(
                        &node_l.prefix,
                        node_l.value.as_ref(),
                        node_r.value.as_ref(),
                        lpm_l,
                        lpm_r,
                    ) {
                        return Some(x);
                    }
                }
                UnionIndex::FirstL(l, r) => {
                    let node_l = &self.table_l[l];
                    self.extend(
                        next_indices_first_l(
                            self.table_l,
                            self.table_r,
                            l,
                            node_l.left,
                            node_l.right,
                            r,
                        ),
                        lpm_l,
                        lpm_r,
                    );
                    if let Some(x) =
                        self.get_next(&node_l.prefix, node_l.value.as_ref(), None, lpm_l, lpm_r)
                    {
                        return Some(x);
                    }
                }
                UnionIndex::FirstR(l, r) => {
                    let node_r = &self.table_r[r];
                    self.extend(
                        next_indices_first_r(
                            self.table_l,
                            self.table_r,
                            l,
                            r,
                            node_r.left,
                            node_r.right,
                        ),
                        lpm_l,
                        lpm_r,
                    );
                    if let Some(x) =
                        self.get_next(&node_r.prefix, None, node_r.value.as_ref(), lpm_l, lpm_r)
                    {
                        return Some(x);
                    }
                }
                UnionIndex::OnlyL(l) => {
                    let node_l = &self.table_l[l];
                    if let Some(right) = node_l.right {
                        self.extend([UnionIndex::OnlyL(right.get())], lpm_l, lpm_r);
                    }
                    if let Some(left) = node_l.left {
                        self.extend([UnionIndex::OnlyL(left.get())], lpm_l, lpm_r);
                    }
                    if let Some(x) =
                        self.get_next(&node_l.prefix, node_l.value.as_ref(), None, lpm_l, lpm_r)
                    {
                        return Some(x);
                    }
                }
                UnionIndex::OnlyR(r) => {
                    let node_r = &self.table_r[r];
                    if let Some(right) = node_r.right {
                        self.extend([UnionIndex::OnlyR(right.get())], lpm_l, lpm_r);
                    }
                    if let Some(left) = node_r.left {
                        self.extend([UnionIndex::OnlyR(left.get())], lpm_l, lpm_r);
                    }
                    if let Some(x) =
                        self.get_next(&node_r.prefix, None, node_r.value.as_ref(), lpm_l, lpm_r)
                    {
                        return Some(x);
                    }
                }
            }
        }
        None
    }
}

impl<'a, P: Prefix, L, R> Iterator for UnionMut<'a, P, L, R> {
    type Item = (&'a P, Option<&'a mut L>, Option<&'a mut R>);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(cur) = self.nodes.pop() {
            // safety: map is a tree. Every node is visited exactly once during the iteration
            // (self.nodes is not public). Therefore, each in each iteration of this loop (also
            // between multiple calls to `next`), the index `cur` is different to any of the earlier
            // iterations. It is therefore safe to extend the lifetime of the elements to 'a (which
            // is the lifetime for which `self` has an exclusive reference over the map).
            match cur {
                UnionIndex::Both(l, r) => {
                    let node_l = &self.table_l[l];
                    let node_r = &self.table_r[r];
                    self.nodes.extend(next_indices(
                        self.table_l,
                        self.table_r,
                        node_l.right,
                        node_r.right,
                    ));
                    self.nodes.extend(next_indices(
                        self.table_l,
                        self.table_r,
                        node_l.left,
                        node_r.left,
                    ));
                    let node_l = unsafe { self.table_l.get_mut(l) };
                    let node_r = unsafe { self.table_r.get_mut(r) };
                    if node_l.value.is_some() || node_r.value.is_some() {
                        return Some((
                            &node_l.prefix,
                            node_l.value.as_mut(),
                            node_r.value.as_mut(),
                        ));
                    }
                }
                UnionIndex::FirstL(l, r) => {
                    let node_l = &self.table_l[l];
                    self.nodes.extend(next_indices_first_l(
                        self.table_l,
                        self.table_r,
                        l,
                        node_l.left,
                        node_l.right,
                        r,
                    ));
                    let node_l = unsafe { self.table_l.get_mut(l) };
                    if node_l.value.is_some() {
                        return Some((&node_l.prefix, node_l.value.as_mut(), None));
                    }
                }
                UnionIndex::FirstR(l, r) => {
                    let node_r = &self.table_r[r];
                    self.nodes.extend(next_indices_first_r(
                        self.table_l,
                        self.table_r,
                        l,
                        r,
                        node_r.left,
                        node_r.right,
                    ));
                    let node_r = unsafe { self.table_r.get_mut(r) };
                    if node_r.value.is_some() {
                        return Some((&node_r.prefix, None, node_r.value.as_mut()));
                    }
                }
                UnionIndex::OnlyL(l) => {
                    let node_l = unsafe { self.table_l.get_mut(l) };
                    if let Some(right) = node_l.right {
                        self.nodes.push(UnionIndex::OnlyL(right.get()));
                    }
                    if let Some(left) = node_l.left {
                        self.nodes.push(UnionIndex::OnlyL(left.get()));
                    }
                    if node_l.value.is_some() {
                        return Some((&node_l.prefix, node_l.value.as_mut(), None));
                    }
                }
                UnionIndex::OnlyR(r) => {
                    let node_r = unsafe { self.table_r.get_mut(r) };
                    if let Some(right) = node_r.right {
                        self.nodes.push(UnionIndex::OnlyR(right.get()));
                    }
                    if let Some(left) = node_r.left {
                        self.nodes.push(UnionIndex::OnlyR(left.get()));
                    }
                    if node_r.value.is_some() {
                        return Some((&node_r.prefix, None, node_r.value.as_mut()));
                    }
                }
            }
        }
        None
    }
}

fn next_indices<'a, P: Prefix, L, R>(
    table_l: &'a Table<P, L>,
    table_r: &'a Table<P, R>,
    l: Option<impl Into<usize>>,
    r: Option<impl Into<usize>>,
) -> Vec<UnionIndex> {
    match (l.map(|l| l.into()), r.map(|r| r.into())) {
        (None, Some(r)) => vec![UnionIndex::OnlyR(r)],
        (Some(l), None) => vec![UnionIndex::OnlyL(l)],
        (Some(l), Some(r)) => {
            let p_l = &table_l[l].prefix;
            let p_r = &table_r[r].prefix;
            if p_l.prefix_len() == p_r.prefix_len() {
                match p_l.mask().cmp(&p_r.mask()) {
                    std::cmp::Ordering::Less => {
                        vec![UnionIndex::OnlyR(r), UnionIndex::OnlyL(l)]
                    }
                    std::cmp::Ordering::Equal => {
                        vec![UnionIndex::Both(l, r)]
                    }
                    std::cmp::Ordering::Greater => {
                        vec![UnionIndex::OnlyL(l), UnionIndex::OnlyR(r)]
                    }
                }
            } else if p_l.contains(p_r) {
                vec![UnionIndex::FirstL(l, r)]
            } else if p_r.contains(p_l) {
                vec![UnionIndex::FirstR(l, r)]
            } else {
                if p_l.mask() < p_r.mask() {
                    vec![UnionIndex::OnlyR(r), UnionIndex::OnlyL(l)]
                } else {
                    vec![UnionIndex::OnlyL(l), UnionIndex::OnlyR(r)]
                }
            }
        }
        _ => vec![],
    }
}

fn next_indices_first_l<'a, P: Prefix, L, R>(
    table_l: &'a Table<P, L>,
    table_r: &'a Table<P, R>,
    l: usize,
    ll: Option<NonZeroUsize>,
    lr: Option<NonZeroUsize>,
    r: usize,
) -> Vec<UnionIndex> {
    match (ll, lr) {
        (None, None) => vec![UnionIndex::OnlyR(r)],
        (None, Some(lr)) => next_indices(table_l, table_r, Some(lr), Some(r)),
        (Some(ll), None) => next_indices(table_l, table_r, Some(ll), Some(r)),
        (Some(ll), Some(lr)) => {
            if to_right(&table_l[l].prefix, &table_r[r].prefix) {
                let mut idxes = next_indices(table_l, table_r, Some(lr), Some(r));
                idxes.push(UnionIndex::OnlyL(ll.get()));
                idxes
            } else {
                let mut idxes = next_indices(table_l, table_r, Some(ll), Some(r));
                idxes.insert(0, UnionIndex::OnlyL(lr.get()));
                idxes
            }
        }
    }
}

fn next_indices_first_r<'a, P: Prefix, L, R>(
    table_l: &'a Table<P, L>,
    table_r: &'a Table<P, R>,
    l: usize,
    r: usize,
    rl: Option<NonZeroUsize>,
    rr: Option<NonZeroUsize>,
) -> Vec<UnionIndex> {
    match (rl, rr) {
        (None, None) => vec![UnionIndex::OnlyL(l)],
        (None, Some(rr)) => next_indices(table_l, table_r, Some(l), Some(rr)),
        (Some(rl), None) => next_indices(table_l, table_r, Some(l), Some(rl)),
        (Some(rl), Some(rr)) => {
            if to_right(&table_r[r].prefix, &table_l[l].prefix) {
                let mut idxes = next_indices(table_l, table_r, Some(l), Some(rr));
                idxes.push(UnionIndex::OnlyR(rl.get()));
                idxes
            } else {
                let mut idxes = next_indices(table_l, table_r, Some(l), Some(rl));
                idxes.insert(0, UnionIndex::OnlyR(rr.get()));
                idxes
            }
        }
    }
}

fn extend_lpm<'a, P: Prefix, L, R>(
    table_l: &'a Table<P, L>,
    table_r: &'a Table<P, R>,
    lpm_l: Lpm<'a, P, L>,
    lpm_r: Lpm<'a, P, R>,
    indices: impl IntoIterator<Item = UnionIndex> + 'static,
) -> impl Iterator<Item = Node<'a, P, L, R>> + 'a {
    let get_lpm_l = move |l: usize| table_l[l].prefix_value().or(lpm_l);
    let get_lpm_r = move |r: usize| table_r[r].prefix_value().or(lpm_r);
    indices.into_iter().map(move |x| match x {
        UnionIndex::Both(l, r) => (x, get_lpm_l(l), get_lpm_r(r)),
        UnionIndex::FirstL(l, _) | UnionIndex::OnlyL(l) => (x, get_lpm_l(l), lpm_r),
        UnionIndex::FirstR(_, r) | UnionIndex::OnlyR(r) => (x, lpm_l, get_lpm_r(r)),
    })
}
