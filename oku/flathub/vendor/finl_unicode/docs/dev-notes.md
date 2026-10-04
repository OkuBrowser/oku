# Dev notes

Started this because real work happens maybe once every couple of years, so it helps to be able to keep track of what I was thinking.

## 1.5.0 Unicode 18.0.0 changes

The big change here is the modification of rule GB9c. 

The old version:

```
\p{InCB=Consonant} [ \p{InCB=Extend} \p{InCB=Linker} ]* \p{InCB=Linker} [ \p{InCB=Extend} \p{InCB=Linker} ]*	×	\p{InCB=Consonant}
```

The new version:

```
\p{InCB=Linker} \p{InCB=Extend}*	×	\p{InCB=Consonant}
```

So now, we only enter the new state with a linker and the cluster extends with zero or more extend clusters and terminates with a consonant.

We start cluster rules in the `first_chataracter` function, so we will trigger the transition to `IndicClusterStart` on
`IN_LINKER` rather than `IN_CONSONANT`, then continue with `EXTEND` (`\p{InCB=Extend)` is equivalent to the `Extend` rule for our purposes) and then if 
we get `IN_CONSONANT` we break after, otherwise we break before. 

I would note that the old code would treat `\p{InCB=Consonant} \p{InCB=Linker} \p{InCB=Consonant} \p{InCB=Linker} \p{InCB=Consonant}` as a single 
cluster which passed the validation tests, but I’m not entirely certain if it was intended. The new rule will treat this as three clusters, unambiguously.

### Bugs found once the 18.0.0 GraphemeBreakTest.txt data actually ran

The initial implementation above only worked when a Linker was literally the *first* character
`ClusterMachine` saw (i.e. `first_character` was the one to see `IN_LINKER`). That's the rare case.
The overwhelmingly common case is `Consonant Linker Consonant`, where the base consonant puts us in
`Other`/`CcsBase`/`CcsExtend`/etc. first, and the Linker arrives through the generic mid-cluster
match arms — which just treated it as a normal "Extend-like" continuation character (it happened to
fall out of the `is_continuation` bit trick as "attach") without ever switching into
`IndicClusterStart`. So GB9c's "don't break before the Consonant" never actually armed for real text.
Fixed by making every state that can see a Linker call a shared `handle_linker` helper that arms
`IndicClusterStart`.

Also, `IndicClusterStart` closed with `Break::After` on the closing consonant, ending the cluster
immediately — which is wrong if that consonant is followed by more combining marks or another
`Linker Extend* Consonant` run (e.g. a double conjunct like क्ष्ण). It now returns `Break::None` and
falls back to `Other`, so the same cluster keeps growing.

Second layer of bugs, once real Unicode data was used instead of hand-picked examples:
- `InCB=Linker` is a separate property from `Grapheme_Cluster_Break=Extend` — most viramas happen to
  have both, but some real characters (U+1CF5/U+1CF6 Vedic jihvamuliya/upadhmaniya) are `InCB=Linker`
  with `Grapheme_Cluster_Break=Other`. Checking for the old combined constant `IN_LINKER == 0x11`
  missed these. Fixed by testing the InCB=Linker bit (`0x10`) on its own via `is_incb_linker`,
  independently of whatever base `Grapheme_Cluster_Break` value it's OR'd onto.
- `InCB=Extend` is *not* the same set as `Grapheme_Cluster_Break=Extend`. ZWNJ (U+200C) is GCB=Extend
  but deliberately excluded from InCB=Extend (that's precisely how you tell the renderer *not* to
  form a conjunct). generate-sources previously didn't encode `InCB; Extend` at all, so there was no
  way to tell "real" InCB=Extend marks apart from a ZWNJ sitting between a Linker and a Consonant.
  Added a third bit (`0x40`) for it in `build_grapheme_break_property`, and `IndicClusterStart` now
  requires that bit (or another Linker) to stay "armed" for the no-break-before-Consonant rule — a
  ZWNJ still attaches per plain GB9, it just no longer lets a following consonant merge in.
- Adding that third InCB bit meant a huge fraction of ordinary combining marks (basically all of
  U+0300–U+036F, etc.) no longer equal the plain `GraphemeProperty::EXTEND`/`SPACING_MARK`/`ZWJ`
  constants exactly, since they now carry the extra bit too. Every match that switched on the raw
  property byte for a base `Grapheme_Cluster_Break` category had to mask the InCB bits off first
  (`base_property`) before comparing — otherwise ordinary text with combining marks stopped forming
  clusters at all whenever the mark was InCB=Extend.

Verified against the full generated `GraphemeBreakTest.txt`-derived suite in `src/data/grapheme_test.rs` (856 cases, all Unicode 18.0.0 categories) — all passing.
