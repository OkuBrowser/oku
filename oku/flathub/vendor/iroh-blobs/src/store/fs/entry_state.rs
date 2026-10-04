use std::{fmt::Debug, path::PathBuf};

use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use super::meta::{ActorError, ActorResult};
use crate::store::util::SliceInfoExt;

/// Location of the data.
///
/// Data can be inlined in the database, a file conceptually owned by the store,
/// or a number of external files conceptually owned by the user.
///
/// Only complete data can be inlined.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum DataLocation<I = (), E = ()> {
    /// Data is in the inline_data table.
    Inline(I),
    /// Data is in the canonical location in the data directory.
    Owned(E),
    /// Data is in several external locations. This should be a non-empty list.
    External(Vec<PathBuf>, E),
}

impl<I: AsRef<[u8]>, E: Debug> DataLocation<I, E> {
    fn fmt_short(&self) -> String {
        match self {
            DataLocation::Inline(d) => {
                format!("Inline({}, addr={})", d.as_ref().len(), d.addr_short())
            }
            DataLocation::Owned(e) => format!("Owned({e:?})"),
            DataLocation::External(paths, e) => {
                let paths = paths.iter().map(|p| p.display()).collect::<Vec<_>>();
                format!("External({paths:?}, {e:?})")
            }
        }
    }
}

impl<X> DataLocation<X, u64> {
    #[allow(clippy::result_large_err)]
    fn union(self, that: DataLocation<X, u64>) -> ActorResult<Self> {
        Ok(match (self, that) {
            (
                DataLocation::External(mut paths, a_size),
                DataLocation::External(b_paths, b_size),
            ) => {
                if a_size != b_size {
                    return Err(ActorError::inconsistent(format!(
                        "complete size mismatch {a_size} {b_size}"
                    )));
                }
                paths.extend(b_paths);
                paths.sort();
                paths.dedup();
                DataLocation::External(paths, a_size)
            }
            (_, b @ DataLocation::Owned(_)) => {
                // owned needs to win, since it has an associated file. Choosing
                // external would orphan the file.
                b
            }
            (a @ DataLocation::Owned(_), _) => {
                // owned needs to win, since it has an associated file. Choosing
                // external would orphan the file.
                a
            }
            (_, b @ DataLocation::Inline(_)) => {
                // inline needs to win, since it has associated data. Choosing
                // external would orphan the file.
                b
            }
            (a @ DataLocation::Inline(_), _) => {
                // inline needs to win, since it has associated data. Choosing
                // external would orphan the file.
                a
            }
        })
    }
}

impl<I, E> DataLocation<I, E> {
    #[allow(dead_code)]
    pub fn discard_inline_data(self) -> DataLocation<(), E> {
        match self {
            DataLocation::Inline(_) => DataLocation::Inline(()),
            DataLocation::Owned(x) => DataLocation::Owned(x),
            DataLocation::External(paths, x) => DataLocation::External(paths, x),
        }
    }

    pub fn split_inline_data(self) -> (DataLocation<(), E>, Option<I>) {
        match self {
            DataLocation::Inline(x) => (DataLocation::Inline(()), Some(x)),
            DataLocation::Owned(x) => (DataLocation::Owned(x), None),
            DataLocation::External(paths, x) => (DataLocation::External(paths, x), None),
        }
    }
}

/// Location of the outboard.
///
/// Outboard can be inlined in the database or a file conceptually owned by the store.
/// Outboards are implementation specific to the store and as such are always owned.
///
/// Only complete outboards can be inlined.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum OutboardLocation<I = ()> {
    /// Outboard is in the inline_outboard table.
    Inline(I),
    /// Outboard is in the canonical location in the data directory.
    Owned,
    /// Outboard is not needed
    NotNeeded,
}

impl<I: AsRef<[u8]>> OutboardLocation<I> {
    fn fmt_short(&self) -> String {
        match self {
            OutboardLocation::Inline(d) => format!("Inline({})", d.as_ref().len()),
            OutboardLocation::Owned => "Owned".to_string(),
            OutboardLocation::NotNeeded => "NotNeeded".to_string(),
        }
    }
}

impl<I> OutboardLocation<I> {
    pub fn inline(data: I) -> Self
    where
        I: AsRef<[u8]>,
    {
        if data.as_ref().is_empty() {
            OutboardLocation::NotNeeded
        } else {
            OutboardLocation::Inline(data)
        }
    }

    #[allow(dead_code)]
    pub fn discard_extra_data(self) -> OutboardLocation<()> {
        match self {
            Self::Inline(_) => OutboardLocation::Inline(()),
            Self::Owned => OutboardLocation::Owned,
            Self::NotNeeded => OutboardLocation::NotNeeded,
        }
    }

    pub fn split_inline_data(self) -> (OutboardLocation<()>, Option<I>) {
        match self {
            Self::Inline(x) => (OutboardLocation::Inline(()), Some(x)),
            Self::Owned => (OutboardLocation::Owned, None),
            Self::NotNeeded => (OutboardLocation::NotNeeded, None),
        }
    }
}

/// The information about an entry that we keep in the entry table for quick access.
///
/// The exact info to store here is TBD, so usually you should use the accessor methods.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum EntryState<I = ()> {
    /// For a complete entry we always know the size. It does not make much sense
    /// to write to a complete entry, so they are much easier to share.
    Complete {
        /// Location of the data.
        data_location: DataLocation<I, u64>,
        /// Location of the outboard.
        outboard_location: OutboardLocation<I>,
    },
    /// Partial entries are entries for which we know the hash, but don't have
    /// all the data. They are created when syncing from somewhere else by hash.
    ///
    /// As such they are always owned. There is also no inline storage for them.
    /// Non short lived partial entries always live in the file system, and for
    /// short lived ones we never create a database entry in the first place.
    Partial {
        /// Once we get the last chunk of a partial entry, we have validated
        /// the size of the entry despite it still being incomplete.
        ///
        /// E.g. a giant file where we just requested the last chunk.
        size: Option<u64>,
    },
}

impl<I> EntryState<I> {
    pub fn is_complete(&self) -> bool {
        matches!(self, Self::Complete { .. })
    }

    pub fn is_partial(&self) -> bool {
        matches!(self, Self::Partial { .. })
    }
}

impl Default for EntryState {
    fn default() -> Self {
        Self::Partial { size: None }
    }
}

impl<I: AsRef<[u8]>> EntryState<I> {
    pub fn fmt_short(&self) -> String {
        match self {
            Self::Complete {
                data_location,
                outboard_location,
            } => format!(
                "Complete {{ data: {}, outboard: {} }}",
                data_location.fmt_short(),
                outboard_location.fmt_short()
            ),
            Self::Partial { size } => format!("Partial {{ size: {size:?} }}"),
        }
    }
}

impl EntryState {
    #[allow(clippy::result_large_err)]
    pub fn union(old: Self, new: Self) -> ActorResult<Self> {
        match (old, new) {
            (
                Self::Complete {
                    data_location,
                    outboard_location,
                },
                Self::Complete {
                    data_location: b_data_location,
                    ..
                },
            ) => Ok(Self::Complete {
                // combine external paths if needed
                data_location: data_location.union(b_data_location)?,
                outboard_location,
            }),
            (a @ Self::Complete { .. }, Self::Partial { .. }) =>
            // complete wins over partial
            {
                Ok(a)
            }
            (Self::Partial { .. }, b @ Self::Complete { .. }) =>
            // complete wins over partial
            {
                Ok(b)
            }
            (Self::Partial { size: a_size }, Self::Partial { size: b_size }) =>
            // keep known size from either entry
            {
                let size = match (a_size, b_size) {
                    (Some(a_size), Some(b_size)) => {
                        // validated sizes are different. this means that at
                        // least one validation was wrong, which would be a bug
                        // in bao-tree.
                        if a_size != b_size {
                            return Err(ActorError::inconsistent(format!(
                                "validated size mismatch {a_size} {b_size}"
                            )));
                        }
                        Some(a_size)
                    }
                    (Some(a_size), None) => Some(a_size),
                    (None, Some(b_size)) => Some(b_size),
                    (None, None) => None,
                };
                Ok(Self::Partial { size })
            }
        }
    }
}

impl redb::Value for EntryState {
    type SelfType<'a> = EntryState;

    type AsBytes<'a> = SmallVec<[u8; 128]>;

    fn fixed_width() -> Option<usize> {
        None
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        postcard::from_bytes(data).unwrap()
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'a,
        Self: 'b,
    {
        postcard::to_extend(value, SmallVec::new()).unwrap()
    }

    fn type_name() -> redb::TypeName {
        redb::TypeName::new("EntryState")
    }
}
