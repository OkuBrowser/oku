[crates-badge]: https://img.shields.io/crates/v/netdev.svg
[crates-url]: https://crates.io/crates/netdev
[license-badge]: https://img.shields.io/crates/l/netdev.svg
[examples-url]: https://github.com/shellrow/netdev/tree/main/examples
[doc-url]: https://docs.rs/netdev/latest/netdev
[doc-interface-url]: https://docs.rs/netdev/latest/netdev/interface/interface/struct.Interface.html
[netdev-github-url]: https://github.com/shellrow/netdev
[default-net-github-url]: https://github.com/shellrow/default-net
[default-net-crates-io-url]: https://crates.io/crates/default-net

# netdev [![Crates.io][crates-badge]][crates-url] ![License][license-badge]

Cross-platform library for enumerating network interfaces with metadata.    
`netdev` provides a unified API for discovering local network interfaces
and retrieving commonly used metadata across platforms.

## Key Features
- Enumerate all available network interfaces
- Detect the default network interface
- Retrieve interface metadata:
  - Interface type
  - MAC address
  - IPv4 / IPv6 addresses and prefixes
  - MTU, flags, operational state, etc...
- Native traffic statistics (RX/TX bytes) for each interface
- Designed for **cross-platform**

See the [Interface][doc-interface-url] struct documentation for more details.

## Supported platforms
- Linux
- macOS
- Windows
- Android
- iOS
- BSDs

## Usage
Add `netdev` to your `Cargo.toml`:
```toml
[dependencies]
netdev = "0.45"
```

For more details, see [examples][examples-url] or [doc][doc-url].  

## Feature flags
- `gateway` (default)
  - Enables default interface and default gateway helpers.
- `apple-system-configuration-extra` (default)
  - Enables deeper Apple metadata enrichment using `SystemConfiguration` APIs.
  - On Apple targets, this adds metadata such as interface display names, DHCP hints, and iOS DNS resolver lookup when the platform exposes them.
- `android-extra` (default)
  - Enables deeper Android metadata enrichment using Android platform APIs through JNI bindings.
  - On Android, this can add metadata such as traffic stats, DNS servers, DHCP hints, and Wi-Fi link speed when the app provides the required Android context and permissions.

To opt out of the additional Apple metadata enrichment while keeping gateway helpers:

```toml
[dependencies]
netdev = { version = "0.45", default-features = false, features = ["gateway"] }
```

## Apple behavior
`netdev` links `SystemConfiguration.framework` automatically on `macOS` and `iOS` through its build script.
If your app is ultimately linked by Xcode, you may still need to add `SystemConfiguration.framework` to the app target manually.

## Android behavior
If you want Android-specific values such as DNS servers, DHCP hints, or Wi-Fi link speed, your app may still need to initialize the Android context for Rust and declare Android permissions such as `ACCESS_NETWORK_STATE` and `ACCESS_WIFI_STATE`.

## Project History
This crate was originally published as [default-net][default-net-crates-io-url]
and was later rebranded to `netdev` by the author for future expansion, clearer naming, and long-term maintenance.

## Tested on
- Linux
    - Ubuntu 
        - 24.04
        - 22.04
        - 21.10 
        - 20.04 
        - 18.04
    - Kali 
        - 2024.2
        - 2023.2
        - 2022.1
        - 2021.1
    - Arch 
        - 2024.05.01
- macOS (Apple Silicon)
    - 26.3.1
    - 14.7.6
- macOS (Intel)
    - 13.4.1
    - 11.6
- Windows 
    - 11 25H2 26200.8037
    - 11 24H2 26100.6584
    - 11 23H2 22631.4602
    - 11 Pro 22H2 22621.3155
    - 11 22H2 22621.3155
    - 10 21H2 19044.1586
- FreeBSD
    - 14
- Android (arm64)
    - 16.0
- Android (x86_64)
    - 16.0
- iOS
    - 18.6.2
    - 18.1.1
