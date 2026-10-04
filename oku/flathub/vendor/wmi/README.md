# wmi

[![check](https://github.com/ohadravid/wmi-rs/actions/workflows/check.yml/badge.svg?branch=main)](https://github.com/ohadravid/wmi-rs/actions/workflows/check.yml)
[![crates.io](https://img.shields.io/crates/v/wmi.svg)](https://crates.io/crates/wmi)
[![docs.rs](https://docs.rs/wmi/badge.svg)](https://docs.rs/crate/wmi)

WMI (Windows Management Instrumentation) crate for rust.

```toml
# Cargo.toml
[dependencies]
wmi = "0.18"
```

## Examples

Queries can be deserialized into a free-form `HashMap` or a `struct`:

```rust
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use serde::Deserialize;
use wmi::{Variant, WMIConnection, WMIDateTime};
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let wmi_con = WMIConnection::new()?;

    let results: Vec<HashMap<String, Variant>> = wmi_con.raw_query("SELECT * FROM Win32_OperatingSystem")?;

    for os in results {
        println!("{:#?}", os);
    }

    #[derive(Deserialize, Debug)]
    struct Win32_OperatingSystem {
        Caption: String,
        Name: String,
        CurrentTimeZone: i16,
        Debug: bool,
        EncryptionLevel: u32,
        ForegroundApplicationBoost: u8,
        LastBootUpTime: WMIDateTime,
    }

    let results: Vec<Win32_OperatingSystem> = wmi_con.query()?;

    for os in results {
        println!("{:#?}", os);
    }

    Ok(())
}
```

### `chrono` vs `time`

If you prefer to use the `time` crate instead of the default `chrono`, include `wmi` as

```toml
[dependencies]
wmi-rs = { version = "*", default-features = false, features = ["time"] }
```

and use the `WMIOffsetDateTime` wrapper instead of the `WMIDateTime` wrapper.

## Async Queries

WMI supports async queries, with methods
like [ExecAsyncQuery](https://docs.microsoft.com/en-us/windows/win32/api/wbemcli/nf-wbemcli-iwbemservices-execqueryasync).

```rust
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use serde::Deserialize;
use wmi::{Variant, WMIConnection, WMIDateTime};
use std::collections::HashMap;
use futures::executor::block_on;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let wmi_con = WMIConnection::new()?;

    block_on(exec_async_query(&wmi_con))?;

    Ok(())
}

async fn exec_async_query(wmi_con: &WMIConnection) -> Result<(), Box<dyn std::error::Error>> {
    let results: Vec<HashMap<String, Variant>> =
        wmi_con.async_raw_query("SELECT * FROM Win32_OperatingSystem").await?;

    for os in results {
        println!("{:#?}", os);
    }

    #[derive(Deserialize, Debug)]
    struct Win32_OperatingSystem {
        Caption: String,
        Name: String,
        CurrentTimeZone: i16,
        Debug: bool,
        EncryptionLevel: u32,
        ForegroundApplicationBoost: u8,
        LastBootUpTime: WMIDateTime,
    }

    let results: Vec<Win32_OperatingSystem> = wmi_con.async_query().await?;

    for os in results {
        println!("{:#?}", os);
    }

    Ok(())
}
```

## Custom Authentication Levels

Some WMI namespaces require specific authentication levels when accessing
security-sensitive information. For example, BitLocker encryption status requires
packet-level encryption (`RPC_C_AUTHN_LEVEL_PKT_PRIVACY`) to protect cryptographic
data during transmission.

Use `set_proxy_blanket()` to set authentication requirements:

```rust,no_run
use wmi::{AuthLevel, WMIConnection};
use serde::Deserialize;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to BitLocker namespace with packet privacy authentication
    let wmi_con = WMIConnection::with_namespace_path(
        "ROOT\\CIMV2\\Security\\MicrosoftVolumeEncryption"
    )?;
    wmi_con.set_proxy_blanket(AuthLevel::PktPrivacy)?;

    #[derive(Deserialize, Debug)]
    #[serde(rename = "Win32_EncryptableVolume")]
    #[serde(rename_all = "PascalCase")]
    struct EncryptableVolume {
        device_id: String,
        drive_letter: Option<String>,
        protection_status: Option<u32>,  // 0=Unprotected, 1=Protected, 2=Unknown
    }

    let volumes: Vec<EncryptableVolume> = wmi_con.query()?;

    for volume in volumes {
        println!("Drive: {:?}, Protection: {:?}", volume.drive_letter, volume.protection_status);
    }

    Ok(())
}
```

**Note**: Querying BitLocker requires administrator privileges. The authentication
level ensures the query data is encrypted during transmission.

## License

The `wmi` crate is licensed under either of

```text
Apache License, Version 2.0, (LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0)
MIT license (LICENSE-MIT or https://opensource.org/licenses/MIT)
```

at your option.
