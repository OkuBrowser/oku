# 📝 Release notes

## 0.8.2

*May 10th, 2026*

No public API changes

- [Bump dev dependency `toml` to 1.x.x][#30]

  *Contributed by [**Michel Lind**](https://github.com/michel-slm)*

[#30]: https://github.com/rossmacarthur/emojis/pull/30

## 0.8.1

*April 13th, 2026*

- [Fix license declaration to include Unicode-3.0][#29]. All published versions
  of this crate shipped generated source code derived from Unicode data, so they
  should have included the Unicode-3.0 license.

  *Contributed by [**Michel Lind**](https://github.com/michel-slm)*

[#29]: https://github.com/rossmacarthur/emojis/pull/29

## 0.8.0

*October 20th, 2025*

- [Upgrade to `phf v0.13`][c125053c]. This raises the minimum supported Rust
  version to 1.66.0.
- [Import Unicode 17.0 emojis][298370be]
- [Fix `with_skin_tone` for emojis with only 5 variants][f53cf122]

[c125053c]: https://github.com/rossmacarthur/emojis/commit/c125053cde13babc0ae301acdf66c7b7ca2191a7
[298370be]: https://github.com/rossmacarthur/emojis/commit/298370be15b86b451f9620e9f49e75603da37da5
[f53cf122]: https://github.com/rossmacarthur/emojis/commit/f53cf1221ad0bcb93c3925de80962a7a1dba1bad

## 0.7.2

*August 12th, 2025*

- [Implement `PartialOrd` and `Ord` for `Emoji`][e83473d3]. This allows emojis
  to be compared and sorted. Note that the order is based on the string
  representation (UTF-8 encoding) of the emoji, not the CLDR order.

[e83473d3]: https://github.com/rossmacarthur/emojis/commit/e83473d3a67b9eab3f2b918d08c4ad9960e2c94f

## 0.7.1

*July 31st, 2025*

- [Expose the Unicode version that the emojis are based on][b458bc88]. Adds a
  public constant `UNICODE_VERSION` which is the version of the Unicode
  specification used to generate the emojis in this crate.

[b458bc88]: https://github.com/rossmacarthur/emojis/commit/b458bc8853cf5e2b79e951b1ae4edacd4c892ccb

## 0.7.0

*June 24th, 2025*

- [Add missing emoji variation sequences][4ef3c598]. The Unicode specification
  allows for suffixing some sequences with `\u{fe0f}` to request an emoji
  presentation of the sequence. This change adds the missing sequences defined
  [here](https://unicode.org/Public/16.0.0/ucd/emoji/emoji-variation-sequences.txt)
  to the Unicode map.

- [Upgrade to `phf v0.12.1`][5c8969fe]. This raises the minimum supported Rust
  version to 1.61.0.

- [Improve codegen to not change with Rust version][640a3f18]. Previously the
  debug format for string keys was used to construct the `phf` maps which used
  escape sequences for some Unicode code points. But this meant as the Rust
  version is updated the generated code would change.

- [Add "# Storing the `Emoji` type" section to docs][c40b8923]. Clarify how to
  store the `&'static Emoji` type.

[4ef3c598]: https://github.com/rossmacarthur/emojis/commit/4ef3c5982b7de18c999857bb76d1e07a855654a9
[5c8969fe]: https://github.com/rossmacarthur/emojis/commit/5c8969fe4daad470aa381d3be52c9a7bb20fa48f
[640a3f18]: https://github.com/rossmacarthur/emojis/commit/640a3f18918603b5a9c6196f0cf4864bc5b8da16
[c40b8923]: https://github.com/rossmacarthur/emojis/commit/c40b8923b637509188e7dc47a180e8c708e85246

## 0.6.4

*September 29th, 2024*

- [Update to Unicode 16.0 emojis][2ce453c8].

  *Contributed by [**Linda_pp**](https://github.com/rhysd)*

[2ce453c8]: https://github.com/rossmacarthur/emojis/commit/2ce453c88d795a54c4ca41839b14ecf81b24b63d

## 0.6.3

*July 24th, 2024*

- [Add `serde` support for types][d741c777]. The `&'static Emoji`, `Group`, and
  `SkinTone` types now support serialization and deserialization when the
  `serde` feature is enabled.

[d741c777]: https://github.com/rossmacarthur/emojis/commit/d741c777c1dcaddc25db1cf069a768cd089ddd5e

## 0.6.2

*April 21st, 2024*

- [Add `Clone` to returned iterators where possible][de2a5852].

[de2a5852]: https://github.com/rossmacarthur/emojis/commit/de2a58524b3ef350bf6360d95b5fad0e7b1bebf0

## 0.6.1

*September 4th, 2023*

- [Update to Unicode 15.1 emojis][cb8f13f6].

[cb8f13f6]: https://github.com/rossmacarthur/emojis/commit/cb8f13f622fc39fa2b737830de918421a7ffb7d2

## 0.6.0

*April 18th, 2023*

- [Add a generous sprinkling of `#[inline]` attrs][73e5410b].

- [Enumerate all skin tone combinations][e1c85965]. This adds support for
  multi skin tone emojis, like those that display multiple people.

[73e5410b]: https://github.com/rossmacarthur/emojis/commit/73e5410be38c9aa65c19fd0c7057ad508352e76c
[e1c85965]: https://github.com/rossmacarthur/emojis/commit/e1c859651308ee4a29740b76519a183d82664810

## 0.5.3

*April 17th, 2023*

- [Update to gemoji v4.1.0 shortcodes][fde8f8d2].

[fde8f8d2]: https://github.com/rossmacarthur/emojis/commit/fde8f8d20f8d395119505e04a8907e23e0a76b07

## 0.5.2

*November 25th, 2022*

- [Update to gemoji v4.0.1 shortcodes][7ecc42b3].

  *Contributed by [**Linda_pp**](https://github.com/rhysd)*

[7ecc42b3]: https://github.com/rossmacarthur/emojis/commit/7ecc42b35d7ce7ba07ad51612a1dae0a24026aba

## 0.5.1

*October 17th, 2022*

### Features

- [Add function to iterate over groups][5bab0ac2]. You can now iterate over all
  emoji groups using the `Group::iter()` function.

- [Add `shortcodes()` to iterate over an emoji's shortcodes][74b3a18c]. Some
  emojis have multiple shortcodes, this function allows you to iterate over
  them.

  *Contributed by [**Finn Bear**](https://github.com/FinnBear)*

[5bab0ac2]: https://github.com/rossmacarthur/emojis/commit/5bab0ac2384bd894666235e73e5edf0f26038840
[74b3a18c]: https://github.com/rossmacarthur/emojis/commit/74b3a18c393a5aad6597c185f41a048553190310

## 0.5.0

*October 4th, 2022*

- [Update to gemoji v4.0.0.rc3 shortcodes][ab7f2412].

- [Update to Unicode 15.0 emojis][da0fad25].

- [Use `phf` crate for O(1) lookups][de06cb56]. Generate compile-time maps using
  `phf_codegen`. This allows us to lookup emojis in O(1) time instead of the
  previous linear search.

[da0fad25]: https://github.com/rossmacarthur/emojis/commit/da0fad25e0db8d224d1ff0cb8a393753ab97d9ec
[ab7f2412]: https://github.com/rossmacarthur/emojis/commit/ab7f24125d1f2804fd01b1061765a19f5e665f40
[de06cb56]: https://github.com/rossmacarthur/emojis/commit/de06cb566622d93f7a2818d1e18878eff893598f

## 0.4.0

*March 29th, 2022*

- [Remove `search()`][58587f4e].

- [Add `.as_bytes()` method][509ea67f]. This allows you to get the UTF-8 bytes
  of the emoji.

- [Simplify API to `get` and `get_by_shortcode`][712f85db]. The API has been
  simplified to just two main lookup functions.

[58587f4e]: https://github.com/rossmacarthur/emojis/commit/58587f4e3074998056954de94af565a1684ba853
[509ea67f]: https://github.com/rossmacarthur/emojis/commit/509ea67f7c1eeadd3f048de810fec52e57817893
[712f85db]: https://github.com/rossmacarthur/emojis/commit/712f85db9dae14d079b5f80f355fd51690c72ab2

## 0.3.0

*March 1st, 2022*

- [Update to Unicode 14.0 emojis][e8bbe29b].

- [Drop internal id and implement traits using emoji][c6d22463].

- [Add Unicode version][924a0ca3]. This is accessible on the emoji using
  `.unicode_version()`. This can be used to filter emojis by version.


[e8bbe29b]: https://github.com/rossmacarthur/emojis/commit/e8bbe29bdce4f2b9d26672803fecb1b98c030adb
[c6d22463]: https://github.com/rossmacarthur/emojis/commit/c6d22463fbb85e27cec810802f5c83b98dae9bd1
[924a0ca3]: https://github.com/rossmacarthur/emojis/commit/924a0ca3bc8d1b6ab6c936912b53e828fbf58e68

## 0.2.1

*November 19th, 2021*

- [Fix some skin tone bugs][c6c7988b]. Return the default skin tone emojis in
  main emoji iterator and handle emojis that can have multiple skin tones.

[c6c7988b]: https://github.com/rossmacarthur/emojis/commit/c6c7988bfe48219ea7de75708e6e8b63541fce9f

## 0.2.0

*November 18th, 2021*

- [Ergonomically support skin tones][399bf03b]. This adds better support for
  handling emojis with different skin tone variations.

- [Make `lookup` return a reference][13d9ffa1].

- [Remove `Deref` implementation][ea6ce739]. `Emoji` is not a smart pointer,
  let's not abuse `Deref`. `.as_str()` works perfectly fine.

- [Remove search functionality][7b53df3b]. This was removed because fuzzy
  searching is quite a hard problem to solve can be quite dependent on the use
  case. It helps simplify this crate to leave searching up to the user.

[399bf03b]: https://github.com/rossmacarthur/emojis/commit/399bf03ba6071a6ca418a928383baecb873fca66
[13d9ffa1]: https://github.com/rossmacarthur/emojis/commit/13d9ffa1db3bc2dadfc241b98b094854040372e1
[ea6ce739]: https://github.com/rossmacarthur/emojis/commit/ea6ce739fcd343ec990dfa5f74ddd1679e689b3b
[7b53df3b]: https://github.com/rossmacarthur/emojis/commit/7b53df3b938c324c180e5bc16ba70ea5c2965de7

## 0.1.2

*April 7th, 2021*

- [Implement `Display` for `Emoji`][a0e4c6b2].

- [Support looking up variations][35e4263e]. Unqualified and minimally qualified
  variations of emojis can now be looked up. The fully qualified variation will
  be returned. Skin tone variations of emojis can now be looked up and the
  default skin tone variation will be returned.

[a0e4c6b2]: https://github.com/rossmacarthur/emojis/commit/a0e4c6b220b1d04fbf1ce9e27d7d121e97c970d5
[35e4263e]: https://github.com/rossmacarthur/emojis/commit/35e4263eee261847202cde9955ab9ea8e3ae79ff

## 0.1.1

*March 29th, 2021*

- [Improve search score algorithm][c9fda861]. Add multiplier for cases where the
  emoji description or alias starts with the query.

[c9fda861]: https://github.com/rossmacarthur/emojis/commit/c9fda861769e420ee6a49b937b35af86fef4ad3a

## 0.1.0

*March 12th, 2021*

- [Merge `lookup()` and `lookup_shortcode()`][b37ef644]. The lookup functions
  have been consolidated.

- [Improve `search()` function][8219c356]. Switch to `simstr` crate and search
  aliases as well.

- [Support GitHub (gemoji) shortcodes][1e70c5c1]. Automatically generated from
  the GitHub/gemoji repository data. Add `lookup_shortcode()` function to lookup
  by shortcode.

[b37ef644]: https://github.com/rossmacarthur/emojis/commit/b37ef644a131b2fcac614ba446aeba3a0b62ced5
[8219c356]: https://github.com/rossmacarthur/emojis/commit/8219c35628e01674d7f7fd1069bb6b2afb1575cf
[1e70c5c1]: https://github.com/rossmacarthur/emojis/commit/1e70c5c184c65cd80db4ac9b9bb4df6b8a742952

## 0.0.0

*March 6th, 2021*

First version.
