# finl Unicode support

This crate is designed for the Unicode needs of the finl project, but is designed to be usable by other software as well.
In the current release (1.0.x), support is provided for character code identification and grapheme segmentation and Unicode14.0.0.

## Overview 

### Category identification

Loading the `finl_unicode` crate with the `categories` feature will add methods onto the char type to test the category of a character
or identify its category. See the rustdoc for detail.

### Grapheme clusters

Loading the `finl_unicode` crate with the `grapheme_clusters` feature will extend `Peekable<CharIndices>` to have a `next_cluster()` method which will return the next grapheme cluster from the iterator.
There is also a pure cluster iterator available by calling `Graphemes::new(s)` on a `&str`. I don’t use this in finl, but wrote it using the same algorithm as the extension of `Peekable<CharIndices>` for the purposes of benchmarking.¹

## Why?

There *are* existing crates for these purposes, but segmentation lacked the interface for segmentation that I wanted (which was to be able to extend `Peekable<CharIndices>` with a method to fetch the next grapheme cluster if it existed). 
I incorrectly assumed that this would require character code identification, which turned out to be incorrect. 
It turned out, though, that the crate I was using was outdated and possibly abandoned and had an inefficient algorithm so it turned out to be a good thing that I wrote it.
I benchmarked my code against existing crates and discovered that I had managed to eke out performance gains against all of them, so that’s an added bonus.

###  Benchmark results

All benchmarks are generated using Criterion You can replicate them by running `cargo bench` from the project directory. Three numbers are given for all results: low/mean/high, all from the output of Criterion. The mean value is given in **bold**. 

#### Unicode categories
I ran three benchmarks to compare the performance of the crates on my M3 Max MacBook Pro. 
The Japanese text benchmark reads the Project Gutenberg EBook of *Kumogata monsho* by John Falkner and counts the characters in it which are Unicode letters.
The Czech text benchmark reads the Project Gutenberg EBook of *Cítanka pro skoly obecné* by Jan Stastný and Jan Lepar and Josef Sokol (this was to exercise testing against a Latin-alphabet text with lots of diacriticals). 
All letters are counted in the first benchmark and lowercase letters only are counted in the second.
The English text benchmark reads the Project Gutenberg eBook of *Frankenstein* by Mary Wollstonecraft Shelley (to run against a text which is pure ASCII).
All letters and lowercase letters are counted in two benchmarks as with the Czech text. The source code check is from neovim. Again, letters and lowercase letters are counted in the sample.

I compared against [unicode_categories](https://docs.rs/unicode_categories/latest/unicode_categories/) 0.1.1. All times are in ms. Smaller is better.

| Benchmark                | `finl_unicode`                 | `unicode_categories`        |
|--------------------------|--------------------------------|-----------------------------|
| Japanese text            | 0.26186/**0.26309**/0.26426    | 6.7822/**6.8678**/6.9641    |
| Czech text               | 0.07606/**0.07626**/0.07647    | 1.7192/**1.7210**/1.7228    |
| Czech text (lowercase)   | 0.07676/**0.07696**/0.07718    | 0.58069/**0.58166**/0.58256 |
| English text             | 0.24904/**0.24938**/0.24973    | 7.0234/**7.0307**/7.0384    |
| English text (lowercase) | 0.24759/**0.24806**/0.24849    | 2.6752/**2.6807**/2.6871    |
| Source code              | 0.02762/**0.02766**/0.02771    | 1.0610/**1.0628**/1.0646    |
| Source code (lowercase)  | 0.02753/**0.02756**/0.02760    | 0.30376/**0.30419**/0.30464 |

As you can see, this is a clear win (the difference is the choice of algorithm. `finl_unicode` uses two-step table lookup to be able to store categories compactly while `unicode_categories` uses a combination of range checks and binary searches on tables).

#### Grapheme clusters

I compared against [unicode_segmentation](https://docs.rs/unicode-segmentation/latest/unicode_segmentation/) 1.13.3 (part of the unicode-rs project) and [bstr](https://docs.rs/bstr/latest/bstr/) 1.13.1. 
Comparisons are run against graphemes.txt, derived from the Unicode test suite, plus several language
texts that were part of the `unicode_segmentation` benchmark suite. 

All times are in µs, smaller is better.

| Benchmark         | `finl_unicode`           | `unicode_segmentation`   | `bstr`                   |
|-------------------|--------------------------|--------------------------|--------------------------|
| Unicode graphemes | 59.278/**59.467**/59.655 | 221.38/**221.85**/222.30 | 264.32/**264.69**/265.11 |
| Arabic text       | 114.18/**114.38**/114.64 | 319.08/**319.66**/320.32 | 1026.9/**1029.4**/1031.9 |
| English text      | 153.52/**153.93**/154.32 | 457.82/**458.61**/459.50 | 342.12/**342.75**/343.39 |
| Hindi text        | 93.938/**94.297**/94.643 | 398.63/**399.29**/400.07 | 818.92/**820.40**/821.83 |
| Japanese text     | 65.340/**65.505**/65.674 | 321.58/**322.04**/322.51 | 961.61/**962.63**/963.81 |
| Korean text       | 146.77/**147.12**/147.48 | 405.77/**406.28**/406.82 | 1273.6/**1275.9**/1278.1 |
| Mandarin text     | 62.777/**62.919**/63.079 | 303.46/**303.88**/304.33 | 875.31/**877.98**/880.79 |
| Russian text      | 114.97/**115.21**/115.48 | 324.40/**324.88**/325.42 | 898.84/**903.20**/907.83 |
| Source code       | 162.18/**162.47**/162.80 | 451.11/**451.79**/452.55 | 480.84/**481.80**/482.94 |

With the move from benchmarking on Intel to Apple Silicon, the performance difference for my code versus the other
libraries were generally expanded, but it appears that with updates, that difference has narrowed in some cases.
In the course of the latest updates to handle indic conjunct clustering, some minor internal changes were done to the
routine which gave minor speed increases for non-Hindi text after an initial performance regression.

## Why not?

You may want to avoid this if you need `no_std` (I’m looking at an update that will create a new contract for clusters
which will remove the allocations and I just need to decide whether this will involve removing the existing interface or merely
deprecating it). 

If you need other clustering algorithms, I have no near future plans to implement them (but I would do it for money). 

There is no equivalent to `unicode_segmentation`’s `GraphemeCursor` as I don’t need that functionality 
for finl. Reverse iteration over graphemes is not supported, nor do I have plans to support it.

I do not support legacy clustering algorithms which are supported by `unicode-segmentation`. However, the Unicode
specification discourages the use of legacy clustering which is only documented for backwards compatability with very old versions of the Unicode standard.²


## Unicode copyright notice

This package incorporates data from Unicode Inc.
Copyright © 1991–2025 Unicode, Inc. All rights reserved.

## Support

I’ve released this under an MIT/Apache License. Do what you like with it. 
I wouldn’t mind contributions to the ongoing support of developing finl, but they’re not necessary (although if you’re Microsoft or Google and you use my code, surely you can throw some dollars in my bank account).
I guarantee no warranty or support, although if you care to throw some money my way, I can prioritize your requests.

## Version history

- **1.0.0** Initial release
- **1.0.1** Build-process changes to make docs.rs documentation build
- **1.0.2** More changes because the first round apparently weren’t enough
- **1.1.0** Add support for Unicode 15.0.0, added new benchmark comparisons.
- **1.2.0** Allow grapheme clustering to work on any `Peekable` iterator over `char` or `(usize,char)`.
- **1.3.0** Add support for Unicode 16.0.0 (significant changes required for Indic Conjunct clusters), update license documentation and benchmark comparisons.
- **1.4.0** Add support for Unicode 17.0.0
- **1.5.0** Add support for Unicode 18.0.0 (more changes for Indic Conjunct clusters), some internal improvements.

---

1. For technical reasons, the iterator extension returns `Option<String>` rather than `Option<&str>` and thus will automatically underperform other implementations which are returning *all* the grapheme clusters. 
For finl, however, I would need an owned value for the string containing the cluster anyway and since I only occasionally need a cluster, I decided it was acceptable to take the performance hit. 
But see the benchmark results for the fact that I apparently managed to implement a faster algorithm anyway when doing an apples-to-apples comparison of speeds. 
2. Pure speculation, but I think that this might be the entire reason for the difference in performance between `finl_unicode` and `unicode_segmentation`. However, I have not looked at the source code to confirm my suspicion.