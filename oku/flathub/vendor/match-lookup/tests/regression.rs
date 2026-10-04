//! Regression tests for `gen_char_match!` / `gen_ascii_match!`.

use match_lookup::{gen_ascii_match, gen_char_match};

/// The exact base256emoji alphabet, as used by `base256emoji` 1.0.2.
const EMOJI: &str = "🚀🪐☄🛰🌌🌑🌒🌓🌔🌕🌖🌗🌘🌍🌏🌎🐉☀💻🖥💾💿😂❤😍🤣😊🙏💕😭😘👍😅👏😁🔥🥰💔💖💙😢🤔😆🙄💪😉☺👌🤗💜😔😎😇🌹🤦🎉💞✌✨🤷😱😌🌸🙌😋💗💚😏💛🙂💓🤩😄😀🖤😃💯🙈👇🎶😒🤭❣😜💋👀😪😑💥🙋😞😩😡🤪👊🥳😥🤤👉💃😳✋😚😝😴🌟😬🙃🍀🌷😻😓⭐✅🥺🌈😈🤘💦✔😣🏃💐☹🎊💘😠☝🌺🎂🌻😐🖕💝🙊😹🗣💫💀👑🎵🤞😛🔴😤🌼😫⚽🤙☕🏆🤫👈😮🙆🍻🍃🐶💁😲🌿🧡🎁⚡🌞🎈❌✊👋😰🤨😶🤝🚶💰🍓💢🤟🙁🚨💨🤬✈🎀🍺🤓😙💟🌱😖👶🥴▶➡❓💎💸⬇😨🌚🦋😷🕺⚠🙅😟😵👎🤲🤠🤧📌🔵💅🧐🐾🍒😗🤑🌊🤯🐷☎💧😯💆👆🎤🙇🍑❄🌴💣🐸💌📍🥀🤢👅💡💩👐📸👻🤐🤮🎼🥵🚩🍎🍊👼💍📣🥂😕";

fn emoji_index(c: char) -> Option<usize> {
    gen_char_match!(c, "🚀🪐☄🛰🌌🌑🌒🌓🌔🌕🌖🌗🌘🌍🌏🌎🐉☀💻🖥💾💿😂❤😍🤣😊🙏💕😭😘👍😅👏😁🔥🥰💔💖💙😢🤔😆🙄💪😉☺👌🤗💜😔😎😇🌹🤦🎉💞✌✨🤷😱😌🌸🙌😋💗💚😏💛🙂💓🤩😄😀🖤😃💯🙈👇🎶😒🤭❣😜💋👀😪😑💥🙋😞😩😡🤪👊🥳😥🤤👉💃😳✋😚😝😴🌟😬🙃🍀🌷😻😓⭐✅🥺🌈😈🤘💦✔😣🏃💐☹🎊💘😠☝🌺🎂🌻😐🖕💝🙊😹🗣💫💀👑🎵🤞😛🔴😤🌼😫⚽🤙☕🏆🤫👈😮🙆🍻🍃🐶💁😲🌿🧡🎁⚡🌞🎈❌✊👋😰🤨😶🤝🚶💰🍓💢🤟🙁🚨💨🤬✈🎀🍺🤓😙💟🌱😖👶🥴▶➡❓💎💸⬇😨🌚🦋😷🕺⚠🙅😟😵👎🤲🤠🤧📌🔵💅🧐🐾🍒😗🤑🌊🤯🐷☎💧😯💆👆🎤🙇🍑❄🌴💣🐸💌📍🥀🤢👅💡💩👐📸👻🤐🤮🎼🥵🚩🍎🍊👼💍📣🥂😕")
}

#[test]
fn emoji_alphabet_has_256_chars() {
    assert_eq!(EMOJI.chars().count(), 256);
}

/// Every index is the char ordinal, and all 256 fit in a `u8`.
#[test]
fn emoji_index_is_char_ordinal() {
    for (i, c) in EMOJI.chars().enumerate() {
        assert_eq!(emoji_index(c), Some(i), "wrong index for {c:?}");
        assert!(i <= u8::MAX as usize);
    }
}

/// Mirrors base256emoji's `decode`: `get_index(c).map(|i| i as u8)`.
/// With 0.1.2 this returned byte offsets truncated to u8, e.g. 0x01 -> 0x04.
#[test]
fn emoji_roundtrip_all_bytes() {
    let alphabet: Vec<char> = EMOJI.chars().collect();
    let data: Vec<u8> = (0u8..=255).collect();

    let encoded: String = data.iter().map(|&b| alphabet[b as usize]).collect();
    let decoded: Vec<u8> = encoded
        .chars()
        .map(|c| emoji_index(c).expect("in alphabet") as u8)
        .collect();

    assert_eq!(decoded, data);
}

/// The concrete failure from the issue: byte 0x01 decoded as 0x04 because
/// '🚀' is 4 bytes wide.
#[test]
fn emoji_second_char_is_index_one_not_four() {
    assert_eq!(emoji_index('🪐'), Some(1));
    assert_ne!(emoji_index('🪐'), Some('🚀'.len_utf8()));
}

/// Index must not depend on the byte width of *preceding* chars.
/// Alphabet mixes 1-, 2-, 3- and 4-byte UTF-8 encodings.
#[test]
fn mixed_width_alphabet() {
    const ALPHABET: &str = "a\u{e9}\u{20ac}\u{1f600}b\u{1f680}\u{4e2d}z";
    fn index(c: char) -> Option<usize> {
        gen_char_match!(c, "a\u{e9}\u{20ac}\u{1f600}b\u{1f680}\u{4e2d}z")
    }

    let widths: Vec<usize> = ALPHABET.chars().map(char::len_utf8).collect();
    assert_eq!(widths, [1, 2, 3, 4, 1, 4, 3, 1], "test alphabet lost its mix");

    for (i, c) in ALPHABET.chars().enumerate() {
        assert_eq!(index(c), Some(i), "wrong index for {c:?}");
    }
}

/// A char that is only *byte-offset* reachable must not be matched.
#[test]
fn chars_outside_alphabet_return_none() {
    fn index(c: char) -> Option<usize> {
        gen_char_match!(c, "🚀🪐☄")
    }
    for c in ['a', '\u{0}', '\u{4}', '\u{8}', '🛰', char::MAX] {
        assert_eq!(index(c), None, "{c:?} should not be in alphabet");
    }
}

/// Alphabets with escaped literals: the macro must index the *decoded*
/// string, so escapes and literal chars agree.
#[test]
fn escaped_and_literal_forms_agree() {
    fn escaped(c: char) -> Option<usize> {
        gen_char_match!(c, "\u{1f680}\u{1fa90}\u{2604}")
    }
    fn literal(c: char) -> Option<usize> {
        gen_char_match!(c, "🚀🪐☄")
    }
    for c in ['🚀', '🪐', '☄', 'x'] {
        assert_eq!(escaped(c), literal(c));
    }
}

/// ASCII alphabets are unaffected either way, but pin them too.
#[test]
fn ascii_alphabet_via_char_match() {
    const BASE58: &str = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    fn index(c: char) -> Option<usize> {
        gen_char_match!(c, "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz")
    }
    for (i, c) in BASE58.chars().enumerate() {
        assert_eq!(index(c), Some(i));
    }
    assert_eq!(index('0'), None);
    assert_eq!(index('O'), None);
    assert_eq!(index('I'), None);
    assert_eq!(index('l'), None);
}

#[test]
fn ascii_match_is_byte_ordinal() {
    const BASE58: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    fn index(b: u8) -> Option<usize> {
        gen_ascii_match!(b, b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz")
    }
    for (i, &b) in BASE58.iter().enumerate() {
        assert_eq!(index(b), Some(i));
    }
    assert_eq!(index(b'0'), None);
    assert_eq!(index(0x80), None);
}

/// Both macros must agree on an ASCII alphabet.
#[test]
fn char_and_ascii_macros_agree() {
    fn by_char(c: char) -> Option<usize> {
        gen_char_match!(c, "0123456789abcdef")
    }
    fn by_byte(b: u8) -> Option<usize> {
        gen_ascii_match!(b, b"0123456789abcdef")
    }
    for b in 0u8..=127 {
        assert_eq!(by_char(b as char), by_byte(b), "mismatch for {b:#x}");
    }
}

/// Matches must work when the scrutinee is an arbitrary expression,
/// not just an identifier.
#[test]
fn scrutinee_can_be_expression() {
    let s = "☄🚀";
    let first = gen_char_match!(s.chars().next().unwrap(), "🚀🪐☄");
    let last = gen_char_match!(s.chars().last().unwrap(), "🚀🪐☄");
    assert_eq!(first, Some(2));
    assert_eq!(last, Some(0));
}
