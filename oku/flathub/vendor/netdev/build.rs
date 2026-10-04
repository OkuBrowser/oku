fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    if matches!(target_os.as_str(), "macos" | "ios") {
        println!("cargo:rustc-link-lib=framework=SystemConfiguration");
    }
}
