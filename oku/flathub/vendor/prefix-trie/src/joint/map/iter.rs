//! Module that contains the implementation for the iterators

use crate::joint::JointPrefix;

use super::JointPrefixMap;

/// An iterator over all entries of a [`JointPrefixMap`] in lexicographic order.
pub struct Iter<'a, P: JointPrefix, T> {
    pub(crate) i1: Option<crate::map::Iter<'a, P::P1, T>>,
    pub(crate) i2: Option<crate::map::Iter<'a, P::P2, T>>,
}

impl<P: JointPrefix, T> Default for Iter<'_, P, T> {
    /// The default iterator is empty.
    ///
    /// ```
    /// use prefix_trie::joint;
    /// # #[cfg(feature = "ipnet")]
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// assert_eq!(joint::map::Iter::<ipnet::IpNet, usize>::default().count(), 0);
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "ipnet"))]
    /// # fn main() {}
    /// ```
    fn default() -> Self {
        Self { i1: None, i2: None }
    }
}

impl<P: JointPrefix, T> Clone for Iter<'_, P, T> {
    /// You can clone an iterator, which maintains its state.
    ///
    /// ```
    /// # use prefix_trie::joint::*;
    /// # #[cfg(feature = "ipnet")]
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut pm: JointPrefixMap<ipnet::IpNet, _> = JointPrefixMap::new();
    /// pm.insert("2001::1:0:0/96".parse()?, 1);
    /// pm.insert("192.168.0.0/22".parse()?, 2);
    /// pm.insert("192.168.0.0/23".parse()?, 3);
    /// let mut iter = pm.iter();
    ///
    /// assert_eq!(iter.next(), Some(("192.168.0.0/22".parse()?, &2)));
    ///
    /// let clone = iter.clone();
    /// assert_eq!(
    ///     iter.collect::<Vec<_>>(),
    ///     vec![
    ///         ("192.168.0.0/23".parse()?, &3),
    ///         ("2001::1:0:0/96".parse()?, &1),
    ///     ]
    /// );
    /// assert_eq!(
    ///     clone.collect::<Vec<_>>(),
    ///     vec![
    ///         ("192.168.0.0/23".parse()?, &3),
    ///         ("2001::1:0:0/96".parse()?, &1),
    ///     ]
    /// );
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "ipnet"))]
    /// # fn main() {}
    /// ```
    fn clone(&self) -> Self {
        Self {
            i1: self.i1.clone(),
            i2: self.i2.clone(),
        }
    }
}

impl<'a, P: JointPrefix, T> Iterator for Iter<'a, P, T> {
    type Item = (P, &'a T);

    fn next(&mut self) -> Option<(P, &'a T)> {
        if let Some(i1) = self.i1.as_mut() {
            if let Some((p, t)) = i1.next() {
                return Some((P::from_p1(p), t));
            }
            // iterator 1 is empty
            self.i1 = None;
        }
        if let Some(i2) = self.i2.as_mut() {
            if let Some((p, t)) = i2.next() {
                return Some((P::from_p2(p), t));
            }
            // iterator 1 is empty
            self.i2 = None;
        }
        None
    }
}

/// An iterator over all prefixes of a [`JointPrefixMap`] in lexicographic order.
#[derive(Clone, Default)]
pub struct Keys<'a, P: JointPrefix, T> {
    pub(crate) inner: Iter<'a, P, T>,
}

impl<P: JointPrefix, T> Iterator for Keys<'_, P, T> {
    type Item = P;

    fn next(&mut self) -> Option<P> {
        self.inner.next().map(|(k, _)| k)
    }
}

/// An iterator over all values of a [`JointPrefixMap`] in lexicographic order of their associated
/// prefixes.
#[derive(Clone, Default)]
pub struct Values<'a, P: JointPrefix, T> {
    pub(crate) inner: Iter<'a, P, T>,
}

impl<'a, P: JointPrefix, T> Iterator for Values<'a, P, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        self.inner.next().map(|(_, v)| v)
    }
}

/// An iterator over all owned entries of a [`JointPrefixMap`] in lexicographic order.
#[derive(Clone)]
pub struct IntoIter<P: JointPrefix, T> {
    pub(crate) i1: Option<crate::map::IntoIter<P::P1, T>>,
    pub(crate) i2: Option<crate::map::IntoIter<P::P2, T>>,
}

impl<P: JointPrefix, T> Iterator for IntoIter<P, T> {
    type Item = (P, T);

    fn next(&mut self) -> Option<(P, T)> {
        if let Some(i1) = self.i1.as_mut() {
            if let Some((p, t)) = i1.next() {
                return Some((P::from_p1(&p), t));
            }
            // iterator 1 is empty
            self.i1 = None;
        }
        if let Some(i2) = self.i2.as_mut() {
            if let Some((p, t)) = i2.next() {
                return Some((P::from_p2(&p), t));
            }
            // iterator 1 is empty
            self.i2 = None;
        }
        None
    }
}

/// An iterator over all prefixes of a [`JointPrefixMap`] in lexicographic order.
#[derive(Clone)]
pub struct IntoKeys<P: JointPrefix, T> {
    pub(crate) inner: IntoIter<P, T>,
}

impl<P: JointPrefix, T> Iterator for IntoKeys<P, T> {
    type Item = P;

    fn next(&mut self) -> Option<P> {
        self.inner.next().map(|(k, _)| k)
    }
}

/// An iterator over all values of a [`JointPrefixMap`] in lexicographic order of their associated
/// prefix.
#[derive(Clone)]
pub struct IntoValues<P: JointPrefix, T> {
    pub(crate) inner: IntoIter<P, T>,
}

impl<P: JointPrefix, T> Iterator for IntoValues<P, T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        self.inner.next().map(|(_, v)| v)
    }
}

impl<P: JointPrefix, T> IntoIterator for JointPrefixMap<P, T> {
    type Item = (P, T);

    type IntoIter = IntoIter<P, T>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            i1: Some(self.t1.into_iter()),
            i2: Some(self.t2.into_iter()),
        }
    }
}

impl<'a, P: JointPrefix, T> IntoIterator for &'a JointPrefixMap<P, T> {
    type Item = (P, &'a T);

    type IntoIter = Iter<'a, P, T>;

    fn into_iter(self) -> Self::IntoIter {
        Iter {
            i1: Some(self.t1.iter()),
            i2: Some(self.t2.iter()),
        }
    }
}

/// A mutable iterator over a [`JointPrefixMap`]. This iterator yields elements in lexicographic order of
/// their associated prefix.
pub struct IterMut<'a, P: JointPrefix, T> {
    pub(super) i1: Option<crate::map::IterMut<'a, P::P1, T>>,
    pub(super) i2: Option<crate::map::IterMut<'a, P::P2, T>>,
}

impl<P: JointPrefix, T> Default for IterMut<'_, P, T> {
    /// The default iterator is empty.
    ///
    /// ```
    /// use prefix_trie::joint;
    /// # #[cfg(feature = "ipnet")]
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// assert_eq!(joint::map::IterMut::<ipnet::IpNet, usize>::default().count(), 0);
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "ipnet"))]
    /// # fn main() {}
    /// ```
    fn default() -> Self {
        Self { i1: None, i2: None }
    }
}

impl<'a, P: JointPrefix, T> Iterator for IterMut<'a, P, T> {
    type Item = (P, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(i1) = self.i1.as_mut() {
            if let Some((p, t)) = i1.next() {
                return Some((P::from_p1(p), t));
            }
            // iterator 1 is empty
            self.i1 = None;
        }
        if let Some(i2) = self.i2.as_mut() {
            if let Some((p, t)) = i2.next() {
                return Some((P::from_p2(p), t));
            }
            // iterator 1 is empty
            self.i2 = None;
        }
        None
    }
}

/// A mutable iterator over values of [`JointPrefixMap`]. This iterator yields elements in
/// lexicographic order.
#[derive(Default)]
pub struct ValuesMut<'a, P: JointPrefix, T> {
    // # Safety
    // You must ensure that there only ever exists one such iterator for each tree. You may create
    // multiple such iterators for the same tree if you start with distinct starting nodes! This
    // ensures that any one iteration will never yield elements of the other iterator.
    pub(crate) inner: IterMut<'a, P, T>,
}

impl<'a, P: JointPrefix, T> Iterator for ValuesMut<'a, P, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(_, v)| v)
    }
}

impl<P: JointPrefix, T> FromIterator<(P, T)> for JointPrefixMap<P, T> {
    fn from_iter<I: IntoIterator<Item = (P, T)>>(iter: I) -> Self {
        let mut map = Self::new();
        iter.into_iter().for_each(|(p, v)| {
            map.insert(p, v);
        });
        map
    }
}

/// An iterator that yields all items in a `JointPrefixMap` that covers a given prefix (including
/// the prefix itself if preseint). See [`crate::joint::JointPrefixMap::cover`] for how to create
/// this iterator.
pub enum Cover<'a, 'p, P: JointPrefix, T> {
    /// Cover of the first prefix variant
    P1(crate::map::Cover<'a, 'p, P::P1, T>),
    /// Cover of the second prefix variant
    P2(crate::map::Cover<'a, 'p, P::P2, T>),
}

impl<'a, P: JointPrefix, T> Iterator for Cover<'a, '_, P, T> {
    type Item = (P, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Cover::P1(cover) => cover.next().map(|(p, t)| (P::from_p1(p), t)),
            Cover::P2(cover) => cover.next().map(|(p, t)| (P::from_p2(p), t)),
        }
    }
}

/// An iterator that yields all keys (prefixes) in a `JointPrefixMap` that covers a given prefix
/// (including the prefix itself if preseint). See [`crate::joint::JointPrefixMap::cover_keys`] for
/// how to create this iterator.
pub struct CoverKeys<'a, 'p, P: JointPrefix, T>(pub(crate) Cover<'a, 'p, P, T>);

impl<P: JointPrefix, T> Iterator for CoverKeys<'_, '_, P, T> {
    type Item = P;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(p, _)| p)
    }
}

/// An iterator that yields all values in a `JointPrefixMap` that covers a given prefix (including
/// the prefix itself if preseint). See [`crate::joint::JointPrefixMap::cover_values`] for how to
/// create this iterator.
pub struct CoverValues<'a, 'p, P: JointPrefix, T>(pub(super) Cover<'a, 'p, P, T>);

impl<'a, P: JointPrefix, T> Iterator for CoverValues<'a, '_, P, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(_, t)| t)
    }
}

/// An iterator over the union of two [`JointPrefixMap`]s. The iterator yields first prefixes of
/// `P1`, followed by those of `P2`.
pub struct Union<'a, P: JointPrefix, L, R> {
    pub(crate) i1: Option<crate::trieview::Union<'a, P::P1, L, R>>,
    pub(crate) i2: Option<crate::trieview::Union<'a, P::P2, L, R>>,
}

impl<'a, P: JointPrefix, L, R> Iterator for Union<'a, P, L, R> {
    type Item = UnionItem<'a, P, L, R>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(i1) = self.i1.as_mut() {
            if let Some(next) = i1.next() {
                return Some(UnionItem::from_p1(next));
            }
            self.i1 = None;
        }
        if let Some(i2) = self.i2.as_mut() {
            if let Some(next) = i2.next() {
                return Some(UnionItem::from_p2(next));
            }
            self.i2 = None;
        }
        None
    }
}

/// An item of the [`Union`] iterator (over a [`JointPrefixMap`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]

pub enum UnionItem<'a, P, L, R> {
    /// The prefix is only present in the left TrieView (`self`).
    Left {
        /// The prefix of the element.
        prefix: P,
        /// The value of the element in the left TrieView (`self`).
        left: &'a L,
        /// The longest prefix match in the right TrieView (`other`).
        right: Option<(P, &'a R)>,
    },

    /// The prefix is only present in the right TrieView (`other`).
    Right {
        /// The prefix of the element.
        prefix: P,
        /// The longest prefix match in the left TrieView (`self`).
        left: Option<(P, &'a L)>,
        /// The value of the element in the right TrieView (`other`).
        right: &'a R,
    },

    /// The prefix is only present in the right TrieView (`other`).
    Both {
        /// The prefix of the element.
        prefix: P,
        /// The value of the element in the left TrieView (`self`).
        left: &'a L,
        /// The value of the element in the right TrieView (`other`).
        right: &'a R,
    },
}

impl<'a, P, L, R> UnionItem<'a, P, L, R> {
    /// Get the prefix of the current element (in the exact match).
    ///
    /// ```
    /// # use prefix_trie::joint::*;
    /// # use prefix_trie::joint::map::UnionItem;
    /// # #[cfg(feature = "ipnet")]
    /// macro_rules! net { ($x:literal) => {$x.parse::<ipnet::IpNet>().unwrap()}; }
    ///
    /// # #[cfg(feature = "ipnet")]
    /// # {
    /// let mut map_a: JointPrefixMap<ipnet::IpNet, usize> = JointPrefixMap::from_iter([
    ///     (net!("2001::1:0:0/96"), 1),
    ///     (net!("192.168.0.0/22"), 2),
    ///     (net!("192.168.0.0/24"), 3),
    /// ]);
    /// let mut map_b: JointPrefixMap<ipnet::IpNet, &'static str> = JointPrefixMap::from_iter([
    ///     (net!("192.168.0.0/22"), "a"),
    ///     (net!("192.168.0.0/23"), "b"),
    /// ]);
    /// assert_eq!(
    ///     map_a.union(&map_b).map(|x| *x.prefix()).collect::<Vec<_>>(),
    ///     vec![
    ///         net!("192.168.0.0/22"),
    ///         net!("192.168.0.0/23"),
    ///         net!("192.168.0.0/24"),
    ///         net!("2001::1:0:0/96"),
    ///     ]
    /// );
    /// # }
    /// ```
    pub fn prefix(&self) -> &P {
        match self {
            UnionItem::Left { prefix, .. }
            | UnionItem::Right { prefix, .. }
            | UnionItem::Both { prefix, .. } => prefix,
        }
    }

    /// Get the prefix of the current element (in the exact match).
    ///
    /// ```
    /// # use prefix_trie::joint::*;
    /// # use prefix_trie::joint::map::UnionItem;
    /// # #[cfg(feature = "ipnet")]
    /// macro_rules! net { ($x:literal) => {$x.parse::<ipnet::IpNet>().unwrap()}; }
    ///
    /// # #[cfg(feature = "ipnet")]
    /// # {
    /// let mut map_a: JointPrefixMap<ipnet::IpNet, usize> = JointPrefixMap::from_iter([
    ///     (net!("2001::1:0:0/96"), 1),
    ///     (net!("192.168.0.0/22"), 2),
    ///     (net!("192.168.0.0/24"), 3),
    /// ]);
    /// let mut map_b: JointPrefixMap<ipnet::IpNet, &'static str> = JointPrefixMap::from_iter([
    ///     (net!("192.168.0.0/22"), "a"),
    ///     (net!("192.168.0.0/23"), "b"),
    /// ]);
    /// assert_eq!(
    ///     map_a.union(&map_b).map(|x| x.into_prefix()).collect::<Vec<_>>(),
    ///     vec![
    ///         net!("192.168.0.0/22"),
    ///         net!("192.168.0.0/23"),
    ///         net!("192.168.0.0/24"),
    ///         net!("2001::1:0:0/96"),
    ///     ]
    /// );
    /// # }
    /// ```
    pub fn into_prefix(self) -> P {
        match self {
            UnionItem::Left { prefix, .. }
            | UnionItem::Right { prefix, .. }
            | UnionItem::Both { prefix, .. } => prefix,
        }
    }

    /// Get the element in both the left and right map, but only if they are both present. By
    /// doing `a.union(b).filter_map(|x| x.both)`, you get an iterator that yields only elements
    /// that are present in both tries.
    ///
    /// ```
    /// # use prefix_trie::joint::*;
    /// # use prefix_trie::joint::map::UnionItem;
    /// # #[cfg(feature = "ipnet")]
    /// macro_rules! net { ($x:literal) => {$x.parse::<ipnet::IpNet>().unwrap()}; }
    ///
    /// # #[cfg(feature = "ipnet")]
    /// # {
    /// let mut map_a: JointPrefixMap<ipnet::IpNet, usize> = JointPrefixMap::from_iter([
    ///     (net!("2001::1:0:0/96"), 1),
    ///     (net!("192.168.0.0/22"), 2),
    ///     (net!("192.168.0.0/24"), 3),
    /// ]);
    /// let mut map_b: JointPrefixMap<ipnet::IpNet, &'static str> = JointPrefixMap::from_iter([
    ///     (net!("192.168.0.0/22"), "a"),
    ///     (net!("192.168.0.0/23"), "b"),
    /// ]);
    /// assert_eq!(
    ///     map_a
    ///         .union(&map_b)
    ///         .filter_map(|x| x.both().map(|(p, l, r)| (*p, l, r)))
    ///         .collect::<Vec<_>>(),
    ///     vec![(net!("192.168.0.0/22"), &2, &"a")]
    /// );
    /// # }
    /// ```
    pub fn both(&self) -> Option<(&P, &'a L, &'a R)> {
        match self {
            UnionItem::Left { .. } | UnionItem::Right { .. } => None,
            UnionItem::Both {
                prefix,
                left,
                right,
            } => Some((prefix, left, right)),
        }
    }

    /// Get the element in both the left and right map, but only if they are both present. By
    /// doing `a.union(b).filter_map(|x| x.both)`, you get an iterator that yields only elements
    /// that are present in both tries.
    ///
    /// ```
    /// # use prefix_trie::joint::*;
    /// # use prefix_trie::joint::map::UnionItem;
    /// # #[cfg(feature = "ipnet")]
    /// macro_rules! net { ($x:literal) => {$x.parse::<ipnet::IpNet>().unwrap()}; }
    ///
    /// # #[cfg(feature = "ipnet")]
    /// # {
    /// let mut map_a: JointPrefixMap<ipnet::IpNet, usize> = JointPrefixMap::from_iter([
    ///     (net!("2001::1:0:0/96"), 1),
    ///     (net!("192.168.0.0/22"), 2),
    ///     (net!("192.168.0.0/24"), 3),
    /// ]);
    /// let mut map_b: JointPrefixMap<ipnet::IpNet, &'static str> = JointPrefixMap::from_iter([
    ///     (net!("192.168.0.0/22"), "a"),
    ///     (net!("192.168.0.0/23"), "b"),
    /// ]);
    /// assert_eq!(
    ///     map_a.union(&map_b).filter_map(|x| x.into_both()).collect::<Vec<_>>(),
    ///     vec![(net!("192.168.0.0/22"), &2, &"a")]
    /// );
    /// # }
    /// ```
    pub fn into_both(self) -> Option<(P, &'a L, &'a R)> {
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
    ///
    /// ```
    /// # use prefix_trie::joint::*;
    /// # use prefix_trie::joint::map::UnionItem;
    /// # #[cfg(feature = "ipnet")]
    /// macro_rules! net { ($x:literal) => {$x.parse::<ipnet::IpNet>().unwrap()}; }
    ///
    /// # #[cfg(feature = "ipnet")]
    /// # {
    /// let mut map_a: JointPrefixMap<ipnet::IpNet, usize> = JointPrefixMap::from_iter([
    ///     (net!("2001::1:0:0/96"), 1),
    ///     (net!("192.168.0.0/22"), 2),
    ///     (net!("192.168.0.0/24"), 3),
    /// ]);
    /// let mut map_b: JointPrefixMap<ipnet::IpNet, &'static str> = JointPrefixMap::from_iter([
    ///     (net!("192.168.0.0/22"), "a"),
    ///     (net!("192.168.0.0/23"), "b"),
    /// ]);
    /// assert_eq!(
    ///     map_a
    ///         .union(&map_b)
    ///         .map(|x| x.left().map(|(p, l)| (*p, l)))
    ///         .collect::<Vec<_>>(),
    ///     vec![
    ///         Some((net!("192.168.0.0/22"), &2)),
    ///         Some((net!("192.168.0.0/22"), &2)),
    ///         Some((net!("192.168.0.0/24"), &3)),
    ///         Some((net!("2001::1:0:0/96"), &1)),
    ///     ]
    /// );
    /// # }
    /// ```
    pub fn left(&self) -> Option<(&P, &'a L)> {
        match self {
            UnionItem::Right { left, .. } => left.as_ref().map(|(p, l)| (p, *l)),
            UnionItem::Left { prefix, left, .. } | UnionItem::Both { prefix, left, .. } => {
                Some((prefix, left))
            }
        }
    }

    /// Get the value of the left item (`self`). This either returns the exact match or the
    /// longest-prefix match.
    ///
    /// ```
    /// # use prefix_trie::joint::*;
    /// # use prefix_trie::joint::map::UnionItem;
    /// # #[cfg(feature = "ipnet")]
    /// macro_rules! net { ($x:literal) => {$x.parse::<ipnet::IpNet>().unwrap()}; }
    ///
    /// # #[cfg(feature = "ipnet")]
    /// # {
    /// let mut map_a: JointPrefixMap<ipnet::IpNet, usize> = JointPrefixMap::from_iter([
    ///     (net!("2001::1:0:0/96"), 1),
    ///     (net!("2001::2:0:0/96"), 2),
    ///     (net!("2001::2:0:0/98"), 3),
    /// ]);
    /// let mut map_b: JointPrefixMap<ipnet::IpNet, &'static str> = JointPrefixMap::from_iter([
    ///     (net!("2001::2:0:0/96"), "a"),
    ///     (net!("2001::2:0:0/97"), "b"),
    /// ]);
    /// assert_eq!(
    ///     map_a.union(&map_b).map(|x| x.into_left()).collect::<Vec<_>>(),
    ///     vec![
    ///         Some((net!("2001::1:0:0/96"), &1)),
    ///         Some((net!("2001::2:0:0/96"), &2)),
    ///         Some((net!("2001::2:0:0/96"), &2)),
    ///         Some((net!("2001::2:0:0/98"), &3)),
    ///     ]
    /// );
    /// # }
    /// ```
    pub fn into_left(self) -> Option<(P, &'a L)> {
        match self {
            UnionItem::Right { left, .. } => left,
            UnionItem::Left { prefix, left, .. } | UnionItem::Both { prefix, left, .. } => {
                Some((prefix, left))
            }
        }
    }

    /// Get the value of the right item (`other`). This either returns the exact match or the
    /// longest-prefix match.
    ///
    /// ```
    /// # use prefix_trie::joint::*;
    /// # use prefix_trie::joint::map::UnionItem;
    /// # #[cfg(feature = "ipnet")]
    /// macro_rules! net { ($x:literal) => {$x.parse::<ipnet::IpNet>().unwrap()}; }
    ///
    /// # #[cfg(feature = "ipnet")]
    /// # {
    /// let mut map_a: JointPrefixMap<ipnet::IpNet, usize> = JointPrefixMap::from_iter([
    ///     (net!("2001::1:0:0/96"), 1),
    ///     (net!("192.168.0.0/22"), 2),
    ///     (net!("192.168.0.0/24"), 3),
    /// ]);
    /// let mut map_b: JointPrefixMap<ipnet::IpNet, &'static str> = JointPrefixMap::from_iter([
    ///     (net!("192.168.0.0/22"), "a"),
    ///     (net!("192.168.0.0/23"), "b"),
    /// ]);
    /// assert_eq!(
    ///     map_a
    ///         .union(&map_b)
    ///         .map(|x| x.right().map(|(p, r)| (*p, r)))
    ///         .collect::<Vec<_>>(),
    ///     vec![
    ///         Some((net!("192.168.0.0/22"), &"a")),
    ///         Some((net!("192.168.0.0/23"), &"b")),
    ///         Some((net!("192.168.0.0/23"), &"b")),
    ///         None,
    ///     ]
    /// );
    /// # }
    /// ```
    pub fn right(&self) -> Option<(&P, &'a R)> {
        match self {
            UnionItem::Left { right, .. } => right.as_ref().map(|(p, r)| (p, *r)),
            UnionItem::Right { prefix, right, .. } | UnionItem::Both { prefix, right, .. } => {
                Some((prefix, right))
            }
        }
    }

    /// Get the value of the right item (`other`). This either returns the exact match or the
    /// longest-prefix match.
    ///
    /// ```
    /// # use prefix_trie::joint::*;
    /// # use prefix_trie::joint::map::UnionItem;
    /// # #[cfg(feature = "ipnet")]
    /// macro_rules! net { ($x:literal) => {$x.parse::<ipnet::IpNet>().unwrap()}; }
    ///
    /// # #[cfg(feature = "ipnet")]
    /// # {
    /// let mut map_a: JointPrefixMap<ipnet::IpNet, usize> = JointPrefixMap::from_iter([
    ///     (net!("2001::1:0:0/96"), 1),
    ///     (net!("2001::2:0:0/96"), 2),
    ///     (net!("2001::2:0:0/98"), 3),
    /// ]);
    /// let mut map_b: JointPrefixMap<ipnet::IpNet, &'static str> = JointPrefixMap::from_iter([
    ///     (net!("2001::2:0:0/96"), "a"),
    ///     (net!("2001::2:0:0/97"), "b"),
    /// ]);
    /// assert_eq!(
    ///     map_a.union(&map_b).map(|x| x.into_right()).collect::<Vec<_>>(),
    ///     vec![
    ///         None,
    ///         Some((net!("2001::2:0:0/96"), &"a")),
    ///         Some((net!("2001::2:0:0/97"), &"b")),
    ///         Some((net!("2001::2:0:0/97"), &"b")),
    ///     ]
    /// );
    /// # }
    /// ```
    pub fn into_right(self) -> Option<(P, &'a R)> {
        match self {
            UnionItem::Left { right, .. } => right,
            UnionItem::Right { prefix, right, .. } | UnionItem::Both { prefix, right, .. } => {
                Some((prefix, right))
            }
        }
    }
}

impl<'a, P: JointPrefix, L, R> UnionItem<'a, P, L, R> {
    fn from_p1(value: crate::trieview::UnionItem<'a, P::P1, L, R>) -> Self {
        match value {
            crate::trieview::UnionItem::Left {
                prefix,
                left,
                right,
            } => UnionItem::Left {
                prefix: P::from_p1(prefix),
                left,
                right: right.map(|(p, r)| (P::from_p1(p), r)),
            },
            crate::trieview::UnionItem::Right {
                prefix,
                left,
                right,
            } => UnionItem::Right {
                prefix: P::from_p1(prefix),
                left: left.map(|(p, l)| (P::from_p1(p), l)),
                right,
            },
            crate::trieview::UnionItem::Both {
                prefix,
                left,
                right,
            } => UnionItem::Both {
                prefix: P::from_p1(prefix),
                left,
                right,
            },
        }
    }

    fn from_p2(value: crate::trieview::UnionItem<'a, P::P2, L, R>) -> Self {
        match value {
            crate::trieview::UnionItem::Left {
                prefix,
                left,
                right,
            } => UnionItem::Left {
                prefix: P::from_p2(prefix),
                left,
                right: right.map(|(p, r)| (P::from_p2(p), r)),
            },
            crate::trieview::UnionItem::Right {
                prefix,
                left,
                right,
            } => UnionItem::Right {
                prefix: P::from_p2(prefix),
                left: left.map(|(p, l)| (P::from_p2(p), l)),
                right,
            },
            crate::trieview::UnionItem::Both {
                prefix,
                left,
                right,
            } => UnionItem::Both {
                prefix: P::from_p2(prefix),
                left,
                right,
            },
        }
    }
}

/// An iterator over the intersection of two [`JointPrefixMap`]s. The iterator yields first prefixes
/// of `P1`, followed by those of `P2`.
pub struct Intersection<'a, P: JointPrefix, L, R> {
    pub(crate) i1: Option<crate::trieview::Intersection<'a, P::P1, L, R>>,
    pub(crate) i2: Option<crate::trieview::Intersection<'a, P::P2, L, R>>,
}

impl<'a, P: JointPrefix, L, R> Iterator for Intersection<'a, P, L, R> {
    type Item = (P, &'a L, &'a R);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(i1) = self.i1.as_mut() {
            if let Some((p, l, r)) = i1.next() {
                return Some((P::from_p1(p), l, r));
            }
            self.i1 = None;
        }
        if let Some(i2) = self.i2.as_mut() {
            if let Some((p, l, r)) = i2.next() {
                return Some((P::from_p2(p), l, r));
            }
            self.i2 = None;
        }
        None
    }
}

/// An iterator over the difference of two [`JointPrefixMap`]s, i.e., prefixes that are in `self`
/// but not in `other`. The iterator yields first prefixes of `P1`, followed by those of `P2`.
pub struct Difference<'a, P: JointPrefix, L, R> {
    pub(crate) i1: Option<crate::trieview::Difference<'a, P::P1, L, R>>,
    pub(crate) i2: Option<crate::trieview::Difference<'a, P::P2, L, R>>,
}

impl<'a, P: JointPrefix, L, R> Iterator for Difference<'a, P, L, R> {
    type Item = DifferenceItem<'a, P, L, R>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(i1) = self.i1.as_mut() {
            if let Some(next) = i1.next() {
                return Some(DifferenceItem::from_p1(next));
            }
            self.i1 = None;
        }
        if let Some(i2) = self.i2.as_mut() {
            if let Some(next) = i2.next() {
                return Some(DifferenceItem::from_p2(next));
            }
            self.i2 = None;
        }
        None
    }
}

/// An iterator over the covering difference of two [`JointPrefixMap`]s, i.e., prefixes of `self` that
/// are not covered by `other`. The iterator yields first prefixes of `P1`, followed by those of
/// `P2`.
pub struct CoveringDifference<'a, P: JointPrefix, L, R> {
    pub(crate) i1: Option<crate::trieview::CoveringDifference<'a, P::P1, L, R>>,
    pub(crate) i2: Option<crate::trieview::CoveringDifference<'a, P::P2, L, R>>,
}

impl<'a, P: JointPrefix, L, R> Iterator for CoveringDifference<'a, P, L, R> {
    type Item = (P, &'a L);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(i1) = self.i1.as_mut() {
            if let Some((p, l)) = i1.next() {
                return Some((P::from_p1(p), l));
            }
            self.i1 = None;
        }
        if let Some(i2) = self.i2.as_mut() {
            if let Some((p, l)) = i2.next() {
                return Some((P::from_p2(p), l));
            }
            self.i2 = None;
        }
        None
    }
}

/// An item of the [`Difference`] iterator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DifferenceItem<'a, P, L, R> {
    /// The prefix that is present in `self` but not in `other`.
    pub prefix: P,
    /// The value stored in `self`.
    pub value: &'a L,
    /// The longest-prefix-match that is present in `other`.
    pub right: Option<(P, &'a R)>,
}

impl<'a, P: JointPrefix, L, R> DifferenceItem<'a, P, L, R> {
    fn from_p1(value: crate::trieview::DifferenceItem<'a, P::P1, L, R>) -> Self {
        Self {
            prefix: P::from_p1(value.prefix),
            value: value.value,
            right: value.right.map(|(p, r)| (P::from_p1(p), r)),
        }
    }

    fn from_p2(value: crate::trieview::DifferenceItem<'a, P::P2, L, R>) -> Self {
        Self {
            prefix: P::from_p2(value.prefix),
            value: value.value,
            right: value.right.map(|(p, r)| (P::from_p2(p), r)),
        }
    }
}
