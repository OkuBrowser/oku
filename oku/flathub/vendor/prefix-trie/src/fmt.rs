//! Formatting implementation for the PrefixMap

use std::fmt::{Debug, Formatter, Result};

use super::*;

impl<P: Debug, T: Debug> Debug for PrefixMap<P, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        DebugPrefixMap(self, 0).fmt(f)
    }
}

struct DebugPrefixMap<'a, P, T>(&'a PrefixMap<P, T>, usize);

impl<P: Debug, T: Debug> Debug for DebugPrefixMap<'_, P, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let map = self.0;
        let idx = self.1;
        let node = &map.table[idx];
        match (node.value.as_ref(), node.left, node.right) {
            (None, None, None) => node.prefix.fmt(f),
            (None, None, Some(child)) | (None, Some(child), None) => f
                .debug_map()
                .entry(&node.prefix, &DebugPrefixMap(map, child.get()))
                .finish(),
            (None, Some(left), Some(right)) => f
                .debug_map()
                .entry(
                    &node.prefix,
                    &(
                        DebugPrefixMap(map, left.get()),
                        DebugPrefixMap(map, right.get()),
                    ),
                )
                .finish(),
            (Some(v), None, None) => f.debug_map().entry(&node.prefix, v).finish(),
            (Some(v), None, Some(child)) | (Some(v), Some(child), None) => f
                .debug_map()
                .entry(&node.prefix, &(v, DebugPrefixMap(map, child.get())))
                .finish(),
            (Some(v), Some(left), Some(right)) => f
                .debug_map()
                .entry(
                    &node.prefix,
                    &(
                        v,
                        DebugPrefixMap(map, left.get()),
                        DebugPrefixMap(map, right.get()),
                    ),
                )
                .finish(),
        }
    }
}

impl<P: Debug> Debug for PrefixSet<P> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        DebugPrefixMap(&self.0, 0).fmt(f)
    }
}
