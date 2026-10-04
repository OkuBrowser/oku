//! The `deunicode` library transliterates Unicode strings such as "Æneid" into pure
//! ASCII ones such as "AEneid."
//!
//! Supports no-std. Stores Unicode data in a compact format.
//!
//! It started as a Rust port of [`Text::Unidecode`](http://search.cpan.org/~sburke/Text-Unidecode-1.30/lib/Text/Unidecode.pm) Perl module, and was extended to support emoji.
//!
//! See [README](https://github.com/kornelski/deunicode/blob/master/README.md) for more info.
//!
//! Examples
//! --------
#![cfg_attr(feature = "alloc", doc = "```rust")]
#![cfg_attr(not(feature = "alloc"), doc = "```rust,ignore")]
//! use deunicode::deunicode;
//!
//! assert_eq!(deunicode("Æneid"), "AEneid");
//! assert_eq!(deunicode("étude"), "etude");
//! assert_eq!(deunicode("北亰"), "Bei Jing");
//! assert_eq!(deunicode("ᔕᓇᓇ"), "shanana");
//! assert_eq!(deunicode("げんまい茶"), "genmaiCha");
//! assert_eq!(deunicode("🦄☣"), "unicorn biohazard");
//! assert_eq!(deunicode("…"), "...");
//!
//! // format without a temporary string
//! use deunicode::AsciiChars;
//! format!("what's up {}", "🐶".ascii_chars());
#![doc = "```"] // to mollify some syntax highlighters

#![no_std]

#[cfg(any(test, feature = "alloc"))]
extern crate alloc;
#[cfg(feature = "alloc")]
use alloc::borrow::Cow;
#[cfg(feature = "alloc")]
use alloc::string::String;

use core::iter::FusedIterator;
use core::str::Chars;

const MAPPING: &str = include_str!("mapping.txt");

#[repr(C)]
#[derive(Copy, Clone)]
struct Ptr {
    /// if len <= 2, it's the string itself,
    /// otherwise it's an u16 offset into MAPPING
    chr: [u8; 2],
    len: u8,
}

const POINTERS_BYTES: &[u8] = include_bytes!("pointers.bin");
/// POINTERS format is described by struct Ptr
const POINTERS: &[Ptr] = unsafe { core::slice::from_raw_parts(POINTERS_BYTES.as_ptr().cast(), POINTERS_BYTES.len() / core::mem::size_of::<Ptr>()) };

/// This function takes any Unicode string and returns an ASCII transliteration
/// of that string.
///
/// Guarantees and Warnings
/// -----------------------
/// Here are some guarantees you have when calling [`deunicode()`]:
///   * The `String` returned will be valid ASCII; the decimal representation of
///     every `char` in the string will be between 0 and 127, inclusive.
///   * Every ASCII character (0x0000 - 0x007F) is mapped to itself.
///   * All Unicode characters will translate to a string containing newlines
///     (`"\n"`) or ASCII characters in the range 0x0020 - 0x007E. So for example,
///     no Unicode character will translate to `\u{01}`. The exception is if the
///     ASCII character itself is passed in, in which case it will be mapped to
///     itself. (So `'\u{01}'` will be mapped to `"\u{01}"`.)
///
/// There are, however, some things you should keep in mind:
///   * As stated, some transliterations do produce `\n` characters.
///   * Some Unicode characters transliterate to an empty string on purpose.
///   * Some Unicode characters are unknown and transliterate to `"[?]"` (see [`deunicode_with_tofu()`])
///   * Many Unicode characters transliterate to multi-character strings. For
///     example, 北 is transliterated as "Bei ".
///   * Han characters are mapped to Mandarin, and will be mostly illegible to Japanese readers.
#[inline(always)]
#[cfg(feature = "alloc")]
#[must_use]
pub fn deunicode(s: &str) -> String {
    deunicode_with_tofu(s, "[?]")
}

/// Same as [`deunicode()`], but unknown characters can be replaced with a custom string.
///
/// You can use "\u{FFFD}" to use the usual Unicode Replacement Character.
///
/// "Tofu" is a nickname for a replacement character, which in Unicode fonts usually
/// looks like a block of tofu.
#[inline]
#[cfg(feature = "alloc")]
#[must_use]
pub fn deunicode_with_tofu(s: &str, custom_placeholder: &str) -> String {
    deunicode_with_tofu_cow(s, custom_placeholder).into_owned()
}

/// Same as [`deunicode_with_tofu()`], but avoids allocating a new `String` if not necessary.
///
/// You can use "\u{FFFD}" to use the usual Unicode Replacement Character.
///
/// "Tofu" is a nickname for a replacement character, which in Unicode fonts usually
/// looks like a block of tofu.
#[cfg(feature = "alloc")]
#[must_use]
pub fn deunicode_with_tofu_cow<'input>(s: &'input str, custom_placeholder: &str) -> Cow<'input, str> {
    // Fast path to skip over ASCII chars at the beginning of the string
    let ascii_len = s.as_bytes().iter().take_while(|&&c| c < 0x7F).count();
    if ascii_len >= s.len() { // >= elides bounds check in split_at
        return Cow::Borrowed(s);
    }

    let (ascii, rest) = s.as_bytes().split_at(ascii_len);
    // safe, because it's been checked to be ASCII only
    debug_assert!(core::str::from_utf8(ascii).is_ok());
    let ascii = unsafe { core::str::from_utf8_unchecked(ascii) };

    // reserve a bit more space to avoid reallocations on longer transliterations
    // but instead of `+ 16` uses `| 15` to stay in the smallest allocation bucket for short strings
    let mut out = String::new();
    // this generates less code than with_capacity()
    out.try_reserve_exact(s.len() | 15).unwrap_or_else(|_| panic!());

    // this if optimizes out unused realloc code from push_str
    let needs_to_grow = ascii.as_bytes().len() > out.capacity().wrapping_sub(out.len());
    if !needs_to_grow {
        out.push_str(ascii);
    }

    // safe, because UTF-8 codepoint can't start with < 7F byte
    debug_assert!(core::str::from_utf8(rest).is_ok());
    let s = unsafe { core::str::from_utf8_unchecked(rest) };

    out.extend(s.ascii_chars().map(move |ch| ch.unwrap_or(custom_placeholder)));
    Cow::Owned(out)
}

/// This function takes a single Unicode character and returns an ASCII
/// transliteration.
///
/// The warnings and guarantees of [`deunicode()`] apply to this function as well.
///
/// Examples
/// --------
/// ```rust
/// # use deunicode::deunicode_char;
/// assert_eq!(deunicode_char('Æ'), Some("AE"));
/// assert_eq!(deunicode_char('北'), Some("Bei "));
/// ```
#[inline]
#[must_use]
pub fn deunicode_char(ch: char) -> Option<&'static str> {
    if let Some(p) = POINTERS.get(ch as usize) {
        // if length is 1 or 2, then the "pointer" data is used to store the char
        if p.len <= 2 {
            let chars = p.chr.get(..p.len as usize)?;
            // safe, because we're returning only ASCII
            debug_assert!(core::str::from_utf8(chars).is_ok());
            unsafe {
                Some(core::str::from_utf8_unchecked(chars))
            }
        } else {
            let map_pos = (u16::from(p.chr[0]) | u16::from(p.chr[1]) << 8) as usize;
            // unknown characters are intentionally mapped to out of range length
            MAPPING.get(map_pos..map_pos + p.len as usize)
        }
    } else {
        None
    }
}

/// Convenience functions for deunicode. `use deunicode::AsciiChars`
pub trait AsciiChars {
    /// Iterate over Unicode characters converted to ASCII sequences.
    ///
    /// Items of this iterator may be `None` for some characters.
    /// Use `.map(|ch| ch.unwrap_or("?"))` to replace invalid characters.
    ///
    /// Alternatively, this iterator can be used in formatters:
    #[cfg_attr(feature = "alloc", doc = "```rust")]
    #[cfg_attr(not(feature = "alloc"), doc = "```rust,ignore")]
    /// use deunicode::AsciiChars;
    /// format!("what's up {}", "🐶".ascii_chars());
    #[doc = "```"]
    fn ascii_chars(&self) -> AsciiCharsIter<'_>;

    /// Convert any Unicode string to ASCII-only string.
    ///
    /// Characters are converted to closest ASCII equivalent.
    /// Characters that can't be converted are replaced with `"[?]"`.
    #[cfg(feature = "alloc")]
    fn to_ascii_lossy(&self) -> String;
}

#[cfg(feature = "alloc")]
impl AsciiChars for String {
    #[inline(always)]
    fn ascii_chars(&self) -> AsciiCharsIter<'_> {
        AsciiCharsIter::new(self)
    }
    #[inline(always)]
    fn to_ascii_lossy(&self) -> String {
        deunicode(self)
    }
}

impl AsciiChars for str {
    #[inline(always)]
    fn ascii_chars(&self) -> AsciiCharsIter<'_> {
        AsciiCharsIter::new(self)
    }
    #[inline(always)]
    #[cfg(feature = "alloc")]
    fn to_ascii_lossy(&self) -> String {
        deunicode(self)
    }
}

/// Iterator that translates Unicode characters to ASCII strings.
///
/// See [`AsciiChars`] trait's `str.ascii_chars()` method.
///
/// Additionally, it implements `Display` for formatting strings without allocations.
///
#[cfg_attr(feature = "alloc", doc = "```rust")]
#[cfg_attr(not(feature = "alloc"), doc = "```rust,ignore")]
/// use deunicode::AsciiChars;
/// format!("what's up {}", "🐶".ascii_chars());
#[doc = "```"]
#[derive(Clone)]
pub struct AsciiCharsIter<'a> {
    next_char: Option<Option<&'static str>>,
    chars: Chars<'a>,
}

/// Use `.map(|ch| ch.unwrap_or("?"))` to replace invalid characters.
impl<'a> AsciiCharsIter<'a> {
    #[inline]
    pub fn new(unicode_string: &'a str) -> Self {
        let mut chars = unicode_string.chars();
        Self {
            next_char: chars.next().map(deunicode_char),
            chars,
        }
    }
}

impl<'a> FusedIterator for AsciiCharsIter<'a> {}

impl<'a> Iterator for AsciiCharsIter<'a> {
    type Item = Option<&'static str>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let dch = self.next_char?;
        self.next_char = self.chars.next().map(deunicode_char);
        let dch = match dch {
            None => return Some(None),
            Some(dch) => dch,
        };
        // ends with space
        let trim_last_char = dch.as_bytes().len() > 1 && dch.as_bytes().last().copied() == Some(b' ') &&
            self.next_char.map_or(true, |ch| { // true if end
            ch.map_or(false, |ch| ch.as_bytes().first().copied() == Some(b' ')) // space next (assume placeholder is not space)
        });
        Some(if !trim_last_char {
            Some(dch)
        } else {
            dch.get(..dch.len()-1)
        })
    }

    #[inline]
    fn count(self) -> usize {
        self.chars.count() + if self.next_char.is_some() {1} else {0}
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.chars.size_hint().0 + if self.next_char.is_some() {1} else {0}, None)
    }
}

/// Format without a temporary string
///
#[cfg_attr(feature = "alloc", doc = "```rust")]
#[cfg_attr(not(feature = "alloc"), doc = "```rust,ignore")]
/// use deunicode::AsciiChars;
/// format!("what's up {}", "🐶".ascii_chars());
#[doc = "```"]
impl core::fmt::Display for AsciiCharsIter<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.clone().try_for_each(|ch| f.write_str(ch.unwrap_or("\u{FFFD}")))
    }
}

#[test]
fn iter_test() {
    use alloc::vec::Vec;
    let chars: Vec<_> = AsciiCharsIter::new("🄏中国").flatten().collect();
    assert_eq!(&chars, &["NonCommercial", "Zhong ", "Guo"]);
    let chars: Vec<_> = "中国x🅶".ascii_chars().flatten().collect();
    assert_eq!(&chars, &["Zhong ", "Guo ", "x", "G"]);
    let chars: Vec<_> = "☃中 国".ascii_chars().flatten().collect();
    assert_eq!(&chars, &["snowman ", "Zhong", " ", "Guo"]);
}

#[test]
fn zalgo() {
    assert_eq!(deunicode_with_tofu("h̵̡̢̛̻̬͔̦͓̥̞̳͇̭̣̪̰̞̲̩̭̤͚͖͓̰̭̝̬̖̭͇͇̰͇͓̠͑͆͐͛̏͒͆̊́̊̂̉̉̈́̿̆̾̌̀͒͌́͗͋͜͝͝͝ͅĕ̷̡̧̡̧̜̮͙̗͙͕͖̩͈͙̞̞̭͙̯͖̰͖̙̹͖͚̦̬̄̀̓̈́͗̆̓̽͛̀͛̄͂̉͒̓̐̃̑́͊̀͋͊͗́̈́͑͗̐̔̈͊͋̓͊̓́̏̍̍̓͘̕͝͝͠ͅl̶̠̮̺̦̩͓̣̪͚͌̊̈́̀̄̈́̉͗̀̏͋̆̈̈́̉̋̊̉̉̌̈́̚̕͠͠l̴̨̡͍͇̝̟̩̙̤̰̬̬͖͙̺̟̯͓̥̯͔̤̠̻̤̮̘̋͑̑̿͗͂̃̓̓̉͒̑͜͠ͅo̸̢̧̨̜͉̜͓͙̰̳̙̖̰͇̺͈̝̬̩̫͛̅̍͌̎̅̿̂̚̕͜ ̵̛̗͍̊̈͋̀̊͒̄̔̔͋͋̆͋̅̀͂͂̍́̀̈́̈́͂̂̂̆̅͗̄̈́̀̈́̅̒̈̋͊̍̈́͂̑̓̽̂̂̓̚̕̚̕̚͠͝w̷̨͍͖̗͔͖͎̩̠̜͖̞͍̘̤͕̮̥̭͛̆̎̋̄͒̓̈́͆̀̆̚ǫ̷̢̢̧̧̨̧̧̨̢̼̮̺̬͇͓̪̯͖̥͙̠͍̭̩̰͎̘̺̝̲̖̮̞̝̠̠͎̻̠͙̫͙̞̫̭͖̱͉̱̮̌͑̈̅̈́̊̓͌̇͌̏̾̆͗̉͊̐̈́̾́̔̆͐́͘͜͜͝ͅŗ̵̡̛̛̟̭͉̰̮̺̜̼̰̟̲͖͔͕̰͕͇̪̲̫̬͚̱̮͎̭̩̩̉̇̉̀̉͑̔͋͆͌͜͠ļ̴̢̨̢̛͙̳̮̠͔͇͈̟͇̦̯͖̖͚̺̤͈̻͔̤̤̪̫͔͕̻̟̥̤̩͚̟̳͔̘̤͈͍͍̯̻̙̺̪̄̈́́͊̋̊́̅͛̉̊̉̅̋̆̔͑̈́͋̑͂̍̌̓̾̆̕̕͝ͅḏ̶̡̨̢̡̛̙͕̘̜͚̺̬̭̜͖͎͚̹̖͈̖̤͎̙̫͎̜̩̰̬̪̣̎͛̓̏̃͊̈́̽̆̒̈́̎̄̍́͘̚̚͝͠͠ͅ!̶̨̨̨̛̛̟̳̼̘͎͔̜͎͚̖̮̰͕̞̦̩̗̫̠͔͕͎͎͎̦̬̫̩̰̲̈́͋̽̀̒͆̄̑̐̀̐̋͆̈́̊̽̊̅̊̀͆͆͑̈͋̌͆͑̂̊͑̚͝͝ͅͅͅ", ""), "hello world!");
}
