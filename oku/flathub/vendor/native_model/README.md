# Native model

[![Crates.io](https://img.shields.io/crates/v/native_model)](https://crates.io/crates/native_model)
[![Build Test Release](https://github.com/vincent-herlemont/native_model/actions/workflows/build_and_test_release.yml/badge.svg)](https://github.com/vincent-herlemont/native_model/actions/workflows/build_and_test_release.yml)
[![Documentation](https://docs.rs/native_model/badge.svg)](https://docs.rs/native_model)
[![License](https://img.shields.io/crates/l/native_model)](LICENSE)

Add interoperability                  on the top  of   serialization formats like bincode, postcard etc.

See [concepts](#concepts) for more details.

## Goals

- **Interoperability**: Allows different applications to work together, even if they are using different 
  versions of the data model.
- **Data Consistency**: Ensure that we process the data expected model.
- **Flexibility**: You can use any serialization format you want. More details [here](#setup-your-serialization-format).
- **Performance**: A minimal overhead (encode: ~20 ns, decode: ~40 ps). More details [here](#performance).

## Usage

```
       Application 1 (DotV1)        Application 2 (DotV1 and DotV2)
                |                                  |
   Encode DotV1 |--------------------------------> | Decode DotV1 to DotV2
                |                                  | Modify DotV2
   Decode DotV1 | <--------------------------------| Encode DotV2 back to DotV1
                |                                  |
```


```rust,skt-main
// Application 1
let dot = DotV1(1, 2);
let bytes = native_model::encode(&dot).unwrap();

// Application 1 sends bytes to Application 2.

// Application 2
// We are able to decode the bytes directly into a new type DotV2 (upgrade).
let (mut dot, source_version) = native_model::decode::<DotV2>(bytes).unwrap();
assert_eq!(dot, DotV2 { 
    name: "".to_string(), 
    x: 1, 
    y: 2 
});
dot.name = "Dot".to_string();
dot.x = 5;
// For interoperability, we encode the data with the version compatible with Application 1 (downgrade).
let bytes = native_model::encode_downgrade(dot, source_version).unwrap();

// Application 2 sends bytes to Application 1.

// Application 1
let (dot, _) = native_model::decode::<DotV1>(bytes).unwrap();
assert_eq!(dot, DotV1(5, 2));
 ```

 - Full example [here](./tests_crate/tests/example/example_main.rs).

## Serialization format

You can use  default serialization formats via  the feature flags, like:

```toml
[dependencies]
native_model = { version = "0.1", features = ["bincode_2_rc"] }
```

Each feature flag corresponds to a specific minor version of the serialization format. In order to avoid breaking
changes, the default serialization format is the oldest one.

- `bincode_1_3`: [bincode](https://docs.rs/bincode/1.3.3/bincode/) v1.3 (default)
- `bincode_2_rc`: [bincode](https://docs.rs/bincode/2.0.0-rc.3/bincode/) v2.0.0-rc3
- `postcard_1_0`: [postcard](https://docs.rs/postcard/1.0.0/postcard/) v1.0

### Custom serialization format

Define a struct with the name you want. This struct must implement [`native_model::Encode`](https://docs.rs/native_model/latest/native_model/trait.Encode.html) and [`native_model::Decode`](https://docs.rs/native_model/latest/native_model/trait.Decode.html) traits.

Full examples: 
- [bincode with encode/decode](./tests_crate/tests/example/custom_codec/bincode.rs)
- [bincode with serde](./tests_crate/tests/example/custom_codec/bincode_serde.rs)

Others examples,  see the default implementations:
- [bincode v1.3](./src/codec/bincode_1_3.rs)
-  [bincode v2.0 (rc)](./src/codec/bincode_2_rc.rs)
-  [postcard v1.0](./src/codec/postcard_1_0.rs)

## Data model

Define your model using the macro [`native_model`](file:///home/vincentherlemont/IdeaProjects/native_model/target/doc/native_model/attr.native_model.html).

Attributes:
- `id = u32`: The unique identifier of the model.
- `version = u32`: The version of the model.
- `with = type`: The serialization format that you use for the Encode/Decode implementation. Setup [here](#setup-your-serialization-format).
- `from = type`: Optional, the previous version of the model.
    - `type`: The previous version of the model that you use for the From implementation.
- `try_from = (type, error)`: Optional, the previous version of the model with error handling.
    - `type`: The previous version of the model that you use for the TryFrom implementation.
    - `error`: The error type that you use for the TryFrom implementation.

```rust,skt-define-models
use native_model::native_model;

#[derive(Deserialize, Serialize, PartialEq, Debug)]
#[native_model(id = 1, version = 1)]
struct DotV1(u32, u32);

#[derive(Deserialize, Serialize, PartialEq, Debug)]
#[native_model(id = 1, version = 2, from = DotV1)]
struct DotV2 {
    name: String,
    x: u64,
    y: u64,
}

// Implement the conversion between versions From<DotV1> for DotV2 and From<DotV2> for DotV1.

#[derive(Deserialize, Serialize, PartialEq, Debug)]
#[native_model(id = 1, version = 3, try_from = (DotV2, anyhow::Error))]
struct DotV3 {
    name: String,
    cord: Cord,
}

#[derive(Deserialize, Serialize, PartialEq, Debug)]
struct Cord {
    x: u64,
    y: u64,
}

// Implement the conversion between versions From<DotV2> for DotV3 and From<DotV3> for DotV2.
```

## Status

Early development. Not ready for production.

## Concepts

In order to understand how the native model works, you need to understand the following concepts.

- **Identity**(`id`): The identity is the unique identifier of the model. It is used to identify the model and 
  prevent to decode a model into the wrong Rust type.
- **Version**(`version`) The version is the version of the model. It is used to check the compatibility between two 
  models.
- **Encode**: The encode is the process of converting a model into a byte array.
- **Decode**: The decode is the process of converting a byte array into a model.
- **Downgrade**: The downgrade is the process of converting a model into a previous version of the model.
- **Upgrade**: The upgrade is the process of converting a model into a newer version of the model.

Under the hood, the native model is a thin wrapper around serialized data. The `id` and the `version` are twice encoded with a [`little_endian::U32`](https://docs.rs/zerocopy/latest/zerocopy/byteorder/little_endian/type.U32.html). That represents 8 bytes, that are added at the beginning of the data.

```
+------------------+------------------+------------------------------------+
|     ID (4 bytes) | Version (4 bytes)| Data (indeterminate-length bytes)  |
+------------------+------------------+------------------------------------+
```

Full example [here](tests/example/example_define_model.rs).

## Performance

Native model has
been designed to have a minimal and constant overhead. That means that the overhead is the same
whatever the size of the data. Under the hood we use the [zerocopy](https://docs.rs/zerocopy/latest/zerocopy/) crate 
to avoid unnecessary copies.

👉 To know the total time of the encode/decode, you need to add the time of your serialization format.

Resume:
- **Encode**: ~20 ns
- **Decode**: ~40 ps

|      data size       |   encode time (ns)    | decode time (ps)        |
|:--------------------:|:---------------------:|:-----------------------:|
|         1 B          | 19.769 ns - 20.154 ns | 40.526 ps - 40.617 ps   |
|        1 KiB         | 19.597 ns - 19.971 ns | 40.534 ps - 40.633 ps   |
|        1 MiB         | 19.662 ns - 19.910 ns | 40.508 ps - 40.632 ps   |
|        10 MiB        | 19.591 ns - 19.980 ns | 40.504 ps - 40.605 ps   |
|       100 MiB        | 19.669 ns - 19.867 ns | 40.520 ps - 40.644 ps   |

Benchmark of the native model overhead [here](benches/overhead.rs).

