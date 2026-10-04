// GENERATED CODE DO NOT MANUALLY EDIT

use crate::grapheme_clusters::tests::grapheme_test;

#[test]
fn grapheme_cluster_test_1() {
	grapheme_test("\u{000D}\u{000D}",
		&["\u{000D}", "\u{000D}"],
		"  ÷ [1.0] <CARRIAGE RETURN (CR)> (CR) ÷ [4.0] <CARRIAGE RETURN (CR)> (CR) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_2() {
	grapheme_test("\u{000D}\u{0308}\u{000D}",
		&["\u{000D}", "\u{0308}", "\u{000D}"],
		"  ÷ [1.0] <CARRIAGE RETURN (CR)> (CR) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <CARRIAGE RETURN (CR)> (CR) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_3() {
	grapheme_test("\u{000D}\u{000A}",
		&["\u{000D}\u{000A}"],
		"  ÷ [1.0] <CARRIAGE RETURN (CR)> (CR) × [3.0] <LINE FEED (LF)> (LF) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_4() {
	grapheme_test("\u{000D}\u{0308}\u{000A}",
		&["\u{000D}", "\u{0308}", "\u{000A}"],
		"  ÷ [1.0] <CARRIAGE RETURN (CR)> (CR) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <LINE FEED (LF)> (LF) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_5() {
	grapheme_test("\u{000D}\u{0000}",
		&["\u{000D}", "\u{0000}"],
		"  ÷ [1.0] <CARRIAGE RETURN (CR)> (CR) ÷ [4.0] <NULL> (Control) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_6() {
	grapheme_test("\u{000D}\u{0308}\u{0000}",
		&["\u{000D}", "\u{0308}", "\u{0000}"],
		"  ÷ [1.0] <CARRIAGE RETURN (CR)> (CR) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <NULL> (Control) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_7() {
	grapheme_test("\u{000D}\u{094D}",
		&["\u{000D}", "\u{094D}"],
		"  ÷ [1.0] <CARRIAGE RETURN (CR)> (CR) ÷ [4.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_8() {
	grapheme_test("\u{000D}\u{0308}\u{094D}",
		&["\u{000D}", "\u{0308}\u{094D}"],
		"  ÷ [1.0] <CARRIAGE RETURN (CR)> (CR) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_9() {
	grapheme_test("\u{000D}\u{0300}",
		&["\u{000D}", "\u{0300}"],
		"  ÷ [1.0] <CARRIAGE RETURN (CR)> (CR) ÷ [4.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_10() {
	grapheme_test("\u{000D}\u{0308}\u{0300}",
		&["\u{000D}", "\u{0308}\u{0300}"],
		"  ÷ [1.0] <CARRIAGE RETURN (CR)> (CR) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_11() {
	grapheme_test("\u{000D}\u{200C}",
		&["\u{000D}", "\u{200C}"],
		"  ÷ [1.0] <CARRIAGE RETURN (CR)> (CR) ÷ [4.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_12() {
	grapheme_test("\u{000D}\u{0308}\u{200C}",
		&["\u{000D}", "\u{0308}\u{200C}"],
		"  ÷ [1.0] <CARRIAGE RETURN (CR)> (CR) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_13() {
	grapheme_test("\u{000D}\u{200D}",
		&["\u{000D}", "\u{200D}"],
		"  ÷ [1.0] <CARRIAGE RETURN (CR)> (CR) ÷ [4.0] ZERO WIDTH JOINER (ZWJ) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_14() {
	grapheme_test("\u{000D}\u{0308}\u{200D}",
		&["\u{000D}", "\u{0308}\u{200D}"],
		"  ÷ [1.0] <CARRIAGE RETURN (CR)> (CR) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] ZERO WIDTH JOINER (ZWJ) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_15() {
	grapheme_test("\u{000D}\u{1F1E6}",
		&["\u{000D}", "\u{1F1E6}"],
		"  ÷ [1.0] <CARRIAGE RETURN (CR)> (CR) ÷ [4.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_16() {
	grapheme_test("\u{000D}\u{0308}\u{1F1E6}",
		&["\u{000D}", "\u{0308}", "\u{1F1E6}"],
		"  ÷ [1.0] <CARRIAGE RETURN (CR)> (CR) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_17() {
	grapheme_test("\u{000D}\u{06DD}",
		&["\u{000D}", "\u{06DD}"],
		"  ÷ [1.0] <CARRIAGE RETURN (CR)> (CR) ÷ [4.0] ARABIC END OF AYAH (Prepend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_18() {
	grapheme_test("\u{000D}\u{0308}\u{06DD}",
		&["\u{000D}", "\u{0308}", "\u{06DD}"],
		"  ÷ [1.0] <CARRIAGE RETURN (CR)> (CR) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] ARABIC END OF AYAH (Prepend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_19() {
	grapheme_test("\u{000D}\u{0903}",
		&["\u{000D}", "\u{0903}"],
		"  ÷ [1.0] <CARRIAGE RETURN (CR)> (CR) ÷ [4.0] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_20() {
	grapheme_test("\u{000D}\u{0308}\u{0903}",
		&["\u{000D}", "\u{0308}\u{0903}"],
		"  ÷ [1.0] <CARRIAGE RETURN (CR)> (CR) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.1] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_21() {
	grapheme_test("\u{000D}\u{1100}",
		&["\u{000D}", "\u{1100}"],
		"  ÷ [1.0] <CARRIAGE RETURN (CR)> (CR) ÷ [4.0] HANGUL CHOSEONG KIYEOK (L) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_22() {
	grapheme_test("\u{000D}\u{0308}\u{1100}",
		&["\u{000D}", "\u{0308}", "\u{1100}"],
		"  ÷ [1.0] <CARRIAGE RETURN (CR)> (CR) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL CHOSEONG KIYEOK (L) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_23() {
	grapheme_test("\u{000D}\u{1160}",
		&["\u{000D}", "\u{1160}"],
		"  ÷ [1.0] <CARRIAGE RETURN (CR)> (CR) ÷ [4.0] HANGUL JUNGSEONG FILLER (V) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_24() {
	grapheme_test("\u{000D}\u{0308}\u{1160}",
		&["\u{000D}", "\u{0308}", "\u{1160}"],
		"  ÷ [1.0] <CARRIAGE RETURN (CR)> (CR) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL JUNGSEONG FILLER (V) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_25() {
	grapheme_test("\u{000D}\u{11A8}",
		&["\u{000D}", "\u{11A8}"],
		"  ÷ [1.0] <CARRIAGE RETURN (CR)> (CR) ÷ [4.0] HANGUL JONGSEONG KIYEOK (T) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_26() {
	grapheme_test("\u{000D}\u{0308}\u{11A8}",
		&["\u{000D}", "\u{0308}", "\u{11A8}"],
		"  ÷ [1.0] <CARRIAGE RETURN (CR)> (CR) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL JONGSEONG KIYEOK (T) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_27() {
	grapheme_test("\u{000D}\u{AC00}",
		&["\u{000D}", "\u{AC00}"],
		"  ÷ [1.0] <CARRIAGE RETURN (CR)> (CR) ÷ [4.0] HANGUL SYLLABLE GA (LV) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_28() {
	grapheme_test("\u{000D}\u{0308}\u{AC00}",
		&["\u{000D}", "\u{0308}", "\u{AC00}"],
		"  ÷ [1.0] <CARRIAGE RETURN (CR)> (CR) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL SYLLABLE GA (LV) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_29() {
	grapheme_test("\u{000D}\u{AC01}",
		&["\u{000D}", "\u{AC01}"],
		"  ÷ [1.0] <CARRIAGE RETURN (CR)> (CR) ÷ [4.0] HANGUL SYLLABLE GAG (LVT) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_30() {
	grapheme_test("\u{000D}\u{0308}\u{AC01}",
		&["\u{000D}", "\u{0308}", "\u{AC01}"],
		"  ÷ [1.0] <CARRIAGE RETURN (CR)> (CR) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL SYLLABLE GAG (LVT) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_31() {
	grapheme_test("\u{000D}\u{1CF5}",
		&["\u{000D}", "\u{1CF5}"],
		"  ÷ [1.0] <CARRIAGE RETURN (CR)> (CR) ÷ [4.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_32() {
	grapheme_test("\u{000D}\u{0308}\u{1CF5}",
		&["\u{000D}", "\u{0308}", "\u{1CF5}"],
		"  ÷ [1.0] <CARRIAGE RETURN (CR)> (CR) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_33() {
	grapheme_test("\u{000D}\u{0915}",
		&["\u{000D}", "\u{0915}"],
		"  ÷ [1.0] <CARRIAGE RETURN (CR)> (CR) ÷ [4.0] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_34() {
	grapheme_test("\u{000D}\u{0308}\u{0915}",
		&["\u{000D}", "\u{0308}", "\u{0915}"],
		"  ÷ [1.0] <CARRIAGE RETURN (CR)> (CR) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_35() {
	grapheme_test("\u{000D}\u{00A9}",
		&["\u{000D}", "\u{00A9}"],
		"  ÷ [1.0] <CARRIAGE RETURN (CR)> (CR) ÷ [4.0] COPYRIGHT SIGN (ExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_36() {
	grapheme_test("\u{000D}\u{0308}\u{00A9}",
		&["\u{000D}", "\u{0308}", "\u{00A9}"],
		"  ÷ [1.0] <CARRIAGE RETURN (CR)> (CR) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] COPYRIGHT SIGN (ExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_37() {
	grapheme_test("\u{000D}\u{0020}",
		&["\u{000D}", "\u{0020}"],
		"  ÷ [1.0] <CARRIAGE RETURN (CR)> (CR) ÷ [4.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_38() {
	grapheme_test("\u{000D}\u{0308}\u{0020}",
		&["\u{000D}", "\u{0308}", "\u{0020}"],
		"  ÷ [1.0] <CARRIAGE RETURN (CR)> (CR) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_39() {
	grapheme_test("\u{000D}\u{0378}",
		&["\u{000D}", "\u{0378}"],
		"  ÷ [1.0] <CARRIAGE RETURN (CR)> (CR) ÷ [4.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_40() {
	grapheme_test("\u{000D}\u{0308}\u{0378}",
		&["\u{000D}", "\u{0308}", "\u{0378}"],
		"  ÷ [1.0] <CARRIAGE RETURN (CR)> (CR) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_41() {
	grapheme_test("\u{000A}\u{000D}",
		&["\u{000A}", "\u{000D}"],
		"  ÷ [1.0] <LINE FEED (LF)> (LF) ÷ [4.0] <CARRIAGE RETURN (CR)> (CR) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_42() {
	grapheme_test("\u{000A}\u{0308}\u{000D}",
		&["\u{000A}", "\u{0308}", "\u{000D}"],
		"  ÷ [1.0] <LINE FEED (LF)> (LF) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <CARRIAGE RETURN (CR)> (CR) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_43() {
	grapheme_test("\u{000A}\u{000A}",
		&["\u{000A}", "\u{000A}"],
		"  ÷ [1.0] <LINE FEED (LF)> (LF) ÷ [4.0] <LINE FEED (LF)> (LF) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_44() {
	grapheme_test("\u{000A}\u{0308}\u{000A}",
		&["\u{000A}", "\u{0308}", "\u{000A}"],
		"  ÷ [1.0] <LINE FEED (LF)> (LF) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <LINE FEED (LF)> (LF) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_45() {
	grapheme_test("\u{000A}\u{0000}",
		&["\u{000A}", "\u{0000}"],
		"  ÷ [1.0] <LINE FEED (LF)> (LF) ÷ [4.0] <NULL> (Control) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_46() {
	grapheme_test("\u{000A}\u{0308}\u{0000}",
		&["\u{000A}", "\u{0308}", "\u{0000}"],
		"  ÷ [1.0] <LINE FEED (LF)> (LF) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <NULL> (Control) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_47() {
	grapheme_test("\u{000A}\u{094D}",
		&["\u{000A}", "\u{094D}"],
		"  ÷ [1.0] <LINE FEED (LF)> (LF) ÷ [4.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_48() {
	grapheme_test("\u{000A}\u{0308}\u{094D}",
		&["\u{000A}", "\u{0308}\u{094D}"],
		"  ÷ [1.0] <LINE FEED (LF)> (LF) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_49() {
	grapheme_test("\u{000A}\u{0300}",
		&["\u{000A}", "\u{0300}"],
		"  ÷ [1.0] <LINE FEED (LF)> (LF) ÷ [4.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_50() {
	grapheme_test("\u{000A}\u{0308}\u{0300}",
		&["\u{000A}", "\u{0308}\u{0300}"],
		"  ÷ [1.0] <LINE FEED (LF)> (LF) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_51() {
	grapheme_test("\u{000A}\u{200C}",
		&["\u{000A}", "\u{200C}"],
		"  ÷ [1.0] <LINE FEED (LF)> (LF) ÷ [4.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_52() {
	grapheme_test("\u{000A}\u{0308}\u{200C}",
		&["\u{000A}", "\u{0308}\u{200C}"],
		"  ÷ [1.0] <LINE FEED (LF)> (LF) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_53() {
	grapheme_test("\u{000A}\u{200D}",
		&["\u{000A}", "\u{200D}"],
		"  ÷ [1.0] <LINE FEED (LF)> (LF) ÷ [4.0] ZERO WIDTH JOINER (ZWJ) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_54() {
	grapheme_test("\u{000A}\u{0308}\u{200D}",
		&["\u{000A}", "\u{0308}\u{200D}"],
		"  ÷ [1.0] <LINE FEED (LF)> (LF) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] ZERO WIDTH JOINER (ZWJ) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_55() {
	grapheme_test("\u{000A}\u{1F1E6}",
		&["\u{000A}", "\u{1F1E6}"],
		"  ÷ [1.0] <LINE FEED (LF)> (LF) ÷ [4.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_56() {
	grapheme_test("\u{000A}\u{0308}\u{1F1E6}",
		&["\u{000A}", "\u{0308}", "\u{1F1E6}"],
		"  ÷ [1.0] <LINE FEED (LF)> (LF) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_57() {
	grapheme_test("\u{000A}\u{06DD}",
		&["\u{000A}", "\u{06DD}"],
		"  ÷ [1.0] <LINE FEED (LF)> (LF) ÷ [4.0] ARABIC END OF AYAH (Prepend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_58() {
	grapheme_test("\u{000A}\u{0308}\u{06DD}",
		&["\u{000A}", "\u{0308}", "\u{06DD}"],
		"  ÷ [1.0] <LINE FEED (LF)> (LF) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] ARABIC END OF AYAH (Prepend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_59() {
	grapheme_test("\u{000A}\u{0903}",
		&["\u{000A}", "\u{0903}"],
		"  ÷ [1.0] <LINE FEED (LF)> (LF) ÷ [4.0] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_60() {
	grapheme_test("\u{000A}\u{0308}\u{0903}",
		&["\u{000A}", "\u{0308}\u{0903}"],
		"  ÷ [1.0] <LINE FEED (LF)> (LF) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.1] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_61() {
	grapheme_test("\u{000A}\u{1100}",
		&["\u{000A}", "\u{1100}"],
		"  ÷ [1.0] <LINE FEED (LF)> (LF) ÷ [4.0] HANGUL CHOSEONG KIYEOK (L) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_62() {
	grapheme_test("\u{000A}\u{0308}\u{1100}",
		&["\u{000A}", "\u{0308}", "\u{1100}"],
		"  ÷ [1.0] <LINE FEED (LF)> (LF) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL CHOSEONG KIYEOK (L) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_63() {
	grapheme_test("\u{000A}\u{1160}",
		&["\u{000A}", "\u{1160}"],
		"  ÷ [1.0] <LINE FEED (LF)> (LF) ÷ [4.0] HANGUL JUNGSEONG FILLER (V) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_64() {
	grapheme_test("\u{000A}\u{0308}\u{1160}",
		&["\u{000A}", "\u{0308}", "\u{1160}"],
		"  ÷ [1.0] <LINE FEED (LF)> (LF) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL JUNGSEONG FILLER (V) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_65() {
	grapheme_test("\u{000A}\u{11A8}",
		&["\u{000A}", "\u{11A8}"],
		"  ÷ [1.0] <LINE FEED (LF)> (LF) ÷ [4.0] HANGUL JONGSEONG KIYEOK (T) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_66() {
	grapheme_test("\u{000A}\u{0308}\u{11A8}",
		&["\u{000A}", "\u{0308}", "\u{11A8}"],
		"  ÷ [1.0] <LINE FEED (LF)> (LF) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL JONGSEONG KIYEOK (T) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_67() {
	grapheme_test("\u{000A}\u{AC00}",
		&["\u{000A}", "\u{AC00}"],
		"  ÷ [1.0] <LINE FEED (LF)> (LF) ÷ [4.0] HANGUL SYLLABLE GA (LV) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_68() {
	grapheme_test("\u{000A}\u{0308}\u{AC00}",
		&["\u{000A}", "\u{0308}", "\u{AC00}"],
		"  ÷ [1.0] <LINE FEED (LF)> (LF) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL SYLLABLE GA (LV) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_69() {
	grapheme_test("\u{000A}\u{AC01}",
		&["\u{000A}", "\u{AC01}"],
		"  ÷ [1.0] <LINE FEED (LF)> (LF) ÷ [4.0] HANGUL SYLLABLE GAG (LVT) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_70() {
	grapheme_test("\u{000A}\u{0308}\u{AC01}",
		&["\u{000A}", "\u{0308}", "\u{AC01}"],
		"  ÷ [1.0] <LINE FEED (LF)> (LF) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL SYLLABLE GAG (LVT) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_71() {
	grapheme_test("\u{000A}\u{1CF5}",
		&["\u{000A}", "\u{1CF5}"],
		"  ÷ [1.0] <LINE FEED (LF)> (LF) ÷ [4.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_72() {
	grapheme_test("\u{000A}\u{0308}\u{1CF5}",
		&["\u{000A}", "\u{0308}", "\u{1CF5}"],
		"  ÷ [1.0] <LINE FEED (LF)> (LF) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_73() {
	grapheme_test("\u{000A}\u{0915}",
		&["\u{000A}", "\u{0915}"],
		"  ÷ [1.0] <LINE FEED (LF)> (LF) ÷ [4.0] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_74() {
	grapheme_test("\u{000A}\u{0308}\u{0915}",
		&["\u{000A}", "\u{0308}", "\u{0915}"],
		"  ÷ [1.0] <LINE FEED (LF)> (LF) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_75() {
	grapheme_test("\u{000A}\u{00A9}",
		&["\u{000A}", "\u{00A9}"],
		"  ÷ [1.0] <LINE FEED (LF)> (LF) ÷ [4.0] COPYRIGHT SIGN (ExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_76() {
	grapheme_test("\u{000A}\u{0308}\u{00A9}",
		&["\u{000A}", "\u{0308}", "\u{00A9}"],
		"  ÷ [1.0] <LINE FEED (LF)> (LF) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] COPYRIGHT SIGN (ExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_77() {
	grapheme_test("\u{000A}\u{0020}",
		&["\u{000A}", "\u{0020}"],
		"  ÷ [1.0] <LINE FEED (LF)> (LF) ÷ [4.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_78() {
	grapheme_test("\u{000A}\u{0308}\u{0020}",
		&["\u{000A}", "\u{0308}", "\u{0020}"],
		"  ÷ [1.0] <LINE FEED (LF)> (LF) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_79() {
	grapheme_test("\u{000A}\u{0378}",
		&["\u{000A}", "\u{0378}"],
		"  ÷ [1.0] <LINE FEED (LF)> (LF) ÷ [4.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_80() {
	grapheme_test("\u{000A}\u{0308}\u{0378}",
		&["\u{000A}", "\u{0308}", "\u{0378}"],
		"  ÷ [1.0] <LINE FEED (LF)> (LF) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_81() {
	grapheme_test("\u{0000}\u{000D}",
		&["\u{0000}", "\u{000D}"],
		"  ÷ [1.0] <NULL> (Control) ÷ [4.0] <CARRIAGE RETURN (CR)> (CR) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_82() {
	grapheme_test("\u{0000}\u{0308}\u{000D}",
		&["\u{0000}", "\u{0308}", "\u{000D}"],
		"  ÷ [1.0] <NULL> (Control) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <CARRIAGE RETURN (CR)> (CR) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_83() {
	grapheme_test("\u{0000}\u{000A}",
		&["\u{0000}", "\u{000A}"],
		"  ÷ [1.0] <NULL> (Control) ÷ [4.0] <LINE FEED (LF)> (LF) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_84() {
	grapheme_test("\u{0000}\u{0308}\u{000A}",
		&["\u{0000}", "\u{0308}", "\u{000A}"],
		"  ÷ [1.0] <NULL> (Control) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <LINE FEED (LF)> (LF) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_85() {
	grapheme_test("\u{0000}\u{0000}",
		&["\u{0000}", "\u{0000}"],
		"  ÷ [1.0] <NULL> (Control) ÷ [4.0] <NULL> (Control) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_86() {
	grapheme_test("\u{0000}\u{0308}\u{0000}",
		&["\u{0000}", "\u{0308}", "\u{0000}"],
		"  ÷ [1.0] <NULL> (Control) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <NULL> (Control) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_87() {
	grapheme_test("\u{0000}\u{094D}",
		&["\u{0000}", "\u{094D}"],
		"  ÷ [1.0] <NULL> (Control) ÷ [4.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_88() {
	grapheme_test("\u{0000}\u{0308}\u{094D}",
		&["\u{0000}", "\u{0308}\u{094D}"],
		"  ÷ [1.0] <NULL> (Control) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_89() {
	grapheme_test("\u{0000}\u{0300}",
		&["\u{0000}", "\u{0300}"],
		"  ÷ [1.0] <NULL> (Control) ÷ [4.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_90() {
	grapheme_test("\u{0000}\u{0308}\u{0300}",
		&["\u{0000}", "\u{0308}\u{0300}"],
		"  ÷ [1.0] <NULL> (Control) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_91() {
	grapheme_test("\u{0000}\u{200C}",
		&["\u{0000}", "\u{200C}"],
		"  ÷ [1.0] <NULL> (Control) ÷ [4.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_92() {
	grapheme_test("\u{0000}\u{0308}\u{200C}",
		&["\u{0000}", "\u{0308}\u{200C}"],
		"  ÷ [1.0] <NULL> (Control) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_93() {
	grapheme_test("\u{0000}\u{200D}",
		&["\u{0000}", "\u{200D}"],
		"  ÷ [1.0] <NULL> (Control) ÷ [4.0] ZERO WIDTH JOINER (ZWJ) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_94() {
	grapheme_test("\u{0000}\u{0308}\u{200D}",
		&["\u{0000}", "\u{0308}\u{200D}"],
		"  ÷ [1.0] <NULL> (Control) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] ZERO WIDTH JOINER (ZWJ) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_95() {
	grapheme_test("\u{0000}\u{1F1E6}",
		&["\u{0000}", "\u{1F1E6}"],
		"  ÷ [1.0] <NULL> (Control) ÷ [4.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_96() {
	grapheme_test("\u{0000}\u{0308}\u{1F1E6}",
		&["\u{0000}", "\u{0308}", "\u{1F1E6}"],
		"  ÷ [1.0] <NULL> (Control) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_97() {
	grapheme_test("\u{0000}\u{06DD}",
		&["\u{0000}", "\u{06DD}"],
		"  ÷ [1.0] <NULL> (Control) ÷ [4.0] ARABIC END OF AYAH (Prepend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_98() {
	grapheme_test("\u{0000}\u{0308}\u{06DD}",
		&["\u{0000}", "\u{0308}", "\u{06DD}"],
		"  ÷ [1.0] <NULL> (Control) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] ARABIC END OF AYAH (Prepend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_99() {
	grapheme_test("\u{0000}\u{0903}",
		&["\u{0000}", "\u{0903}"],
		"  ÷ [1.0] <NULL> (Control) ÷ [4.0] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_100() {
	grapheme_test("\u{0000}\u{0308}\u{0903}",
		&["\u{0000}", "\u{0308}\u{0903}"],
		"  ÷ [1.0] <NULL> (Control) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.1] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_101() {
	grapheme_test("\u{0000}\u{1100}",
		&["\u{0000}", "\u{1100}"],
		"  ÷ [1.0] <NULL> (Control) ÷ [4.0] HANGUL CHOSEONG KIYEOK (L) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_102() {
	grapheme_test("\u{0000}\u{0308}\u{1100}",
		&["\u{0000}", "\u{0308}", "\u{1100}"],
		"  ÷ [1.0] <NULL> (Control) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL CHOSEONG KIYEOK (L) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_103() {
	grapheme_test("\u{0000}\u{1160}",
		&["\u{0000}", "\u{1160}"],
		"  ÷ [1.0] <NULL> (Control) ÷ [4.0] HANGUL JUNGSEONG FILLER (V) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_104() {
	grapheme_test("\u{0000}\u{0308}\u{1160}",
		&["\u{0000}", "\u{0308}", "\u{1160}"],
		"  ÷ [1.0] <NULL> (Control) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL JUNGSEONG FILLER (V) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_105() {
	grapheme_test("\u{0000}\u{11A8}",
		&["\u{0000}", "\u{11A8}"],
		"  ÷ [1.0] <NULL> (Control) ÷ [4.0] HANGUL JONGSEONG KIYEOK (T) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_106() {
	grapheme_test("\u{0000}\u{0308}\u{11A8}",
		&["\u{0000}", "\u{0308}", "\u{11A8}"],
		"  ÷ [1.0] <NULL> (Control) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL JONGSEONG KIYEOK (T) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_107() {
	grapheme_test("\u{0000}\u{AC00}",
		&["\u{0000}", "\u{AC00}"],
		"  ÷ [1.0] <NULL> (Control) ÷ [4.0] HANGUL SYLLABLE GA (LV) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_108() {
	grapheme_test("\u{0000}\u{0308}\u{AC00}",
		&["\u{0000}", "\u{0308}", "\u{AC00}"],
		"  ÷ [1.0] <NULL> (Control) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL SYLLABLE GA (LV) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_109() {
	grapheme_test("\u{0000}\u{AC01}",
		&["\u{0000}", "\u{AC01}"],
		"  ÷ [1.0] <NULL> (Control) ÷ [4.0] HANGUL SYLLABLE GAG (LVT) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_110() {
	grapheme_test("\u{0000}\u{0308}\u{AC01}",
		&["\u{0000}", "\u{0308}", "\u{AC01}"],
		"  ÷ [1.0] <NULL> (Control) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL SYLLABLE GAG (LVT) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_111() {
	grapheme_test("\u{0000}\u{1CF5}",
		&["\u{0000}", "\u{1CF5}"],
		"  ÷ [1.0] <NULL> (Control) ÷ [4.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_112() {
	grapheme_test("\u{0000}\u{0308}\u{1CF5}",
		&["\u{0000}", "\u{0308}", "\u{1CF5}"],
		"  ÷ [1.0] <NULL> (Control) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_113() {
	grapheme_test("\u{0000}\u{0915}",
		&["\u{0000}", "\u{0915}"],
		"  ÷ [1.0] <NULL> (Control) ÷ [4.0] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_114() {
	grapheme_test("\u{0000}\u{0308}\u{0915}",
		&["\u{0000}", "\u{0308}", "\u{0915}"],
		"  ÷ [1.0] <NULL> (Control) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_115() {
	grapheme_test("\u{0000}\u{00A9}",
		&["\u{0000}", "\u{00A9}"],
		"  ÷ [1.0] <NULL> (Control) ÷ [4.0] COPYRIGHT SIGN (ExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_116() {
	grapheme_test("\u{0000}\u{0308}\u{00A9}",
		&["\u{0000}", "\u{0308}", "\u{00A9}"],
		"  ÷ [1.0] <NULL> (Control) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] COPYRIGHT SIGN (ExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_117() {
	grapheme_test("\u{0000}\u{0020}",
		&["\u{0000}", "\u{0020}"],
		"  ÷ [1.0] <NULL> (Control) ÷ [4.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_118() {
	grapheme_test("\u{0000}\u{0308}\u{0020}",
		&["\u{0000}", "\u{0308}", "\u{0020}"],
		"  ÷ [1.0] <NULL> (Control) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_119() {
	grapheme_test("\u{0000}\u{0378}",
		&["\u{0000}", "\u{0378}"],
		"  ÷ [1.0] <NULL> (Control) ÷ [4.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_120() {
	grapheme_test("\u{0000}\u{0308}\u{0378}",
		&["\u{0000}", "\u{0308}", "\u{0378}"],
		"  ÷ [1.0] <NULL> (Control) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_121() {
	grapheme_test("\u{094D}\u{000D}",
		&["\u{094D}", "\u{000D}"],
		"  ÷ [1.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [5.0] <CARRIAGE RETURN (CR)> (CR) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_122() {
	grapheme_test("\u{094D}\u{0308}\u{000D}",
		&["\u{094D}\u{0308}", "\u{000D}"],
		"  ÷ [1.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <CARRIAGE RETURN (CR)> (CR) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_123() {
	grapheme_test("\u{094D}\u{000A}",
		&["\u{094D}", "\u{000A}"],
		"  ÷ [1.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [5.0] <LINE FEED (LF)> (LF) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_124() {
	grapheme_test("\u{094D}\u{0308}\u{000A}",
		&["\u{094D}\u{0308}", "\u{000A}"],
		"  ÷ [1.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <LINE FEED (LF)> (LF) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_125() {
	grapheme_test("\u{094D}\u{0000}",
		&["\u{094D}", "\u{0000}"],
		"  ÷ [1.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [5.0] <NULL> (Control) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_126() {
	grapheme_test("\u{094D}\u{0308}\u{0000}",
		&["\u{094D}\u{0308}", "\u{0000}"],
		"  ÷ [1.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <NULL> (Control) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_127() {
	grapheme_test("\u{094D}\u{094D}",
		&["\u{094D}\u{094D}"],
		"  ÷ [1.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) × [9.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_128() {
	grapheme_test("\u{094D}\u{0308}\u{094D}",
		&["\u{094D}\u{0308}\u{094D}"],
		"  ÷ [1.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_129() {
	grapheme_test("\u{094D}\u{0300}",
		&["\u{094D}\u{0300}"],
		"  ÷ [1.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) × [9.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_130() {
	grapheme_test("\u{094D}\u{0308}\u{0300}",
		&["\u{094D}\u{0308}\u{0300}"],
		"  ÷ [1.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_131() {
	grapheme_test("\u{094D}\u{200C}",
		&["\u{094D}\u{200C}"],
		"  ÷ [1.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) × [9.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_132() {
	grapheme_test("\u{094D}\u{0308}\u{200C}",
		&["\u{094D}\u{0308}\u{200C}"],
		"  ÷ [1.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_133() {
	grapheme_test("\u{094D}\u{200D}",
		&["\u{094D}\u{200D}"],
		"  ÷ [1.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) × [9.0] ZERO WIDTH JOINER (ZWJ) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_134() {
	grapheme_test("\u{094D}\u{0308}\u{200D}",
		&["\u{094D}\u{0308}\u{200D}"],
		"  ÷ [1.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] ZERO WIDTH JOINER (ZWJ) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_135() {
	grapheme_test("\u{094D}\u{1F1E6}",
		&["\u{094D}", "\u{1F1E6}"],
		"  ÷ [1.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [999.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_136() {
	grapheme_test("\u{094D}\u{0308}\u{1F1E6}",
		&["\u{094D}\u{0308}", "\u{1F1E6}"],
		"  ÷ [1.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_137() {
	grapheme_test("\u{094D}\u{06DD}",
		&["\u{094D}", "\u{06DD}"],
		"  ÷ [1.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [999.0] ARABIC END OF AYAH (Prepend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_138() {
	grapheme_test("\u{094D}\u{0308}\u{06DD}",
		&["\u{094D}\u{0308}", "\u{06DD}"],
		"  ÷ [1.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] ARABIC END OF AYAH (Prepend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_139() {
	grapheme_test("\u{094D}\u{0903}",
		&["\u{094D}\u{0903}"],
		"  ÷ [1.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) × [9.1] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_140() {
	grapheme_test("\u{094D}\u{0308}\u{0903}",
		&["\u{094D}\u{0308}\u{0903}"],
		"  ÷ [1.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.1] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_141() {
	grapheme_test("\u{094D}\u{1100}",
		&["\u{094D}", "\u{1100}"],
		"  ÷ [1.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [999.0] HANGUL CHOSEONG KIYEOK (L) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_142() {
	grapheme_test("\u{094D}\u{0308}\u{1100}",
		&["\u{094D}\u{0308}", "\u{1100}"],
		"  ÷ [1.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL CHOSEONG KIYEOK (L) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_143() {
	grapheme_test("\u{094D}\u{1160}",
		&["\u{094D}", "\u{1160}"],
		"  ÷ [1.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [999.0] HANGUL JUNGSEONG FILLER (V) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_144() {
	grapheme_test("\u{094D}\u{0308}\u{1160}",
		&["\u{094D}\u{0308}", "\u{1160}"],
		"  ÷ [1.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL JUNGSEONG FILLER (V) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_145() {
	grapheme_test("\u{094D}\u{11A8}",
		&["\u{094D}", "\u{11A8}"],
		"  ÷ [1.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [999.0] HANGUL JONGSEONG KIYEOK (T) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_146() {
	grapheme_test("\u{094D}\u{0308}\u{11A8}",
		&["\u{094D}\u{0308}", "\u{11A8}"],
		"  ÷ [1.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL JONGSEONG KIYEOK (T) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_147() {
	grapheme_test("\u{094D}\u{AC00}",
		&["\u{094D}", "\u{AC00}"],
		"  ÷ [1.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [999.0] HANGUL SYLLABLE GA (LV) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_148() {
	grapheme_test("\u{094D}\u{0308}\u{AC00}",
		&["\u{094D}\u{0308}", "\u{AC00}"],
		"  ÷ [1.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL SYLLABLE GA (LV) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_149() {
	grapheme_test("\u{094D}\u{AC01}",
		&["\u{094D}", "\u{AC01}"],
		"  ÷ [1.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [999.0] HANGUL SYLLABLE GAG (LVT) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_150() {
	grapheme_test("\u{094D}\u{0308}\u{AC01}",
		&["\u{094D}\u{0308}", "\u{AC01}"],
		"  ÷ [1.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL SYLLABLE GAG (LVT) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_151() {
	grapheme_test("\u{094D}\u{1CF5}",
		&["\u{094D}", "\u{1CF5}"],
		"  ÷ [1.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [999.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_152() {
	grapheme_test("\u{094D}\u{0308}\u{1CF5}",
		&["\u{094D}\u{0308}", "\u{1CF5}"],
		"  ÷ [1.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_153() {
	grapheme_test("\u{094D}\u{0915}",
		&["\u{094D}\u{0915}"],
		"  ÷ [1.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) × [9.3] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_154() {
	grapheme_test("\u{094D}\u{0308}\u{0915}",
		&["\u{094D}\u{0308}\u{0915}"],
		"  ÷ [1.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.3] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_155() {
	grapheme_test("\u{094D}\u{00A9}",
		&["\u{094D}", "\u{00A9}"],
		"  ÷ [1.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [999.0] COPYRIGHT SIGN (ExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_156() {
	grapheme_test("\u{094D}\u{0308}\u{00A9}",
		&["\u{094D}\u{0308}", "\u{00A9}"],
		"  ÷ [1.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] COPYRIGHT SIGN (ExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_157() {
	grapheme_test("\u{094D}\u{0020}",
		&["\u{094D}", "\u{0020}"],
		"  ÷ [1.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [999.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_158() {
	grapheme_test("\u{094D}\u{0308}\u{0020}",
		&["\u{094D}\u{0308}", "\u{0020}"],
		"  ÷ [1.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_159() {
	grapheme_test("\u{094D}\u{0378}",
		&["\u{094D}", "\u{0378}"],
		"  ÷ [1.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [999.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_160() {
	grapheme_test("\u{094D}\u{0308}\u{0378}",
		&["\u{094D}\u{0308}", "\u{0378}"],
		"  ÷ [1.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_161() {
	grapheme_test("\u{0300}\u{000D}",
		&["\u{0300}", "\u{000D}"],
		"  ÷ [1.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [5.0] <CARRIAGE RETURN (CR)> (CR) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_162() {
	grapheme_test("\u{0300}\u{0308}\u{000D}",
		&["\u{0300}\u{0308}", "\u{000D}"],
		"  ÷ [1.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <CARRIAGE RETURN (CR)> (CR) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_163() {
	grapheme_test("\u{0300}\u{000A}",
		&["\u{0300}", "\u{000A}"],
		"  ÷ [1.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [5.0] <LINE FEED (LF)> (LF) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_164() {
	grapheme_test("\u{0300}\u{0308}\u{000A}",
		&["\u{0300}\u{0308}", "\u{000A}"],
		"  ÷ [1.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <LINE FEED (LF)> (LF) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_165() {
	grapheme_test("\u{0300}\u{0000}",
		&["\u{0300}", "\u{0000}"],
		"  ÷ [1.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [5.0] <NULL> (Control) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_166() {
	grapheme_test("\u{0300}\u{0308}\u{0000}",
		&["\u{0300}\u{0308}", "\u{0000}"],
		"  ÷ [1.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <NULL> (Control) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_167() {
	grapheme_test("\u{0300}\u{094D}",
		&["\u{0300}\u{094D}"],
		"  ÷ [1.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) × [9.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_168() {
	grapheme_test("\u{0300}\u{0308}\u{094D}",
		&["\u{0300}\u{0308}\u{094D}"],
		"  ÷ [1.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_169() {
	grapheme_test("\u{0300}\u{0300}",
		&["\u{0300}\u{0300}"],
		"  ÷ [1.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) × [9.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_170() {
	grapheme_test("\u{0300}\u{0308}\u{0300}",
		&["\u{0300}\u{0308}\u{0300}"],
		"  ÷ [1.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_171() {
	grapheme_test("\u{0300}\u{200C}",
		&["\u{0300}\u{200C}"],
		"  ÷ [1.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) × [9.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_172() {
	grapheme_test("\u{0300}\u{0308}\u{200C}",
		&["\u{0300}\u{0308}\u{200C}"],
		"  ÷ [1.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_173() {
	grapheme_test("\u{0300}\u{200D}",
		&["\u{0300}\u{200D}"],
		"  ÷ [1.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) × [9.0] ZERO WIDTH JOINER (ZWJ) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_174() {
	grapheme_test("\u{0300}\u{0308}\u{200D}",
		&["\u{0300}\u{0308}\u{200D}"],
		"  ÷ [1.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] ZERO WIDTH JOINER (ZWJ) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_175() {
	grapheme_test("\u{0300}\u{1F1E6}",
		&["\u{0300}", "\u{1F1E6}"],
		"  ÷ [1.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [999.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_176() {
	grapheme_test("\u{0300}\u{0308}\u{1F1E6}",
		&["\u{0300}\u{0308}", "\u{1F1E6}"],
		"  ÷ [1.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_177() {
	grapheme_test("\u{0300}\u{06DD}",
		&["\u{0300}", "\u{06DD}"],
		"  ÷ [1.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [999.0] ARABIC END OF AYAH (Prepend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_178() {
	grapheme_test("\u{0300}\u{0308}\u{06DD}",
		&["\u{0300}\u{0308}", "\u{06DD}"],
		"  ÷ [1.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] ARABIC END OF AYAH (Prepend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_179() {
	grapheme_test("\u{0300}\u{0903}",
		&["\u{0300}\u{0903}"],
		"  ÷ [1.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) × [9.1] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_180() {
	grapheme_test("\u{0300}\u{0308}\u{0903}",
		&["\u{0300}\u{0308}\u{0903}"],
		"  ÷ [1.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.1] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_181() {
	grapheme_test("\u{0300}\u{1100}",
		&["\u{0300}", "\u{1100}"],
		"  ÷ [1.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [999.0] HANGUL CHOSEONG KIYEOK (L) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_182() {
	grapheme_test("\u{0300}\u{0308}\u{1100}",
		&["\u{0300}\u{0308}", "\u{1100}"],
		"  ÷ [1.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL CHOSEONG KIYEOK (L) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_183() {
	grapheme_test("\u{0300}\u{1160}",
		&["\u{0300}", "\u{1160}"],
		"  ÷ [1.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [999.0] HANGUL JUNGSEONG FILLER (V) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_184() {
	grapheme_test("\u{0300}\u{0308}\u{1160}",
		&["\u{0300}\u{0308}", "\u{1160}"],
		"  ÷ [1.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL JUNGSEONG FILLER (V) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_185() {
	grapheme_test("\u{0300}\u{11A8}",
		&["\u{0300}", "\u{11A8}"],
		"  ÷ [1.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [999.0] HANGUL JONGSEONG KIYEOK (T) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_186() {
	grapheme_test("\u{0300}\u{0308}\u{11A8}",
		&["\u{0300}\u{0308}", "\u{11A8}"],
		"  ÷ [1.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL JONGSEONG KIYEOK (T) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_187() {
	grapheme_test("\u{0300}\u{AC00}",
		&["\u{0300}", "\u{AC00}"],
		"  ÷ [1.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [999.0] HANGUL SYLLABLE GA (LV) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_188() {
	grapheme_test("\u{0300}\u{0308}\u{AC00}",
		&["\u{0300}\u{0308}", "\u{AC00}"],
		"  ÷ [1.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL SYLLABLE GA (LV) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_189() {
	grapheme_test("\u{0300}\u{AC01}",
		&["\u{0300}", "\u{AC01}"],
		"  ÷ [1.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [999.0] HANGUL SYLLABLE GAG (LVT) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_190() {
	grapheme_test("\u{0300}\u{0308}\u{AC01}",
		&["\u{0300}\u{0308}", "\u{AC01}"],
		"  ÷ [1.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL SYLLABLE GAG (LVT) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_191() {
	grapheme_test("\u{0300}\u{1CF5}",
		&["\u{0300}", "\u{1CF5}"],
		"  ÷ [1.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [999.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_192() {
	grapheme_test("\u{0300}\u{0308}\u{1CF5}",
		&["\u{0300}\u{0308}", "\u{1CF5}"],
		"  ÷ [1.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_193() {
	grapheme_test("\u{0300}\u{0915}",
		&["\u{0300}", "\u{0915}"],
		"  ÷ [1.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [999.0] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_194() {
	grapheme_test("\u{0300}\u{0308}\u{0915}",
		&["\u{0300}\u{0308}", "\u{0915}"],
		"  ÷ [1.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_195() {
	grapheme_test("\u{0300}\u{00A9}",
		&["\u{0300}", "\u{00A9}"],
		"  ÷ [1.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [999.0] COPYRIGHT SIGN (ExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_196() {
	grapheme_test("\u{0300}\u{0308}\u{00A9}",
		&["\u{0300}\u{0308}", "\u{00A9}"],
		"  ÷ [1.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] COPYRIGHT SIGN (ExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_197() {
	grapheme_test("\u{0300}\u{0020}",
		&["\u{0300}", "\u{0020}"],
		"  ÷ [1.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [999.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_198() {
	grapheme_test("\u{0300}\u{0308}\u{0020}",
		&["\u{0300}\u{0308}", "\u{0020}"],
		"  ÷ [1.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_199() {
	grapheme_test("\u{0300}\u{0378}",
		&["\u{0300}", "\u{0378}"],
		"  ÷ [1.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [999.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_200() {
	grapheme_test("\u{0300}\u{0308}\u{0378}",
		&["\u{0300}\u{0308}", "\u{0378}"],
		"  ÷ [1.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_201() {
	grapheme_test("\u{200C}\u{000D}",
		&["\u{200C}", "\u{000D}"],
		"  ÷ [1.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [5.0] <CARRIAGE RETURN (CR)> (CR) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_202() {
	grapheme_test("\u{200C}\u{0308}\u{000D}",
		&["\u{200C}\u{0308}", "\u{000D}"],
		"  ÷ [1.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <CARRIAGE RETURN (CR)> (CR) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_203() {
	grapheme_test("\u{200C}\u{000A}",
		&["\u{200C}", "\u{000A}"],
		"  ÷ [1.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [5.0] <LINE FEED (LF)> (LF) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_204() {
	grapheme_test("\u{200C}\u{0308}\u{000A}",
		&["\u{200C}\u{0308}", "\u{000A}"],
		"  ÷ [1.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <LINE FEED (LF)> (LF) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_205() {
	grapheme_test("\u{200C}\u{0000}",
		&["\u{200C}", "\u{0000}"],
		"  ÷ [1.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [5.0] <NULL> (Control) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_206() {
	grapheme_test("\u{200C}\u{0308}\u{0000}",
		&["\u{200C}\u{0308}", "\u{0000}"],
		"  ÷ [1.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <NULL> (Control) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_207() {
	grapheme_test("\u{200C}\u{094D}",
		&["\u{200C}\u{094D}"],
		"  ÷ [1.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) × [9.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_208() {
	grapheme_test("\u{200C}\u{0308}\u{094D}",
		&["\u{200C}\u{0308}\u{094D}"],
		"  ÷ [1.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_209() {
	grapheme_test("\u{200C}\u{0300}",
		&["\u{200C}\u{0300}"],
		"  ÷ [1.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) × [9.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_210() {
	grapheme_test("\u{200C}\u{0308}\u{0300}",
		&["\u{200C}\u{0308}\u{0300}"],
		"  ÷ [1.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_211() {
	grapheme_test("\u{200C}\u{200C}",
		&["\u{200C}\u{200C}"],
		"  ÷ [1.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) × [9.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_212() {
	grapheme_test("\u{200C}\u{0308}\u{200C}",
		&["\u{200C}\u{0308}\u{200C}"],
		"  ÷ [1.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_213() {
	grapheme_test("\u{200C}\u{200D}",
		&["\u{200C}\u{200D}"],
		"  ÷ [1.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) × [9.0] ZERO WIDTH JOINER (ZWJ) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_214() {
	grapheme_test("\u{200C}\u{0308}\u{200D}",
		&["\u{200C}\u{0308}\u{200D}"],
		"  ÷ [1.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] ZERO WIDTH JOINER (ZWJ) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_215() {
	grapheme_test("\u{200C}\u{1F1E6}",
		&["\u{200C}", "\u{1F1E6}"],
		"  ÷ [1.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [999.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_216() {
	grapheme_test("\u{200C}\u{0308}\u{1F1E6}",
		&["\u{200C}\u{0308}", "\u{1F1E6}"],
		"  ÷ [1.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_217() {
	grapheme_test("\u{200C}\u{06DD}",
		&["\u{200C}", "\u{06DD}"],
		"  ÷ [1.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [999.0] ARABIC END OF AYAH (Prepend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_218() {
	grapheme_test("\u{200C}\u{0308}\u{06DD}",
		&["\u{200C}\u{0308}", "\u{06DD}"],
		"  ÷ [1.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] ARABIC END OF AYAH (Prepend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_219() {
	grapheme_test("\u{200C}\u{0903}",
		&["\u{200C}\u{0903}"],
		"  ÷ [1.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) × [9.1] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_220() {
	grapheme_test("\u{200C}\u{0308}\u{0903}",
		&["\u{200C}\u{0308}\u{0903}"],
		"  ÷ [1.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.1] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_221() {
	grapheme_test("\u{200C}\u{1100}",
		&["\u{200C}", "\u{1100}"],
		"  ÷ [1.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [999.0] HANGUL CHOSEONG KIYEOK (L) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_222() {
	grapheme_test("\u{200C}\u{0308}\u{1100}",
		&["\u{200C}\u{0308}", "\u{1100}"],
		"  ÷ [1.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL CHOSEONG KIYEOK (L) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_223() {
	grapheme_test("\u{200C}\u{1160}",
		&["\u{200C}", "\u{1160}"],
		"  ÷ [1.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [999.0] HANGUL JUNGSEONG FILLER (V) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_224() {
	grapheme_test("\u{200C}\u{0308}\u{1160}",
		&["\u{200C}\u{0308}", "\u{1160}"],
		"  ÷ [1.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL JUNGSEONG FILLER (V) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_225() {
	grapheme_test("\u{200C}\u{11A8}",
		&["\u{200C}", "\u{11A8}"],
		"  ÷ [1.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [999.0] HANGUL JONGSEONG KIYEOK (T) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_226() {
	grapheme_test("\u{200C}\u{0308}\u{11A8}",
		&["\u{200C}\u{0308}", "\u{11A8}"],
		"  ÷ [1.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL JONGSEONG KIYEOK (T) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_227() {
	grapheme_test("\u{200C}\u{AC00}",
		&["\u{200C}", "\u{AC00}"],
		"  ÷ [1.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [999.0] HANGUL SYLLABLE GA (LV) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_228() {
	grapheme_test("\u{200C}\u{0308}\u{AC00}",
		&["\u{200C}\u{0308}", "\u{AC00}"],
		"  ÷ [1.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL SYLLABLE GA (LV) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_229() {
	grapheme_test("\u{200C}\u{AC01}",
		&["\u{200C}", "\u{AC01}"],
		"  ÷ [1.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [999.0] HANGUL SYLLABLE GAG (LVT) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_230() {
	grapheme_test("\u{200C}\u{0308}\u{AC01}",
		&["\u{200C}\u{0308}", "\u{AC01}"],
		"  ÷ [1.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL SYLLABLE GAG (LVT) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_231() {
	grapheme_test("\u{200C}\u{1CF5}",
		&["\u{200C}", "\u{1CF5}"],
		"  ÷ [1.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [999.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_232() {
	grapheme_test("\u{200C}\u{0308}\u{1CF5}",
		&["\u{200C}\u{0308}", "\u{1CF5}"],
		"  ÷ [1.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_233() {
	grapheme_test("\u{200C}\u{0915}",
		&["\u{200C}", "\u{0915}"],
		"  ÷ [1.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [999.0] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_234() {
	grapheme_test("\u{200C}\u{0308}\u{0915}",
		&["\u{200C}\u{0308}", "\u{0915}"],
		"  ÷ [1.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_235() {
	grapheme_test("\u{200C}\u{00A9}",
		&["\u{200C}", "\u{00A9}"],
		"  ÷ [1.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [999.0] COPYRIGHT SIGN (ExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_236() {
	grapheme_test("\u{200C}\u{0308}\u{00A9}",
		&["\u{200C}\u{0308}", "\u{00A9}"],
		"  ÷ [1.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] COPYRIGHT SIGN (ExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_237() {
	grapheme_test("\u{200C}\u{0020}",
		&["\u{200C}", "\u{0020}"],
		"  ÷ [1.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [999.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_238() {
	grapheme_test("\u{200C}\u{0308}\u{0020}",
		&["\u{200C}\u{0308}", "\u{0020}"],
		"  ÷ [1.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_239() {
	grapheme_test("\u{200C}\u{0378}",
		&["\u{200C}", "\u{0378}"],
		"  ÷ [1.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [999.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_240() {
	grapheme_test("\u{200C}\u{0308}\u{0378}",
		&["\u{200C}\u{0308}", "\u{0378}"],
		"  ÷ [1.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_241() {
	grapheme_test("\u{200D}\u{000D}",
		&["\u{200D}", "\u{000D}"],
		"  ÷ [1.0] ZERO WIDTH JOINER (ZWJ) ÷ [5.0] <CARRIAGE RETURN (CR)> (CR) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_242() {
	grapheme_test("\u{200D}\u{0308}\u{000D}",
		&["\u{200D}\u{0308}", "\u{000D}"],
		"  ÷ [1.0] ZERO WIDTH JOINER (ZWJ) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <CARRIAGE RETURN (CR)> (CR) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_243() {
	grapheme_test("\u{200D}\u{000A}",
		&["\u{200D}", "\u{000A}"],
		"  ÷ [1.0] ZERO WIDTH JOINER (ZWJ) ÷ [5.0] <LINE FEED (LF)> (LF) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_244() {
	grapheme_test("\u{200D}\u{0308}\u{000A}",
		&["\u{200D}\u{0308}", "\u{000A}"],
		"  ÷ [1.0] ZERO WIDTH JOINER (ZWJ) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <LINE FEED (LF)> (LF) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_245() {
	grapheme_test("\u{200D}\u{0000}",
		&["\u{200D}", "\u{0000}"],
		"  ÷ [1.0] ZERO WIDTH JOINER (ZWJ) ÷ [5.0] <NULL> (Control) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_246() {
	grapheme_test("\u{200D}\u{0308}\u{0000}",
		&["\u{200D}\u{0308}", "\u{0000}"],
		"  ÷ [1.0] ZERO WIDTH JOINER (ZWJ) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <NULL> (Control) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_247() {
	grapheme_test("\u{200D}\u{094D}",
		&["\u{200D}\u{094D}"],
		"  ÷ [1.0] ZERO WIDTH JOINER (ZWJ) × [9.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_248() {
	grapheme_test("\u{200D}\u{0308}\u{094D}",
		&["\u{200D}\u{0308}\u{094D}"],
		"  ÷ [1.0] ZERO WIDTH JOINER (ZWJ) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_249() {
	grapheme_test("\u{200D}\u{0300}",
		&["\u{200D}\u{0300}"],
		"  ÷ [1.0] ZERO WIDTH JOINER (ZWJ) × [9.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_250() {
	grapheme_test("\u{200D}\u{0308}\u{0300}",
		&["\u{200D}\u{0308}\u{0300}"],
		"  ÷ [1.0] ZERO WIDTH JOINER (ZWJ) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_251() {
	grapheme_test("\u{200D}\u{200C}",
		&["\u{200D}\u{200C}"],
		"  ÷ [1.0] ZERO WIDTH JOINER (ZWJ) × [9.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_252() {
	grapheme_test("\u{200D}\u{0308}\u{200C}",
		&["\u{200D}\u{0308}\u{200C}"],
		"  ÷ [1.0] ZERO WIDTH JOINER (ZWJ) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_253() {
	grapheme_test("\u{200D}\u{200D}",
		&["\u{200D}\u{200D}"],
		"  ÷ [1.0] ZERO WIDTH JOINER (ZWJ) × [9.0] ZERO WIDTH JOINER (ZWJ) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_254() {
	grapheme_test("\u{200D}\u{0308}\u{200D}",
		&["\u{200D}\u{0308}\u{200D}"],
		"  ÷ [1.0] ZERO WIDTH JOINER (ZWJ) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] ZERO WIDTH JOINER (ZWJ) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_255() {
	grapheme_test("\u{200D}\u{1F1E6}",
		&["\u{200D}", "\u{1F1E6}"],
		"  ÷ [1.0] ZERO WIDTH JOINER (ZWJ) ÷ [999.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_256() {
	grapheme_test("\u{200D}\u{0308}\u{1F1E6}",
		&["\u{200D}\u{0308}", "\u{1F1E6}"],
		"  ÷ [1.0] ZERO WIDTH JOINER (ZWJ) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_257() {
	grapheme_test("\u{200D}\u{06DD}",
		&["\u{200D}", "\u{06DD}"],
		"  ÷ [1.0] ZERO WIDTH JOINER (ZWJ) ÷ [999.0] ARABIC END OF AYAH (Prepend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_258() {
	grapheme_test("\u{200D}\u{0308}\u{06DD}",
		&["\u{200D}\u{0308}", "\u{06DD}"],
		"  ÷ [1.0] ZERO WIDTH JOINER (ZWJ) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] ARABIC END OF AYAH (Prepend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_259() {
	grapheme_test("\u{200D}\u{0903}",
		&["\u{200D}\u{0903}"],
		"  ÷ [1.0] ZERO WIDTH JOINER (ZWJ) × [9.1] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_260() {
	grapheme_test("\u{200D}\u{0308}\u{0903}",
		&["\u{200D}\u{0308}\u{0903}"],
		"  ÷ [1.0] ZERO WIDTH JOINER (ZWJ) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.1] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_261() {
	grapheme_test("\u{200D}\u{1100}",
		&["\u{200D}", "\u{1100}"],
		"  ÷ [1.0] ZERO WIDTH JOINER (ZWJ) ÷ [999.0] HANGUL CHOSEONG KIYEOK (L) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_262() {
	grapheme_test("\u{200D}\u{0308}\u{1100}",
		&["\u{200D}\u{0308}", "\u{1100}"],
		"  ÷ [1.0] ZERO WIDTH JOINER (ZWJ) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL CHOSEONG KIYEOK (L) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_263() {
	grapheme_test("\u{200D}\u{1160}",
		&["\u{200D}", "\u{1160}"],
		"  ÷ [1.0] ZERO WIDTH JOINER (ZWJ) ÷ [999.0] HANGUL JUNGSEONG FILLER (V) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_264() {
	grapheme_test("\u{200D}\u{0308}\u{1160}",
		&["\u{200D}\u{0308}", "\u{1160}"],
		"  ÷ [1.0] ZERO WIDTH JOINER (ZWJ) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL JUNGSEONG FILLER (V) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_265() {
	grapheme_test("\u{200D}\u{11A8}",
		&["\u{200D}", "\u{11A8}"],
		"  ÷ [1.0] ZERO WIDTH JOINER (ZWJ) ÷ [999.0] HANGUL JONGSEONG KIYEOK (T) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_266() {
	grapheme_test("\u{200D}\u{0308}\u{11A8}",
		&["\u{200D}\u{0308}", "\u{11A8}"],
		"  ÷ [1.0] ZERO WIDTH JOINER (ZWJ) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL JONGSEONG KIYEOK (T) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_267() {
	grapheme_test("\u{200D}\u{AC00}",
		&["\u{200D}", "\u{AC00}"],
		"  ÷ [1.0] ZERO WIDTH JOINER (ZWJ) ÷ [999.0] HANGUL SYLLABLE GA (LV) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_268() {
	grapheme_test("\u{200D}\u{0308}\u{AC00}",
		&["\u{200D}\u{0308}", "\u{AC00}"],
		"  ÷ [1.0] ZERO WIDTH JOINER (ZWJ) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL SYLLABLE GA (LV) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_269() {
	grapheme_test("\u{200D}\u{AC01}",
		&["\u{200D}", "\u{AC01}"],
		"  ÷ [1.0] ZERO WIDTH JOINER (ZWJ) ÷ [999.0] HANGUL SYLLABLE GAG (LVT) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_270() {
	grapheme_test("\u{200D}\u{0308}\u{AC01}",
		&["\u{200D}\u{0308}", "\u{AC01}"],
		"  ÷ [1.0] ZERO WIDTH JOINER (ZWJ) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL SYLLABLE GAG (LVT) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_271() {
	grapheme_test("\u{200D}\u{1CF5}",
		&["\u{200D}", "\u{1CF5}"],
		"  ÷ [1.0] ZERO WIDTH JOINER (ZWJ) ÷ [999.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_272() {
	grapheme_test("\u{200D}\u{0308}\u{1CF5}",
		&["\u{200D}\u{0308}", "\u{1CF5}"],
		"  ÷ [1.0] ZERO WIDTH JOINER (ZWJ) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_273() {
	grapheme_test("\u{200D}\u{0915}",
		&["\u{200D}", "\u{0915}"],
		"  ÷ [1.0] ZERO WIDTH JOINER (ZWJ) ÷ [999.0] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_274() {
	grapheme_test("\u{200D}\u{0308}\u{0915}",
		&["\u{200D}\u{0308}", "\u{0915}"],
		"  ÷ [1.0] ZERO WIDTH JOINER (ZWJ) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_275() {
	grapheme_test("\u{200D}\u{00A9}",
		&["\u{200D}", "\u{00A9}"],
		"  ÷ [1.0] ZERO WIDTH JOINER (ZWJ) ÷ [999.0] COPYRIGHT SIGN (ExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_276() {
	grapheme_test("\u{200D}\u{0308}\u{00A9}",
		&["\u{200D}\u{0308}", "\u{00A9}"],
		"  ÷ [1.0] ZERO WIDTH JOINER (ZWJ) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] COPYRIGHT SIGN (ExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_277() {
	grapheme_test("\u{200D}\u{0020}",
		&["\u{200D}", "\u{0020}"],
		"  ÷ [1.0] ZERO WIDTH JOINER (ZWJ) ÷ [999.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_278() {
	grapheme_test("\u{200D}\u{0308}\u{0020}",
		&["\u{200D}\u{0308}", "\u{0020}"],
		"  ÷ [1.0] ZERO WIDTH JOINER (ZWJ) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_279() {
	grapheme_test("\u{200D}\u{0378}",
		&["\u{200D}", "\u{0378}"],
		"  ÷ [1.0] ZERO WIDTH JOINER (ZWJ) ÷ [999.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_280() {
	grapheme_test("\u{200D}\u{0308}\u{0378}",
		&["\u{200D}\u{0308}", "\u{0378}"],
		"  ÷ [1.0] ZERO WIDTH JOINER (ZWJ) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_281() {
	grapheme_test("\u{1F1E6}\u{000D}",
		&["\u{1F1E6}", "\u{000D}"],
		"  ÷ [1.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [5.0] <CARRIAGE RETURN (CR)> (CR) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_282() {
	grapheme_test("\u{1F1E6}\u{0308}\u{000D}",
		&["\u{1F1E6}\u{0308}", "\u{000D}"],
		"  ÷ [1.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <CARRIAGE RETURN (CR)> (CR) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_283() {
	grapheme_test("\u{1F1E6}\u{000A}",
		&["\u{1F1E6}", "\u{000A}"],
		"  ÷ [1.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [5.0] <LINE FEED (LF)> (LF) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_284() {
	grapheme_test("\u{1F1E6}\u{0308}\u{000A}",
		&["\u{1F1E6}\u{0308}", "\u{000A}"],
		"  ÷ [1.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <LINE FEED (LF)> (LF) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_285() {
	grapheme_test("\u{1F1E6}\u{0000}",
		&["\u{1F1E6}", "\u{0000}"],
		"  ÷ [1.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [5.0] <NULL> (Control) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_286() {
	grapheme_test("\u{1F1E6}\u{0308}\u{0000}",
		&["\u{1F1E6}\u{0308}", "\u{0000}"],
		"  ÷ [1.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <NULL> (Control) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_287() {
	grapheme_test("\u{1F1E6}\u{094D}",
		&["\u{1F1E6}\u{094D}"],
		"  ÷ [1.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) × [9.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_288() {
	grapheme_test("\u{1F1E6}\u{0308}\u{094D}",
		&["\u{1F1E6}\u{0308}\u{094D}"],
		"  ÷ [1.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_289() {
	grapheme_test("\u{1F1E6}\u{0300}",
		&["\u{1F1E6}\u{0300}"],
		"  ÷ [1.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) × [9.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_290() {
	grapheme_test("\u{1F1E6}\u{0308}\u{0300}",
		&["\u{1F1E6}\u{0308}\u{0300}"],
		"  ÷ [1.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_291() {
	grapheme_test("\u{1F1E6}\u{200C}",
		&["\u{1F1E6}\u{200C}"],
		"  ÷ [1.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) × [9.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_292() {
	grapheme_test("\u{1F1E6}\u{0308}\u{200C}",
		&["\u{1F1E6}\u{0308}\u{200C}"],
		"  ÷ [1.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_293() {
	grapheme_test("\u{1F1E6}\u{200D}",
		&["\u{1F1E6}\u{200D}"],
		"  ÷ [1.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) × [9.0] ZERO WIDTH JOINER (ZWJ) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_294() {
	grapheme_test("\u{1F1E6}\u{0308}\u{200D}",
		&["\u{1F1E6}\u{0308}\u{200D}"],
		"  ÷ [1.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] ZERO WIDTH JOINER (ZWJ) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_295() {
	grapheme_test("\u{1F1E6}\u{1F1E6}",
		&["\u{1F1E6}\u{1F1E6}"],
		"  ÷ [1.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) × [12.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_296() {
	grapheme_test("\u{1F1E6}\u{0308}\u{1F1E6}",
		&["\u{1F1E6}\u{0308}", "\u{1F1E6}"],
		"  ÷ [1.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_297() {
	grapheme_test("\u{1F1E6}\u{06DD}",
		&["\u{1F1E6}", "\u{06DD}"],
		"  ÷ [1.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [999.0] ARABIC END OF AYAH (Prepend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_298() {
	grapheme_test("\u{1F1E6}\u{0308}\u{06DD}",
		&["\u{1F1E6}\u{0308}", "\u{06DD}"],
		"  ÷ [1.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] ARABIC END OF AYAH (Prepend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_299() {
	grapheme_test("\u{1F1E6}\u{0903}",
		&["\u{1F1E6}\u{0903}"],
		"  ÷ [1.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) × [9.1] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_300() {
	grapheme_test("\u{1F1E6}\u{0308}\u{0903}",
		&["\u{1F1E6}\u{0308}\u{0903}"],
		"  ÷ [1.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.1] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_301() {
	grapheme_test("\u{1F1E6}\u{1100}",
		&["\u{1F1E6}", "\u{1100}"],
		"  ÷ [1.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [999.0] HANGUL CHOSEONG KIYEOK (L) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_302() {
	grapheme_test("\u{1F1E6}\u{0308}\u{1100}",
		&["\u{1F1E6}\u{0308}", "\u{1100}"],
		"  ÷ [1.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL CHOSEONG KIYEOK (L) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_303() {
	grapheme_test("\u{1F1E6}\u{1160}",
		&["\u{1F1E6}", "\u{1160}"],
		"  ÷ [1.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [999.0] HANGUL JUNGSEONG FILLER (V) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_304() {
	grapheme_test("\u{1F1E6}\u{0308}\u{1160}",
		&["\u{1F1E6}\u{0308}", "\u{1160}"],
		"  ÷ [1.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL JUNGSEONG FILLER (V) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_305() {
	grapheme_test("\u{1F1E6}\u{11A8}",
		&["\u{1F1E6}", "\u{11A8}"],
		"  ÷ [1.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [999.0] HANGUL JONGSEONG KIYEOK (T) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_306() {
	grapheme_test("\u{1F1E6}\u{0308}\u{11A8}",
		&["\u{1F1E6}\u{0308}", "\u{11A8}"],
		"  ÷ [1.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL JONGSEONG KIYEOK (T) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_307() {
	grapheme_test("\u{1F1E6}\u{AC00}",
		&["\u{1F1E6}", "\u{AC00}"],
		"  ÷ [1.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [999.0] HANGUL SYLLABLE GA (LV) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_308() {
	grapheme_test("\u{1F1E6}\u{0308}\u{AC00}",
		&["\u{1F1E6}\u{0308}", "\u{AC00}"],
		"  ÷ [1.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL SYLLABLE GA (LV) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_309() {
	grapheme_test("\u{1F1E6}\u{AC01}",
		&["\u{1F1E6}", "\u{AC01}"],
		"  ÷ [1.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [999.0] HANGUL SYLLABLE GAG (LVT) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_310() {
	grapheme_test("\u{1F1E6}\u{0308}\u{AC01}",
		&["\u{1F1E6}\u{0308}", "\u{AC01}"],
		"  ÷ [1.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL SYLLABLE GAG (LVT) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_311() {
	grapheme_test("\u{1F1E6}\u{1CF5}",
		&["\u{1F1E6}", "\u{1CF5}"],
		"  ÷ [1.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [999.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_312() {
	grapheme_test("\u{1F1E6}\u{0308}\u{1CF5}",
		&["\u{1F1E6}\u{0308}", "\u{1CF5}"],
		"  ÷ [1.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_313() {
	grapheme_test("\u{1F1E6}\u{0915}",
		&["\u{1F1E6}", "\u{0915}"],
		"  ÷ [1.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [999.0] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_314() {
	grapheme_test("\u{1F1E6}\u{0308}\u{0915}",
		&["\u{1F1E6}\u{0308}", "\u{0915}"],
		"  ÷ [1.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_315() {
	grapheme_test("\u{1F1E6}\u{00A9}",
		&["\u{1F1E6}", "\u{00A9}"],
		"  ÷ [1.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [999.0] COPYRIGHT SIGN (ExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_316() {
	grapheme_test("\u{1F1E6}\u{0308}\u{00A9}",
		&["\u{1F1E6}\u{0308}", "\u{00A9}"],
		"  ÷ [1.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] COPYRIGHT SIGN (ExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_317() {
	grapheme_test("\u{1F1E6}\u{0020}",
		&["\u{1F1E6}", "\u{0020}"],
		"  ÷ [1.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [999.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_318() {
	grapheme_test("\u{1F1E6}\u{0308}\u{0020}",
		&["\u{1F1E6}\u{0308}", "\u{0020}"],
		"  ÷ [1.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_319() {
	grapheme_test("\u{1F1E6}\u{0378}",
		&["\u{1F1E6}", "\u{0378}"],
		"  ÷ [1.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [999.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_320() {
	grapheme_test("\u{1F1E6}\u{0308}\u{0378}",
		&["\u{1F1E6}\u{0308}", "\u{0378}"],
		"  ÷ [1.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_321() {
	grapheme_test("\u{06DD}\u{000D}",
		&["\u{06DD}", "\u{000D}"],
		"  ÷ [1.0] ARABIC END OF AYAH (Prepend) ÷ [5.0] <CARRIAGE RETURN (CR)> (CR) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_322() {
	grapheme_test("\u{06DD}\u{0308}\u{000D}",
		&["\u{06DD}\u{0308}", "\u{000D}"],
		"  ÷ [1.0] ARABIC END OF AYAH (Prepend) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <CARRIAGE RETURN (CR)> (CR) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_323() {
	grapheme_test("\u{06DD}\u{000A}",
		&["\u{06DD}", "\u{000A}"],
		"  ÷ [1.0] ARABIC END OF AYAH (Prepend) ÷ [5.0] <LINE FEED (LF)> (LF) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_324() {
	grapheme_test("\u{06DD}\u{0308}\u{000A}",
		&["\u{06DD}\u{0308}", "\u{000A}"],
		"  ÷ [1.0] ARABIC END OF AYAH (Prepend) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <LINE FEED (LF)> (LF) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_325() {
	grapheme_test("\u{06DD}\u{0000}",
		&["\u{06DD}", "\u{0000}"],
		"  ÷ [1.0] ARABIC END OF AYAH (Prepend) ÷ [5.0] <NULL> (Control) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_326() {
	grapheme_test("\u{06DD}\u{0308}\u{0000}",
		&["\u{06DD}\u{0308}", "\u{0000}"],
		"  ÷ [1.0] ARABIC END OF AYAH (Prepend) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <NULL> (Control) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_327() {
	grapheme_test("\u{06DD}\u{094D}",
		&["\u{06DD}\u{094D}"],
		"  ÷ [1.0] ARABIC END OF AYAH (Prepend) × [9.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_328() {
	grapheme_test("\u{06DD}\u{0308}\u{094D}",
		&["\u{06DD}\u{0308}\u{094D}"],
		"  ÷ [1.0] ARABIC END OF AYAH (Prepend) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_329() {
	grapheme_test("\u{06DD}\u{0300}",
		&["\u{06DD}\u{0300}"],
		"  ÷ [1.0] ARABIC END OF AYAH (Prepend) × [9.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_330() {
	grapheme_test("\u{06DD}\u{0308}\u{0300}",
		&["\u{06DD}\u{0308}\u{0300}"],
		"  ÷ [1.0] ARABIC END OF AYAH (Prepend) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_331() {
	grapheme_test("\u{06DD}\u{200C}",
		&["\u{06DD}\u{200C}"],
		"  ÷ [1.0] ARABIC END OF AYAH (Prepend) × [9.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_332() {
	grapheme_test("\u{06DD}\u{0308}\u{200C}",
		&["\u{06DD}\u{0308}\u{200C}"],
		"  ÷ [1.0] ARABIC END OF AYAH (Prepend) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_333() {
	grapheme_test("\u{06DD}\u{200D}",
		&["\u{06DD}\u{200D}"],
		"  ÷ [1.0] ARABIC END OF AYAH (Prepend) × [9.0] ZERO WIDTH JOINER (ZWJ) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_334() {
	grapheme_test("\u{06DD}\u{0308}\u{200D}",
		&["\u{06DD}\u{0308}\u{200D}"],
		"  ÷ [1.0] ARABIC END OF AYAH (Prepend) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] ZERO WIDTH JOINER (ZWJ) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_335() {
	grapheme_test("\u{06DD}\u{1F1E6}",
		&["\u{06DD}\u{1F1E6}"],
		"  ÷ [1.0] ARABIC END OF AYAH (Prepend) × [9.2] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_336() {
	grapheme_test("\u{06DD}\u{0308}\u{1F1E6}",
		&["\u{06DD}\u{0308}", "\u{1F1E6}"],
		"  ÷ [1.0] ARABIC END OF AYAH (Prepend) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_337() {
	grapheme_test("\u{06DD}\u{06DD}",
		&["\u{06DD}\u{06DD}"],
		"  ÷ [1.0] ARABIC END OF AYAH (Prepend) × [9.2] ARABIC END OF AYAH (Prepend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_338() {
	grapheme_test("\u{06DD}\u{0308}\u{06DD}",
		&["\u{06DD}\u{0308}", "\u{06DD}"],
		"  ÷ [1.0] ARABIC END OF AYAH (Prepend) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] ARABIC END OF AYAH (Prepend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_339() {
	grapheme_test("\u{06DD}\u{0903}",
		&["\u{06DD}\u{0903}"],
		"  ÷ [1.0] ARABIC END OF AYAH (Prepend) × [9.1] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_340() {
	grapheme_test("\u{06DD}\u{0308}\u{0903}",
		&["\u{06DD}\u{0308}\u{0903}"],
		"  ÷ [1.0] ARABIC END OF AYAH (Prepend) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.1] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_341() {
	grapheme_test("\u{06DD}\u{1100}",
		&["\u{06DD}\u{1100}"],
		"  ÷ [1.0] ARABIC END OF AYAH (Prepend) × [9.2] HANGUL CHOSEONG KIYEOK (L) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_342() {
	grapheme_test("\u{06DD}\u{0308}\u{1100}",
		&["\u{06DD}\u{0308}", "\u{1100}"],
		"  ÷ [1.0] ARABIC END OF AYAH (Prepend) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL CHOSEONG KIYEOK (L) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_343() {
	grapheme_test("\u{06DD}\u{1160}",
		&["\u{06DD}\u{1160}"],
		"  ÷ [1.0] ARABIC END OF AYAH (Prepend) × [9.2] HANGUL JUNGSEONG FILLER (V) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_344() {
	grapheme_test("\u{06DD}\u{0308}\u{1160}",
		&["\u{06DD}\u{0308}", "\u{1160}"],
		"  ÷ [1.0] ARABIC END OF AYAH (Prepend) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL JUNGSEONG FILLER (V) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_345() {
	grapheme_test("\u{06DD}\u{11A8}",
		&["\u{06DD}\u{11A8}"],
		"  ÷ [1.0] ARABIC END OF AYAH (Prepend) × [9.2] HANGUL JONGSEONG KIYEOK (T) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_346() {
	grapheme_test("\u{06DD}\u{0308}\u{11A8}",
		&["\u{06DD}\u{0308}", "\u{11A8}"],
		"  ÷ [1.0] ARABIC END OF AYAH (Prepend) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL JONGSEONG KIYEOK (T) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_347() {
	grapheme_test("\u{06DD}\u{AC00}",
		&["\u{06DD}\u{AC00}"],
		"  ÷ [1.0] ARABIC END OF AYAH (Prepend) × [9.2] HANGUL SYLLABLE GA (LV) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_348() {
	grapheme_test("\u{06DD}\u{0308}\u{AC00}",
		&["\u{06DD}\u{0308}", "\u{AC00}"],
		"  ÷ [1.0] ARABIC END OF AYAH (Prepend) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL SYLLABLE GA (LV) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_349() {
	grapheme_test("\u{06DD}\u{AC01}",
		&["\u{06DD}\u{AC01}"],
		"  ÷ [1.0] ARABIC END OF AYAH (Prepend) × [9.2] HANGUL SYLLABLE GAG (LVT) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_350() {
	grapheme_test("\u{06DD}\u{0308}\u{AC01}",
		&["\u{06DD}\u{0308}", "\u{AC01}"],
		"  ÷ [1.0] ARABIC END OF AYAH (Prepend) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL SYLLABLE GAG (LVT) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_351() {
	grapheme_test("\u{06DD}\u{1CF5}",
		&["\u{06DD}\u{1CF5}"],
		"  ÷ [1.0] ARABIC END OF AYAH (Prepend) × [9.2] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_352() {
	grapheme_test("\u{06DD}\u{0308}\u{1CF5}",
		&["\u{06DD}\u{0308}", "\u{1CF5}"],
		"  ÷ [1.0] ARABIC END OF AYAH (Prepend) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_353() {
	grapheme_test("\u{06DD}\u{0915}",
		&["\u{06DD}\u{0915}"],
		"  ÷ [1.0] ARABIC END OF AYAH (Prepend) × [9.2] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_354() {
	grapheme_test("\u{06DD}\u{0308}\u{0915}",
		&["\u{06DD}\u{0308}", "\u{0915}"],
		"  ÷ [1.0] ARABIC END OF AYAH (Prepend) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_355() {
	grapheme_test("\u{06DD}\u{00A9}",
		&["\u{06DD}\u{00A9}"],
		"  ÷ [1.0] ARABIC END OF AYAH (Prepend) × [9.2] COPYRIGHT SIGN (ExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_356() {
	grapheme_test("\u{06DD}\u{0308}\u{00A9}",
		&["\u{06DD}\u{0308}", "\u{00A9}"],
		"  ÷ [1.0] ARABIC END OF AYAH (Prepend) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] COPYRIGHT SIGN (ExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_357() {
	grapheme_test("\u{06DD}\u{0020}",
		&["\u{06DD}\u{0020}"],
		"  ÷ [1.0] ARABIC END OF AYAH (Prepend) × [9.2] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_358() {
	grapheme_test("\u{06DD}\u{0308}\u{0020}",
		&["\u{06DD}\u{0308}", "\u{0020}"],
		"  ÷ [1.0] ARABIC END OF AYAH (Prepend) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_359() {
	grapheme_test("\u{06DD}\u{0378}",
		&["\u{06DD}\u{0378}"],
		"  ÷ [1.0] ARABIC END OF AYAH (Prepend) × [9.2] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_360() {
	grapheme_test("\u{06DD}\u{0308}\u{0378}",
		&["\u{06DD}\u{0308}", "\u{0378}"],
		"  ÷ [1.0] ARABIC END OF AYAH (Prepend) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_361() {
	grapheme_test("\u{0903}\u{000D}",
		&["\u{0903}", "\u{000D}"],
		"  ÷ [1.0] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [5.0] <CARRIAGE RETURN (CR)> (CR) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_362() {
	grapheme_test("\u{0903}\u{0308}\u{000D}",
		&["\u{0903}\u{0308}", "\u{000D}"],
		"  ÷ [1.0] DEVANAGARI SIGN VISARGA (SpacingMark) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <CARRIAGE RETURN (CR)> (CR) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_363() {
	grapheme_test("\u{0903}\u{000A}",
		&["\u{0903}", "\u{000A}"],
		"  ÷ [1.0] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [5.0] <LINE FEED (LF)> (LF) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_364() {
	grapheme_test("\u{0903}\u{0308}\u{000A}",
		&["\u{0903}\u{0308}", "\u{000A}"],
		"  ÷ [1.0] DEVANAGARI SIGN VISARGA (SpacingMark) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <LINE FEED (LF)> (LF) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_365() {
	grapheme_test("\u{0903}\u{0000}",
		&["\u{0903}", "\u{0000}"],
		"  ÷ [1.0] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [5.0] <NULL> (Control) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_366() {
	grapheme_test("\u{0903}\u{0308}\u{0000}",
		&["\u{0903}\u{0308}", "\u{0000}"],
		"  ÷ [1.0] DEVANAGARI SIGN VISARGA (SpacingMark) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <NULL> (Control) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_367() {
	grapheme_test("\u{0903}\u{094D}",
		&["\u{0903}\u{094D}"],
		"  ÷ [1.0] DEVANAGARI SIGN VISARGA (SpacingMark) × [9.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_368() {
	grapheme_test("\u{0903}\u{0308}\u{094D}",
		&["\u{0903}\u{0308}\u{094D}"],
		"  ÷ [1.0] DEVANAGARI SIGN VISARGA (SpacingMark) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_369() {
	grapheme_test("\u{0903}\u{0300}",
		&["\u{0903}\u{0300}"],
		"  ÷ [1.0] DEVANAGARI SIGN VISARGA (SpacingMark) × [9.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_370() {
	grapheme_test("\u{0903}\u{0308}\u{0300}",
		&["\u{0903}\u{0308}\u{0300}"],
		"  ÷ [1.0] DEVANAGARI SIGN VISARGA (SpacingMark) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_371() {
	grapheme_test("\u{0903}\u{200C}",
		&["\u{0903}\u{200C}"],
		"  ÷ [1.0] DEVANAGARI SIGN VISARGA (SpacingMark) × [9.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_372() {
	grapheme_test("\u{0903}\u{0308}\u{200C}",
		&["\u{0903}\u{0308}\u{200C}"],
		"  ÷ [1.0] DEVANAGARI SIGN VISARGA (SpacingMark) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_373() {
	grapheme_test("\u{0903}\u{200D}",
		&["\u{0903}\u{200D}"],
		"  ÷ [1.0] DEVANAGARI SIGN VISARGA (SpacingMark) × [9.0] ZERO WIDTH JOINER (ZWJ) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_374() {
	grapheme_test("\u{0903}\u{0308}\u{200D}",
		&["\u{0903}\u{0308}\u{200D}"],
		"  ÷ [1.0] DEVANAGARI SIGN VISARGA (SpacingMark) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] ZERO WIDTH JOINER (ZWJ) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_375() {
	grapheme_test("\u{0903}\u{1F1E6}",
		&["\u{0903}", "\u{1F1E6}"],
		"  ÷ [1.0] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [999.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_376() {
	grapheme_test("\u{0903}\u{0308}\u{1F1E6}",
		&["\u{0903}\u{0308}", "\u{1F1E6}"],
		"  ÷ [1.0] DEVANAGARI SIGN VISARGA (SpacingMark) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_377() {
	grapheme_test("\u{0903}\u{06DD}",
		&["\u{0903}", "\u{06DD}"],
		"  ÷ [1.0] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [999.0] ARABIC END OF AYAH (Prepend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_378() {
	grapheme_test("\u{0903}\u{0308}\u{06DD}",
		&["\u{0903}\u{0308}", "\u{06DD}"],
		"  ÷ [1.0] DEVANAGARI SIGN VISARGA (SpacingMark) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] ARABIC END OF AYAH (Prepend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_379() {
	grapheme_test("\u{0903}\u{0903}",
		&["\u{0903}\u{0903}"],
		"  ÷ [1.0] DEVANAGARI SIGN VISARGA (SpacingMark) × [9.1] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_380() {
	grapheme_test("\u{0903}\u{0308}\u{0903}",
		&["\u{0903}\u{0308}\u{0903}"],
		"  ÷ [1.0] DEVANAGARI SIGN VISARGA (SpacingMark) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.1] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_381() {
	grapheme_test("\u{0903}\u{1100}",
		&["\u{0903}", "\u{1100}"],
		"  ÷ [1.0] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [999.0] HANGUL CHOSEONG KIYEOK (L) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_382() {
	grapheme_test("\u{0903}\u{0308}\u{1100}",
		&["\u{0903}\u{0308}", "\u{1100}"],
		"  ÷ [1.0] DEVANAGARI SIGN VISARGA (SpacingMark) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL CHOSEONG KIYEOK (L) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_383() {
	grapheme_test("\u{0903}\u{1160}",
		&["\u{0903}", "\u{1160}"],
		"  ÷ [1.0] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [999.0] HANGUL JUNGSEONG FILLER (V) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_384() {
	grapheme_test("\u{0903}\u{0308}\u{1160}",
		&["\u{0903}\u{0308}", "\u{1160}"],
		"  ÷ [1.0] DEVANAGARI SIGN VISARGA (SpacingMark) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL JUNGSEONG FILLER (V) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_385() {
	grapheme_test("\u{0903}\u{11A8}",
		&["\u{0903}", "\u{11A8}"],
		"  ÷ [1.0] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [999.0] HANGUL JONGSEONG KIYEOK (T) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_386() {
	grapheme_test("\u{0903}\u{0308}\u{11A8}",
		&["\u{0903}\u{0308}", "\u{11A8}"],
		"  ÷ [1.0] DEVANAGARI SIGN VISARGA (SpacingMark) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL JONGSEONG KIYEOK (T) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_387() {
	grapheme_test("\u{0903}\u{AC00}",
		&["\u{0903}", "\u{AC00}"],
		"  ÷ [1.0] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [999.0] HANGUL SYLLABLE GA (LV) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_388() {
	grapheme_test("\u{0903}\u{0308}\u{AC00}",
		&["\u{0903}\u{0308}", "\u{AC00}"],
		"  ÷ [1.0] DEVANAGARI SIGN VISARGA (SpacingMark) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL SYLLABLE GA (LV) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_389() {
	grapheme_test("\u{0903}\u{AC01}",
		&["\u{0903}", "\u{AC01}"],
		"  ÷ [1.0] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [999.0] HANGUL SYLLABLE GAG (LVT) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_390() {
	grapheme_test("\u{0903}\u{0308}\u{AC01}",
		&["\u{0903}\u{0308}", "\u{AC01}"],
		"  ÷ [1.0] DEVANAGARI SIGN VISARGA (SpacingMark) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL SYLLABLE GAG (LVT) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_391() {
	grapheme_test("\u{0903}\u{1CF5}",
		&["\u{0903}", "\u{1CF5}"],
		"  ÷ [1.0] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [999.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_392() {
	grapheme_test("\u{0903}\u{0308}\u{1CF5}",
		&["\u{0903}\u{0308}", "\u{1CF5}"],
		"  ÷ [1.0] DEVANAGARI SIGN VISARGA (SpacingMark) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_393() {
	grapheme_test("\u{0903}\u{0915}",
		&["\u{0903}", "\u{0915}"],
		"  ÷ [1.0] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [999.0] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_394() {
	grapheme_test("\u{0903}\u{0308}\u{0915}",
		&["\u{0903}\u{0308}", "\u{0915}"],
		"  ÷ [1.0] DEVANAGARI SIGN VISARGA (SpacingMark) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_395() {
	grapheme_test("\u{0903}\u{00A9}",
		&["\u{0903}", "\u{00A9}"],
		"  ÷ [1.0] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [999.0] COPYRIGHT SIGN (ExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_396() {
	grapheme_test("\u{0903}\u{0308}\u{00A9}",
		&["\u{0903}\u{0308}", "\u{00A9}"],
		"  ÷ [1.0] DEVANAGARI SIGN VISARGA (SpacingMark) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] COPYRIGHT SIGN (ExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_397() {
	grapheme_test("\u{0903}\u{0020}",
		&["\u{0903}", "\u{0020}"],
		"  ÷ [1.0] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [999.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_398() {
	grapheme_test("\u{0903}\u{0308}\u{0020}",
		&["\u{0903}\u{0308}", "\u{0020}"],
		"  ÷ [1.0] DEVANAGARI SIGN VISARGA (SpacingMark) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_399() {
	grapheme_test("\u{0903}\u{0378}",
		&["\u{0903}", "\u{0378}"],
		"  ÷ [1.0] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [999.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_400() {
	grapheme_test("\u{0903}\u{0308}\u{0378}",
		&["\u{0903}\u{0308}", "\u{0378}"],
		"  ÷ [1.0] DEVANAGARI SIGN VISARGA (SpacingMark) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_401() {
	grapheme_test("\u{1100}\u{000D}",
		&["\u{1100}", "\u{000D}"],
		"  ÷ [1.0] HANGUL CHOSEONG KIYEOK (L) ÷ [5.0] <CARRIAGE RETURN (CR)> (CR) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_402() {
	grapheme_test("\u{1100}\u{0308}\u{000D}",
		&["\u{1100}\u{0308}", "\u{000D}"],
		"  ÷ [1.0] HANGUL CHOSEONG KIYEOK (L) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <CARRIAGE RETURN (CR)> (CR) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_403() {
	grapheme_test("\u{1100}\u{000A}",
		&["\u{1100}", "\u{000A}"],
		"  ÷ [1.0] HANGUL CHOSEONG KIYEOK (L) ÷ [5.0] <LINE FEED (LF)> (LF) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_404() {
	grapheme_test("\u{1100}\u{0308}\u{000A}",
		&["\u{1100}\u{0308}", "\u{000A}"],
		"  ÷ [1.0] HANGUL CHOSEONG KIYEOK (L) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <LINE FEED (LF)> (LF) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_405() {
	grapheme_test("\u{1100}\u{0000}",
		&["\u{1100}", "\u{0000}"],
		"  ÷ [1.0] HANGUL CHOSEONG KIYEOK (L) ÷ [5.0] <NULL> (Control) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_406() {
	grapheme_test("\u{1100}\u{0308}\u{0000}",
		&["\u{1100}\u{0308}", "\u{0000}"],
		"  ÷ [1.0] HANGUL CHOSEONG KIYEOK (L) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <NULL> (Control) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_407() {
	grapheme_test("\u{1100}\u{094D}",
		&["\u{1100}\u{094D}"],
		"  ÷ [1.0] HANGUL CHOSEONG KIYEOK (L) × [9.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_408() {
	grapheme_test("\u{1100}\u{0308}\u{094D}",
		&["\u{1100}\u{0308}\u{094D}"],
		"  ÷ [1.0] HANGUL CHOSEONG KIYEOK (L) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_409() {
	grapheme_test("\u{1100}\u{0300}",
		&["\u{1100}\u{0300}"],
		"  ÷ [1.0] HANGUL CHOSEONG KIYEOK (L) × [9.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_410() {
	grapheme_test("\u{1100}\u{0308}\u{0300}",
		&["\u{1100}\u{0308}\u{0300}"],
		"  ÷ [1.0] HANGUL CHOSEONG KIYEOK (L) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_411() {
	grapheme_test("\u{1100}\u{200C}",
		&["\u{1100}\u{200C}"],
		"  ÷ [1.0] HANGUL CHOSEONG KIYEOK (L) × [9.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_412() {
	grapheme_test("\u{1100}\u{0308}\u{200C}",
		&["\u{1100}\u{0308}\u{200C}"],
		"  ÷ [1.0] HANGUL CHOSEONG KIYEOK (L) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_413() {
	grapheme_test("\u{1100}\u{200D}",
		&["\u{1100}\u{200D}"],
		"  ÷ [1.0] HANGUL CHOSEONG KIYEOK (L) × [9.0] ZERO WIDTH JOINER (ZWJ) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_414() {
	grapheme_test("\u{1100}\u{0308}\u{200D}",
		&["\u{1100}\u{0308}\u{200D}"],
		"  ÷ [1.0] HANGUL CHOSEONG KIYEOK (L) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] ZERO WIDTH JOINER (ZWJ) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_415() {
	grapheme_test("\u{1100}\u{1F1E6}",
		&["\u{1100}", "\u{1F1E6}"],
		"  ÷ [1.0] HANGUL CHOSEONG KIYEOK (L) ÷ [999.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_416() {
	grapheme_test("\u{1100}\u{0308}\u{1F1E6}",
		&["\u{1100}\u{0308}", "\u{1F1E6}"],
		"  ÷ [1.0] HANGUL CHOSEONG KIYEOK (L) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_417() {
	grapheme_test("\u{1100}\u{06DD}",
		&["\u{1100}", "\u{06DD}"],
		"  ÷ [1.0] HANGUL CHOSEONG KIYEOK (L) ÷ [999.0] ARABIC END OF AYAH (Prepend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_418() {
	grapheme_test("\u{1100}\u{0308}\u{06DD}",
		&["\u{1100}\u{0308}", "\u{06DD}"],
		"  ÷ [1.0] HANGUL CHOSEONG KIYEOK (L) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] ARABIC END OF AYAH (Prepend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_419() {
	grapheme_test("\u{1100}\u{0903}",
		&["\u{1100}\u{0903}"],
		"  ÷ [1.0] HANGUL CHOSEONG KIYEOK (L) × [9.1] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_420() {
	grapheme_test("\u{1100}\u{0308}\u{0903}",
		&["\u{1100}\u{0308}\u{0903}"],
		"  ÷ [1.0] HANGUL CHOSEONG KIYEOK (L) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.1] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_421() {
	grapheme_test("\u{1100}\u{1100}",
		&["\u{1100}\u{1100}"],
		"  ÷ [1.0] HANGUL CHOSEONG KIYEOK (L) × [6.0] HANGUL CHOSEONG KIYEOK (L) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_422() {
	grapheme_test("\u{1100}\u{0308}\u{1100}",
		&["\u{1100}\u{0308}", "\u{1100}"],
		"  ÷ [1.0] HANGUL CHOSEONG KIYEOK (L) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL CHOSEONG KIYEOK (L) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_423() {
	grapheme_test("\u{1100}\u{1160}",
		&["\u{1100}\u{1160}"],
		"  ÷ [1.0] HANGUL CHOSEONG KIYEOK (L) × [6.0] HANGUL JUNGSEONG FILLER (V) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_424() {
	grapheme_test("\u{1100}\u{0308}\u{1160}",
		&["\u{1100}\u{0308}", "\u{1160}"],
		"  ÷ [1.0] HANGUL CHOSEONG KIYEOK (L) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL JUNGSEONG FILLER (V) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_425() {
	grapheme_test("\u{1100}\u{11A8}",
		&["\u{1100}", "\u{11A8}"],
		"  ÷ [1.0] HANGUL CHOSEONG KIYEOK (L) ÷ [999.0] HANGUL JONGSEONG KIYEOK (T) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_426() {
	grapheme_test("\u{1100}\u{0308}\u{11A8}",
		&["\u{1100}\u{0308}", "\u{11A8}"],
		"  ÷ [1.0] HANGUL CHOSEONG KIYEOK (L) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL JONGSEONG KIYEOK (T) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_427() {
	grapheme_test("\u{1100}\u{AC00}",
		&["\u{1100}\u{AC00}"],
		"  ÷ [1.0] HANGUL CHOSEONG KIYEOK (L) × [6.0] HANGUL SYLLABLE GA (LV) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_428() {
	grapheme_test("\u{1100}\u{0308}\u{AC00}",
		&["\u{1100}\u{0308}", "\u{AC00}"],
		"  ÷ [1.0] HANGUL CHOSEONG KIYEOK (L) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL SYLLABLE GA (LV) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_429() {
	grapheme_test("\u{1100}\u{AC01}",
		&["\u{1100}\u{AC01}"],
		"  ÷ [1.0] HANGUL CHOSEONG KIYEOK (L) × [6.0] HANGUL SYLLABLE GAG (LVT) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_430() {
	grapheme_test("\u{1100}\u{0308}\u{AC01}",
		&["\u{1100}\u{0308}", "\u{AC01}"],
		"  ÷ [1.0] HANGUL CHOSEONG KIYEOK (L) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL SYLLABLE GAG (LVT) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_431() {
	grapheme_test("\u{1100}\u{1CF5}",
		&["\u{1100}", "\u{1CF5}"],
		"  ÷ [1.0] HANGUL CHOSEONG KIYEOK (L) ÷ [999.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_432() {
	grapheme_test("\u{1100}\u{0308}\u{1CF5}",
		&["\u{1100}\u{0308}", "\u{1CF5}"],
		"  ÷ [1.0] HANGUL CHOSEONG KIYEOK (L) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_433() {
	grapheme_test("\u{1100}\u{0915}",
		&["\u{1100}", "\u{0915}"],
		"  ÷ [1.0] HANGUL CHOSEONG KIYEOK (L) ÷ [999.0] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_434() {
	grapheme_test("\u{1100}\u{0308}\u{0915}",
		&["\u{1100}\u{0308}", "\u{0915}"],
		"  ÷ [1.0] HANGUL CHOSEONG KIYEOK (L) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_435() {
	grapheme_test("\u{1100}\u{00A9}",
		&["\u{1100}", "\u{00A9}"],
		"  ÷ [1.0] HANGUL CHOSEONG KIYEOK (L) ÷ [999.0] COPYRIGHT SIGN (ExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_436() {
	grapheme_test("\u{1100}\u{0308}\u{00A9}",
		&["\u{1100}\u{0308}", "\u{00A9}"],
		"  ÷ [1.0] HANGUL CHOSEONG KIYEOK (L) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] COPYRIGHT SIGN (ExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_437() {
	grapheme_test("\u{1100}\u{0020}",
		&["\u{1100}", "\u{0020}"],
		"  ÷ [1.0] HANGUL CHOSEONG KIYEOK (L) ÷ [999.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_438() {
	grapheme_test("\u{1100}\u{0308}\u{0020}",
		&["\u{1100}\u{0308}", "\u{0020}"],
		"  ÷ [1.0] HANGUL CHOSEONG KIYEOK (L) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_439() {
	grapheme_test("\u{1100}\u{0378}",
		&["\u{1100}", "\u{0378}"],
		"  ÷ [1.0] HANGUL CHOSEONG KIYEOK (L) ÷ [999.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_440() {
	grapheme_test("\u{1100}\u{0308}\u{0378}",
		&["\u{1100}\u{0308}", "\u{0378}"],
		"  ÷ [1.0] HANGUL CHOSEONG KIYEOK (L) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_441() {
	grapheme_test("\u{1160}\u{000D}",
		&["\u{1160}", "\u{000D}"],
		"  ÷ [1.0] HANGUL JUNGSEONG FILLER (V) ÷ [5.0] <CARRIAGE RETURN (CR)> (CR) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_442() {
	grapheme_test("\u{1160}\u{0308}\u{000D}",
		&["\u{1160}\u{0308}", "\u{000D}"],
		"  ÷ [1.0] HANGUL JUNGSEONG FILLER (V) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <CARRIAGE RETURN (CR)> (CR) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_443() {
	grapheme_test("\u{1160}\u{000A}",
		&["\u{1160}", "\u{000A}"],
		"  ÷ [1.0] HANGUL JUNGSEONG FILLER (V) ÷ [5.0] <LINE FEED (LF)> (LF) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_444() {
	grapheme_test("\u{1160}\u{0308}\u{000A}",
		&["\u{1160}\u{0308}", "\u{000A}"],
		"  ÷ [1.0] HANGUL JUNGSEONG FILLER (V) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <LINE FEED (LF)> (LF) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_445() {
	grapheme_test("\u{1160}\u{0000}",
		&["\u{1160}", "\u{0000}"],
		"  ÷ [1.0] HANGUL JUNGSEONG FILLER (V) ÷ [5.0] <NULL> (Control) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_446() {
	grapheme_test("\u{1160}\u{0308}\u{0000}",
		&["\u{1160}\u{0308}", "\u{0000}"],
		"  ÷ [1.0] HANGUL JUNGSEONG FILLER (V) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <NULL> (Control) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_447() {
	grapheme_test("\u{1160}\u{094D}",
		&["\u{1160}\u{094D}"],
		"  ÷ [1.0] HANGUL JUNGSEONG FILLER (V) × [9.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_448() {
	grapheme_test("\u{1160}\u{0308}\u{094D}",
		&["\u{1160}\u{0308}\u{094D}"],
		"  ÷ [1.0] HANGUL JUNGSEONG FILLER (V) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_449() {
	grapheme_test("\u{1160}\u{0300}",
		&["\u{1160}\u{0300}"],
		"  ÷ [1.0] HANGUL JUNGSEONG FILLER (V) × [9.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_450() {
	grapheme_test("\u{1160}\u{0308}\u{0300}",
		&["\u{1160}\u{0308}\u{0300}"],
		"  ÷ [1.0] HANGUL JUNGSEONG FILLER (V) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_451() {
	grapheme_test("\u{1160}\u{200C}",
		&["\u{1160}\u{200C}"],
		"  ÷ [1.0] HANGUL JUNGSEONG FILLER (V) × [9.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_452() {
	grapheme_test("\u{1160}\u{0308}\u{200C}",
		&["\u{1160}\u{0308}\u{200C}"],
		"  ÷ [1.0] HANGUL JUNGSEONG FILLER (V) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_453() {
	grapheme_test("\u{1160}\u{200D}",
		&["\u{1160}\u{200D}"],
		"  ÷ [1.0] HANGUL JUNGSEONG FILLER (V) × [9.0] ZERO WIDTH JOINER (ZWJ) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_454() {
	grapheme_test("\u{1160}\u{0308}\u{200D}",
		&["\u{1160}\u{0308}\u{200D}"],
		"  ÷ [1.0] HANGUL JUNGSEONG FILLER (V) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] ZERO WIDTH JOINER (ZWJ) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_455() {
	grapheme_test("\u{1160}\u{1F1E6}",
		&["\u{1160}", "\u{1F1E6}"],
		"  ÷ [1.0] HANGUL JUNGSEONG FILLER (V) ÷ [999.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_456() {
	grapheme_test("\u{1160}\u{0308}\u{1F1E6}",
		&["\u{1160}\u{0308}", "\u{1F1E6}"],
		"  ÷ [1.0] HANGUL JUNGSEONG FILLER (V) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_457() {
	grapheme_test("\u{1160}\u{06DD}",
		&["\u{1160}", "\u{06DD}"],
		"  ÷ [1.0] HANGUL JUNGSEONG FILLER (V) ÷ [999.0] ARABIC END OF AYAH (Prepend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_458() {
	grapheme_test("\u{1160}\u{0308}\u{06DD}",
		&["\u{1160}\u{0308}", "\u{06DD}"],
		"  ÷ [1.0] HANGUL JUNGSEONG FILLER (V) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] ARABIC END OF AYAH (Prepend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_459() {
	grapheme_test("\u{1160}\u{0903}",
		&["\u{1160}\u{0903}"],
		"  ÷ [1.0] HANGUL JUNGSEONG FILLER (V) × [9.1] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_460() {
	grapheme_test("\u{1160}\u{0308}\u{0903}",
		&["\u{1160}\u{0308}\u{0903}"],
		"  ÷ [1.0] HANGUL JUNGSEONG FILLER (V) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.1] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_461() {
	grapheme_test("\u{1160}\u{1100}",
		&["\u{1160}", "\u{1100}"],
		"  ÷ [1.0] HANGUL JUNGSEONG FILLER (V) ÷ [999.0] HANGUL CHOSEONG KIYEOK (L) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_462() {
	grapheme_test("\u{1160}\u{0308}\u{1100}",
		&["\u{1160}\u{0308}", "\u{1100}"],
		"  ÷ [1.0] HANGUL JUNGSEONG FILLER (V) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL CHOSEONG KIYEOK (L) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_463() {
	grapheme_test("\u{1160}\u{1160}",
		&["\u{1160}\u{1160}"],
		"  ÷ [1.0] HANGUL JUNGSEONG FILLER (V) × [7.0] HANGUL JUNGSEONG FILLER (V) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_464() {
	grapheme_test("\u{1160}\u{0308}\u{1160}",
		&["\u{1160}\u{0308}", "\u{1160}"],
		"  ÷ [1.0] HANGUL JUNGSEONG FILLER (V) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL JUNGSEONG FILLER (V) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_465() {
	grapheme_test("\u{1160}\u{11A8}",
		&["\u{1160}\u{11A8}"],
		"  ÷ [1.0] HANGUL JUNGSEONG FILLER (V) × [7.0] HANGUL JONGSEONG KIYEOK (T) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_466() {
	grapheme_test("\u{1160}\u{0308}\u{11A8}",
		&["\u{1160}\u{0308}", "\u{11A8}"],
		"  ÷ [1.0] HANGUL JUNGSEONG FILLER (V) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL JONGSEONG KIYEOK (T) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_467() {
	grapheme_test("\u{1160}\u{AC00}",
		&["\u{1160}", "\u{AC00}"],
		"  ÷ [1.0] HANGUL JUNGSEONG FILLER (V) ÷ [999.0] HANGUL SYLLABLE GA (LV) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_468() {
	grapheme_test("\u{1160}\u{0308}\u{AC00}",
		&["\u{1160}\u{0308}", "\u{AC00}"],
		"  ÷ [1.0] HANGUL JUNGSEONG FILLER (V) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL SYLLABLE GA (LV) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_469() {
	grapheme_test("\u{1160}\u{AC01}",
		&["\u{1160}", "\u{AC01}"],
		"  ÷ [1.0] HANGUL JUNGSEONG FILLER (V) ÷ [999.0] HANGUL SYLLABLE GAG (LVT) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_470() {
	grapheme_test("\u{1160}\u{0308}\u{AC01}",
		&["\u{1160}\u{0308}", "\u{AC01}"],
		"  ÷ [1.0] HANGUL JUNGSEONG FILLER (V) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL SYLLABLE GAG (LVT) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_471() {
	grapheme_test("\u{1160}\u{1CF5}",
		&["\u{1160}", "\u{1CF5}"],
		"  ÷ [1.0] HANGUL JUNGSEONG FILLER (V) ÷ [999.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_472() {
	grapheme_test("\u{1160}\u{0308}\u{1CF5}",
		&["\u{1160}\u{0308}", "\u{1CF5}"],
		"  ÷ [1.0] HANGUL JUNGSEONG FILLER (V) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_473() {
	grapheme_test("\u{1160}\u{0915}",
		&["\u{1160}", "\u{0915}"],
		"  ÷ [1.0] HANGUL JUNGSEONG FILLER (V) ÷ [999.0] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_474() {
	grapheme_test("\u{1160}\u{0308}\u{0915}",
		&["\u{1160}\u{0308}", "\u{0915}"],
		"  ÷ [1.0] HANGUL JUNGSEONG FILLER (V) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_475() {
	grapheme_test("\u{1160}\u{00A9}",
		&["\u{1160}", "\u{00A9}"],
		"  ÷ [1.0] HANGUL JUNGSEONG FILLER (V) ÷ [999.0] COPYRIGHT SIGN (ExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_476() {
	grapheme_test("\u{1160}\u{0308}\u{00A9}",
		&["\u{1160}\u{0308}", "\u{00A9}"],
		"  ÷ [1.0] HANGUL JUNGSEONG FILLER (V) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] COPYRIGHT SIGN (ExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_477() {
	grapheme_test("\u{1160}\u{0020}",
		&["\u{1160}", "\u{0020}"],
		"  ÷ [1.0] HANGUL JUNGSEONG FILLER (V) ÷ [999.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_478() {
	grapheme_test("\u{1160}\u{0308}\u{0020}",
		&["\u{1160}\u{0308}", "\u{0020}"],
		"  ÷ [1.0] HANGUL JUNGSEONG FILLER (V) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_479() {
	grapheme_test("\u{1160}\u{0378}",
		&["\u{1160}", "\u{0378}"],
		"  ÷ [1.0] HANGUL JUNGSEONG FILLER (V) ÷ [999.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_480() {
	grapheme_test("\u{1160}\u{0308}\u{0378}",
		&["\u{1160}\u{0308}", "\u{0378}"],
		"  ÷ [1.0] HANGUL JUNGSEONG FILLER (V) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_481() {
	grapheme_test("\u{11A8}\u{000D}",
		&["\u{11A8}", "\u{000D}"],
		"  ÷ [1.0] HANGUL JONGSEONG KIYEOK (T) ÷ [5.0] <CARRIAGE RETURN (CR)> (CR) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_482() {
	grapheme_test("\u{11A8}\u{0308}\u{000D}",
		&["\u{11A8}\u{0308}", "\u{000D}"],
		"  ÷ [1.0] HANGUL JONGSEONG KIYEOK (T) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <CARRIAGE RETURN (CR)> (CR) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_483() {
	grapheme_test("\u{11A8}\u{000A}",
		&["\u{11A8}", "\u{000A}"],
		"  ÷ [1.0] HANGUL JONGSEONG KIYEOK (T) ÷ [5.0] <LINE FEED (LF)> (LF) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_484() {
	grapheme_test("\u{11A8}\u{0308}\u{000A}",
		&["\u{11A8}\u{0308}", "\u{000A}"],
		"  ÷ [1.0] HANGUL JONGSEONG KIYEOK (T) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <LINE FEED (LF)> (LF) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_485() {
	grapheme_test("\u{11A8}\u{0000}",
		&["\u{11A8}", "\u{0000}"],
		"  ÷ [1.0] HANGUL JONGSEONG KIYEOK (T) ÷ [5.0] <NULL> (Control) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_486() {
	grapheme_test("\u{11A8}\u{0308}\u{0000}",
		&["\u{11A8}\u{0308}", "\u{0000}"],
		"  ÷ [1.0] HANGUL JONGSEONG KIYEOK (T) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <NULL> (Control) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_487() {
	grapheme_test("\u{11A8}\u{094D}",
		&["\u{11A8}\u{094D}"],
		"  ÷ [1.0] HANGUL JONGSEONG KIYEOK (T) × [9.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_488() {
	grapheme_test("\u{11A8}\u{0308}\u{094D}",
		&["\u{11A8}\u{0308}\u{094D}"],
		"  ÷ [1.0] HANGUL JONGSEONG KIYEOK (T) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_489() {
	grapheme_test("\u{11A8}\u{0300}",
		&["\u{11A8}\u{0300}"],
		"  ÷ [1.0] HANGUL JONGSEONG KIYEOK (T) × [9.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_490() {
	grapheme_test("\u{11A8}\u{0308}\u{0300}",
		&["\u{11A8}\u{0308}\u{0300}"],
		"  ÷ [1.0] HANGUL JONGSEONG KIYEOK (T) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_491() {
	grapheme_test("\u{11A8}\u{200C}",
		&["\u{11A8}\u{200C}"],
		"  ÷ [1.0] HANGUL JONGSEONG KIYEOK (T) × [9.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_492() {
	grapheme_test("\u{11A8}\u{0308}\u{200C}",
		&["\u{11A8}\u{0308}\u{200C}"],
		"  ÷ [1.0] HANGUL JONGSEONG KIYEOK (T) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_493() {
	grapheme_test("\u{11A8}\u{200D}",
		&["\u{11A8}\u{200D}"],
		"  ÷ [1.0] HANGUL JONGSEONG KIYEOK (T) × [9.0] ZERO WIDTH JOINER (ZWJ) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_494() {
	grapheme_test("\u{11A8}\u{0308}\u{200D}",
		&["\u{11A8}\u{0308}\u{200D}"],
		"  ÷ [1.0] HANGUL JONGSEONG KIYEOK (T) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] ZERO WIDTH JOINER (ZWJ) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_495() {
	grapheme_test("\u{11A8}\u{1F1E6}",
		&["\u{11A8}", "\u{1F1E6}"],
		"  ÷ [1.0] HANGUL JONGSEONG KIYEOK (T) ÷ [999.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_496() {
	grapheme_test("\u{11A8}\u{0308}\u{1F1E6}",
		&["\u{11A8}\u{0308}", "\u{1F1E6}"],
		"  ÷ [1.0] HANGUL JONGSEONG KIYEOK (T) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_497() {
	grapheme_test("\u{11A8}\u{06DD}",
		&["\u{11A8}", "\u{06DD}"],
		"  ÷ [1.0] HANGUL JONGSEONG KIYEOK (T) ÷ [999.0] ARABIC END OF AYAH (Prepend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_498() {
	grapheme_test("\u{11A8}\u{0308}\u{06DD}",
		&["\u{11A8}\u{0308}", "\u{06DD}"],
		"  ÷ [1.0] HANGUL JONGSEONG KIYEOK (T) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] ARABIC END OF AYAH (Prepend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_499() {
	grapheme_test("\u{11A8}\u{0903}",
		&["\u{11A8}\u{0903}"],
		"  ÷ [1.0] HANGUL JONGSEONG KIYEOK (T) × [9.1] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_500() {
	grapheme_test("\u{11A8}\u{0308}\u{0903}",
		&["\u{11A8}\u{0308}\u{0903}"],
		"  ÷ [1.0] HANGUL JONGSEONG KIYEOK (T) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.1] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_501() {
	grapheme_test("\u{11A8}\u{1100}",
		&["\u{11A8}", "\u{1100}"],
		"  ÷ [1.0] HANGUL JONGSEONG KIYEOK (T) ÷ [999.0] HANGUL CHOSEONG KIYEOK (L) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_502() {
	grapheme_test("\u{11A8}\u{0308}\u{1100}",
		&["\u{11A8}\u{0308}", "\u{1100}"],
		"  ÷ [1.0] HANGUL JONGSEONG KIYEOK (T) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL CHOSEONG KIYEOK (L) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_503() {
	grapheme_test("\u{11A8}\u{1160}",
		&["\u{11A8}", "\u{1160}"],
		"  ÷ [1.0] HANGUL JONGSEONG KIYEOK (T) ÷ [999.0] HANGUL JUNGSEONG FILLER (V) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_504() {
	grapheme_test("\u{11A8}\u{0308}\u{1160}",
		&["\u{11A8}\u{0308}", "\u{1160}"],
		"  ÷ [1.0] HANGUL JONGSEONG KIYEOK (T) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL JUNGSEONG FILLER (V) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_505() {
	grapheme_test("\u{11A8}\u{11A8}",
		&["\u{11A8}\u{11A8}"],
		"  ÷ [1.0] HANGUL JONGSEONG KIYEOK (T) × [8.0] HANGUL JONGSEONG KIYEOK (T) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_506() {
	grapheme_test("\u{11A8}\u{0308}\u{11A8}",
		&["\u{11A8}\u{0308}", "\u{11A8}"],
		"  ÷ [1.0] HANGUL JONGSEONG KIYEOK (T) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL JONGSEONG KIYEOK (T) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_507() {
	grapheme_test("\u{11A8}\u{AC00}",
		&["\u{11A8}", "\u{AC00}"],
		"  ÷ [1.0] HANGUL JONGSEONG KIYEOK (T) ÷ [999.0] HANGUL SYLLABLE GA (LV) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_508() {
	grapheme_test("\u{11A8}\u{0308}\u{AC00}",
		&["\u{11A8}\u{0308}", "\u{AC00}"],
		"  ÷ [1.0] HANGUL JONGSEONG KIYEOK (T) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL SYLLABLE GA (LV) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_509() {
	grapheme_test("\u{11A8}\u{AC01}",
		&["\u{11A8}", "\u{AC01}"],
		"  ÷ [1.0] HANGUL JONGSEONG KIYEOK (T) ÷ [999.0] HANGUL SYLLABLE GAG (LVT) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_510() {
	grapheme_test("\u{11A8}\u{0308}\u{AC01}",
		&["\u{11A8}\u{0308}", "\u{AC01}"],
		"  ÷ [1.0] HANGUL JONGSEONG KIYEOK (T) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL SYLLABLE GAG (LVT) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_511() {
	grapheme_test("\u{11A8}\u{1CF5}",
		&["\u{11A8}", "\u{1CF5}"],
		"  ÷ [1.0] HANGUL JONGSEONG KIYEOK (T) ÷ [999.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_512() {
	grapheme_test("\u{11A8}\u{0308}\u{1CF5}",
		&["\u{11A8}\u{0308}", "\u{1CF5}"],
		"  ÷ [1.0] HANGUL JONGSEONG KIYEOK (T) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_513() {
	grapheme_test("\u{11A8}\u{0915}",
		&["\u{11A8}", "\u{0915}"],
		"  ÷ [1.0] HANGUL JONGSEONG KIYEOK (T) ÷ [999.0] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_514() {
	grapheme_test("\u{11A8}\u{0308}\u{0915}",
		&["\u{11A8}\u{0308}", "\u{0915}"],
		"  ÷ [1.0] HANGUL JONGSEONG KIYEOK (T) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_515() {
	grapheme_test("\u{11A8}\u{00A9}",
		&["\u{11A8}", "\u{00A9}"],
		"  ÷ [1.0] HANGUL JONGSEONG KIYEOK (T) ÷ [999.0] COPYRIGHT SIGN (ExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_516() {
	grapheme_test("\u{11A8}\u{0308}\u{00A9}",
		&["\u{11A8}\u{0308}", "\u{00A9}"],
		"  ÷ [1.0] HANGUL JONGSEONG KIYEOK (T) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] COPYRIGHT SIGN (ExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_517() {
	grapheme_test("\u{11A8}\u{0020}",
		&["\u{11A8}", "\u{0020}"],
		"  ÷ [1.0] HANGUL JONGSEONG KIYEOK (T) ÷ [999.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_518() {
	grapheme_test("\u{11A8}\u{0308}\u{0020}",
		&["\u{11A8}\u{0308}", "\u{0020}"],
		"  ÷ [1.0] HANGUL JONGSEONG KIYEOK (T) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_519() {
	grapheme_test("\u{11A8}\u{0378}",
		&["\u{11A8}", "\u{0378}"],
		"  ÷ [1.0] HANGUL JONGSEONG KIYEOK (T) ÷ [999.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_520() {
	grapheme_test("\u{11A8}\u{0308}\u{0378}",
		&["\u{11A8}\u{0308}", "\u{0378}"],
		"  ÷ [1.0] HANGUL JONGSEONG KIYEOK (T) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_521() {
	grapheme_test("\u{AC00}\u{000D}",
		&["\u{AC00}", "\u{000D}"],
		"  ÷ [1.0] HANGUL SYLLABLE GA (LV) ÷ [5.0] <CARRIAGE RETURN (CR)> (CR) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_522() {
	grapheme_test("\u{AC00}\u{0308}\u{000D}",
		&["\u{AC00}\u{0308}", "\u{000D}"],
		"  ÷ [1.0] HANGUL SYLLABLE GA (LV) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <CARRIAGE RETURN (CR)> (CR) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_523() {
	grapheme_test("\u{AC00}\u{000A}",
		&["\u{AC00}", "\u{000A}"],
		"  ÷ [1.0] HANGUL SYLLABLE GA (LV) ÷ [5.0] <LINE FEED (LF)> (LF) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_524() {
	grapheme_test("\u{AC00}\u{0308}\u{000A}",
		&["\u{AC00}\u{0308}", "\u{000A}"],
		"  ÷ [1.0] HANGUL SYLLABLE GA (LV) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <LINE FEED (LF)> (LF) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_525() {
	grapheme_test("\u{AC00}\u{0000}",
		&["\u{AC00}", "\u{0000}"],
		"  ÷ [1.0] HANGUL SYLLABLE GA (LV) ÷ [5.0] <NULL> (Control) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_526() {
	grapheme_test("\u{AC00}\u{0308}\u{0000}",
		&["\u{AC00}\u{0308}", "\u{0000}"],
		"  ÷ [1.0] HANGUL SYLLABLE GA (LV) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <NULL> (Control) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_527() {
	grapheme_test("\u{AC00}\u{094D}",
		&["\u{AC00}\u{094D}"],
		"  ÷ [1.0] HANGUL SYLLABLE GA (LV) × [9.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_528() {
	grapheme_test("\u{AC00}\u{0308}\u{094D}",
		&["\u{AC00}\u{0308}\u{094D}"],
		"  ÷ [1.0] HANGUL SYLLABLE GA (LV) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_529() {
	grapheme_test("\u{AC00}\u{0300}",
		&["\u{AC00}\u{0300}"],
		"  ÷ [1.0] HANGUL SYLLABLE GA (LV) × [9.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_530() {
	grapheme_test("\u{AC00}\u{0308}\u{0300}",
		&["\u{AC00}\u{0308}\u{0300}"],
		"  ÷ [1.0] HANGUL SYLLABLE GA (LV) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_531() {
	grapheme_test("\u{AC00}\u{200C}",
		&["\u{AC00}\u{200C}"],
		"  ÷ [1.0] HANGUL SYLLABLE GA (LV) × [9.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_532() {
	grapheme_test("\u{AC00}\u{0308}\u{200C}",
		&["\u{AC00}\u{0308}\u{200C}"],
		"  ÷ [1.0] HANGUL SYLLABLE GA (LV) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_533() {
	grapheme_test("\u{AC00}\u{200D}",
		&["\u{AC00}\u{200D}"],
		"  ÷ [1.0] HANGUL SYLLABLE GA (LV) × [9.0] ZERO WIDTH JOINER (ZWJ) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_534() {
	grapheme_test("\u{AC00}\u{0308}\u{200D}",
		&["\u{AC00}\u{0308}\u{200D}"],
		"  ÷ [1.0] HANGUL SYLLABLE GA (LV) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] ZERO WIDTH JOINER (ZWJ) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_535() {
	grapheme_test("\u{AC00}\u{1F1E6}",
		&["\u{AC00}", "\u{1F1E6}"],
		"  ÷ [1.0] HANGUL SYLLABLE GA (LV) ÷ [999.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_536() {
	grapheme_test("\u{AC00}\u{0308}\u{1F1E6}",
		&["\u{AC00}\u{0308}", "\u{1F1E6}"],
		"  ÷ [1.0] HANGUL SYLLABLE GA (LV) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_537() {
	grapheme_test("\u{AC00}\u{06DD}",
		&["\u{AC00}", "\u{06DD}"],
		"  ÷ [1.0] HANGUL SYLLABLE GA (LV) ÷ [999.0] ARABIC END OF AYAH (Prepend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_538() {
	grapheme_test("\u{AC00}\u{0308}\u{06DD}",
		&["\u{AC00}\u{0308}", "\u{06DD}"],
		"  ÷ [1.0] HANGUL SYLLABLE GA (LV) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] ARABIC END OF AYAH (Prepend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_539() {
	grapheme_test("\u{AC00}\u{0903}",
		&["\u{AC00}\u{0903}"],
		"  ÷ [1.0] HANGUL SYLLABLE GA (LV) × [9.1] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_540() {
	grapheme_test("\u{AC00}\u{0308}\u{0903}",
		&["\u{AC00}\u{0308}\u{0903}"],
		"  ÷ [1.0] HANGUL SYLLABLE GA (LV) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.1] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_541() {
	grapheme_test("\u{AC00}\u{1100}",
		&["\u{AC00}", "\u{1100}"],
		"  ÷ [1.0] HANGUL SYLLABLE GA (LV) ÷ [999.0] HANGUL CHOSEONG KIYEOK (L) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_542() {
	grapheme_test("\u{AC00}\u{0308}\u{1100}",
		&["\u{AC00}\u{0308}", "\u{1100}"],
		"  ÷ [1.0] HANGUL SYLLABLE GA (LV) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL CHOSEONG KIYEOK (L) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_543() {
	grapheme_test("\u{AC00}\u{1160}",
		&["\u{AC00}\u{1160}"],
		"  ÷ [1.0] HANGUL SYLLABLE GA (LV) × [7.0] HANGUL JUNGSEONG FILLER (V) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_544() {
	grapheme_test("\u{AC00}\u{0308}\u{1160}",
		&["\u{AC00}\u{0308}", "\u{1160}"],
		"  ÷ [1.0] HANGUL SYLLABLE GA (LV) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL JUNGSEONG FILLER (V) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_545() {
	grapheme_test("\u{AC00}\u{11A8}",
		&["\u{AC00}\u{11A8}"],
		"  ÷ [1.0] HANGUL SYLLABLE GA (LV) × [7.0] HANGUL JONGSEONG KIYEOK (T) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_546() {
	grapheme_test("\u{AC00}\u{0308}\u{11A8}",
		&["\u{AC00}\u{0308}", "\u{11A8}"],
		"  ÷ [1.0] HANGUL SYLLABLE GA (LV) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL JONGSEONG KIYEOK (T) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_547() {
	grapheme_test("\u{AC00}\u{AC00}",
		&["\u{AC00}", "\u{AC00}"],
		"  ÷ [1.0] HANGUL SYLLABLE GA (LV) ÷ [999.0] HANGUL SYLLABLE GA (LV) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_548() {
	grapheme_test("\u{AC00}\u{0308}\u{AC00}",
		&["\u{AC00}\u{0308}", "\u{AC00}"],
		"  ÷ [1.0] HANGUL SYLLABLE GA (LV) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL SYLLABLE GA (LV) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_549() {
	grapheme_test("\u{AC00}\u{AC01}",
		&["\u{AC00}", "\u{AC01}"],
		"  ÷ [1.0] HANGUL SYLLABLE GA (LV) ÷ [999.0] HANGUL SYLLABLE GAG (LVT) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_550() {
	grapheme_test("\u{AC00}\u{0308}\u{AC01}",
		&["\u{AC00}\u{0308}", "\u{AC01}"],
		"  ÷ [1.0] HANGUL SYLLABLE GA (LV) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL SYLLABLE GAG (LVT) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_551() {
	grapheme_test("\u{AC00}\u{1CF5}",
		&["\u{AC00}", "\u{1CF5}"],
		"  ÷ [1.0] HANGUL SYLLABLE GA (LV) ÷ [999.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_552() {
	grapheme_test("\u{AC00}\u{0308}\u{1CF5}",
		&["\u{AC00}\u{0308}", "\u{1CF5}"],
		"  ÷ [1.0] HANGUL SYLLABLE GA (LV) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_553() {
	grapheme_test("\u{AC00}\u{0915}",
		&["\u{AC00}", "\u{0915}"],
		"  ÷ [1.0] HANGUL SYLLABLE GA (LV) ÷ [999.0] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_554() {
	grapheme_test("\u{AC00}\u{0308}\u{0915}",
		&["\u{AC00}\u{0308}", "\u{0915}"],
		"  ÷ [1.0] HANGUL SYLLABLE GA (LV) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_555() {
	grapheme_test("\u{AC00}\u{00A9}",
		&["\u{AC00}", "\u{00A9}"],
		"  ÷ [1.0] HANGUL SYLLABLE GA (LV) ÷ [999.0] COPYRIGHT SIGN (ExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_556() {
	grapheme_test("\u{AC00}\u{0308}\u{00A9}",
		&["\u{AC00}\u{0308}", "\u{00A9}"],
		"  ÷ [1.0] HANGUL SYLLABLE GA (LV) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] COPYRIGHT SIGN (ExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_557() {
	grapheme_test("\u{AC00}\u{0020}",
		&["\u{AC00}", "\u{0020}"],
		"  ÷ [1.0] HANGUL SYLLABLE GA (LV) ÷ [999.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_558() {
	grapheme_test("\u{AC00}\u{0308}\u{0020}",
		&["\u{AC00}\u{0308}", "\u{0020}"],
		"  ÷ [1.0] HANGUL SYLLABLE GA (LV) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_559() {
	grapheme_test("\u{AC00}\u{0378}",
		&["\u{AC00}", "\u{0378}"],
		"  ÷ [1.0] HANGUL SYLLABLE GA (LV) ÷ [999.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_560() {
	grapheme_test("\u{AC00}\u{0308}\u{0378}",
		&["\u{AC00}\u{0308}", "\u{0378}"],
		"  ÷ [1.0] HANGUL SYLLABLE GA (LV) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_561() {
	grapheme_test("\u{AC01}\u{000D}",
		&["\u{AC01}", "\u{000D}"],
		"  ÷ [1.0] HANGUL SYLLABLE GAG (LVT) ÷ [5.0] <CARRIAGE RETURN (CR)> (CR) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_562() {
	grapheme_test("\u{AC01}\u{0308}\u{000D}",
		&["\u{AC01}\u{0308}", "\u{000D}"],
		"  ÷ [1.0] HANGUL SYLLABLE GAG (LVT) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <CARRIAGE RETURN (CR)> (CR) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_563() {
	grapheme_test("\u{AC01}\u{000A}",
		&["\u{AC01}", "\u{000A}"],
		"  ÷ [1.0] HANGUL SYLLABLE GAG (LVT) ÷ [5.0] <LINE FEED (LF)> (LF) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_564() {
	grapheme_test("\u{AC01}\u{0308}\u{000A}",
		&["\u{AC01}\u{0308}", "\u{000A}"],
		"  ÷ [1.0] HANGUL SYLLABLE GAG (LVT) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <LINE FEED (LF)> (LF) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_565() {
	grapheme_test("\u{AC01}\u{0000}",
		&["\u{AC01}", "\u{0000}"],
		"  ÷ [1.0] HANGUL SYLLABLE GAG (LVT) ÷ [5.0] <NULL> (Control) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_566() {
	grapheme_test("\u{AC01}\u{0308}\u{0000}",
		&["\u{AC01}\u{0308}", "\u{0000}"],
		"  ÷ [1.0] HANGUL SYLLABLE GAG (LVT) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <NULL> (Control) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_567() {
	grapheme_test("\u{AC01}\u{094D}",
		&["\u{AC01}\u{094D}"],
		"  ÷ [1.0] HANGUL SYLLABLE GAG (LVT) × [9.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_568() {
	grapheme_test("\u{AC01}\u{0308}\u{094D}",
		&["\u{AC01}\u{0308}\u{094D}"],
		"  ÷ [1.0] HANGUL SYLLABLE GAG (LVT) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_569() {
	grapheme_test("\u{AC01}\u{0300}",
		&["\u{AC01}\u{0300}"],
		"  ÷ [1.0] HANGUL SYLLABLE GAG (LVT) × [9.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_570() {
	grapheme_test("\u{AC01}\u{0308}\u{0300}",
		&["\u{AC01}\u{0308}\u{0300}"],
		"  ÷ [1.0] HANGUL SYLLABLE GAG (LVT) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_571() {
	grapheme_test("\u{AC01}\u{200C}",
		&["\u{AC01}\u{200C}"],
		"  ÷ [1.0] HANGUL SYLLABLE GAG (LVT) × [9.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_572() {
	grapheme_test("\u{AC01}\u{0308}\u{200C}",
		&["\u{AC01}\u{0308}\u{200C}"],
		"  ÷ [1.0] HANGUL SYLLABLE GAG (LVT) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_573() {
	grapheme_test("\u{AC01}\u{200D}",
		&["\u{AC01}\u{200D}"],
		"  ÷ [1.0] HANGUL SYLLABLE GAG (LVT) × [9.0] ZERO WIDTH JOINER (ZWJ) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_574() {
	grapheme_test("\u{AC01}\u{0308}\u{200D}",
		&["\u{AC01}\u{0308}\u{200D}"],
		"  ÷ [1.0] HANGUL SYLLABLE GAG (LVT) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] ZERO WIDTH JOINER (ZWJ) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_575() {
	grapheme_test("\u{AC01}\u{1F1E6}",
		&["\u{AC01}", "\u{1F1E6}"],
		"  ÷ [1.0] HANGUL SYLLABLE GAG (LVT) ÷ [999.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_576() {
	grapheme_test("\u{AC01}\u{0308}\u{1F1E6}",
		&["\u{AC01}\u{0308}", "\u{1F1E6}"],
		"  ÷ [1.0] HANGUL SYLLABLE GAG (LVT) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_577() {
	grapheme_test("\u{AC01}\u{06DD}",
		&["\u{AC01}", "\u{06DD}"],
		"  ÷ [1.0] HANGUL SYLLABLE GAG (LVT) ÷ [999.0] ARABIC END OF AYAH (Prepend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_578() {
	grapheme_test("\u{AC01}\u{0308}\u{06DD}",
		&["\u{AC01}\u{0308}", "\u{06DD}"],
		"  ÷ [1.0] HANGUL SYLLABLE GAG (LVT) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] ARABIC END OF AYAH (Prepend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_579() {
	grapheme_test("\u{AC01}\u{0903}",
		&["\u{AC01}\u{0903}"],
		"  ÷ [1.0] HANGUL SYLLABLE GAG (LVT) × [9.1] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_580() {
	grapheme_test("\u{AC01}\u{0308}\u{0903}",
		&["\u{AC01}\u{0308}\u{0903}"],
		"  ÷ [1.0] HANGUL SYLLABLE GAG (LVT) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.1] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_581() {
	grapheme_test("\u{AC01}\u{1100}",
		&["\u{AC01}", "\u{1100}"],
		"  ÷ [1.0] HANGUL SYLLABLE GAG (LVT) ÷ [999.0] HANGUL CHOSEONG KIYEOK (L) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_582() {
	grapheme_test("\u{AC01}\u{0308}\u{1100}",
		&["\u{AC01}\u{0308}", "\u{1100}"],
		"  ÷ [1.0] HANGUL SYLLABLE GAG (LVT) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL CHOSEONG KIYEOK (L) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_583() {
	grapheme_test("\u{AC01}\u{1160}",
		&["\u{AC01}", "\u{1160}"],
		"  ÷ [1.0] HANGUL SYLLABLE GAG (LVT) ÷ [999.0] HANGUL JUNGSEONG FILLER (V) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_584() {
	grapheme_test("\u{AC01}\u{0308}\u{1160}",
		&["\u{AC01}\u{0308}", "\u{1160}"],
		"  ÷ [1.0] HANGUL SYLLABLE GAG (LVT) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL JUNGSEONG FILLER (V) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_585() {
	grapheme_test("\u{AC01}\u{11A8}",
		&["\u{AC01}\u{11A8}"],
		"  ÷ [1.0] HANGUL SYLLABLE GAG (LVT) × [8.0] HANGUL JONGSEONG KIYEOK (T) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_586() {
	grapheme_test("\u{AC01}\u{0308}\u{11A8}",
		&["\u{AC01}\u{0308}", "\u{11A8}"],
		"  ÷ [1.0] HANGUL SYLLABLE GAG (LVT) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL JONGSEONG KIYEOK (T) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_587() {
	grapheme_test("\u{AC01}\u{AC00}",
		&["\u{AC01}", "\u{AC00}"],
		"  ÷ [1.0] HANGUL SYLLABLE GAG (LVT) ÷ [999.0] HANGUL SYLLABLE GA (LV) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_588() {
	grapheme_test("\u{AC01}\u{0308}\u{AC00}",
		&["\u{AC01}\u{0308}", "\u{AC00}"],
		"  ÷ [1.0] HANGUL SYLLABLE GAG (LVT) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL SYLLABLE GA (LV) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_589() {
	grapheme_test("\u{AC01}\u{AC01}",
		&["\u{AC01}", "\u{AC01}"],
		"  ÷ [1.0] HANGUL SYLLABLE GAG (LVT) ÷ [999.0] HANGUL SYLLABLE GAG (LVT) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_590() {
	grapheme_test("\u{AC01}\u{0308}\u{AC01}",
		&["\u{AC01}\u{0308}", "\u{AC01}"],
		"  ÷ [1.0] HANGUL SYLLABLE GAG (LVT) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL SYLLABLE GAG (LVT) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_591() {
	grapheme_test("\u{AC01}\u{1CF5}",
		&["\u{AC01}", "\u{1CF5}"],
		"  ÷ [1.0] HANGUL SYLLABLE GAG (LVT) ÷ [999.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_592() {
	grapheme_test("\u{AC01}\u{0308}\u{1CF5}",
		&["\u{AC01}\u{0308}", "\u{1CF5}"],
		"  ÷ [1.0] HANGUL SYLLABLE GAG (LVT) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_593() {
	grapheme_test("\u{AC01}\u{0915}",
		&["\u{AC01}", "\u{0915}"],
		"  ÷ [1.0] HANGUL SYLLABLE GAG (LVT) ÷ [999.0] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_594() {
	grapheme_test("\u{AC01}\u{0308}\u{0915}",
		&["\u{AC01}\u{0308}", "\u{0915}"],
		"  ÷ [1.0] HANGUL SYLLABLE GAG (LVT) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_595() {
	grapheme_test("\u{AC01}\u{00A9}",
		&["\u{AC01}", "\u{00A9}"],
		"  ÷ [1.0] HANGUL SYLLABLE GAG (LVT) ÷ [999.0] COPYRIGHT SIGN (ExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_596() {
	grapheme_test("\u{AC01}\u{0308}\u{00A9}",
		&["\u{AC01}\u{0308}", "\u{00A9}"],
		"  ÷ [1.0] HANGUL SYLLABLE GAG (LVT) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] COPYRIGHT SIGN (ExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_597() {
	grapheme_test("\u{AC01}\u{0020}",
		&["\u{AC01}", "\u{0020}"],
		"  ÷ [1.0] HANGUL SYLLABLE GAG (LVT) ÷ [999.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_598() {
	grapheme_test("\u{AC01}\u{0308}\u{0020}",
		&["\u{AC01}\u{0308}", "\u{0020}"],
		"  ÷ [1.0] HANGUL SYLLABLE GAG (LVT) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_599() {
	grapheme_test("\u{AC01}\u{0378}",
		&["\u{AC01}", "\u{0378}"],
		"  ÷ [1.0] HANGUL SYLLABLE GAG (LVT) ÷ [999.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_600() {
	grapheme_test("\u{AC01}\u{0308}\u{0378}",
		&["\u{AC01}\u{0308}", "\u{0378}"],
		"  ÷ [1.0] HANGUL SYLLABLE GAG (LVT) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_601() {
	grapheme_test("\u{1CF5}\u{000D}",
		&["\u{1CF5}", "\u{000D}"],
		"  ÷ [1.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [5.0] <CARRIAGE RETURN (CR)> (CR) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_602() {
	grapheme_test("\u{1CF5}\u{0308}\u{000D}",
		&["\u{1CF5}\u{0308}", "\u{000D}"],
		"  ÷ [1.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <CARRIAGE RETURN (CR)> (CR) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_603() {
	grapheme_test("\u{1CF5}\u{000A}",
		&["\u{1CF5}", "\u{000A}"],
		"  ÷ [1.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [5.0] <LINE FEED (LF)> (LF) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_604() {
	grapheme_test("\u{1CF5}\u{0308}\u{000A}",
		&["\u{1CF5}\u{0308}", "\u{000A}"],
		"  ÷ [1.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <LINE FEED (LF)> (LF) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_605() {
	grapheme_test("\u{1CF5}\u{0000}",
		&["\u{1CF5}", "\u{0000}"],
		"  ÷ [1.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [5.0] <NULL> (Control) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_606() {
	grapheme_test("\u{1CF5}\u{0308}\u{0000}",
		&["\u{1CF5}\u{0308}", "\u{0000}"],
		"  ÷ [1.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <NULL> (Control) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_607() {
	grapheme_test("\u{1CF5}\u{094D}",
		&["\u{1CF5}\u{094D}"],
		"  ÷ [1.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) × [9.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_608() {
	grapheme_test("\u{1CF5}\u{0308}\u{094D}",
		&["\u{1CF5}\u{0308}\u{094D}"],
		"  ÷ [1.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_609() {
	grapheme_test("\u{1CF5}\u{0300}",
		&["\u{1CF5}\u{0300}"],
		"  ÷ [1.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) × [9.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_610() {
	grapheme_test("\u{1CF5}\u{0308}\u{0300}",
		&["\u{1CF5}\u{0308}\u{0300}"],
		"  ÷ [1.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_611() {
	grapheme_test("\u{1CF5}\u{200C}",
		&["\u{1CF5}\u{200C}"],
		"  ÷ [1.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) × [9.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_612() {
	grapheme_test("\u{1CF5}\u{0308}\u{200C}",
		&["\u{1CF5}\u{0308}\u{200C}"],
		"  ÷ [1.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_613() {
	grapheme_test("\u{1CF5}\u{200D}",
		&["\u{1CF5}\u{200D}"],
		"  ÷ [1.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) × [9.0] ZERO WIDTH JOINER (ZWJ) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_614() {
	grapheme_test("\u{1CF5}\u{0308}\u{200D}",
		&["\u{1CF5}\u{0308}\u{200D}"],
		"  ÷ [1.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] ZERO WIDTH JOINER (ZWJ) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_615() {
	grapheme_test("\u{1CF5}\u{1F1E6}",
		&["\u{1CF5}", "\u{1F1E6}"],
		"  ÷ [1.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [999.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_616() {
	grapheme_test("\u{1CF5}\u{0308}\u{1F1E6}",
		&["\u{1CF5}\u{0308}", "\u{1F1E6}"],
		"  ÷ [1.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_617() {
	grapheme_test("\u{1CF5}\u{06DD}",
		&["\u{1CF5}", "\u{06DD}"],
		"  ÷ [1.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [999.0] ARABIC END OF AYAH (Prepend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_618() {
	grapheme_test("\u{1CF5}\u{0308}\u{06DD}",
		&["\u{1CF5}\u{0308}", "\u{06DD}"],
		"  ÷ [1.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] ARABIC END OF AYAH (Prepend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_619() {
	grapheme_test("\u{1CF5}\u{0903}",
		&["\u{1CF5}\u{0903}"],
		"  ÷ [1.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) × [9.1] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_620() {
	grapheme_test("\u{1CF5}\u{0308}\u{0903}",
		&["\u{1CF5}\u{0308}\u{0903}"],
		"  ÷ [1.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.1] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_621() {
	grapheme_test("\u{1CF5}\u{1100}",
		&["\u{1CF5}", "\u{1100}"],
		"  ÷ [1.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [999.0] HANGUL CHOSEONG KIYEOK (L) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_622() {
	grapheme_test("\u{1CF5}\u{0308}\u{1100}",
		&["\u{1CF5}\u{0308}", "\u{1100}"],
		"  ÷ [1.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL CHOSEONG KIYEOK (L) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_623() {
	grapheme_test("\u{1CF5}\u{1160}",
		&["\u{1CF5}", "\u{1160}"],
		"  ÷ [1.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [999.0] HANGUL JUNGSEONG FILLER (V) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_624() {
	grapheme_test("\u{1CF5}\u{0308}\u{1160}",
		&["\u{1CF5}\u{0308}", "\u{1160}"],
		"  ÷ [1.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL JUNGSEONG FILLER (V) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_625() {
	grapheme_test("\u{1CF5}\u{11A8}",
		&["\u{1CF5}", "\u{11A8}"],
		"  ÷ [1.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [999.0] HANGUL JONGSEONG KIYEOK (T) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_626() {
	grapheme_test("\u{1CF5}\u{0308}\u{11A8}",
		&["\u{1CF5}\u{0308}", "\u{11A8}"],
		"  ÷ [1.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL JONGSEONG KIYEOK (T) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_627() {
	grapheme_test("\u{1CF5}\u{AC00}",
		&["\u{1CF5}", "\u{AC00}"],
		"  ÷ [1.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [999.0] HANGUL SYLLABLE GA (LV) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_628() {
	grapheme_test("\u{1CF5}\u{0308}\u{AC00}",
		&["\u{1CF5}\u{0308}", "\u{AC00}"],
		"  ÷ [1.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL SYLLABLE GA (LV) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_629() {
	grapheme_test("\u{1CF5}\u{AC01}",
		&["\u{1CF5}", "\u{AC01}"],
		"  ÷ [1.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [999.0] HANGUL SYLLABLE GAG (LVT) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_630() {
	grapheme_test("\u{1CF5}\u{0308}\u{AC01}",
		&["\u{1CF5}\u{0308}", "\u{AC01}"],
		"  ÷ [1.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL SYLLABLE GAG (LVT) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_631() {
	grapheme_test("\u{1CF5}\u{1CF5}",
		&["\u{1CF5}", "\u{1CF5}"],
		"  ÷ [1.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [999.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_632() {
	grapheme_test("\u{1CF5}\u{0308}\u{1CF5}",
		&["\u{1CF5}\u{0308}", "\u{1CF5}"],
		"  ÷ [1.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_633() {
	grapheme_test("\u{1CF5}\u{0915}",
		&["\u{1CF5}\u{0915}"],
		"  ÷ [1.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) × [9.3] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_634() {
	grapheme_test("\u{1CF5}\u{0308}\u{0915}",
		&["\u{1CF5}\u{0308}\u{0915}"],
		"  ÷ [1.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.3] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_635() {
	grapheme_test("\u{1CF5}\u{00A9}",
		&["\u{1CF5}", "\u{00A9}"],
		"  ÷ [1.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [999.0] COPYRIGHT SIGN (ExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_636() {
	grapheme_test("\u{1CF5}\u{0308}\u{00A9}",
		&["\u{1CF5}\u{0308}", "\u{00A9}"],
		"  ÷ [1.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] COPYRIGHT SIGN (ExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_637() {
	grapheme_test("\u{1CF5}\u{0020}",
		&["\u{1CF5}", "\u{0020}"],
		"  ÷ [1.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [999.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_638() {
	grapheme_test("\u{1CF5}\u{0308}\u{0020}",
		&["\u{1CF5}\u{0308}", "\u{0020}"],
		"  ÷ [1.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_639() {
	grapheme_test("\u{1CF5}\u{0378}",
		&["\u{1CF5}", "\u{0378}"],
		"  ÷ [1.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [999.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_640() {
	grapheme_test("\u{1CF5}\u{0308}\u{0378}",
		&["\u{1CF5}\u{0308}", "\u{0378}"],
		"  ÷ [1.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_641() {
	grapheme_test("\u{0915}\u{000D}",
		&["\u{0915}", "\u{000D}"],
		"  ÷ [1.0] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [5.0] <CARRIAGE RETURN (CR)> (CR) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_642() {
	grapheme_test("\u{0915}\u{0308}\u{000D}",
		&["\u{0915}\u{0308}", "\u{000D}"],
		"  ÷ [1.0] DEVANAGARI LETTER KA (LinkingConsonant) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <CARRIAGE RETURN (CR)> (CR) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_643() {
	grapheme_test("\u{0915}\u{000A}",
		&["\u{0915}", "\u{000A}"],
		"  ÷ [1.0] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [5.0] <LINE FEED (LF)> (LF) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_644() {
	grapheme_test("\u{0915}\u{0308}\u{000A}",
		&["\u{0915}\u{0308}", "\u{000A}"],
		"  ÷ [1.0] DEVANAGARI LETTER KA (LinkingConsonant) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <LINE FEED (LF)> (LF) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_645() {
	grapheme_test("\u{0915}\u{0000}",
		&["\u{0915}", "\u{0000}"],
		"  ÷ [1.0] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [5.0] <NULL> (Control) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_646() {
	grapheme_test("\u{0915}\u{0308}\u{0000}",
		&["\u{0915}\u{0308}", "\u{0000}"],
		"  ÷ [1.0] DEVANAGARI LETTER KA (LinkingConsonant) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <NULL> (Control) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_647() {
	grapheme_test("\u{0915}\u{094D}",
		&["\u{0915}\u{094D}"],
		"  ÷ [1.0] DEVANAGARI LETTER KA (LinkingConsonant) × [9.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_648() {
	grapheme_test("\u{0915}\u{0308}\u{094D}",
		&["\u{0915}\u{0308}\u{094D}"],
		"  ÷ [1.0] DEVANAGARI LETTER KA (LinkingConsonant) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_649() {
	grapheme_test("\u{0915}\u{0300}",
		&["\u{0915}\u{0300}"],
		"  ÷ [1.0] DEVANAGARI LETTER KA (LinkingConsonant) × [9.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_650() {
	grapheme_test("\u{0915}\u{0308}\u{0300}",
		&["\u{0915}\u{0308}\u{0300}"],
		"  ÷ [1.0] DEVANAGARI LETTER KA (LinkingConsonant) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_651() {
	grapheme_test("\u{0915}\u{200C}",
		&["\u{0915}\u{200C}"],
		"  ÷ [1.0] DEVANAGARI LETTER KA (LinkingConsonant) × [9.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_652() {
	grapheme_test("\u{0915}\u{0308}\u{200C}",
		&["\u{0915}\u{0308}\u{200C}"],
		"  ÷ [1.0] DEVANAGARI LETTER KA (LinkingConsonant) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_653() {
	grapheme_test("\u{0915}\u{200D}",
		&["\u{0915}\u{200D}"],
		"  ÷ [1.0] DEVANAGARI LETTER KA (LinkingConsonant) × [9.0] ZERO WIDTH JOINER (ZWJ) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_654() {
	grapheme_test("\u{0915}\u{0308}\u{200D}",
		&["\u{0915}\u{0308}\u{200D}"],
		"  ÷ [1.0] DEVANAGARI LETTER KA (LinkingConsonant) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] ZERO WIDTH JOINER (ZWJ) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_655() {
	grapheme_test("\u{0915}\u{1F1E6}",
		&["\u{0915}", "\u{1F1E6}"],
		"  ÷ [1.0] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [999.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_656() {
	grapheme_test("\u{0915}\u{0308}\u{1F1E6}",
		&["\u{0915}\u{0308}", "\u{1F1E6}"],
		"  ÷ [1.0] DEVANAGARI LETTER KA (LinkingConsonant) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_657() {
	grapheme_test("\u{0915}\u{06DD}",
		&["\u{0915}", "\u{06DD}"],
		"  ÷ [1.0] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [999.0] ARABIC END OF AYAH (Prepend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_658() {
	grapheme_test("\u{0915}\u{0308}\u{06DD}",
		&["\u{0915}\u{0308}", "\u{06DD}"],
		"  ÷ [1.0] DEVANAGARI LETTER KA (LinkingConsonant) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] ARABIC END OF AYAH (Prepend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_659() {
	grapheme_test("\u{0915}\u{0903}",
		&["\u{0915}\u{0903}"],
		"  ÷ [1.0] DEVANAGARI LETTER KA (LinkingConsonant) × [9.1] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_660() {
	grapheme_test("\u{0915}\u{0308}\u{0903}",
		&["\u{0915}\u{0308}\u{0903}"],
		"  ÷ [1.0] DEVANAGARI LETTER KA (LinkingConsonant) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.1] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_661() {
	grapheme_test("\u{0915}\u{1100}",
		&["\u{0915}", "\u{1100}"],
		"  ÷ [1.0] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [999.0] HANGUL CHOSEONG KIYEOK (L) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_662() {
	grapheme_test("\u{0915}\u{0308}\u{1100}",
		&["\u{0915}\u{0308}", "\u{1100}"],
		"  ÷ [1.0] DEVANAGARI LETTER KA (LinkingConsonant) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL CHOSEONG KIYEOK (L) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_663() {
	grapheme_test("\u{0915}\u{1160}",
		&["\u{0915}", "\u{1160}"],
		"  ÷ [1.0] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [999.0] HANGUL JUNGSEONG FILLER (V) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_664() {
	grapheme_test("\u{0915}\u{0308}\u{1160}",
		&["\u{0915}\u{0308}", "\u{1160}"],
		"  ÷ [1.0] DEVANAGARI LETTER KA (LinkingConsonant) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL JUNGSEONG FILLER (V) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_665() {
	grapheme_test("\u{0915}\u{11A8}",
		&["\u{0915}", "\u{11A8}"],
		"  ÷ [1.0] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [999.0] HANGUL JONGSEONG KIYEOK (T) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_666() {
	grapheme_test("\u{0915}\u{0308}\u{11A8}",
		&["\u{0915}\u{0308}", "\u{11A8}"],
		"  ÷ [1.0] DEVANAGARI LETTER KA (LinkingConsonant) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL JONGSEONG KIYEOK (T) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_667() {
	grapheme_test("\u{0915}\u{AC00}",
		&["\u{0915}", "\u{AC00}"],
		"  ÷ [1.0] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [999.0] HANGUL SYLLABLE GA (LV) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_668() {
	grapheme_test("\u{0915}\u{0308}\u{AC00}",
		&["\u{0915}\u{0308}", "\u{AC00}"],
		"  ÷ [1.0] DEVANAGARI LETTER KA (LinkingConsonant) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL SYLLABLE GA (LV) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_669() {
	grapheme_test("\u{0915}\u{AC01}",
		&["\u{0915}", "\u{AC01}"],
		"  ÷ [1.0] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [999.0] HANGUL SYLLABLE GAG (LVT) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_670() {
	grapheme_test("\u{0915}\u{0308}\u{AC01}",
		&["\u{0915}\u{0308}", "\u{AC01}"],
		"  ÷ [1.0] DEVANAGARI LETTER KA (LinkingConsonant) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL SYLLABLE GAG (LVT) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_671() {
	grapheme_test("\u{0915}\u{1CF5}",
		&["\u{0915}", "\u{1CF5}"],
		"  ÷ [1.0] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [999.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_672() {
	grapheme_test("\u{0915}\u{0308}\u{1CF5}",
		&["\u{0915}\u{0308}", "\u{1CF5}"],
		"  ÷ [1.0] DEVANAGARI LETTER KA (LinkingConsonant) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_673() {
	grapheme_test("\u{0915}\u{0915}",
		&["\u{0915}", "\u{0915}"],
		"  ÷ [1.0] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [999.0] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_674() {
	grapheme_test("\u{0915}\u{0308}\u{0915}",
		&["\u{0915}\u{0308}", "\u{0915}"],
		"  ÷ [1.0] DEVANAGARI LETTER KA (LinkingConsonant) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_675() {
	grapheme_test("\u{0915}\u{00A9}",
		&["\u{0915}", "\u{00A9}"],
		"  ÷ [1.0] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [999.0] COPYRIGHT SIGN (ExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_676() {
	grapheme_test("\u{0915}\u{0308}\u{00A9}",
		&["\u{0915}\u{0308}", "\u{00A9}"],
		"  ÷ [1.0] DEVANAGARI LETTER KA (LinkingConsonant) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] COPYRIGHT SIGN (ExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_677() {
	grapheme_test("\u{0915}\u{0020}",
		&["\u{0915}", "\u{0020}"],
		"  ÷ [1.0] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [999.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_678() {
	grapheme_test("\u{0915}\u{0308}\u{0020}",
		&["\u{0915}\u{0308}", "\u{0020}"],
		"  ÷ [1.0] DEVANAGARI LETTER KA (LinkingConsonant) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_679() {
	grapheme_test("\u{0915}\u{0378}",
		&["\u{0915}", "\u{0378}"],
		"  ÷ [1.0] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [999.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_680() {
	grapheme_test("\u{0915}\u{0308}\u{0378}",
		&["\u{0915}\u{0308}", "\u{0378}"],
		"  ÷ [1.0] DEVANAGARI LETTER KA (LinkingConsonant) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_681() {
	grapheme_test("\u{00A9}\u{000D}",
		&["\u{00A9}", "\u{000D}"],
		"  ÷ [1.0] COPYRIGHT SIGN (ExtPict) ÷ [5.0] <CARRIAGE RETURN (CR)> (CR) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_682() {
	grapheme_test("\u{00A9}\u{0308}\u{000D}",
		&["\u{00A9}\u{0308}", "\u{000D}"],
		"  ÷ [1.0] COPYRIGHT SIGN (ExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <CARRIAGE RETURN (CR)> (CR) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_683() {
	grapheme_test("\u{00A9}\u{000A}",
		&["\u{00A9}", "\u{000A}"],
		"  ÷ [1.0] COPYRIGHT SIGN (ExtPict) ÷ [5.0] <LINE FEED (LF)> (LF) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_684() {
	grapheme_test("\u{00A9}\u{0308}\u{000A}",
		&["\u{00A9}\u{0308}", "\u{000A}"],
		"  ÷ [1.0] COPYRIGHT SIGN (ExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <LINE FEED (LF)> (LF) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_685() {
	grapheme_test("\u{00A9}\u{0000}",
		&["\u{00A9}", "\u{0000}"],
		"  ÷ [1.0] COPYRIGHT SIGN (ExtPict) ÷ [5.0] <NULL> (Control) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_686() {
	grapheme_test("\u{00A9}\u{0308}\u{0000}",
		&["\u{00A9}\u{0308}", "\u{0000}"],
		"  ÷ [1.0] COPYRIGHT SIGN (ExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <NULL> (Control) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_687() {
	grapheme_test("\u{00A9}\u{094D}",
		&["\u{00A9}\u{094D}"],
		"  ÷ [1.0] COPYRIGHT SIGN (ExtPict) × [9.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_688() {
	grapheme_test("\u{00A9}\u{0308}\u{094D}",
		&["\u{00A9}\u{0308}\u{094D}"],
		"  ÷ [1.0] COPYRIGHT SIGN (ExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_689() {
	grapheme_test("\u{00A9}\u{0300}",
		&["\u{00A9}\u{0300}"],
		"  ÷ [1.0] COPYRIGHT SIGN (ExtPict) × [9.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_690() {
	grapheme_test("\u{00A9}\u{0308}\u{0300}",
		&["\u{00A9}\u{0308}\u{0300}"],
		"  ÷ [1.0] COPYRIGHT SIGN (ExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_691() {
	grapheme_test("\u{00A9}\u{200C}",
		&["\u{00A9}\u{200C}"],
		"  ÷ [1.0] COPYRIGHT SIGN (ExtPict) × [9.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_692() {
	grapheme_test("\u{00A9}\u{0308}\u{200C}",
		&["\u{00A9}\u{0308}\u{200C}"],
		"  ÷ [1.0] COPYRIGHT SIGN (ExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_693() {
	grapheme_test("\u{00A9}\u{200D}",
		&["\u{00A9}\u{200D}"],
		"  ÷ [1.0] COPYRIGHT SIGN (ExtPict) × [9.0] ZERO WIDTH JOINER (ZWJ) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_694() {
	grapheme_test("\u{00A9}\u{0308}\u{200D}",
		&["\u{00A9}\u{0308}\u{200D}"],
		"  ÷ [1.0] COPYRIGHT SIGN (ExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] ZERO WIDTH JOINER (ZWJ) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_695() {
	grapheme_test("\u{00A9}\u{1F1E6}",
		&["\u{00A9}", "\u{1F1E6}"],
		"  ÷ [1.0] COPYRIGHT SIGN (ExtPict) ÷ [999.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_696() {
	grapheme_test("\u{00A9}\u{0308}\u{1F1E6}",
		&["\u{00A9}\u{0308}", "\u{1F1E6}"],
		"  ÷ [1.0] COPYRIGHT SIGN (ExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_697() {
	grapheme_test("\u{00A9}\u{06DD}",
		&["\u{00A9}", "\u{06DD}"],
		"  ÷ [1.0] COPYRIGHT SIGN (ExtPict) ÷ [999.0] ARABIC END OF AYAH (Prepend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_698() {
	grapheme_test("\u{00A9}\u{0308}\u{06DD}",
		&["\u{00A9}\u{0308}", "\u{06DD}"],
		"  ÷ [1.0] COPYRIGHT SIGN (ExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] ARABIC END OF AYAH (Prepend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_699() {
	grapheme_test("\u{00A9}\u{0903}",
		&["\u{00A9}\u{0903}"],
		"  ÷ [1.0] COPYRIGHT SIGN (ExtPict) × [9.1] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_700() {
	grapheme_test("\u{00A9}\u{0308}\u{0903}",
		&["\u{00A9}\u{0308}\u{0903}"],
		"  ÷ [1.0] COPYRIGHT SIGN (ExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.1] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_701() {
	grapheme_test("\u{00A9}\u{1100}",
		&["\u{00A9}", "\u{1100}"],
		"  ÷ [1.0] COPYRIGHT SIGN (ExtPict) ÷ [999.0] HANGUL CHOSEONG KIYEOK (L) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_702() {
	grapheme_test("\u{00A9}\u{0308}\u{1100}",
		&["\u{00A9}\u{0308}", "\u{1100}"],
		"  ÷ [1.0] COPYRIGHT SIGN (ExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL CHOSEONG KIYEOK (L) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_703() {
	grapheme_test("\u{00A9}\u{1160}",
		&["\u{00A9}", "\u{1160}"],
		"  ÷ [1.0] COPYRIGHT SIGN (ExtPict) ÷ [999.0] HANGUL JUNGSEONG FILLER (V) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_704() {
	grapheme_test("\u{00A9}\u{0308}\u{1160}",
		&["\u{00A9}\u{0308}", "\u{1160}"],
		"  ÷ [1.0] COPYRIGHT SIGN (ExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL JUNGSEONG FILLER (V) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_705() {
	grapheme_test("\u{00A9}\u{11A8}",
		&["\u{00A9}", "\u{11A8}"],
		"  ÷ [1.0] COPYRIGHT SIGN (ExtPict) ÷ [999.0] HANGUL JONGSEONG KIYEOK (T) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_706() {
	grapheme_test("\u{00A9}\u{0308}\u{11A8}",
		&["\u{00A9}\u{0308}", "\u{11A8}"],
		"  ÷ [1.0] COPYRIGHT SIGN (ExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL JONGSEONG KIYEOK (T) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_707() {
	grapheme_test("\u{00A9}\u{AC00}",
		&["\u{00A9}", "\u{AC00}"],
		"  ÷ [1.0] COPYRIGHT SIGN (ExtPict) ÷ [999.0] HANGUL SYLLABLE GA (LV) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_708() {
	grapheme_test("\u{00A9}\u{0308}\u{AC00}",
		&["\u{00A9}\u{0308}", "\u{AC00}"],
		"  ÷ [1.0] COPYRIGHT SIGN (ExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL SYLLABLE GA (LV) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_709() {
	grapheme_test("\u{00A9}\u{AC01}",
		&["\u{00A9}", "\u{AC01}"],
		"  ÷ [1.0] COPYRIGHT SIGN (ExtPict) ÷ [999.0] HANGUL SYLLABLE GAG (LVT) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_710() {
	grapheme_test("\u{00A9}\u{0308}\u{AC01}",
		&["\u{00A9}\u{0308}", "\u{AC01}"],
		"  ÷ [1.0] COPYRIGHT SIGN (ExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL SYLLABLE GAG (LVT) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_711() {
	grapheme_test("\u{00A9}\u{1CF5}",
		&["\u{00A9}", "\u{1CF5}"],
		"  ÷ [1.0] COPYRIGHT SIGN (ExtPict) ÷ [999.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_712() {
	grapheme_test("\u{00A9}\u{0308}\u{1CF5}",
		&["\u{00A9}\u{0308}", "\u{1CF5}"],
		"  ÷ [1.0] COPYRIGHT SIGN (ExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_713() {
	grapheme_test("\u{00A9}\u{0915}",
		&["\u{00A9}", "\u{0915}"],
		"  ÷ [1.0] COPYRIGHT SIGN (ExtPict) ÷ [999.0] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_714() {
	grapheme_test("\u{00A9}\u{0308}\u{0915}",
		&["\u{00A9}\u{0308}", "\u{0915}"],
		"  ÷ [1.0] COPYRIGHT SIGN (ExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_715() {
	grapheme_test("\u{00A9}\u{00A9}",
		&["\u{00A9}", "\u{00A9}"],
		"  ÷ [1.0] COPYRIGHT SIGN (ExtPict) ÷ [999.0] COPYRIGHT SIGN (ExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_716() {
	grapheme_test("\u{00A9}\u{0308}\u{00A9}",
		&["\u{00A9}\u{0308}", "\u{00A9}"],
		"  ÷ [1.0] COPYRIGHT SIGN (ExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] COPYRIGHT SIGN (ExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_717() {
	grapheme_test("\u{00A9}\u{0020}",
		&["\u{00A9}", "\u{0020}"],
		"  ÷ [1.0] COPYRIGHT SIGN (ExtPict) ÷ [999.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_718() {
	grapheme_test("\u{00A9}\u{0308}\u{0020}",
		&["\u{00A9}\u{0308}", "\u{0020}"],
		"  ÷ [1.0] COPYRIGHT SIGN (ExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_719() {
	grapheme_test("\u{00A9}\u{0378}",
		&["\u{00A9}", "\u{0378}"],
		"  ÷ [1.0] COPYRIGHT SIGN (ExtPict) ÷ [999.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_720() {
	grapheme_test("\u{00A9}\u{0308}\u{0378}",
		&["\u{00A9}\u{0308}", "\u{0378}"],
		"  ÷ [1.0] COPYRIGHT SIGN (ExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_721() {
	grapheme_test("\u{0020}\u{000D}",
		&["\u{0020}", "\u{000D}"],
		"  ÷ [1.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [5.0] <CARRIAGE RETURN (CR)> (CR) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_722() {
	grapheme_test("\u{0020}\u{0308}\u{000D}",
		&["\u{0020}\u{0308}", "\u{000D}"],
		"  ÷ [1.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <CARRIAGE RETURN (CR)> (CR) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_723() {
	grapheme_test("\u{0020}\u{000A}",
		&["\u{0020}", "\u{000A}"],
		"  ÷ [1.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [5.0] <LINE FEED (LF)> (LF) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_724() {
	grapheme_test("\u{0020}\u{0308}\u{000A}",
		&["\u{0020}\u{0308}", "\u{000A}"],
		"  ÷ [1.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <LINE FEED (LF)> (LF) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_725() {
	grapheme_test("\u{0020}\u{0000}",
		&["\u{0020}", "\u{0000}"],
		"  ÷ [1.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [5.0] <NULL> (Control) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_726() {
	grapheme_test("\u{0020}\u{0308}\u{0000}",
		&["\u{0020}\u{0308}", "\u{0000}"],
		"  ÷ [1.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <NULL> (Control) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_727() {
	grapheme_test("\u{0020}\u{094D}",
		&["\u{0020}\u{094D}"],
		"  ÷ [1.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_728() {
	grapheme_test("\u{0020}\u{0308}\u{094D}",
		&["\u{0020}\u{0308}\u{094D}"],
		"  ÷ [1.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_729() {
	grapheme_test("\u{0020}\u{0300}",
		&["\u{0020}\u{0300}"],
		"  ÷ [1.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_730() {
	grapheme_test("\u{0020}\u{0308}\u{0300}",
		&["\u{0020}\u{0308}\u{0300}"],
		"  ÷ [1.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_731() {
	grapheme_test("\u{0020}\u{200C}",
		&["\u{0020}\u{200C}"],
		"  ÷ [1.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_732() {
	grapheme_test("\u{0020}\u{0308}\u{200C}",
		&["\u{0020}\u{0308}\u{200C}"],
		"  ÷ [1.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_733() {
	grapheme_test("\u{0020}\u{200D}",
		&["\u{0020}\u{200D}"],
		"  ÷ [1.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] ZERO WIDTH JOINER (ZWJ) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_734() {
	grapheme_test("\u{0020}\u{0308}\u{200D}",
		&["\u{0020}\u{0308}\u{200D}"],
		"  ÷ [1.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] ZERO WIDTH JOINER (ZWJ) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_735() {
	grapheme_test("\u{0020}\u{1F1E6}",
		&["\u{0020}", "\u{1F1E6}"],
		"  ÷ [1.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [999.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_736() {
	grapheme_test("\u{0020}\u{0308}\u{1F1E6}",
		&["\u{0020}\u{0308}", "\u{1F1E6}"],
		"  ÷ [1.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_737() {
	grapheme_test("\u{0020}\u{06DD}",
		&["\u{0020}", "\u{06DD}"],
		"  ÷ [1.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [999.0] ARABIC END OF AYAH (Prepend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_738() {
	grapheme_test("\u{0020}\u{0308}\u{06DD}",
		&["\u{0020}\u{0308}", "\u{06DD}"],
		"  ÷ [1.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] ARABIC END OF AYAH (Prepend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_739() {
	grapheme_test("\u{0020}\u{0903}",
		&["\u{0020}\u{0903}"],
		"  ÷ [1.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.1] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_740() {
	grapheme_test("\u{0020}\u{0308}\u{0903}",
		&["\u{0020}\u{0308}\u{0903}"],
		"  ÷ [1.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.1] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_741() {
	grapheme_test("\u{0020}\u{1100}",
		&["\u{0020}", "\u{1100}"],
		"  ÷ [1.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [999.0] HANGUL CHOSEONG KIYEOK (L) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_742() {
	grapheme_test("\u{0020}\u{0308}\u{1100}",
		&["\u{0020}\u{0308}", "\u{1100}"],
		"  ÷ [1.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL CHOSEONG KIYEOK (L) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_743() {
	grapheme_test("\u{0020}\u{1160}",
		&["\u{0020}", "\u{1160}"],
		"  ÷ [1.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [999.0] HANGUL JUNGSEONG FILLER (V) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_744() {
	grapheme_test("\u{0020}\u{0308}\u{1160}",
		&["\u{0020}\u{0308}", "\u{1160}"],
		"  ÷ [1.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL JUNGSEONG FILLER (V) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_745() {
	grapheme_test("\u{0020}\u{11A8}",
		&["\u{0020}", "\u{11A8}"],
		"  ÷ [1.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [999.0] HANGUL JONGSEONG KIYEOK (T) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_746() {
	grapheme_test("\u{0020}\u{0308}\u{11A8}",
		&["\u{0020}\u{0308}", "\u{11A8}"],
		"  ÷ [1.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL JONGSEONG KIYEOK (T) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_747() {
	grapheme_test("\u{0020}\u{AC00}",
		&["\u{0020}", "\u{AC00}"],
		"  ÷ [1.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [999.0] HANGUL SYLLABLE GA (LV) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_748() {
	grapheme_test("\u{0020}\u{0308}\u{AC00}",
		&["\u{0020}\u{0308}", "\u{AC00}"],
		"  ÷ [1.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL SYLLABLE GA (LV) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_749() {
	grapheme_test("\u{0020}\u{AC01}",
		&["\u{0020}", "\u{AC01}"],
		"  ÷ [1.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [999.0] HANGUL SYLLABLE GAG (LVT) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_750() {
	grapheme_test("\u{0020}\u{0308}\u{AC01}",
		&["\u{0020}\u{0308}", "\u{AC01}"],
		"  ÷ [1.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL SYLLABLE GAG (LVT) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_751() {
	grapheme_test("\u{0020}\u{1CF5}",
		&["\u{0020}", "\u{1CF5}"],
		"  ÷ [1.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [999.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_752() {
	grapheme_test("\u{0020}\u{0308}\u{1CF5}",
		&["\u{0020}\u{0308}", "\u{1CF5}"],
		"  ÷ [1.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_753() {
	grapheme_test("\u{0020}\u{0915}",
		&["\u{0020}", "\u{0915}"],
		"  ÷ [1.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [999.0] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_754() {
	grapheme_test("\u{0020}\u{0308}\u{0915}",
		&["\u{0020}\u{0308}", "\u{0915}"],
		"  ÷ [1.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_755() {
	grapheme_test("\u{0020}\u{00A9}",
		&["\u{0020}", "\u{00A9}"],
		"  ÷ [1.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [999.0] COPYRIGHT SIGN (ExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_756() {
	grapheme_test("\u{0020}\u{0308}\u{00A9}",
		&["\u{0020}\u{0308}", "\u{00A9}"],
		"  ÷ [1.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] COPYRIGHT SIGN (ExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_757() {
	grapheme_test("\u{0020}\u{0020}",
		&["\u{0020}", "\u{0020}"],
		"  ÷ [1.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [999.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_758() {
	grapheme_test("\u{0020}\u{0308}\u{0020}",
		&["\u{0020}\u{0308}", "\u{0020}"],
		"  ÷ [1.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_759() {
	grapheme_test("\u{0020}\u{0378}",
		&["\u{0020}", "\u{0378}"],
		"  ÷ [1.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [999.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_760() {
	grapheme_test("\u{0020}\u{0308}\u{0378}",
		&["\u{0020}\u{0308}", "\u{0378}"],
		"  ÷ [1.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_761() {
	grapheme_test("\u{0378}\u{000D}",
		&["\u{0378}", "\u{000D}"],
		"  ÷ [1.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [5.0] <CARRIAGE RETURN (CR)> (CR) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_762() {
	grapheme_test("\u{0378}\u{0308}\u{000D}",
		&["\u{0378}\u{0308}", "\u{000D}"],
		"  ÷ [1.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <CARRIAGE RETURN (CR)> (CR) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_763() {
	grapheme_test("\u{0378}\u{000A}",
		&["\u{0378}", "\u{000A}"],
		"  ÷ [1.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [5.0] <LINE FEED (LF)> (LF) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_764() {
	grapheme_test("\u{0378}\u{0308}\u{000A}",
		&["\u{0378}\u{0308}", "\u{000A}"],
		"  ÷ [1.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <LINE FEED (LF)> (LF) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_765() {
	grapheme_test("\u{0378}\u{0000}",
		&["\u{0378}", "\u{0000}"],
		"  ÷ [1.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [5.0] <NULL> (Control) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_766() {
	grapheme_test("\u{0378}\u{0308}\u{0000}",
		&["\u{0378}\u{0308}", "\u{0000}"],
		"  ÷ [1.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [5.0] <NULL> (Control) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_767() {
	grapheme_test("\u{0378}\u{094D}",
		&["\u{0378}\u{094D}"],
		"  ÷ [1.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_768() {
	grapheme_test("\u{0378}\u{0308}\u{094D}",
		&["\u{0378}\u{0308}\u{094D}"],
		"  ÷ [1.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_769() {
	grapheme_test("\u{0378}\u{0300}",
		&["\u{0378}\u{0300}"],
		"  ÷ [1.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_770() {
	grapheme_test("\u{0378}\u{0308}\u{0300}",
		&["\u{0378}\u{0308}\u{0300}"],
		"  ÷ [1.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] COMBINING GRAVE ACCENT (Extend_ConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_771() {
	grapheme_test("\u{0378}\u{200C}",
		&["\u{0378}\u{200C}"],
		"  ÷ [1.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_772() {
	grapheme_test("\u{0378}\u{0308}\u{200C}",
		&["\u{0378}\u{0308}\u{200C}"],
		"  ÷ [1.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_773() {
	grapheme_test("\u{0378}\u{200D}",
		&["\u{0378}\u{200D}"],
		"  ÷ [1.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] ZERO WIDTH JOINER (ZWJ) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_774() {
	grapheme_test("\u{0378}\u{0308}\u{200D}",
		&["\u{0378}\u{0308}\u{200D}"],
		"  ÷ [1.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] ZERO WIDTH JOINER (ZWJ) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_775() {
	grapheme_test("\u{0378}\u{1F1E6}",
		&["\u{0378}", "\u{1F1E6}"],
		"  ÷ [1.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [999.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_776() {
	grapheme_test("\u{0378}\u{0308}\u{1F1E6}",
		&["\u{0378}\u{0308}", "\u{1F1E6}"],
		"  ÷ [1.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_777() {
	grapheme_test("\u{0378}\u{06DD}",
		&["\u{0378}", "\u{06DD}"],
		"  ÷ [1.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [999.0] ARABIC END OF AYAH (Prepend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_778() {
	grapheme_test("\u{0378}\u{0308}\u{06DD}",
		&["\u{0378}\u{0308}", "\u{06DD}"],
		"  ÷ [1.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] ARABIC END OF AYAH (Prepend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_779() {
	grapheme_test("\u{0378}\u{0903}",
		&["\u{0378}\u{0903}"],
		"  ÷ [1.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.1] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_780() {
	grapheme_test("\u{0378}\u{0308}\u{0903}",
		&["\u{0378}\u{0308}\u{0903}"],
		"  ÷ [1.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.1] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_781() {
	grapheme_test("\u{0378}\u{1100}",
		&["\u{0378}", "\u{1100}"],
		"  ÷ [1.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [999.0] HANGUL CHOSEONG KIYEOK (L) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_782() {
	grapheme_test("\u{0378}\u{0308}\u{1100}",
		&["\u{0378}\u{0308}", "\u{1100}"],
		"  ÷ [1.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL CHOSEONG KIYEOK (L) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_783() {
	grapheme_test("\u{0378}\u{1160}",
		&["\u{0378}", "\u{1160}"],
		"  ÷ [1.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [999.0] HANGUL JUNGSEONG FILLER (V) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_784() {
	grapheme_test("\u{0378}\u{0308}\u{1160}",
		&["\u{0378}\u{0308}", "\u{1160}"],
		"  ÷ [1.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL JUNGSEONG FILLER (V) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_785() {
	grapheme_test("\u{0378}\u{11A8}",
		&["\u{0378}", "\u{11A8}"],
		"  ÷ [1.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [999.0] HANGUL JONGSEONG KIYEOK (T) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_786() {
	grapheme_test("\u{0378}\u{0308}\u{11A8}",
		&["\u{0378}\u{0308}", "\u{11A8}"],
		"  ÷ [1.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL JONGSEONG KIYEOK (T) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_787() {
	grapheme_test("\u{0378}\u{AC00}",
		&["\u{0378}", "\u{AC00}"],
		"  ÷ [1.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [999.0] HANGUL SYLLABLE GA (LV) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_788() {
	grapheme_test("\u{0378}\u{0308}\u{AC00}",
		&["\u{0378}\u{0308}", "\u{AC00}"],
		"  ÷ [1.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL SYLLABLE GA (LV) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_789() {
	grapheme_test("\u{0378}\u{AC01}",
		&["\u{0378}", "\u{AC01}"],
		"  ÷ [1.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [999.0] HANGUL SYLLABLE GAG (LVT) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_790() {
	grapheme_test("\u{0378}\u{0308}\u{AC01}",
		&["\u{0378}\u{0308}", "\u{AC01}"],
		"  ÷ [1.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] HANGUL SYLLABLE GAG (LVT) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_791() {
	grapheme_test("\u{0378}\u{1CF5}",
		&["\u{0378}", "\u{1CF5}"],
		"  ÷ [1.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [999.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_792() {
	grapheme_test("\u{0378}\u{0308}\u{1CF5}",
		&["\u{0378}\u{0308}", "\u{1CF5}"],
		"  ÷ [1.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_793() {
	grapheme_test("\u{0378}\u{0915}",
		&["\u{0378}", "\u{0915}"],
		"  ÷ [1.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [999.0] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_794() {
	grapheme_test("\u{0378}\u{0308}\u{0915}",
		&["\u{0378}\u{0308}", "\u{0915}"],
		"  ÷ [1.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_795() {
	grapheme_test("\u{0378}\u{00A9}",
		&["\u{0378}", "\u{00A9}"],
		"  ÷ [1.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [999.0] COPYRIGHT SIGN (ExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_796() {
	grapheme_test("\u{0378}\u{0308}\u{00A9}",
		&["\u{0378}\u{0308}", "\u{00A9}"],
		"  ÷ [1.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] COPYRIGHT SIGN (ExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_797() {
	grapheme_test("\u{0378}\u{0020}",
		&["\u{0378}", "\u{0020}"],
		"  ÷ [1.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [999.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_798() {
	grapheme_test("\u{0378}\u{0308}\u{0020}",
		&["\u{0378}\u{0308}", "\u{0020}"],
		"  ÷ [1.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_799() {
	grapheme_test("\u{0378}\u{0378}",
		&["\u{0378}", "\u{0378}"],
		"  ÷ [1.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [999.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_800() {
	grapheme_test("\u{0378}\u{0308}\u{0378}",
		&["\u{0378}\u{0308}", "\u{0378}"],
		"  ÷ [1.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] <reserved-0378> (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_801() {
	grapheme_test("\u{000D}\u{000A}\u{0061}\u{000A}\u{0308}",
		&["\u{000D}\u{000A}", "\u{0061}", "\u{000A}", "\u{0308}"],
		"  ÷ [1.0] <CARRIAGE RETURN (CR)> (CR) × [3.0] <LINE FEED (LF)> (LF) ÷ [4.0] LATIN SMALL LETTER A (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [5.0] <LINE FEED (LF)> (LF) ÷ [4.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_802() {
	grapheme_test("\u{0061}\u{0308}",
		&["\u{0061}\u{0308}"],
		"  ÷ [1.0] LATIN SMALL LETTER A (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_803() {
	grapheme_test("\u{0020}\u{200D}\u{0646}",
		&["\u{0020}\u{200D}", "\u{0646}"],
		"  ÷ [1.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] ZERO WIDTH JOINER (ZWJ) ÷ [999.0] ARABIC LETTER NOON (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_804() {
	grapheme_test("\u{0646}\u{200D}\u{0020}",
		&["\u{0646}\u{200D}", "\u{0020}"],
		"  ÷ [1.0] ARABIC LETTER NOON (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] ZERO WIDTH JOINER (ZWJ) ÷ [999.0] SPACE (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_805() {
	grapheme_test("\u{1100}\u{1100}",
		&["\u{1100}\u{1100}"],
		"  ÷ [1.0] HANGUL CHOSEONG KIYEOK (L) × [6.0] HANGUL CHOSEONG KIYEOK (L) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_806() {
	grapheme_test("\u{AC00}\u{11A8}\u{1100}",
		&["\u{AC00}\u{11A8}", "\u{1100}"],
		"  ÷ [1.0] HANGUL SYLLABLE GA (LV) × [7.0] HANGUL JONGSEONG KIYEOK (T) ÷ [999.0] HANGUL CHOSEONG KIYEOK (L) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_807() {
	grapheme_test("\u{AC01}\u{11A8}\u{1100}",
		&["\u{AC01}\u{11A8}", "\u{1100}"],
		"  ÷ [1.0] HANGUL SYLLABLE GAG (LVT) × [8.0] HANGUL JONGSEONG KIYEOK (T) ÷ [999.0] HANGUL CHOSEONG KIYEOK (L) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_808() {
	grapheme_test("\u{1F1E6}\u{1F1E7}\u{1F1E8}\u{0062}",
		&["\u{1F1E6}\u{1F1E7}", "\u{1F1E8}", "\u{0062}"],
		"  ÷ [1.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) × [12.0] REGIONAL INDICATOR SYMBOL LETTER B (RI) ÷ [999.0] REGIONAL INDICATOR SYMBOL LETTER C (RI) ÷ [999.0] LATIN SMALL LETTER B (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_809() {
	grapheme_test("\u{0061}\u{1F1E6}\u{1F1E7}\u{1F1E8}\u{0062}",
		&["\u{0061}", "\u{1F1E6}\u{1F1E7}", "\u{1F1E8}", "\u{0062}"],
		"  ÷ [1.0] LATIN SMALL LETTER A (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [999.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) × [13.0] REGIONAL INDICATOR SYMBOL LETTER B (RI) ÷ [999.0] REGIONAL INDICATOR SYMBOL LETTER C (RI) ÷ [999.0] LATIN SMALL LETTER B (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_810() {
	grapheme_test("\u{0061}\u{1F1E6}\u{1F1E7}\u{200D}\u{1F1E8}\u{0062}",
		&["\u{0061}", "\u{1F1E6}\u{1F1E7}\u{200D}", "\u{1F1E8}", "\u{0062}"],
		"  ÷ [1.0] LATIN SMALL LETTER A (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [999.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) × [13.0] REGIONAL INDICATOR SYMBOL LETTER B (RI) × [9.0] ZERO WIDTH JOINER (ZWJ) ÷ [999.0] REGIONAL INDICATOR SYMBOL LETTER C (RI) ÷ [999.0] LATIN SMALL LETTER B (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_811() {
	grapheme_test("\u{0061}\u{1F1E6}\u{200D}\u{1F1E7}\u{1F1E8}\u{0062}",
		&["\u{0061}", "\u{1F1E6}\u{200D}", "\u{1F1E7}\u{1F1E8}", "\u{0062}"],
		"  ÷ [1.0] LATIN SMALL LETTER A (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [999.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) × [9.0] ZERO WIDTH JOINER (ZWJ) ÷ [999.0] REGIONAL INDICATOR SYMBOL LETTER B (RI) × [13.0] REGIONAL INDICATOR SYMBOL LETTER C (RI) ÷ [999.0] LATIN SMALL LETTER B (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_812() {
	grapheme_test("\u{0061}\u{1F1E6}\u{1F1E7}\u{1F1E8}\u{1F1E9}\u{0062}",
		&["\u{0061}", "\u{1F1E6}\u{1F1E7}", "\u{1F1E8}\u{1F1E9}", "\u{0062}"],
		"  ÷ [1.0] LATIN SMALL LETTER A (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [999.0] REGIONAL INDICATOR SYMBOL LETTER A (RI) × [13.0] REGIONAL INDICATOR SYMBOL LETTER B (RI) ÷ [999.0] REGIONAL INDICATOR SYMBOL LETTER C (RI) × [13.0] REGIONAL INDICATOR SYMBOL LETTER D (RI) ÷ [999.0] LATIN SMALL LETTER B (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_813() {
	grapheme_test("\u{0061}\u{200D}",
		&["\u{0061}\u{200D}"],
		"  ÷ [1.0] LATIN SMALL LETTER A (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] ZERO WIDTH JOINER (ZWJ) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_814() {
	grapheme_test("\u{0061}\u{0308}\u{0062}",
		&["\u{0061}\u{0308}", "\u{0062}"],
		"  ÷ [1.0] LATIN SMALL LETTER A (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) ÷ [999.0] LATIN SMALL LETTER B (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_815() {
	grapheme_test("\u{0061}\u{0903}\u{0062}",
		&["\u{0061}\u{0903}", "\u{0062}"],
		"  ÷ [1.0] LATIN SMALL LETTER A (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.1] DEVANAGARI SIGN VISARGA (SpacingMark) ÷ [999.0] LATIN SMALL LETTER B (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_816() {
	grapheme_test("\u{0061}\u{0600}\u{0062}",
		&["\u{0061}", "\u{0600}\u{0062}"],
		"  ÷ [1.0] LATIN SMALL LETTER A (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [999.0] ARABIC NUMBER SIGN (Prepend) × [9.2] LATIN SMALL LETTER B (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_817() {
	grapheme_test("\u{1F476}\u{1F3FF}\u{1F476}",
		&["\u{1F476}\u{1F3FF}", "\u{1F476}"],
		"  ÷ [1.0] BABY (ExtPict) × [9.0] EMOJI MODIFIER FITZPATRICK TYPE-6 (Extend_ConjunctExtender) ÷ [999.0] BABY (ExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_818() {
	grapheme_test("\u{0061}\u{1F3FF}\u{1F476}",
		&["\u{0061}\u{1F3FF}", "\u{1F476}"],
		"  ÷ [1.0] LATIN SMALL LETTER A (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] EMOJI MODIFIER FITZPATRICK TYPE-6 (Extend_ConjunctExtender) ÷ [999.0] BABY (ExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_819() {
	grapheme_test("\u{0061}\u{1F3FF}\u{1F476}\u{200D}\u{1F6D1}",
		&["\u{0061}\u{1F3FF}", "\u{1F476}\u{200D}\u{1F6D1}"],
		"  ÷ [1.0] LATIN SMALL LETTER A (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] EMOJI MODIFIER FITZPATRICK TYPE-6 (Extend_ConjunctExtender) ÷ [999.0] BABY (ExtPict) × [9.0] ZERO WIDTH JOINER (ZWJ) × [11.0] OCTAGONAL SIGN (ExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_820() {
	grapheme_test("\u{1F476}\u{1F3FF}\u{0308}\u{200D}\u{1F476}\u{1F3FF}",
		&["\u{1F476}\u{1F3FF}\u{0308}\u{200D}\u{1F476}\u{1F3FF}"],
		"  ÷ [1.0] BABY (ExtPict) × [9.0] EMOJI MODIFIER FITZPATRICK TYPE-6 (Extend_ConjunctExtender) × [9.0] COMBINING DIAERESIS (Extend_ConjunctExtender) × [9.0] ZERO WIDTH JOINER (ZWJ) × [11.0] BABY (ExtPict) × [9.0] EMOJI MODIFIER FITZPATRICK TYPE-6 (Extend_ConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_821() {
	grapheme_test("\u{1F6D1}\u{200D}\u{1F6D1}",
		&["\u{1F6D1}\u{200D}\u{1F6D1}"],
		"  ÷ [1.0] OCTAGONAL SIGN (ExtPict) × [9.0] ZERO WIDTH JOINER (ZWJ) × [11.0] OCTAGONAL SIGN (ExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_822() {
	grapheme_test("\u{0061}\u{200D}\u{1F6D1}",
		&["\u{0061}\u{200D}", "\u{1F6D1}"],
		"  ÷ [1.0] LATIN SMALL LETTER A (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] ZERO WIDTH JOINER (ZWJ) ÷ [999.0] OCTAGONAL SIGN (ExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_823() {
	grapheme_test("\u{2701}\u{200D}\u{2701}",
		&["\u{2701}\u{200D}", "\u{2701}"],
		"  ÷ [1.0] UPPER BLADE SCISSORS (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] ZERO WIDTH JOINER (ZWJ) ÷ [999.0] UPPER BLADE SCISSORS (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_824() {
	grapheme_test("\u{0061}\u{200D}\u{2701}",
		&["\u{0061}\u{200D}", "\u{2701}"],
		"  ÷ [1.0] LATIN SMALL LETTER A (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] ZERO WIDTH JOINER (ZWJ) ÷ [999.0] UPPER BLADE SCISSORS (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_825() {
	grapheme_test("\u{0915}\u{0924}",
		&["\u{0915}", "\u{0924}"],
		"  ÷ [1.0] DEVANAGARI LETTER KA (LinkingConsonant) ÷ [999.0] DEVANAGARI LETTER TA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_826() {
	grapheme_test("\u{0915}\u{094D}\u{0924}",
		&["\u{0915}\u{094D}\u{0924}"],
		"  ÷ [1.0] DEVANAGARI LETTER KA (LinkingConsonant) × [9.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) × [9.3] DEVANAGARI LETTER TA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_827() {
	grapheme_test("\u{0915}\u{094D}\u{094D}\u{0924}",
		&["\u{0915}\u{094D}\u{094D}\u{0924}"],
		"  ÷ [1.0] DEVANAGARI LETTER KA (LinkingConsonant) × [9.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) × [9.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) × [9.3] DEVANAGARI LETTER TA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_828() {
	grapheme_test("\u{0915}\u{094D}\u{200D}\u{0924}",
		&["\u{0915}\u{094D}\u{200D}\u{0924}"],
		"  ÷ [1.0] DEVANAGARI LETTER KA (LinkingConsonant) × [9.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) × [9.0] ZERO WIDTH JOINER (ZWJ) × [9.3] DEVANAGARI LETTER TA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_829() {
	grapheme_test("\u{0915}\u{093C}\u{200D}\u{094D}\u{0924}",
		&["\u{0915}\u{093C}\u{200D}\u{094D}\u{0924}"],
		"  ÷ [1.0] DEVANAGARI LETTER KA (LinkingConsonant) × [9.0] DEVANAGARI SIGN NUKTA (Extend_ConjunctExtender) × [9.0] ZERO WIDTH JOINER (ZWJ) × [9.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) × [9.3] DEVANAGARI LETTER TA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_830() {
	grapheme_test("\u{0915}\u{093C}\u{094D}\u{200D}\u{0924}",
		&["\u{0915}\u{093C}\u{094D}\u{200D}\u{0924}"],
		"  ÷ [1.0] DEVANAGARI LETTER KA (LinkingConsonant) × [9.0] DEVANAGARI SIGN NUKTA (Extend_ConjunctExtender) × [9.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) × [9.0] ZERO WIDTH JOINER (ZWJ) × [9.3] DEVANAGARI LETTER TA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_831() {
	grapheme_test("\u{0915}\u{094D}\u{0924}\u{094D}\u{092F}",
		&["\u{0915}\u{094D}\u{0924}\u{094D}\u{092F}"],
		"  ÷ [1.0] DEVANAGARI LETTER KA (LinkingConsonant) × [9.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) × [9.3] DEVANAGARI LETTER TA (LinkingConsonant) × [9.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) × [9.3] DEVANAGARI LETTER YA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_832() {
	grapheme_test("\u{0915}\u{094D}\u{0061}",
		&["\u{0915}\u{094D}", "\u{0061}"],
		"  ÷ [1.0] DEVANAGARI LETTER KA (LinkingConsonant) × [9.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) ÷ [999.0] LATIN SMALL LETTER A (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_833() {
	grapheme_test("\u{0061}\u{094D}\u{0924}",
		&["\u{0061}\u{094D}\u{0924}"],
		"  ÷ [1.0] LATIN SMALL LETTER A (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) × [9.3] DEVANAGARI LETTER TA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_834() {
	grapheme_test("\u{003F}\u{094D}\u{0924}",
		&["\u{003F}\u{094D}\u{0924}"],
		"  ÷ [1.0] QUESTION MARK (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) × [9.3] DEVANAGARI LETTER TA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_835() {
	grapheme_test("\u{0915}\u{094D}\u{094D}\u{0924}",
		&["\u{0915}\u{094D}\u{094D}\u{0924}"],
		"  ÷ [1.0] DEVANAGARI LETTER KA (LinkingConsonant) × [9.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) × [9.0] DEVANAGARI SIGN VIRAMA (Extend_ConjunctLinker) × [9.3] DEVANAGARI LETTER TA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_836() {
	grapheme_test("\u{0AB8}\u{0AFB}\u{0ACD}\u{0AB8}\u{0AFB}",
		&["\u{0AB8}\u{0AFB}\u{0ACD}\u{0AB8}\u{0AFB}"],
		"  ÷ [1.0] GUJARATI LETTER SA (LinkingConsonant) × [9.0] GUJARATI SIGN SHADDA (Extend_ConjunctExtender) × [9.0] GUJARATI SIGN VIRAMA (Extend_ConjunctLinker) × [9.3] GUJARATI LETTER SA (LinkingConsonant) × [9.0] GUJARATI SIGN SHADDA (Extend_ConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_837() {
	grapheme_test("\u{1019}\u{1039}\u{1018}\u{102C}\u{1037}",
		&["\u{1019}\u{1039}\u{1018}", "\u{102C}\u{1037}"],
		"  ÷ [1.0] MYANMAR LETTER MA (LinkingConsonant) × [9.0] MYANMAR SIGN VIRAMA (Extend_ConjunctLinker) × [9.3] MYANMAR LETTER BHA (LinkingConsonant) ÷ [999.0] MYANMAR VOWEL SIGN AA (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] MYANMAR SIGN DOT BELOW (Extend_ConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_838() {
	grapheme_test("\u{1004}\u{103A}\u{1039}\u{1011}\u{1039}\u{1011}",
		&["\u{1004}\u{103A}\u{1039}\u{1011}\u{1039}\u{1011}"],
		"  ÷ [1.0] MYANMAR LETTER NGA (LinkingConsonant) × [9.0] MYANMAR SIGN ASAT (Extend_ConjunctExtender) × [9.0] MYANMAR SIGN VIRAMA (Extend_ConjunctLinker) × [9.3] MYANMAR LETTER THA (LinkingConsonant) × [9.0] MYANMAR SIGN VIRAMA (Extend_ConjunctLinker) × [9.3] MYANMAR LETTER THA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_839() {
	grapheme_test("\u{1B12}\u{1B01}\u{1B32}\u{1B44}\u{1B2F}\u{1B32}\u{1B44}\u{1B22}\u{1B44}\u{1B2C}\u{1B32}\u{1B44}\u{1B22}\u{1B38}",
		&["\u{1B12}\u{1B01}", "\u{1B32}\u{1B44}\u{1B2F}", "\u{1B32}\u{1B44}\u{1B22}\u{1B44}\u{1B2C}", "\u{1B32}\u{1B44}\u{1B22}\u{1B38}"],
		"  ÷ [1.0] BALINESE LETTER OKARA TEDUNG (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] BALINESE SIGN ULU CANDRA (Extend_ConjunctExtender) ÷ [999.0] BALINESE LETTER SA (LinkingConsonant) × [9.0] BALINESE ADEG ADEG (Extend_ConjunctLinker) × [9.3] BALINESE LETTER WA (LinkingConsonant) ÷ [999.0] BALINESE LETTER SA (LinkingConsonant) × [9.0] BALINESE ADEG ADEG (Extend_ConjunctLinker) × [9.3] BALINESE LETTER TA (LinkingConsonant) × [9.0] BALINESE ADEG ADEG (Extend_ConjunctLinker) × [9.3] BALINESE LETTER YA (LinkingConsonant) ÷ [999.0] BALINESE LETTER SA (LinkingConsonant) × [9.0] BALINESE ADEG ADEG (Extend_ConjunctLinker) × [9.3] BALINESE LETTER TA (LinkingConsonant) × [9.0] BALINESE VOWEL SIGN SUKU (Extend_ConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_840() {
	grapheme_test("\u{179F}\u{17D2}\u{178F}\u{17D2}\u{179A}\u{17B8}",
		&["\u{179F}\u{17D2}\u{178F}\u{17D2}\u{179A}\u{17B8}"],
		"  ÷ [1.0] KHMER LETTER SA (LinkingConsonant) × [9.0] KHMER SIGN COENG (Extend_ConjunctLinker) × [9.3] KHMER LETTER TA (LinkingConsonant) × [9.0] KHMER SIGN COENG (Extend_ConjunctLinker) × [9.3] KHMER LETTER RO (LinkingConsonant) × [9.0] KHMER VOWEL SIGN II (Extend_ConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_841() {
	grapheme_test("\u{1B26}\u{1B17}\u{1B44}\u{1B13}",
		&["\u{1B26}", "\u{1B17}\u{1B44}\u{1B13}"],
		"  ÷ [1.0] BALINESE LETTER NA (LinkingConsonant) ÷ [999.0] BALINESE LETTER NGA (LinkingConsonant) × [9.0] BALINESE ADEG ADEG (Extend_ConjunctLinker) × [9.3] BALINESE LETTER KA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_842() {
	grapheme_test("\u{1B27}\u{1B13}\u{1B44}\u{1B0B}\u{1B0B}\u{1B04}",
		&["\u{1B27}", "\u{1B13}\u{1B44}\u{1B0B}", "\u{1B0B}\u{1B04}"],
		"  ÷ [1.0] BALINESE LETTER PA (LinkingConsonant) ÷ [999.0] BALINESE LETTER KA (LinkingConsonant) × [9.0] BALINESE ADEG ADEG (Extend_ConjunctLinker) × [9.3] BALINESE LETTER RA REPA (LinkingConsonant) ÷ [999.0] BALINESE LETTER RA REPA (LinkingConsonant) × [9.1] BALINESE SIGN BISAH (SpacingMark) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_843() {
	grapheme_test("\u{1795}\u{17D2}\u{17AF}\u{1798}",
		&["\u{1795}\u{17D2}\u{17AF}", "\u{1798}"],
		"  ÷ [1.0] KHMER LETTER PHA (LinkingConsonant) × [9.0] KHMER SIGN COENG (Extend_ConjunctLinker) × [9.3] KHMER INDEPENDENT VOWEL QE (LinkingConsonant) ÷ [999.0] KHMER LETTER MO (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_844() {
	grapheme_test("\u{17A0}\u{17D2}\u{17AB}\u{1791}\u{17D0}\u{1799}",
		&["\u{17A0}\u{17D2}\u{17AB}", "\u{1791}\u{17D0}", "\u{1799}"],
		"  ÷ [1.0] KHMER LETTER HA (LinkingConsonant) × [9.0] KHMER SIGN COENG (Extend_ConjunctLinker) × [9.3] KHMER INDEPENDENT VOWEL RY (LinkingConsonant) ÷ [999.0] KHMER LETTER TO (LinkingConsonant) × [9.0] KHMER SIGN SAMYOK SANNYA (Extend_ConjunctExtender) ÷ [999.0] KHMER LETTER YO (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_845() {
	grapheme_test("\u{1B05}\u{1B44}\u{1B33}\u{1B03}",
		&["\u{1B05}\u{1B44}\u{1B33}\u{1B03}"],
		"  ÷ [1.0] BALINESE LETTER AKARA (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] BALINESE ADEG ADEG (Extend_ConjunctLinker) × [9.3] BALINESE LETTER HA (LinkingConsonant) × [9.0] BALINESE SIGN SURANG (Extend_ConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_846() {
	grapheme_test("\u{0CF1}\u{0C95}",
		&["\u{0CF1}", "\u{0C95}"],
		"  ÷ [1.0] KANNADA SIGN JIHVAMULIYA (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [999.0] KANNADA LETTER KA (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_847() {
	grapheme_test("\u{0CF2}\u{0CAB}",
		&["\u{0CF2}", "\u{0CAB}"],
		"  ÷ [1.0] KANNADA SIGN UPADHMANIYA (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [999.0] KANNADA LETTER PHA (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_848() {
	grapheme_test("\u{0CF1}\u{0C95}\u{0CBF}",
		&["\u{0CF1}", "\u{0C95}\u{0CBF}"],
		"  ÷ [1.0] KANNADA SIGN JIHVAMULIYA (XXmConjunctLinkermLinkingConsonantmExtPict) ÷ [999.0] KANNADA LETTER KA (XXmConjunctLinkermLinkingConsonantmExtPict) × [9.0] KANNADA VOWEL SIGN I (Extend_ConjunctExtender) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_849() {
	grapheme_test("\u{1CF5}\u{0995}",
		&["\u{1CF5}\u{0995}"],
		"  ÷ [1.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) × [9.3] BENGALI LETTER KA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_850() {
	grapheme_test("\u{1CF6}\u{09AA}",
		&["\u{1CF6}\u{09AA}"],
		"  ÷ [1.0] VEDIC SIGN UPADHMANIYA (ConjunctLinkermExtend) × [9.3] BENGALI LETTER PA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_851() {
	grapheme_test("\u{1CF5}\u{200C}\u{0995}",
		&["\u{1CF5}\u{200C}", "\u{0995}"],
		"  ÷ [1.0] VEDIC SIGN JIHVAMULIYA (ConjunctLinkermExtend) × [9.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [999.0] BENGALI LETTER KA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_852() {
	grapheme_test("\u{1CF6}\u{200C}\u{09AA}",
		&["\u{1CF6}\u{200C}", "\u{09AA}"],
		"  ÷ [1.0] VEDIC SIGN UPADHMANIYA (ConjunctLinkermExtend) × [9.0] ZERO WIDTH NON-JOINER (ExtendmConjunctLinkermConjunctExtender) ÷ [999.0] BENGALI LETTER PA (LinkingConsonant) ÷ [2.0]"
	);
}
#[test]
fn grapheme_cluster_test_853() {
	grapheme_test("\u{11A3A}\u{11A0B}",
		&["\u{11A3A}\u{11A0B}"],
		"  ÷ [1.0] ZANABAZAR SQUARE CLUSTER-INITIAL LETTER RA (ConjunctLinkermExtend) × [9.3] ZANABAZAR SQUARE LETTER KA (LinkingConsonant) ÷ [2.0]"
	);
}
