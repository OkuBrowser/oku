#[cfg_attr(miri, ignore = "incompatible with miri")]
#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}

#[test]
#[ignore = "must be run on MSRV compiler"]
pub fn macro_expansion() {
    macrotest::expand("tests/expand/*.rs");
}
