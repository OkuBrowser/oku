//! This module provides two interfaces for accessing clusters from an underlying string. The
//! `GraphemeCluster` trait extends the `Peekable` iterators over `Chars` or `CharIndices`
//! to add a `next_cluster` method which returns `Option<String>` with the next
//! cluster if one exists. This is the best method for getting individual clusters from a stream which is normally
//! only getting `char`s but is not recommended if you wish to iterate over clusters.
//! ```
//! # use crate::finl_unicode::grapheme_clusters::GraphemeCluster;
//! let mut char_iterator = "A\u{301}✋🏽🇦🇹!".chars().peekable();
//! assert_eq!(char_iterator.next_cluster(), Some("A\u{301}".to_string()));
//! assert_eq!(char_iterator.next_cluster(), Some("✋🏽".to_string()));
//! assert_eq!(char_iterator.next_cluster(), Some("🇦🇹".to_string()));
//! assert_eq!(char_iterator.next_cluster(), Some("!".to_string()));
//! assert_eq!(char_iterator.next_cluster(), None);
//! ```
//!
//! For the iterating over clusters case there is a struct `Graphemes` which implements `iterator`
//! and can be constructed from a `&str`. This returns references to substrings of the original
//! `&str` and is more performant for that case than the extended iterator provided through
//! `GraphemeCluster` which allocates a new `String` for each cluster found.
//! ```
//! # use crate::finl_unicode::grapheme_clusters::Graphemes;
//! let graphemes = Graphemes::new("A\u{301}✋🏽🇦🇹!");
//! assert_eq!(graphemes.collect::<Vec<&str>>(), ["A\u{301}", "✋🏽", "🇦🇹", "!"])
//! ```

use std::iter::Peekable;
use std::str::CharIndices;
use crate::data::grapheme_property::{GP_PAGES,GP_TABLE};


/// `Graphemes` provides an iterator over the grapheme clusters of a string.
pub struct Graphemes<'a> {
    input: &'a str,
    iter: Peekable<CharIndices<'a>>,
}

impl<'a> Graphemes<'a> {
    /// A new instance of graphemes can be constructed from a string using `Graphemes::new`
    /// ```
    /// # use crate::finl_unicode::grapheme_clusters::Graphemes;
    /// let graphemes = Graphemes::new("some string");
    /// ```
    pub fn new(input: &'a str) -> Graphemes<'a> {
        let iter = input.char_indices().peekable();
        Graphemes {
            input,
            iter
        }
    }
}

impl<'a> Iterator for Graphemes<'a> {
    type Item = &'a str;
    #[inline]
    /// Return a slice of the underlying
    /// string corresponding to the next cluster if one exists, or `None` if the end of the string
    /// has been reached.
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(&(start, _)) = self.iter.peek() {
            let mut cluster_machine = ClusterMachine::new();
            loop {
                if let Some(&(curr_loc, ch)) = self.iter.peek() {
                    match cluster_machine.find_cluster(ch) {
                        Break::None => { self.iter.next(); }
                        Break::Before => {
                            return Some(&self.input[start..curr_loc]);
                        }
                        Break::After => {
                            self.iter.next();
                            return Some(
                                if let Some(&(curr_loc, _)) = self.iter.peek() {
                                    &self.input[start..curr_loc]
                                } else {
                                    &self.input[start..]
                                });
                        }
                    }
                }
                else {
                    return Some(&self.input[start..]);
                }
            }
        } else {
            None
        }
    }
}

/// Get the next grapheme cluster from a stream of characters or char indices
/// This trait is implemented for any `Peekable` iterator over either `char` or `(usize, char)` (so
/// it will work on `Peekable<Chars>` and `Peekable<CharIndices>` as well as any other peekable iterator
/// which meets this requirement.
pub trait GraphemeCluster<T> {
    fn next_cluster(&mut self) -> Option<String>;
}

impl<T> GraphemeCluster<T> for T where T: PeekChar {
    /// Returns the next cluster if there is one in an `Option<String>`. Since this has a heap allocation
    /// it is *not* recommended for iterating over all the clusters in a string. In that case, use
    /// `Graphemes` instead.
    #[inline]
    fn next_cluster(&mut self) -> Option<String> {
        if self.has_next() {
            let mut cluster_machine = ClusterMachine::new();
            let mut rv = String::new();
            loop {
                if let Some(ch) = self.peek_char() {
                    let state = cluster_machine.find_cluster(ch);
                    match state {
                        Break::None => {
                            rv.push(ch);
                            self.next();
                        }
                        Break::Before => { return Some(rv); }
                        Break::After => {
                            rv.push(ch);
                            self.next();
                            return Some(rv);
                        }
                    }
                } else {
                    break;
                }
            }
            Some(rv)
        } else {
            None
        }
    }

}

/// This trait exists primarily to allow a single implementation to be used for both `Peekable<Chars>`
/// and `Peekable<CharIndices>`. You could implement this for some other iterator if you like as
/// long as you can implement the two methods below.
pub trait PeekChar: Iterator {
    /// Returns the next character (if it exists) or `None` otherwise.
    fn peek_char(&mut self) -> Option<char>;
    /// Returns `true` if there is another character available on the iterator, `false` otherwise.
    fn has_next(&mut self) -> bool;
}

trait HasChar {
    fn get_char(& self) -> char;
}

impl HasChar for char {
    fn get_char(& self) -> char {
        *self
    }
}

impl HasChar for (usize, char) {
    fn get_char(&self) -> char {
        self.1
    }
}

impl<CharIter, C: HasChar> PeekChar for Peekable<CharIter>
where CharIter: Iterator<Item = C>
{
    #[inline]
    fn peek_char(&mut self) -> Option<char> {
        self.peek().map(|c| c.get_char())
    }

    #[inline]
    fn has_next(&mut self) -> bool {
        self.peek().is_some()
    }
}

// ------------------------
// Private implementation details follow

#[derive(PartialEq)]
enum ClusterMachineState {
    Start,
    Precore,
    CcsBase,
    CrLf,
    HangulSyllableL,
    HangulSyllableV,
    HangulSyllableT,
    CcsExtend,
    Flag,
    Emoji,
    EmojiZWJ,
    IndicClusterStart,
    Other,
}

#[derive(Debug, PartialEq)]
enum Break {
    None,
    Before,
    After,
}

struct ClusterMachine {
    state: ClusterMachineState,
}

impl ClusterMachine {
    #[inline]
    pub fn new() -> ClusterMachine {
        ClusterMachine {
            state: ClusterMachineState::Start,
        }
    }

    /// If we have a cluster, we return the cluster in a `String` in an `Option` long with a `bool`
    /// If the `bool` is true, it means that we are also consuming the character  in the cluster.
    #[inline]
    pub fn find_cluster(&mut self, c: char) -> Break {
        if self.state == ClusterMachineState::Start {
            return self.first_character(c);
        }
        let property = get_property(c);

        // Fast path: a character with no grapheme-break-relevant property at all (the
        // overwhelming majority of characters in typical text) never continues a cluster
        // except after a Prepend character, where `Precore` always absorbs the next
        // character unconditionally. Every other state below resolves a bare `property == 0`
        // to `Break::Before` (falling through to their catch-all arm without ever matching
        // a more specific one), so we can skip the state dispatch, InCB bit tests, and the
        // `base_property` mask entirely and go straight to the answer. Mutating `self.state`
        // in those catch-all arms is vestigial for a `Break::Before` return: the iterator
        // discards this `ClusterMachine` and builds a fresh one for the next cluster, so the
        // fast path doesn't need to reproduce that assignment.
        //
        // Benchmarked: a clean win on every text except heavily Indic-conjunct text (Hindi),
        // where the branch is almost never taken and costs a small, close-to-noise amount; the
        // 10-20% win everywhere else is worth that.
        if property == 0 && self.state != ClusterMachineState::Precore {
            return Break::Before;
        }
        let base = base_property(property);

        if property == GraphemeProperty::CONTROL {
            return if self.state == ClusterMachineState::CrLf && c == '\n' {
                self.state = ClusterMachineState::Start;
                Break::After
            } else {
                if c == '\r' {
                    self.state = ClusterMachineState::CrLf;
                } else {
                    self.state = ClusterMachineState::Start;
                }
                Break::Before
            }
        }

        match self.state {
            ClusterMachineState::Start => self.first_character(c),
            ClusterMachineState::Precore => {
                self.first_character(c);
                Break::None
            }
            ClusterMachineState::HangulSyllableL => {
                if is_incb_linker(property) { // GB9c
                    self.handle_linker(property)
                } else {
                    match base {
                        GraphemeProperty::L => Break::None,
                        GraphemeProperty::V | GraphemeProperty::LV => {
                            self.state = ClusterMachineState::HangulSyllableV;
                            Break::None
                        }
                        GraphemeProperty::LVT => {
                            self.state = ClusterMachineState::HangulSyllableT;
                            Break::None
                        }
                        GraphemeProperty::EXTEND | GraphemeProperty::SPACING_MARK | GraphemeProperty::ZWJ => {
                            self.state = ClusterMachineState::CcsBase;
                            Break::None
                        }
                        _ => {
                            Break::Before
                        }
                    }
                }
            }
            ClusterMachineState::HangulSyllableV => {
                if is_incb_linker(property) { // GB9c
                    self.handle_linker(property)
                } else {
                    match base {
                        GraphemeProperty::V => Break::None,
                        GraphemeProperty::T => {
                            self.state = ClusterMachineState::HangulSyllableT;
                            Break::None
                        }
                        GraphemeProperty::EXTEND | GraphemeProperty::SPACING_MARK | GraphemeProperty::ZWJ => {
                            self.state = ClusterMachineState::CcsBase;
                            Break::None
                        }
                        _ => {
                            Break::Before
                        }
                    }
                }
            }
            ClusterMachineState::HangulSyllableT => {
                if is_incb_linker(property) { // GB9c
                    self.handle_linker(property)
                } else {
                    match base {
                        GraphemeProperty::T => Break::None,
                        GraphemeProperty::EXTEND | GraphemeProperty::SPACING_MARK | GraphemeProperty::ZWJ => {
                            self.state = ClusterMachineState::CcsBase;
                            Break::None
                        }
                        _ => {
                            Break::Before
                        }
                    }
                }
            }
            ClusterMachineState::CcsExtend => {
                if is_incb_linker(property) { // GB9c
                    self.handle_linker(property)
                } else {
                    match base {
                        GraphemeProperty::EXTEND
                        | GraphemeProperty::SPACING_MARK
                        | GraphemeProperty::ZWJ => Break::None,
                        _ => Break::Before
                    }
                }
            }
            ClusterMachineState::Flag => {
                self.state = ClusterMachineState::Start;
                if is_incb_linker(property) { // GB9c
                    self.handle_linker(property)
                } else {
                    match base {
                        GraphemeProperty::REGIONAL_INDICATOR => {
                            self.state = ClusterMachineState::Other;
                            Break::None
                        }
                        GraphemeProperty::EXTEND
                        | GraphemeProperty::SPACING_MARK
                        | GraphemeProperty::ZWJ => {
                            self.state = ClusterMachineState::CcsExtend;
                            Break::None
                        }
                        _ => {
                            Break::Before
                        }
                    }
                }
            }
            ClusterMachineState::Emoji => {
                if is_incb_linker(property) { // GB9c
                    self.handle_linker(property)
                } else {
                    match base {
                        GraphemeProperty::ZWJ => {
                            self.state = ClusterMachineState::EmojiZWJ;
                            Break::None
                        }
                        GraphemeProperty::EXTEND | GraphemeProperty::SPACING_MARK => {
                            self.state = ClusterMachineState::Emoji;
                            Break::None
                        }
                        _ => {
                            Break::Before
                        }
                    }
                }
            }
            ClusterMachineState::EmojiZWJ => {
                if base == GraphemeProperty::EXTENDED_GRAPHEME {
                    self.state = ClusterMachineState::Emoji;
                    Break::None
                } else {
                    Break::Before
                }
            }
            ClusterMachineState::CrLf => Break::Before,
            ClusterMachineState::IndicClusterStart => {
                // GB9c: \p{InCB=Linker} \p{InCB=Extend}* × \p{InCB=Consonant}
                if is_incb_linker(property) {
                    // A later Linker (e.g. a second virama) re-arms the run for GB9c purposes;
                    // whether it also attaches to what preceded it is an ordinary GB9 question.
                    self.handle_linker(property)
                } else if is_incb_extend(property) {
                    // Genuinely part of the \p{InCB=Extend}* run: stays armed for GB9c.
                    Break::None
                } else if is_extend_like(property) {
                    // Attaches per plain GB9 (Extend/SpacingMark/ZWJ base) but is NOT InCB=Extend
                    // (e.g. ZWNJ, U+200C, which is deliberately excluded from InCB=Extend so it
                    // can be used to suppress conjunct formation). It joins the cluster but
                    // disarms GB9c: a following Consonant no longer gets a free pass.
                    self.state = ClusterMachineState::Other;
                    Break::None
                } else if property == GraphemeProperty::IN_CONSONANT {
                    // The consonant closes this GB9c run, but it becomes an ordinary
                    // base character: further Extend/ZWJ/SpacingMark still attach to it,
                    // and a following Linker can open a new GB9c run in the same cluster.
                    self.state = ClusterMachineState::Other;
                    Break::None
                } else {
                    Break::Before
                }
            }
            _ => {
                if is_incb_linker(property) { // GB9c
                    self.handle_linker(property)
                } else if is_continuation(property) {
                    Break::None
                } else {
                    Break::Before
                }
            }
        }
    }
    #[inline]
    fn handle_linker(&mut self, property: u8) -> Break {
        if is_extend_like(property) {
            self.state = ClusterMachineState::IndicClusterStart;
            Break::None
        } else {
            // A non-attaching Linker (e.g. a bare InCB=Linker char with no Extend-like base)
            // ends the current cluster before it; the discarded `ClusterMachine` doesn't need
            // its state updated (see the comment on the `find_cluster` fast path above).
            Break::Before
        }
    }
    #[inline]
    fn first_character(&mut self, c: char) -> Break {
        if c == '\r' {
            self.state = ClusterMachineState::CrLf;
            return Break::None;
        }
        let property = get_property(c);
        // Fast path: see the comment on the equivalent check in `find_cluster` — a character
        // with no grapheme-break-relevant property always starts an ordinary `Other` cluster.
        if property == 0 {
            self.state = ClusterMachineState::Other;
            return Break::None;
        }
        if property == GraphemeProperty::CONTROL {
            self.state = ClusterMachineState::Start;
            return Break::After;
        }
        if is_incb_linker(property) { // GB9c
            self.state = ClusterMachineState::IndicClusterStart;
            return Break::None;
        }
        match base_property(property) {
            GraphemeProperty::PREPEND => {
                self.state = ClusterMachineState::Precore;
            }
            GraphemeProperty::EXTEND => {
                self.state = ClusterMachineState::CcsExtend;
            }
            GraphemeProperty::SPACING_MARK => {
                self.state = ClusterMachineState::CcsExtend;
            }
            GraphemeProperty::L => {
                self.state = ClusterMachineState::HangulSyllableL;
            }
            GraphemeProperty::V => {
                self.state = ClusterMachineState::HangulSyllableV;
            }
            GraphemeProperty::T => {
                self.state = ClusterMachineState::HangulSyllableT;
            }
            GraphemeProperty::LV => {
                self.state = ClusterMachineState::HangulSyllableV;
            }
            GraphemeProperty::LVT => {
                self.state = ClusterMachineState::HangulSyllableT;
            }
            GraphemeProperty::EXTENDED_GRAPHEME => {
                self.state = ClusterMachineState::Emoji;
            }
            GraphemeProperty::REGIONAL_INDICATOR => {
                self.state = ClusterMachineState::Flag;
            }
            _ => {
                self.state = ClusterMachineState::Other;
            }
        }
        Break::None
    }
}

#[inline]
fn is_continuation(property: u8) -> bool {
    property != 0 && property & 0x2c == 0
}

/// `true` if `property` carries the InCB=Linker bit, regardless of its underlying
/// Grapheme_Cluster_Break base property (a Linker character is not necessarily also Extend,
/// e.g. U+1CF5 VEDIC SIGN JIHVAMULIYA has InCB=Linker but Grapheme_Cluster_Break=Other).
#[inline]
fn is_incb_linker(property: u8) -> bool {
    property & GraphemeProperty::IN_LINKER_BIT != 0
}

/// `true` if `property` carries the InCB=Extend bit. This is *not* the same as having a
/// Grapheme_Cluster_Break base of Extend: most Extend/ZWJ characters are InCB=Extend, but ZWNJ
/// (U+200C) is deliberately excluded from InCB=Extend so that it can be used to suppress
/// conjunct formation, even though its Grapheme_Cluster_Break is Extend.
#[inline]
fn is_incb_extend(property: u8) -> bool {
    property & GraphemeProperty::IN_EXTEND_BIT != 0
}

/// The three bits generate-sources ORs into a character's base Grapheme_Cluster_Break property
/// to record its InCB category (Linker/Consonant/Extend); see
/// generate-sources/src/main.rs build_grapheme_break_property.
const INCB_BITS: u8 = GraphemeProperty::IN_LINKER_BIT | GraphemeProperty::IN_CONSONANT | GraphemeProperty::IN_EXTEND_BIT;

/// Recovers the plain Grapheme_Cluster_Break value (Extend, SpacingMark, L, V, ...) that
/// generate-sources started from, stripping off any InCB bits OR'd on top of it. Needed because
/// InCB and Grapheme_Cluster_Break don't coincide for every character (e.g. a bare InCB=Linker
/// character with no Grapheme_Cluster_Break of its own, or a combining mark that is both
/// Grapheme_Cluster_Break=Extend and InCB=Extend), so exact-matching the raw byte against a base
/// category constant would miss characters that also carry an InCB bit.
#[inline]
fn base_property(property: u8) -> u8 {
    property & !INCB_BITS
}

/// `true` if `property`'s underlying Grapheme_Cluster_Break base (with the InCB Linker/Consonant/
/// Extend bits masked off) is Extend, SpacingMark, or ZWJ, i.e. it attaches to the preceding
/// character per plain GB9/GB9a regardless of any InCB property it also carries.
#[inline]
fn is_extend_like(property: u8) -> bool {
    match base_property(property) {
        GraphemeProperty::EXTEND | GraphemeProperty::SPACING_MARK | GraphemeProperty::ZWJ => true,
        _ => false,
    }
}


// Symbolic names for properties in data tables
struct GraphemeProperty {}
impl GraphemeProperty {
    const EXTEND: u8 = 0x01;
    const SPACING_MARK: u8 = 0x02;
    const ZWJ: u8 = 0x03;
    const CONTROL: u8 = 0x04;
    const PREPEND: u8 = 0x05;
    const EXTENDED_GRAPHEME: u8 = 0x06;
    const REGIONAL_INDICATOR: u8 = 0x07;
    const L: u8 = 0x0c;
    const V: u8 = 0x08;
    const T: u8 = 0x09;
    const LV: u8 = 0x0d;
    const LVT: u8 = 0x0e;
    const IN_CONSONANT: u8 = 0x20;
    // The bit generate-sources ORs into a character's base Grapheme_Cluster_Break property
    // to mark InCB=Linker (see generate-sources/src/main.rs build_grapheme_break_property).
    // A Linker is not necessarily also Extend (e.g. U+1CF5), so this must be tested as a bit,
    // not as an exact value equal to `EXTEND | IN_LINKER_BIT`.
    const IN_LINKER_BIT: u8 = 0x10;
    // The bit generate-sources ORs in for InCB=Extend (see the same function). Kept distinct
    // from IN_LINKER_BIT/IN_CONSONANT since InCB's Linker/Consonant/Extend/None values are
    // mutually exclusive, and distinct from the base Grapheme_Cluster_Break=Extend value because
    // the two properties don't coincide for every character (e.g. ZWNJ).
    const IN_EXTEND_BIT: u8 = 0x40;
}


#[inline]
fn get_property(c: char) -> u8 {
    GP_PAGES[usize::from(GP_TABLE[(c as usize) >> 8])][(c as usize) & 0xff]
}



#[cfg(test)]
pub (crate) mod tests {
    use crate::grapheme_clusters::*;

    #[test]
    fn low_level_interface_test() {
        let mut machine = ClusterMachine::new();
        assert_eq!(machine.find_cluster('\r'), Break::None);
        assert_eq!(machine.find_cluster('a'), Break::Before);
        assert_eq!(machine.find_cluster('\r'), Break::Before);
        assert_eq!(machine.find_cluster('\n'), Break::After);
    }

    #[test]
    fn can_get_clusters() {
        let mut peekable_index = "\r\ne\u{301}f".char_indices().peekable();
        assert_eq!(Some("\r\n".to_string()), peekable_index.next_cluster());
        assert_eq!(Some("e\u{301}".to_string()), peekable_index.next_cluster());
        assert_eq!(Some("f".to_string()), peekable_index.next_cluster());
    }

    pub (crate) fn grapheme_test(input: &str, expected_output: &[&str], message: &str) {
        let mut iter = input.char_indices().peekable();
        let mut clusters = vec!();
        while let Some(cluster) = iter.next_cluster() {
            clusters.push(cluster);
        }
        let codes :Vec<u8> = input.chars().map(get_property).collect();
        assert_eq!(clusters.len(), expected_output.len(), "Lengths did not match on Grapheme Cluster\n\t{message}\n\tOutput: {clusters:?}\n\tExpected: {expected_output:?}\n\tCodes: {codes:x?}");
        clusters.iter().zip(expected_output.into_iter())
            .for_each(|(actual, &expected)| assert_eq!(actual.as_str(), expected, "GraphemeCluster mismatch: {message}"));

        let iter = Graphemes::new(input);
        let clusters = iter.collect::<Vec<&str>>();
        assert_eq!(clusters.len(), expected_output.len(), "Lengths did not match on Grapheme Cluster Indices\n\t{message}\n\tOutput: {clusters:?}\n\tExpected: {expected_output:?}");
        clusters.iter().zip(expected_output.into_iter())
            .for_each(|(actual, &expected)| assert_eq!(*actual, expected, "Grapheme cluster indices mismatch: {message}\n{} ≠ {}", actual.escape_unicode(), expected.escape_unicode()));
    }

}
