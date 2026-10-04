use unicode_normalization::UnicodeNormalization;

extern crate unicode_normalization;

mod case_folding_data;
pub use case_folding_data::UNICODE_VERSION;
use case_folding_data::*;

pub trait Caseless {
    fn default_case_fold(self) -> CaseFold<Self>
    where
        Self: Sized;
    fn default_caseless_match<J: Iterator<Item = char>>(self, other: J) -> bool;
    fn canonical_caseless_match<J: Iterator<Item = char>>(self, other: J) -> bool;
    fn compatibility_caseless_match<J: Iterator<Item = char>>(self, other: J) -> bool;
}

impl<I: Iterator<Item = char>> Caseless for I {
    fn default_case_fold(self) -> CaseFold<I> {
        CaseFold {
            chars: self,
            queue: ['\0', '\0'],
        }
    }

    fn default_caseless_match<J: Iterator<Item = char>>(self, other: J) -> bool {
        iter_eq(self.default_case_fold(), other.default_case_fold())
    }

    fn canonical_caseless_match<J: Iterator<Item = char>>(self, other: J) -> bool {
        // FIXME: Inner NFD can be optimized:
        // "Normalization is not required before case folding,
        //  except for the character U+0345 "combining greek ypogegrammeni"
        //  and any characters that have it as part of their canonical decomposition,
        //  such as U+1FC3 "greek small letter eta with ypogegrammeni".
        //  In practice, optimized versions of canonical caseless matching
        //  can catch these special cases, thereby avoiding an extra normalization
        //  step for each comparison."
        // Unicode Standard, section 3.13 Default Case Algorithms
        iter_eq(
            self.nfd().default_case_fold().nfd(),
            other.nfd().default_case_fold().nfd(),
        )
    }

    fn compatibility_caseless_match<J: Iterator<Item = char>>(self, other: J) -> bool {
        // FIXME: Unclear if the inner NFD can be optimized here like in canonical_caseless_match.
        iter_eq(
            self.nfd()
                .default_case_fold()
                .nfkd()
                .default_case_fold()
                .nfkd(),
            other
                .nfd()
                .default_case_fold()
                .nfkd()
                .default_case_fold()
                .nfkd(),
        )
    }
}

pub fn default_case_fold_str(s: &str) -> String {
    s.chars().default_case_fold().collect()
}

pub fn default_caseless_match_str(a: &str, b: &str) -> bool {
    a.chars().default_caseless_match(b.chars())
}

pub fn canonical_caseless_match_str(a: &str, b: &str) -> bool {
    a.chars().canonical_caseless_match(b.chars())
}

pub fn compatibility_caseless_match_str(a: &str, b: &str) -> bool {
    a.chars().compatibility_caseless_match(b.chars())
}

fn iter_eq<L: Iterator, R: Iterator>(mut a: L, mut b: R) -> bool
where
    L::Item: PartialEq<R::Item>,
{
    loop {
        match (a.next(), b.next()) {
            (None, None) => return true,
            (None, _) | (_, None) => return false,
            (Some(x), Some(y)) => {
                if !x.eq(&y) {
                    return false;
                }
            }
        }
    }
}

pub struct CaseFold<I> {
    chars: I,
    queue: [char; 2],
}

impl<I> Iterator for CaseFold<I>
where
    I: Iterator<Item = char>,
{
    type Item = char;

    fn next(&mut self) -> Option<char> {
        let c = self.queue[0];
        if c != '\0' {
            self.queue[0] = self.queue[1];
            self.queue[1] = '\0';
            return Some(c);
        }
        self.chars.next().map(
            |c| match CASE_FOLDING_TABLE.binary_search_by(|&(x, _)| x.cmp(&c)) {
                Err(_) => c,
                Ok(i) => {
                    let folded = CASE_FOLDING_TABLE[i].1;
                    self.queue = [folded[1], folded[2]];
                    folded[0]
                }
            },
        )
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let queue_len = if self.queue[0] == '\0' {
            0
        } else if self.queue[1] == '\0' {
            1
        } else {
            2
        };
        let (low, high) = self.chars.size_hint();
        (
            low.saturating_add(queue_len),
            high.and_then(|h| h.checked_mul(3))
                .and_then(|h| h.checked_add(queue_len)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::default_case_fold_str;

    #[test]
    fn test_strs() {
        assert_eq!(default_case_fold_str("Test Case"), "test case");
        assert_eq!(default_case_fold_str("Teſt Caſe"), "test case");
        assert_eq!(default_case_fold_str("spiﬃest"), "spiffiest");
        assert_eq!(default_case_fold_str("straße"), "strasse");
    }
}
