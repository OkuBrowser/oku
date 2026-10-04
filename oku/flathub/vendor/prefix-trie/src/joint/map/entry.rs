//! Code for inserting elements and the entry pattern.

use crate::joint::JointPrefix;

/// A mutable view into a single entry in a map, which may either be vacant or occupied.
pub enum Entry<'a, P: JointPrefix, T> {
    /// The entry is not present in the tree.
    Vacant(VacantEntry<'a, P, T>),
    /// The entry is already present in the tree.
    Occupied(OccupiedEntry<'a, P, T>),
}

/// A mutable view into a missing entry. The information within this structure describes a path
/// towards that missing node, and how to insert it.
pub enum VacantEntry<'a, P: JointPrefix, T> {
    /// Vacant entry for the first prefix variant
    P1(crate::map::VacantEntry<'a, P::P1, T>),
    /// Vacant entry for the second prefix variant
    P2(crate::map::VacantEntry<'a, P::P2, T>),
}

/// A mutable view into an occupied entry. An occupied entry represents a node that is already
/// present on the tree.
pub enum OccupiedEntry<'a, P: JointPrefix, T> {
    /// Occupied entry for the first prefix variant
    P1(crate::map::OccupiedEntry<'a, P::P1, T>),
    /// Occupied entry for the second prefix variant
    P2(crate::map::OccupiedEntry<'a, P::P2, T>),
}

impl<'a, P: JointPrefix, T> Entry<'a, P, T> {
    /// Get the value if it exists
    pub fn get(&self) -> Option<&T> {
        match self {
            Entry::Vacant(_) => None,
            Entry::Occupied(e) => Some(e.get()),
        }
    }

    /// Get the value if it exists
    pub fn get_mut(&mut self) -> Option<&mut T> {
        match self {
            Entry::Vacant(_) => None,
            Entry::Occupied(e) => Some(e.get_mut()),
        }
    }

    /// get the key of the current entry
    pub fn key(&self) -> P {
        match self {
            Entry::Vacant(e) => e.key(),
            Entry::Occupied(e) => e.key(),
        }
    }

    /// Replace the current entry, and return the entry that was stored before. This will also
    /// replace the key with the one provided to the `entry` function.
    ///
    /// ```
    /// # use prefix_trie::joint::*;
    /// # #[cfg(feature = "ipnet")]
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut pm: JointPrefixMap<ipnet::IpNet, _> = JointPrefixMap::new();
    /// assert_eq!(pm.entry("192.168.1.0/24".parse()?).insert(1), None);
    /// assert_eq!(pm.entry("192.168.1.0/24".parse()?).insert(2), Some(1));
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "ipnet"))]
    /// # fn main() {}
    /// ```
    ///
    /// This function *will replace* the prefix in the map with the one provided to the `entry` call:
    ///
    /// ```
    /// # use prefix_trie::joint::*;
    /// # #[cfg(feature = "ipnet")]
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut pm: JointPrefixMap<ipnet::IpNet, _> = JointPrefixMap::new();
    /// pm.insert("192.168.1.1/24".parse()?, 1);
    /// pm.entry("192.168.1.2/24".parse()?).insert(2);
    /// assert_eq!(
    ///     pm.get_key_value(&"192.168.1.0/24".parse()?),
    ///     Some(("192.168.1.2/24".parse()?, &2)) // prefix is overwritten
    /// );
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "ipnet"))]
    /// # fn main() {}
    /// ```
    #[inline(always)]
    pub fn insert(self, v: T) -> Option<T> {
        match self {
            Entry::Vacant(e) => {
                e.insert(v);
                None
            }
            Entry::Occupied(e) => Some(e.insert(v)),
        }
    }

    /// Ensures a value is in the entry by inserting the default if empty, and returns a mutable
    /// reference to the value in the entry.
    ///
    /// ```
    /// # use prefix_trie::joint::*;
    /// # #[cfg(feature = "ipnet")]
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut pm: JointPrefixMap<ipnet::IpNet, _> = JointPrefixMap::new();
    /// pm.insert("192.168.1.0/24".parse()?, 1);
    /// assert_eq!(pm.entry("192.168.1.0/24".parse()?).or_insert(10), &1);
    /// assert_eq!(pm.entry("192.168.2.0/24".parse()?).or_insert(20), &20);
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "ipnet"))]
    /// # fn main() {}
    /// ```
    ///
    /// This function will *not* replace the prefix in the map if it already exists.
    ///
    /// ```
    /// # use prefix_trie::joint::*;
    /// # #[cfg(feature = "ipnet")]
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut pm: JointPrefixMap<ipnet::IpNet, _> = JointPrefixMap::new();
    /// pm.insert("192.168.1.1/24".parse()?, 1);
    /// pm.entry("192.168.1.2/24".parse()?).or_insert(2);
    /// assert_eq!(
    ///     pm.get_key_value(&"192.168.1.0/24".parse()?),
    ///     Some(("192.168.1.1/24".parse()?, &1)) // prefix is not overwritten.
    /// );
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "ipnet"))]
    /// # fn main() {}
    /// ```
    #[inline(always)]
    pub fn or_insert(self, default: T) -> &'a mut T {
        match self {
            Entry::Vacant(e) => e.insert(default),
            Entry::Occupied(e) => match e {
                OccupiedEntry::P1(e) => e.node.value.get_or_insert(default),
                OccupiedEntry::P2(e) => e.node.value.get_or_insert(default),
            },
        }
    }

    /// Ensures a value is in the entry by inserting the result of the default function if empty,
    /// and returns a mutable reference to the value in the entry.
    ///
    /// ```
    /// # use prefix_trie::joint::*;
    /// # #[cfg(feature = "ipnet")]
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut pm: JointPrefixMap<ipnet::IpNet, _> = JointPrefixMap::new();
    /// pm.insert("192.168.1.0/24".parse()?, 1);
    /// assert_eq!(pm.entry("192.168.1.0/24".parse()?).or_insert_with(|| 10), &1);
    /// assert_eq!(pm.entry("192.168.2.0/24".parse()?).or_insert_with(|| 20), &20);
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "ipnet"))]
    /// # fn main() {}
    /// ```
    ///
    /// This function will *not* replace the prefix in the map if it already exists.
    ///
    /// ```
    /// # use prefix_trie::joint::*;
    /// # #[cfg(feature = "ipnet")]
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut pm: JointPrefixMap<ipnet::IpNet, _> = JointPrefixMap::new();
    /// pm.insert("192.168.1.1/24".parse()?, 1);
    /// pm.entry("192.168.1.2/24".parse()?).or_insert_with(|| 2);
    /// assert_eq!(
    ///     pm.get_key_value(&"192.168.1.0/24".parse()?),
    ///     Some(("192.168.1.1/24".parse()?, &1)) // prefix is not overwritten.
    /// );
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "ipnet"))]
    /// # fn main() {}
    /// ```
    #[inline(always)]
    pub fn or_insert_with<F: FnOnce() -> T>(self, default: F) -> &'a mut T {
        match self {
            Entry::Vacant(e) => e.insert(default()),
            Entry::Occupied(e) => match e {
                OccupiedEntry::P1(e) => e.node.value.get_or_insert_with(default),
                OccupiedEntry::P2(e) => e.node.value.get_or_insert_with(default),
            },
        }
    }

    /// Provides in-place mutable access to an occupied entry before any potential inserts into the
    /// map.
    ///
    /// ```
    /// # use prefix_trie::joint::*;
    /// # #[cfg(feature = "ipnet")]
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut pm: JointPrefixMap<ipnet::IpNet, _> = JointPrefixMap::new();
    /// pm.insert("192.168.1.0/24".parse()?, 1);
    /// assert_eq!(pm.entry("192.168.1.0/24".parse()?).and_modify(|x| *x += 1).get(), Some(&2));
    /// assert_eq!(pm.entry("192.168.2.0/24".parse()?).and_modify(|x| *x += 1).get(), None);
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "ipnet"))]
    /// # fn main() {}
    /// ```
    ///
    /// This function will *not* replace the prefix in the map if it already exists.
    ///
    /// ```
    /// # use prefix_trie::joint::*;
    /// # #[cfg(feature = "ipnet")]
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut pm: JointPrefixMap<ipnet::IpNet, _> = JointPrefixMap::new();
    /// pm.insert("192.168.1.1/24".parse()?, 1);
    /// pm.entry("192.168.1.2/24".parse()?).and_modify(|x| *x += 1);
    /// assert_eq!(
    ///     pm.get_key_value(&"192.168.1.0/24".parse()?),
    ///     Some(("192.168.1.1/24".parse()?, &2)) // prefix is not overwritten.
    /// );
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "ipnet"))]
    /// # fn main() {}
    /// ```
    #[inline(always)]
    pub fn and_modify<F: FnOnce(&mut T)>(self, f: F) -> Self {
        match self {
            Entry::Vacant(e) => Entry::Vacant(e),
            Entry::Occupied(e) => match e {
                OccupiedEntry::P1(e) => {
                    e.node.value.as_mut().map(f);
                    Entry::Occupied(OccupiedEntry::P1(e))
                }
                OccupiedEntry::P2(e) => {
                    e.node.value.as_mut().map(f);
                    Entry::Occupied(OccupiedEntry::P2(e))
                }
            },
        }
    }
}

impl<'a, P, T> Entry<'a, P, T>
where
    P: JointPrefix,
    T: Default,
{
    /// Ensures a value is in the entry by inserting the default value if empty, and returns a
    /// mutable reference to the value in the entry.
    ///
    /// ```
    /// # use prefix_trie::joint::*;
    /// # #[cfg(feature = "ipnet")]
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut pm: JointPrefixMap<ipnet::IpNet, _> = JointPrefixMap::new();
    /// pm.insert("192.168.1.0/24".parse()?, 1);
    /// assert_eq!(pm.entry("192.168.1.0/24".parse()?).or_default(), &1);
    /// assert_eq!(pm.entry("192.168.2.0/24".parse()?).or_default(), &0);
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "ipnet"))]
    /// # fn main() {}
    /// ```
    ///
    /// This function will *not* replace the prefix in the map if it already exists.
    ///
    /// ```
    /// # use prefix_trie::joint::*;
    /// # #[cfg(feature = "ipnet")]
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut pm: JointPrefixMap<ipnet::IpNet, _> = JointPrefixMap::new();
    /// pm.insert("192.168.1.1/24".parse()?, 1);
    /// pm.entry("192.168.1.2/24".parse()?).or_default();
    /// assert_eq!(
    ///     pm.get_key_value(&"192.168.1.0/24".parse()?),
    ///     Some(("192.168.1.1/24".parse()?, &1)) // prefix is not overwritten.
    /// );
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "ipnet"))]
    /// # fn main() {}
    /// ```
    #[allow(clippy::unwrap_or_default)]
    #[inline(always)]
    pub fn or_default(self) -> &'a mut T {
        self.or_insert_with(Default::default)
    }
}

impl<P: JointPrefix, T> OccupiedEntry<'_, P, T> {
    /// Gets a reference to the key in the entry. This is the key that is currently stored, and not
    /// the key that was used in the insert.
    pub fn key(&self) -> P {
        match self {
            OccupiedEntry::P1(e) => P::from_p1(e.key()),
            OccupiedEntry::P2(e) => P::from_p2(e.key()),
        }
    }

    /// Gets a reference to the value in the entry.
    pub fn get(&self) -> &T {
        match self {
            OccupiedEntry::P1(e) => e.get(),
            OccupiedEntry::P2(e) => e.get(),
        }
    }

    /// Gets a mutable reference to the value in the entry.
    ///
    /// This call will not modify the prefix stored in the tree. In case the prefix used to create
    /// the entry is different from the stored one (has additional information in the host part),
    /// and you wish that prefix to be overwritten, use `insert`.
    ///
    /// ```
    /// # use prefix_trie::joint::*;
    /// use prefix_trie::joint::map::Entry;
    /// # #[cfg(feature = "ipnet")]
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut pm: JointPrefixMap<ipnet::IpNet, _> = JointPrefixMap::new();
    /// pm.insert("192.168.1.1/24".parse()?, 1);
    /// if let Entry::Occupied(mut e) = pm.entry("192.168.1.0/24".parse()?) {
    ///     *e.get_mut() += 1;
    /// }
    /// assert_eq!(
    ///     pm.get_key_value(&"192.168.1.0/24".parse()?),
    ///     Some(("192.168.1.1/24".parse()?, &2)) // prefix is not overwritten
    /// );
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "ipnet"))]
    /// # fn main() {}
    /// ```
    pub fn get_mut(&mut self) -> &mut T {
        match self {
            OccupiedEntry::P1(e) => e.get_mut(),
            OccupiedEntry::P2(e) => e.get_mut(),
        }
    }

    /// Insert a new value into the entry, returning the old value. This operation will also replace
    /// the prefix with the provided one.
    ///
    /// ```
    /// # use prefix_trie::joint::*;
    /// use prefix_trie::joint::map::Entry;
    /// # #[cfg(feature = "ipnet")]
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut pm: JointPrefixMap<ipnet::IpNet, _> = JointPrefixMap::new();
    /// pm.insert("192.168.1.1/24".parse()?, 1);
    /// if let Entry::Occupied(mut e) = pm.entry("192.168.1.0/24".parse()?) {
    ///     e.insert(2); // doing so will overwrite the prefix.
    /// }
    /// assert_eq!(
    ///     pm.get_key_value(&"192.168.1.0/24".parse()?),
    ///     Some(("192.168.1.0/24".parse()?, &2)) // prefix is overwritten
    /// );
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "ipnet"))]
    /// # fn main() {}
    /// ```
    pub fn insert(self, value: T) -> T {
        match self {
            OccupiedEntry::P1(e) => e.insert(value),
            OccupiedEntry::P2(e) => e.insert(value),
        }
    }

    /// Remove the current value and return it. The tree will not be modified (the same effect as
    /// `JointPrefixMap::remove_keep_tree`).
    pub fn remove(&mut self) -> T {
        match self {
            OccupiedEntry::P1(e) => e.remove(),
            OccupiedEntry::P2(e) => e.remove(),
        }
    }
}

impl<'a, P: JointPrefix, T> VacantEntry<'a, P, T> {
    /// Gets a reference to the key in the entry.
    pub fn key(&self) -> P {
        match self {
            VacantEntry::P1(e) => P::from_p1(e.key()),
            VacantEntry::P2(e) => P::from_p2(e.key()),
        }
    }

    /// Get a mutable reference to the value. If the value is yet empty, set it to the given default
    /// value.
    pub fn insert(self, default: T) -> &'a mut T {
        match self {
            VacantEntry::P1(e) => e.insert(default),
            VacantEntry::P2(e) => e.insert(default),
        }
    }

    /// Get a mutable reference to the value. If the value is yet empty, set it to the return value
    /// from the given function.
    pub fn insert_with<F: FnOnce() -> T>(self, default: F) -> &'a mut T {
        match self {
            VacantEntry::P1(e) => e.insert_with(default),
            VacantEntry::P2(e) => e.insert_with(default),
        }
    }
}

impl<'a, P, T> VacantEntry<'a, P, T>
where
    P: JointPrefix,
    T: Default,
{
    /// Get a mutable reference to the value. If the value is yet empty, set it to the default value
    /// using `Default::default()`.
    pub fn default(self) -> &'a mut T {
        match self {
            VacantEntry::P1(e) => e.default(),
            VacantEntry::P2(e) => e.default(),
        }
    }
}
