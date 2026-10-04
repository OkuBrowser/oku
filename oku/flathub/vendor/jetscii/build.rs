use std::env;
use std::fs::{self, File};
use std::io::prelude::*;
use std::path::{Path, PathBuf};

fn main() {
    cfg();
    macros();
    simd_macros();
    println!("cargo:rerun-if-changed=build.rs");
}

fn cfg() {
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let target_feature = env::var("CARGO_CFG_TARGET_FEATURE").unwrap_or_default();

    let ok_arch = matches!(&*target_arch, "x86" | "x86_64");
    let sse4_2_guaranteed = target_feature.split(',').any(|f| f == "sse4.2");

    if sse4_2_guaranteed {
        println!(r#"cargo:rustc-cfg=jetscii_sse4_2="yes""#);
    } else if ok_arch {
        println!(r#"cargo:rustc-cfg=jetscii_sse4_2="maybe""#);
    } else {
        println!(r#"cargo:rustc-cfg=jetscii_sse4_2="no""#);
    }
}

fn macros() {
    let mut base: PathBuf = env::var_os("OUT_DIR").unwrap().into();
    base.push("src");

    fs::create_dir_all(&base)
        .unwrap_or_else(|e| panic!("Could not create directory {}: {}", base.display(), e));

    base.push("macros.rs");

    let mut f = File::create(&base)
        .unwrap_or_else(|e| panic!("Could not create {}: {}", base.display(), e));

    macros_bytes(&mut f, &base);
    macros_ascii_chars(&mut f, &base);
}

fn macros_bytes(f: &mut File, base: &Path) {
    let arms: String = (1..=16)
        .map(|max| {
            let args: Vec<_> = (0..max).map(|i| format!("$b{:02}:expr", i)).collect();
            let args = args.join(", ");

            let arg_values: Vec<_> = (0..max).map(|i| format!("$b{:02} as u8", i)).collect();

            let mut array = arg_values.clone();
            array.extend((max..16).map(|_| String::from("0")));
            let array = array.join(", ");

            let closure_body: Vec<_> = arg_values.iter().map(|n| format!("{} == c", n)).collect();
            let closure = format!("|c| {}", closure_body.join(" || "));

            format!(
                "({}) => ($crate::Bytes::new([{}], {}, {}));\n",
                args, array, max, closure
            )
        })
        .collect();

    write!(
        f,
        r#"
/// A convenience constructor for a [`Bytes`] that automatically
/// implements a fallback. Provide 1 to 16 characters.
#[macro_export]
macro_rules! bytes {{
{}}}
"#,
        arms
    ).unwrap_or_else(|e| panic!("Could not write {}: {}", base.display(), e));
}

fn macros_ascii_chars(f: &mut File, base: &Path) {
    let arms: String = (1..=16)
        .map(|max| {
            let args: Vec<_> = (0..max).map(|i| format!("$b{:02}:expr", i)).collect();
            let args = args.join(", ");

            let arg_values: Vec<_> = (0..max).map(|i| format!("$b{:02} as u8", i)).collect();

            let mut array = arg_values.clone();
            array.extend((max..16).map(|_| String::from("0")));
            let array = array.join(", ");

            let closure_body: Vec<_> = arg_values.iter().map(|n| format!("{} == c", n)).collect();
            let closure = format!("|c| {}", closure_body.join(" || "));

            format!(
                "({}) => ($crate::AsciiChars::new([{}], {}, {}));\n",
                args, array, max, closure
            )
        })
        .collect();

    write!(
        f,
        r#"
/// A convenience constructor for an [`AsciiChars`] that automatically
/// implements a fallback. Provide 1 to 16 characters.
#[macro_export]
macro_rules! ascii_chars {{
{}}}
"#,
        arms
    ).unwrap_or_else(|e| panic!("Could not write {}: {}", base.display(), e));
}

fn simd_macros() {
    let mut base: PathBuf = env::var_os("OUT_DIR").unwrap().into();
    base.push("src");

    fs::create_dir_all(&base)
        .unwrap_or_else(|e| panic!("Could not create directory {}: {}", base.display(), e));

    base.push("simd_macros.rs");

    let mut f = File::create(&base)
        .unwrap_or_else(|e| panic!("Could not create {}: {}", base.display(), e));

    let arms: String = (1..=16)
        .map(|max| {
            let args: Vec<_> = (0..max).map(|i| format!("$b{:02}:expr", i)).collect();
            let args = args.join(", ");

            let mut array: Vec<_> = (0..max).map(|i| format!("$b{:02}", i)).collect();
            array.extend((max..16).map(|_| String::from("0")));
            let array = array.join(", ");

            format!(
                "({}) => ($crate::simd::Bytes::new([{}], {}));\n",
                args, array, max
            )
        })
        .collect();

    write!(
        &mut f,
        r#"
#[allow(unused_macros)]
macro_rules! simd_bytes {{
{}}}
"#,
        arms
    ).unwrap_or_else(|e| panic!("Could not write {}: {}", base.display(), e));
}
