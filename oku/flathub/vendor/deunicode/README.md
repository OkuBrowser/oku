# deunicode

[Documentation](https://docs.rs/deunicode/)

The `deunicode` library transliterates Unicode strings such as "Æneid" into pure
ASCII ones such as "AEneid". It includes support for emoji. It's compatible with no-std Rust environments.

Deunicode is quite fast, supports on-the-fly conversion without allocations. It has a compact representation of Unicode data to minimize memory overhead and executable size (about 75K codepoints mapped to 245K ASCII characters, using 450KB of memory, 160KB gzipped).

## Examples

```rust
use deunicode::deunicode;

assert_eq!(deunicode("Æneid"), "AEneid");
assert_eq!(deunicode("étude"), "etude");
assert_eq!(deunicode("北亰"), "Bei Jing");
assert_eq!(deunicode("ᔕᓇᓇ"), "shanana");
assert_eq!(deunicode("げんまい茶"), "genmaiCha");
assert_eq!(deunicode("🦄☣"), "unicorn biohazard");
```

## When to use it?

It's a better alternative than just stripping all non-ASCII characters or letting them get [mangled](https://en.wikipedia.org/wiki/Mojibake) by some encoding-ignorant system. It's be okay for one-way conversions for things like search indexes and tokenization, as a stronger version of Unicode NFKD. It may be used for generating nice identifiers for file names and URLs, which aren't too user-facing.

However, like most "universal" libraries of this kind, it has a one-size-fits-all 1:1 mapping of Unicode code points, which can't handle language-specific exceptions nor context-dependent romanization rules. These limitations are only slightly suboptimal for European languages and Korean Hangul, but make a mess of Japanese Kanji.

## Guarantees and Warnings

Here are some guarantees you have when calling `deunicode()`:
  * The `String` returned will be valid ASCII; the decimal representation of
    every `char` in the string will be between 0 and 127, inclusive.
  * Every ASCII character (0x00 - 0x7F) is mapped to itself.
  * All Unicode characters will translate to printable ASCII characters
    (`\n` or characters in the range 0x20 - 0x7E).

There are, however, some things you should keep in mind:
  * Some transliterations do produce `\n` characters.
  * Some Unicode characters transliterate to an empty string, either on purpose
    or because `deunicode` does not know about the character.
  * Some Unicode characters are unknown and transliterate to `"[?]"`
    (or a custom placeholder, or `None` if you use a chars iterator).
  * Many Unicode characters transliterate to multi-character strings. For
    example, "北" is transliterated as "Bei".
  * The transliteration is context-free, and not sophisticated enough to produce proper Chinese or Japanese.
    Han characters used in multiple languages are mapped to a single Mandarin pronounciation,
    and will be mostly illegible to Japanese readers. Transliteration can't
    handle cases where a single character has multiple possible pronounciations.

## Unicode data

 * [`Text::Unidecode`](http://search.cpan.org/~sburke/Text-Unidecode-1.30/lib/Text/Unidecode.pm) by Sean M. Burke
 * [Unicodey](https://unicodey.com) by Cal Henderson
 * [gh emoji](https://lib.rs/gh-emoji)
 * [any_ascii](https://anyascii.com/)

For a detailed explanation on the rationale behind the original
dataset, refer to [this article](http://interglacial.com/~sburke/tpj/as_html/tpj22.html) written
by Burke in 2001.

This is a maintained alternative to the [unidecode](https://lib.rs/crates/unidecode) crate, which started as a Rust port of [`Text::Unidecode`](http://search.cpan.org/~sburke/Text-Unidecode-1.30/lib/Text/Unidecode.pm) Perl module.
