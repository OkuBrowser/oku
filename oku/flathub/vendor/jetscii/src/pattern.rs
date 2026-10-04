use std::str::pattern::{Pattern, SearchStep, Searcher};

use super::{AsciiChars, Substring};

trait PatternCore {
    fn find(&self, haystack: &str) -> Option<usize>;
    fn len(&self) -> usize;
}

struct CoreSearcher<'h> {
    haystack: &'h str,
    offset: usize,
}

impl<'h> CoreSearcher<'h> {
    fn new(haystack: &'h str) -> Self {
        CoreSearcher {
            haystack,
            offset: 0,
        }
    }

    #[inline]
    fn next<F>(&mut self, finder: F) -> SearchStep
    where
        F: PatternCore,
    {
        if self.offset >= self.haystack.len() {
            return SearchStep::Done;
        }

        let left_to_search = &self.haystack[self.offset..]; // TODO: unchecked_slice?
        let idx = finder.find(left_to_search);

        // If there's no match, then the rest of the string should be
        // returned.
        let idx = idx.unwrap_or(self.haystack.len());

        let (res, next_offset) = if idx == 0 {
            // A match occurs at the beginning of the string
            let next = self.offset + finder.len();
            (SearchStep::Match(self.offset, next), next)
        } else {
            // A match occurs somewhere further in the string
            let next = self.offset + idx;
            (SearchStep::Reject(self.offset, next), next)
        };

        self.offset = next_offset;
        res
    }
}

impl<'h, F> Pattern<'h> for AsciiChars<F>
where
    F: Fn(u8) -> bool,
{
    type Searcher = AsciiCharsSearcher<'h, F>;

    fn into_searcher(self, haystack: &'h str) -> Self::Searcher {
        AsciiCharsSearcher {
            searcher: CoreSearcher::new(haystack),
            finder: self,
        }
    }
}

impl<'a, F> PatternCore for &'a AsciiChars<F>
where
    F: Fn(u8) -> bool,
{
    fn find(&self, haystack: &str) -> Option<usize> {
        AsciiChars::find(self, haystack)
    }
    fn len(&self) -> usize {
        1
    }
}

pub struct AsciiCharsSearcher<'h, F>
where
    F: Fn(u8) -> bool,
{
    searcher: CoreSearcher<'h>,
    finder: AsciiChars<F>,
}

unsafe impl<'a, F> Searcher<'a> for AsciiCharsSearcher<'a, F>
where
    F: Fn(u8) -> bool,
{
    fn haystack(&self) -> &'a str {
        self.searcher.haystack
    }

    #[inline]
    fn next(&mut self) -> SearchStep {
        self.searcher.next(&self.finder)
    }
}

/// # Warning about empty substrings
///
/// This has different behavior from the standard library when the
/// substring to search for is the empty string. It will never
/// match. This behavior may change in the future to more closely
/// align with the standard library.
impl<'n, 'h> Pattern<'h> for Substring<'n> {
    type Searcher = SubstringSearcher<'n, 'h>;

    fn into_searcher(self, haystack: &'h str) -> Self::Searcher {
        SubstringSearcher {
            searcher: CoreSearcher::new(haystack),
            finder: self,
        }
    }
}

pub struct SubstringSearcher<'n, 'h> {
    searcher: CoreSearcher<'h>,
    finder: Substring<'n>,
}

impl<'a, 'n> PatternCore for &'a Substring<'n> {
    fn find(&self, haystack: &str) -> Option<usize> {
        Substring::find(self, haystack)
    }
    fn len(&self) -> usize {
        self.needle_len()
    }
}

unsafe impl<'n, 'h> Searcher<'h> for SubstringSearcher<'n, 'h> {
    fn haystack(&self) -> &'h str {
        self.searcher.haystack
    }

    #[inline]
    fn next(&mut self) -> SearchStep {
        self.searcher.next(&self.finder)
    }
}

#[cfg(test)]
mod test {
    use proptest::{self, collection::vec as vec_strat, prelude::*};

    use super::*;

    fn ascii_char() -> BoxedStrategy<char> {
        // This is inclusive
        proptest::char::range(0 as char, 127 as char).boxed()
    }

    proptest! {
        #[test]
        fn works_as_find_does_for_single_characters(
            (haystack, needle) in (any::<String>(), ascii_char())
        ) {
            let us = ascii_chars!(needle);
            let them = needle;
            assert_eq!(haystack.find(us), haystack.find(them));
        }

        #[test]
        fn works_as_find_does_for_multiple_characters(
            (haystack, (n1, n2, n3, n4)) in (any::<String>(), (ascii_char(), ascii_char(), ascii_char(), ascii_char()))
        ) {
            let us = ascii_chars!(n1, n2, n3, n4);
            let them = &[n1, n2, n3, n4][..];
            assert_eq!(haystack.find(us), haystack.find(them));
        }

        #[test]
        fn works_as_find_does_for_up_to_and_including_16_characters(
            (haystack, needle_raw) in (any::<String>(), vec_strat(ascii_char(), 0..=16))
        ) {
            let mut bytes = [b'\0'; 16];
            for (&c, b) in needle_raw.iter().zip(&mut bytes) {
                *b = c as u8;
            }

            let us = AsciiChars::new(bytes, needle_raw.len() as i32, |b| {
                needle_raw.iter().any(|&c| c as u8 == b)
            });

            let them = &needle_raw[..];

            assert_eq!(haystack.find(us), haystack.find(them));
        }
    }

    #[test]
    fn can_search_for_nul_bytes() {
        assert_eq!(Some(1), "a\0".find(ascii_chars!('\0')));
        assert_eq!(Some(0), "\0".find(ascii_chars!('\0')));
        assert_eq!(None, "".find(ascii_chars!('\0')));
    }

    #[test]
    fn can_search_in_nul_bytes() {
        assert_eq!(Some(1), "\0a".find(ascii_chars!('a')));
        assert_eq!(None, "\0".find(ascii_chars!('a')));
    }

    #[test]
    fn pattern_does_not_backtrack_after_first() {
        let mut searcher = ascii_chars!(' ').into_searcher("hello w ");
        assert_eq!(SearchStep::Reject(0, 5), searcher.next());
        assert_eq!(SearchStep::Match(5, 6), searcher.next());
        assert_eq!(SearchStep::Reject(6, 7), searcher.next());
        assert_eq!(SearchStep::Match(7, 8), searcher.next());
        assert_eq!(SearchStep::Done, searcher.next());
    }

    proptest! {
        #[test]
        fn works_as_find_does_for_substrings(
            (needle, haystack) in (any::<String>(), any::<String>())
        ) {
            prop_assume!(!needle.is_empty());

            let us = Substring::new(&needle);
            let them: &str = &needle;

            assert_eq!(haystack.find(us), haystack.find(them));
        }
    }

    /// I'm not sure if it's worth it to try to match the standard
    /// library behavior here. If so, we can use this test and remove
    /// the `is_empty()` test in the proptest test above.
    #[test]
    #[ignore]
    fn substring_of_an_empty_needle() {
        let mut searcher = Substring::new("").into_searcher("abc");
        assert_eq!(SearchStep::Match(0, 0), searcher.next());
        assert_eq!(SearchStep::Reject(0, 1), searcher.next());
        assert_eq!(SearchStep::Match(1, 1), searcher.next());
        assert_eq!(SearchStep::Reject(1, 2), searcher.next());
        assert_eq!(SearchStep::Match(2, 2), searcher.next());
        assert_eq!(SearchStep::Reject(2, 3), searcher.next());
        assert_eq!(SearchStep::Match(3, 3), searcher.next());
        assert_eq!(SearchStep::Done, searcher.next());
    }
}
