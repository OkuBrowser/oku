//! JointPrefixSet, that is implemened as a simple binary tree, based on the
//! [`super::JointPrefixMap`].

use crate::AsView;
use crate::PrefixSet;
use either::{Left, Right};

use super::{map::CoverKeys, JointPrefix};

/// Set of prefixes, organized in a tree. This strucutre gives efficient access to the longest
/// prefix in the set that contains another prefix.
///
/// Access the individual sets `self.t1` and `self.t2` to perform set operations (using
/// [`crate::AsView`]).
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(bound(
        serialize = "P::P1: serde::Serialize, P::P2: serde::Serialize",
        deserialize = "P::P1: serde::Deserialize<'de>, P::P2: serde::Deserialize<'de>",
    ))
)]
pub struct JointPrefixSet<P: JointPrefix> {
    /// PrefixSet that corresponds to the first prefix type
    pub t1: PrefixSet<P::P1>,
    /// PrefixSet that corresponds to the second prefix type
    pub t2: PrefixSet<P::P2>,
}

impl<P: JointPrefix> JointPrefixSet<P> {
    /// Create a new, empty prefixset.
    pub fn new() -> Self {
        Self {
            t1: PrefixSet::new(),
            t2: PrefixSet::new(),
        }
    }

    /// Returns the number of elements stored in `self`.
    ///
    /// ```
    /// # use prefix_trie::joint::*;
    /// # #[cfg(feature = "ipnet")]
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut set: JointPrefixSet<ipnet::IpNet> = JointPrefixSet::default();
    /// set.insert("192.168.1.0/24".parse()?);
    /// set.insert("192.168.1.0/25".parse()?);
    /// set.insert("2001::1:0:0/96".parse()?);
    /// # let set = set.clone();
    /// assert_eq!(set.len(), 3);
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "ipnet"))]
    /// # fn main() {}
    /// ```
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.t1.len() + self.t2.len()
    }

    /// Returns `true` if the set contains no elements.
    ///
    /// ```
    /// # use prefix_trie::joint::*;
    /// # #[cfg(feature = "ipnet")]
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut set: JointPrefixSet<ipnet::IpNet> = JointPrefixSet::new();
    /// assert!(set.is_empty());
    /// set.insert("2001::1:0:0/96".parse()?);
    /// assert!(!set.is_empty());
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "ipnet"))]
    /// # fn main() {}
    /// ```
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Check wether some prefix is present in the set, without using longest prefix match.
    ///
    /// ```
    /// # use prefix_trie::joint::*;
    /// # #[cfg(feature = "ipnet")]
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut set: JointPrefixSet<ipnet::IpNet> = JointPrefixSet::new();
    /// set.insert("192.168.1.0/24".parse()?);
    /// assert!(set.contains(&"192.168.1.0/24".parse()?));
    /// assert!(!set.contains(&"192.168.2.0/24".parse()?));
    /// assert!(!set.contains(&"192.168.0.0/23".parse()?));
    /// assert!(!set.contains(&"192.168.1.128/25".parse()?));
    /// assert!(!set.contains(&"c0a8:1::/24".parse()?));
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "ipnet"))]
    /// # fn main() {}
    /// ```
    pub fn contains(&self, prefix: &P) -> bool {
        fork_ref!(self, prefix, contains)
    }

    /// Get a reference to the stored prefix. This function allows you to retrieve the host part of
    /// the prefix. The returned prefix will always have the same network address and prefix length.
    ///
    /// ```
    /// # use prefix_trie::joint::*;
    /// # #[cfg(feature = "ipnet")]
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut set: JointPrefixSet<ipnet::IpNet> = JointPrefixSet::new();
    /// set.insert("192.168.0.254/24".parse()?);
    /// assert_eq!(set.get(&"192.168.0.0/24".parse()?), Some("192.168.0.254/24".parse()?));
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "ipnet"))]
    /// # fn main() {}
    /// ```
    pub fn get(&self, prefix: &P) -> Option<P> {
        fork_ref!(self, prefix as P, get)
    }

    /// Get the longest prefix in the set that contains the given preifx.
    ///
    /// ```
    /// # use prefix_trie::joint::*;
    /// # #[cfg(feature = "ipnet")]
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut set: JointPrefixSet<ipnet::IpNet> = JointPrefixSet::new();
    /// set.insert("192.168.1.0/24".parse()?);
    /// set.insert("192.168.0.0/23".parse()?);
    /// assert_eq!(set.get_lpm(&"192.168.1.1/32".parse()?), Some("192.168.1.0/24".parse()?));
    /// assert_eq!(set.get_lpm(&"192.168.1.0/24".parse()?), Some("192.168.1.0/24".parse()?));
    /// assert_eq!(set.get_lpm(&"192.168.0.0/24".parse()?), Some("192.168.0.0/23".parse()?));
    /// assert_eq!(set.get_lpm(&"192.168.2.0/24".parse()?), None);
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "ipnet"))]
    /// # fn main() {}
    /// ```
    pub fn get_lpm(&self, prefix: &P) -> Option<P> {
        fork_ref!(self, prefix as P, get_lpm)
    }

    /// Get the shortest prefix in the set that contains the given preifx.
    ///
    /// ```
    /// # use prefix_trie::joint::*;
    /// # #[cfg(feature = "ipnet")]
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut set: JointPrefixSet<ipnet::IpNet> = JointPrefixSet::new();
    /// set.insert("192.168.1.0/24".parse()?);
    /// set.insert("192.168.0.0/23".parse()?);
    /// assert_eq!(set.get_spm(&"192.168.1.1/32".parse()?), Some("192.168.0.0/23".parse()?));
    /// assert_eq!(set.get_spm(&"192.168.1.0/24".parse()?), Some("192.168.0.0/23".parse()?));
    /// assert_eq!(set.get_spm(&"192.168.0.0/23".parse()?), Some("192.168.0.0/23".parse()?));
    /// assert_eq!(set.get_spm(&"192.168.2.0/24".parse()?), None);
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "ipnet"))]
    /// # fn main() {}
    /// ```
    pub fn get_spm(&self, prefix: &P) -> Option<P> {
        fork_ref!(self, prefix as P, get_spm)
    }

    /// Adds a value to the set.
    ///
    /// Returns whether the value was newly inserted. That is:
    /// - If the set did not previously contain this value, `true` is returned.
    /// - If the set already contained this value, `false` is returned.
    ///
    /// This operation will always replace the currently stored prefix. This allows you to store
    /// additional information in the host aprt of the prefix.
    ///
    /// ```
    /// # use prefix_trie::joint::*;
    /// # #[cfg(feature = "ipnet")]
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut set: JointPrefixSet<ipnet::IpNet> = JointPrefixSet::new();
    /// assert!(set.insert("192.168.0.0/23".parse()?));
    /// assert!(set.insert("192.168.1.0/24".parse()?));
    /// assert!(!set.insert("192.168.1.0/24".parse()?));
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "ipnet"))]
    /// # fn main() {}
    /// ```
    pub fn insert(&mut self, prefix: P) -> bool {
        fork!(self, prefix, insert)
    }

    /// Removes a value from the set. Returns whether the value was present in the set.
    ///
    /// ```
    /// # use prefix_trie::joint::*;
    /// # #[cfg(feature = "ipnet")]
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut set: JointPrefixSet<ipnet::IpNet> = JointPrefixSet::new();
    /// let prefix = "192.168.1.0/24".parse()?;
    /// set.insert(prefix);
    /// assert!(set.contains(&prefix));
    /// assert!(set.remove(&prefix));
    /// assert!(!set.contains(&prefix));
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "ipnet"))]
    /// # fn main() {}
    /// ```
    pub fn remove(&mut self, prefix: &P) -> bool {
        fork_ref!(self, prefix, remove)
    }

    /// Removes a prefix from the set, returning wether the prefix was present or not. In contrast
    /// to [`Self::remove`], his operation will keep the tree structure as is, but only remove the
    /// element from it. This allows any future `insert` on the same prefix to be faster. However
    /// future reads from the tree might be a bit slower because they need to traverse more nodes.
    ///
    /// ```
    /// # use prefix_trie::joint::*;
    /// # #[cfg(feature = "ipnet")]
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut set: JointPrefixSet<ipnet::IpNet> = JointPrefixSet::new();
    /// let prefix = "192.168.1.0/24".parse()?;
    /// set.insert(prefix);
    /// assert!(set.contains(&prefix));
    /// assert!(set.remove_keep_tree(&prefix));
    /// assert!(!set.contains(&prefix));
    ///
    /// // future inserts of the same key are now faster!
    /// set.insert(prefix);
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "ipnet"))]
    /// # fn main() {}
    /// ```
    pub fn remove_keep_tree(&mut self, prefix: &P) -> bool {
        fork_ref!(self, prefix, remove_keep_tree)
    }

    /// Remove all elements that are contained within `prefix`. This will change the tree
    /// structure. This operation is `O(n)`, as the entries must be freed up one-by-one.
    ///
    /// ```
    /// # use prefix_trie::joint::*;
    /// # #[cfg(feature = "ipnet")]
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut set: JointPrefixSet<ipnet::IpNet> = JointPrefixSet::new();
    /// set.insert("192.168.0.0/22".parse()?);
    /// set.insert("192.168.0.0/23".parse()?);
    /// set.insert("192.168.0.0/24".parse()?);
    /// set.insert("192.168.2.0/23".parse()?);
    /// set.insert("192.168.2.0/24".parse()?);
    /// set.insert("c0a8::/24".parse()?);
    /// set.remove_children(&"192.168.0.0/23".parse()?);
    /// assert!(!set.contains(&"192.168.0.0/23".parse()?));
    /// assert!(!set.contains(&"192.168.0.0/24".parse()?));
    /// assert!(set.contains(&"192.168.2.0/23".parse()?));
    /// assert!(set.contains(&"192.168.2.0/24".parse()?));
    /// assert!(set.contains(&"c0a8::/24".parse()?));
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "ipnet"))]
    /// # fn main() {}
    /// ```
    pub fn remove_children(&mut self, prefix: &P) {
        fork_ref!(self, prefix, remove_children)
    }

    /// Clear the set but keep the allocated memory.
    ///
    /// ```
    /// # use prefix_trie::joint::*;
    /// # #[cfg(feature = "ipnet")]
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut set: JointPrefixSet<ipnet::IpNet> = JointPrefixSet::new();
    /// set.insert("192.168.0.0/24".parse()?);
    /// set.insert("192.168.1.0/24".parse()?);
    /// set.insert("2001::1:0:0/96".parse()?);
    /// set.clear();
    /// assert!(!set.contains(&"192.168.0.0/24".parse()?));
    /// assert!(!set.contains(&"192.168.1.0/24".parse()?));
    /// assert!(!set.contains(&"2001::1:0:0/96".parse()?));
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "ipnet"))]
    /// # fn main() {}
    /// ```
    pub fn clear(&mut self) {
        self.t1.clear();
        self.t2.clear();
    }

    /// Iterate over all prefixes in the set. It iterates over the first, and then over the second
    /// set.
    ///
    /// ```
    /// # use prefix_trie::joint::*;
    /// # #[cfg(feature = "ipnet")]
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut set: JointPrefixSet<ipnet::IpNet> = JointPrefixSet::new();
    /// set.insert("192.168.1.0/24".parse()?);
    /// set.insert("192.168.0.0/24".parse()?);
    /// set.insert("2001::1:0:0/96".parse()?);
    /// assert_eq!(
    ///     set.iter().collect::<Vec<_>>(),
    ///     vec![
    ///         "192.168.0.0/24".parse()?,
    ///         "192.168.1.0/24".parse()?,
    ///         "2001::1:0:0/96".parse()?
    ///     ],
    /// );
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "ipnet"))]
    /// # fn main() {}
    /// ```
    pub fn iter(&self) -> Iter<'_, P> {
        self.into_iter()
    }

    /// Keep only the elements in the map that satisfy the given condition `f`.
    ///
    /// ```
    /// # use prefix_trie::joint::*;
    /// # #[cfg(feature = "ipnet")]
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut set: JointPrefixSet<ipnet::IpNet> = JointPrefixSet::new();
    /// set.insert("192.168.0.0/24".parse()?);
    /// set.insert("192.168.1.0/24".parse()?);
    /// set.insert("192.168.2.0/24".parse()?);
    /// set.insert("192.168.2.0/25".parse()?);
    /// set.insert("2001::/24".parse()?);
    /// set.insert("2001::/25".parse()?);
    /// set.retain(|p| p.prefix_len() == 24);
    /// assert!(set.contains(&"192.168.0.0/24".parse()?));
    /// assert!(set.contains(&"192.168.1.0/24".parse()?));
    /// assert!(set.contains(&"192.168.2.0/24".parse()?));
    /// assert!(!set.contains(&"192.168.2.0/25".parse()?));
    /// assert!(set.contains(&"2001::/24".parse()?));
    /// assert!(!set.contains(&"2001::/25".parse()?));
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "ipnet"))]
    /// # fn main() {}
    /// ```
    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(P) -> bool,
    {
        self.t1.retain(|p| f(P::from_p1(p)));
        self.t2.retain(|p| f(P::from_p2(p)));
    }

    /// Get an iterator over the node itself and all children. All elements returned have a prefix
    /// that is contained within `prefix` itself (or are the same). The iterator yields elements in
    /// lexicographic order.
    ///
    /// **Info**: Use the [`crate::trieview::TrieView`] abstraction that provides more flexibility.
    ///
    /// ```
    /// # use prefix_trie::joint::*;
    /// # #[cfg(feature = "ipnet")]
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut set: JointPrefixSet<ipnet::IpNet> = JointPrefixSet::new();
    /// set.insert("192.168.0.0/22".parse()?);
    /// set.insert("192.168.0.0/23".parse()?);
    /// set.insert("192.168.2.0/23".parse()?);
    /// set.insert("192.168.0.0/24".parse()?);
    /// set.insert("192.168.2.0/24".parse()?);
    /// assert_eq!(
    ///     set.children(&"192.168.0.0/23".parse()?).collect::<Vec<_>>(),
    ///     vec![
    ///         "192.168.0.0/23".parse()?,
    ///         "192.168.0.0/24".parse()?,
    ///     ]
    /// );
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "ipnet"))]
    /// # fn main() {}
    /// ```
    pub fn children<'a>(&'a self, prefix: &P) -> Iter<'a, P> {
        Iter(match prefix.p1_or_p2_ref() {
            Left(p1) => super::map::Iter {
                i1: Some(self.t1.0.children(p1)),
                i2: None,
            },
            Right(p2) => super::map::Iter {
                i1: None,
                i2: Some(self.t2.0.children(p2)),
            },
        })
    }

    /// Iterate over all prefixes in the set that covers the given `prefix` (including `prefix`
    /// itself if that is present in the set). The returned iterator yields `&'a P`.
    ///
    /// The iterator will always yield elements ordered by their prefix length, i.e., their depth in
    /// the tree.
    ///
    /// ```
    /// # use prefix_trie::joint::*;
    /// # #[cfg(feature = "ipnet")]
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut set: JointPrefixSet<ipnet::IpNet> = JointPrefixSet::new();
    /// let p0 = "10.0.0.0/8".parse()?;
    /// let p1 = "10.1.0.0/16".parse()?;
    /// let p2 = "10.1.1.0/24".parse()?;
    /// set.insert(p0);
    /// set.insert(p1);
    /// set.insert(p2);
    /// set.insert("10.1.2.0/24".parse()?); // disjoint prefixes are not covered
    /// set.insert("10.1.1.0/25".parse()?); // more specific prefixes are not covered
    /// set.insert("11.0.0.0/8".parse()?);  // Branch points that don't contain values are skipped
    /// assert_eq!(set.cover(&p2).collect::<Vec<_>>(), vec![p0, p1, p2]);
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "ipnet"))]
    /// # fn main() {}
    /// ```
    pub fn cover<'a, 'p>(&'a self, prefix: &'p P) -> CoverKeys<'a, 'p, P, ()> {
        CoverKeys(match prefix.p1_or_p2_ref() {
            Left(p1) => super::map::Cover::P1(self.t1.0.cover(p1)),
            Right(p2) => super::map::Cover::P2(self.t2.0.cover(p2)),
        })
    }

    /// Iterate over the union of two joint prefix sets. This is roughly equivalent to calling
    /// `self.t1.view().union(&other.t1).chain(self.t2.view().union(&other.t2))`.
    ///
    /// If a prefix is present in both trees, the iterator will yield both elements. Otherwise, the
    /// iterator will yield the element of one map together with the longest prefix match in
    /// the other map. Elements are of type `P`.
    ///
    /// ```
    /// # use prefix_trie::joint::*;
    /// # use prefix_trie::joint::map::UnionItem;
    /// # #[cfg(feature = "ipnet")]
    /// macro_rules! net { ($x:literal) => {$x.parse::<ipnet::IpNet>().unwrap()}; }
    ///
    /// # #[cfg(feature = "ipnet")]
    /// # {
    /// let mut set_a: JointPrefixSet<ipnet::IpNet> = JointPrefixSet::from_iter([
    ///     net!("2001::1:0:0/96"),
    ///     net!("192.168.0.0/22"),
    ///     net!("192.168.0.0/24"),
    /// ]);
    /// let mut set_b: JointPrefixSet<ipnet::IpNet> = JointPrefixSet::from_iter([
    ///     net!("192.168.0.0/22"),
    ///     net!("192.168.0.0/23"),
    /// ]);
    /// assert_eq!(
    ///     set_a.union(&set_b).collect::<Vec<_>>(),
    ///     vec![
    ///         net!("192.168.0.0/22"),
    ///         net!("192.168.0.0/23"),
    ///         net!("192.168.0.0/24"),
    ///         net!("2001::1:0:0/96"),
    ///     ]
    /// );
    /// # }
    /// ```
    pub fn union<'a>(&'a self, other: &'a JointPrefixSet<P>) -> Union<'a, P> {
        Union(super::map::Union {
            i1: Some(self.t1.view().union(&other.t1)),
            i2: Some(self.t2.view().union(&other.t2)),
        })
    }

    /// Iterate over the intersection of two joint prefix sets. This is roughly equivalent to
    /// calling `self.t1.view().intersection(&other.t1).chain(self.t2.view().intersection(&other.t2))`.
    ///
    /// ```
    /// # use prefix_trie::joint::*;
    /// # #[cfg(feature = "ipnet")]
    /// macro_rules! net { ($x:literal) => {$x.parse::<ipnet::IpNet>().unwrap()}; }
    ///
    /// # #[cfg(feature = "ipnet")]
    /// # {
    /// let mut set_a: JointPrefixSet<ipnet::IpNet> = JointPrefixSet::from_iter([
    ///     net!("192.168.0.0/20"),
    ///     net!("192.168.0.0/22"),
    ///     net!("192.168.0.0/24"),
    ///     net!("192.168.2.0/23"),
    ///     net!("2001::1:0:0/96"),
    ///     net!("2001::1:0:0/97"),
    /// ]);
    /// let mut set_b: JointPrefixSet<ipnet::IpNet> = JointPrefixSet::from_iter([
    ///     net!("192.168.0.0/20"),
    ///     net!("192.168.0.0/22"),
    ///     net!("192.168.0.0/23"),
    ///     net!("192.168.0.0/24"),
    ///     net!("192.168.2.0/24"),
    ///     net!("2001::1:0:0/96"),
    ///     net!("2001::0:0:0/97"),
    /// ]);
    /// assert_eq!(
    ///     set_a.intersection(&set_b).collect::<Vec<_>>(),
    ///     vec![
    ///         net!("192.168.0.0/20"),
    ///         net!("192.168.0.0/22"),
    ///         net!("192.168.0.0/24"),
    ///         net!("2001::1:0:0/96"),
    ///     ]
    /// );
    /// # }
    /// ```
    pub fn intersection<'a>(&'a self, other: &'a JointPrefixSet<P>) -> Intersection<'a, P> {
        Intersection(super::map::Intersection {
            i1: Some(self.t1.view().intersection(&other.t1)),
            i2: Some(self.t2.view().intersection(&other.t2)),
        })
    }

    /// Iterate over the all elements in `self` that are not present in `other`. Each item will
    /// return a reference to the prefix and value in `self`, as well as the longest prefix match of
    /// `other`.
    ///
    /// ```
    /// # use prefix_trie::joint::*;
    /// # #[cfg(feature = "ipnet")]
    /// macro_rules! net { ($x:literal) => {$x.parse::<ipnet::IpNet>().unwrap()}; }
    ///
    /// # #[cfg(feature = "ipnet")]
    /// # {
    /// let mut set_a: JointPrefixSet<ipnet::IpNet> = JointPrefixSet::from_iter([
    ///     net!("192.168.0.0/20"),
    ///     net!("192.168.0.0/22"),
    ///     net!("192.168.0.0/24"),
    ///     net!("192.168.2.0/23"),
    ///     net!("2001::1:0:0/96"),
    ///     net!("2001::1:0:0/97"),
    /// ]);
    /// let mut set_b: JointPrefixSet<ipnet::IpNet> = JointPrefixSet::from_iter([
    ///     net!("192.168.0.0/20"),
    ///     net!("192.168.0.0/22"),
    ///     net!("192.168.0.0/23"),
    ///     net!("192.168.2.0/24"),
    ///     net!("2001::1:0:0/96"),
    /// ]);
    /// assert_eq!(
    ///     set_a.difference(&set_b).collect::<Vec<_>>(),
    ///     vec![
    ///         (net!("192.168.0.0/24"), Some(net!("192.168.0.0/23"))),
    ///         (net!("192.168.2.0/23"), Some(net!("192.168.0.0/22"))),
    ///         (net!("2001::1:0:0/97"), Some(net!("2001::1:0:0/96"))),
    ///     ]
    /// );
    /// # }
    /// ```
    pub fn difference<'a>(&'a self, other: &'a JointPrefixSet<P>) -> Difference<'a, P> {
        Difference(super::map::Difference {
            i1: Some(self.t1.view().difference(&other.t1)),
            i2: Some(self.t2.view().difference(&other.t2)),
        })
    }

    /// Iterate over the all elements in `self` that are not covered by `other`.
    ///
    /// ```
    /// # use prefix_trie::joint::*;
    /// # #[cfg(feature = "ipnet")]
    /// macro_rules! net { ($x:literal) => {$x.parse::<ipnet::IpNet>().unwrap()}; }
    ///
    /// # #[cfg(feature = "ipnet")]
    /// # {
    /// let mut set_a: JointPrefixSet<ipnet::IpNet> = JointPrefixSet::from_iter([
    ///     net!("192.168.0.0/20"),
    ///     net!("192.168.0.0/22"),
    ///     net!("192.168.0.0/24"),
    ///     net!("192.168.2.0/23"),
    ///     net!("2001::0:0:0/95"),
    ///     net!("2001::1:0:0/96"),
    ///     net!("2001::1:0:0/97"),
    /// ]);
    /// let mut set_b: JointPrefixSet<ipnet::IpNet> = JointPrefixSet::from_iter([
    ///     net!("192.168.0.0/21"),
    ///     net!("192.168.0.0/22"),
    ///     net!("192.168.0.0/23"),
    ///     net!("192.168.2.0/24"),
    ///     net!("2001::1:0:0/96"),
    /// ]);
    /// assert_eq!(
    ///     set_a.covering_difference(&set_b).collect::<Vec<_>>(),
    ///     vec![net!("192.168.0.0/20"), net!("2001::0:0:0/95")]
    /// );
    /// # }
    /// ```
    pub fn covering_difference<'a>(
        &'a self,
        other: &'a JointPrefixSet<P>,
    ) -> CoveringDifference<'a, P> {
        CoveringDifference(super::map::CoveringDifference {
            i1: Some(self.t1.view().covering_difference(&other.t1)),
            i2: Some(self.t2.view().covering_difference(&other.t2)),
        })
    }
}

impl<P: JointPrefix> Default for JointPrefixSet<P> {
    fn default() -> Self {
        Self::new()
    }
}

impl<P> PartialEq for JointPrefixSet<P>
where
    P: JointPrefix + PartialEq,
{
    /// Compare two prefix sets to contain the same prefixes. This also compares the host-part of
    /// the prefix:
    ///
    /// ```
    /// # use prefix_trie::joint::*;
    /// # #[cfg(feature = "ipnet")]
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut set1: JointPrefixSet<ipnet::IpNet> = ["10.0.0.0/8".parse()?].into_iter().collect();
    /// let mut set2: JointPrefixSet<ipnet::IpNet> = ["10.0.0.1/8".parse()?].into_iter().collect();
    /// assert_ne!(set1, set2);
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "ipnet"))]
    /// # fn main() {}
    /// ```
    fn eq(&self, other: &Self) -> bool {
        self.iter().zip(other.iter()).all(|(a, b)| a == b)
    }
}

impl<P> Eq for JointPrefixSet<P> where P: JointPrefix + Eq {}

/// An iterator over all entries of a [`JointPrefixSet`] in lexicographic order.
pub struct Iter<'a, P: JointPrefix>(super::map::Iter<'a, P, ()>);

impl<P: JointPrefix> Default for Iter<'_, P> {
    /// The default iterator is empty.
    ///
    /// ```
    /// use prefix_trie::joint;
    /// # #[cfg(feature = "ipnet")]
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// assert_eq!(joint::set::Iter::<ipnet::IpNet>::default().count(), 0);
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "ipnet"))]
    /// # fn main() {}
    /// ```
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<P: JointPrefix> Clone for Iter<'_, P> {
    /// You can clone an iterator, which maintains its state.
    ///
    /// ```
    /// # use prefix_trie::joint::*;
    /// # #[cfg(feature = "ipnet")]
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut set: JointPrefixSet<ipnet::IpNet> = JointPrefixSet::new();
    /// set.insert("2001::1:0:0/96".parse()?);
    /// set.insert("192.168.0.0/22".parse()?);
    /// set.insert("192.168.0.0/23".parse()?);
    /// let mut iter = set.iter();
    ///
    /// assert_eq!(iter.next(), Some("192.168.0.0/22".parse()?));
    ///
    /// let clone = iter.clone();
    /// assert_eq!(
    ///     iter.collect::<Vec<_>>(),
    ///     vec!["192.168.0.0/23".parse()?, "2001::1:0:0/96".parse()?]
    /// );
    /// assert_eq!(
    ///     clone.collect::<Vec<_>>(),
    ///     vec!["192.168.0.0/23".parse()?, "2001::1:0:0/96".parse()?]
    /// );
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "ipnet"))]
    /// # fn main() {}
    /// ```
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<P: JointPrefix> Iterator for Iter<'_, P> {
    type Item = P;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(p, _)| p)
    }
}

#[derive(Clone)]
/// A consuming iterator over all entries of a [`JointPrefixSet`] in lexicographic order.
pub struct IntoIter<P: JointPrefix>(super::map::IntoIter<P, ()>);

impl<P: JointPrefix> Iterator for IntoIter<P> {
    type Item = P;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(p, _)| p)
    }
}

impl<P: JointPrefix> IntoIterator for JointPrefixSet<P> {
    type Item = P;

    type IntoIter = IntoIter<P>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter(super::map::IntoIter {
            i1: Some(self.t1.0.into_iter()),
            i2: Some(self.t2.0.into_iter()),
        })
    }
}

impl<'a, P: JointPrefix> IntoIterator for &'a JointPrefixSet<P> {
    type Item = P;

    type IntoIter = Iter<'a, P>;

    fn into_iter(self) -> Self::IntoIter {
        Iter(super::map::Iter {
            i1: Some(self.t1.0.iter()),
            i2: Some(self.t2.0.iter()),
        })
    }
}

impl<P: JointPrefix> FromIterator<P> for JointPrefixSet<P> {
    fn from_iter<I: IntoIterator<Item = P>>(iter: I) -> Self {
        let mut set = Self::new();
        for p in iter {
            set.insert(p);
        }
        set
    }
}

/// An iterator over the union of two [`JointPrefixSet`]s. The iterator yields first prefixes of
/// `P1`, followed by those of `P2`.
pub struct Union<'a, P: JointPrefix>(super::map::Union<'a, P, (), ()>);

impl<P: JointPrefix> Iterator for Union<'_, P> {
    type Item = P;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|x| x.into_prefix())
    }
}

/// An iterator over the intersection of two [`JointPrefixSet`]s. The iterator yields first prefixes of
/// `P1`, followed by those of `P2`.
pub struct Intersection<'a, P: JointPrefix>(super::map::Intersection<'a, P, (), ()>);

impl<P: JointPrefix> Iterator for Intersection<'_, P> {
    type Item = P;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(p, _, _)| p)
    }
}

/// An iterator over the difference of two [`JointPrefixSet`]s. The iterator yields prefixes that
/// are in `self`, but ont in `other`. The iterator also yields the longest prefix match in `other`
/// (if it exists). The iterator yields first prefixes of `P1`, followed by those of `P2`.
pub struct Difference<'a, P: JointPrefix>(super::map::Difference<'a, P, (), ()>);

impl<P: JointPrefix> Iterator for Difference<'_, P> {
    type Item = (P, Option<P>);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|x| (x.prefix, x.right.map(|(p, _)| p)))
    }
}

/// An iterator over the covering difference of two [`JointPrefixSet`]s. The iterator yields
/// prefixes that are in `self`, but not covered by any prefix in `other`. The iterator yields first
/// prefixes of `P1`, followed by those of `P2`.
pub struct CoveringDifference<'a, P: JointPrefix>(super::map::CoveringDifference<'a, P, (), ()>);

impl<P: JointPrefix> Iterator for CoveringDifference<'_, P> {
    type Item = P;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(p, _)| p)
    }
}
