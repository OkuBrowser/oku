//! Emits the TLS/HTTPS transport cfgs from the enabled features, so the rest of
//! the crate can gate on short cfg names instead of long feature expressions.

fn main() {
    let feature = |name: &str| std::env::var(format!("CARGO_FEATURE_{name}")).is_ok();
    let tls = feature("TRANSPORT_TLS");
    let https = feature("TRANSPORT_HTTPS");
    let ring = feature("TLS_RING");
    let aws_lc_rs = feature("TLS_AWS_LC_RS");

    // rustls (ClientConfig, the TLS server name plumbing) is shared by DoT and
    // DoH and does not itself need a crypto provider.
    let with_rustls = tls || https;
    // A rustls crypto provider is compiled in, so a default client config can be
    // built when the caller does not supply one.
    let with_crypto_provider = ring || aws_lc_rs;

    for (name, enabled) in [
        ("transport_tls", tls),
        ("transport_https", https),
        ("with_rustls", with_rustls),
        ("with_crypto_provider", with_crypto_provider),
    ] {
        println!("cargo::rustc-check-cfg=cfg({name})");
        if enabled {
            println!("cargo::rustc-cfg={name}");
        }
    }
}
