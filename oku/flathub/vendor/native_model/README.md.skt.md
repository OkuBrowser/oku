```rust,skt-main
use native_model_macro::native_model;
use serde::{{Deserialize, Serialize}};

#[derive(Deserialize, Serialize, PartialEq, Debug)]
#[native_model(id = 1, version = 1)]
struct DotV1(u32, u32);

#[derive(Deserialize, Serialize, PartialEq, Debug)]
#[native_model(id = 1, version = 2, from = DotV1)]
struct DotV2 {{
    name: String,
    x: u64,
    y: u64,
}}

impl From<DotV1> for DotV2 {{
    fn from(dot: DotV1) -> Self {{
        DotV2 {{
            name: "".to_string(),
            x: dot.0 as u64,
            y: dot.1 as u64,
        }}
    }}
}}

impl From<DotV2> for DotV1 {{
    fn from(dot: DotV2) -> Self {{
        DotV1(dot.x as u32, dot.y as u32)
    }}
}}


fn main() {{
    {}
}}
```

```rust,skt-define-models
use serde::{{Deserialize, Serialize}};

{}

impl From<DotV1> for DotV2 {{
    fn from(dot: DotV1) -> Self {{
        DotV2 {{
            name: "".to_string(),
            x: dot.0 as u64,
            y: dot.1 as u64,
        }}
    }}
}}

impl From<DotV2> for DotV1 {{
    fn from(dot: DotV2) -> Self {{
        DotV1(dot.x as u32, dot.y as u32)
    }}
}}

impl TryFrom<DotV2> for DotV3 {{
    type Error = anyhow::Error;

    fn try_from(dot: DotV2) -> Result<Self, Self::Error> {{
        Ok(DotV3 {{
            name: dot.name,
            cord: Cord {{ x: dot.x, y: dot.y }},
        }})
    }}
}}

impl TryFrom<DotV3> for DotV2 {{
    type Error = anyhow::Error;

    fn try_from(dot: DotV3) -> Result<Self, Self::Error> {{
        Ok(DotV2 {{
            name: dot.name,
            x: dot.cord.x,
            y: dot.cord.y,
        }})
    }}
}}



fn main() {{
    let dot = DotV1(1, 2);
    let bytes = native_model::encode(&dot).unwrap();

    let (dot_decoded, _) = native_model::decode::<DotV1>(bytes.clone()).unwrap();
    assert_eq!(dot, dot_decoded);

    let (dot_decoded, _) = native_model::decode::<DotV2>(bytes.clone()).unwrap();
    assert_eq!(
        DotV2 {{
            name: "".to_string(),
            x: 1,
            y: 2
        }},
        dot_decoded
    );

    let (dot_decoded, _) = native_model::decode::<DotV3>(bytes.clone()).unwrap();
    assert_eq!(
        DotV3 {{
            name: "".to_string(),
            cord: Cord {{ x: 1, y: 2 }}
        }},
        dot_decoded
    );
}}

```

```rust,skt-define-serilization-format
{}

fn main() {{
}}
```