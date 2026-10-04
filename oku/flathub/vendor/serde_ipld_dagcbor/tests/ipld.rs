use std::collections::BTreeMap;

use ipld_core::ipld::Ipld;
use serde_derive::{Deserialize, Serialize};
use serde_tuple::{Deserialize_tuple, Serialize_tuple};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct TupleStruct(String, i32, u64);

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct UnitStruct;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct Struct<'a> {
    tuple_struct: TupleStruct,
    tuple: (String, f32, f64),
    map: BTreeMap<String, String>,
    bytes: &'a [u8],
    array: Vec<String>,
}

use std::iter::FromIterator;

#[allow(clippy::useless_format)]
#[test]
fn serde() {
    let tuple_struct = TupleStruct(format!("test"), -60, 3000);

    let tuple = (format!("hello"), -50.004097, -12.094635556478);

    let map = BTreeMap::from_iter(
        [
            (format!("key1"), format!("value1")),
            (format!("key2"), format!("value2")),
            (format!("key3"), format!("value3")),
            (format!("key4"), format!("value4")),
        ]
        .iter()
        .cloned(),
    );

    let bytes = b"test byte string";

    let array = vec![format!("one"), format!("two"), format!("three")];

    let data = Struct {
        tuple_struct,
        tuple,
        map,
        bytes,
        array,
    };

    let ipld = ipld_core::serde::to_ipld(data.clone()).unwrap();
    println!("{:?}", ipld);

    let data_ser = serde_ipld_dagcbor::to_vec(&ipld).unwrap();
    let data_de_ipld: Ipld = serde_ipld_dagcbor::from_slice(&data_ser).unwrap();

    fn as_object(ipld: &Ipld) -> &BTreeMap<String, Ipld> {
        if let Ipld::Map(ref v) = ipld {
            return v;
        }
        panic!()
    }

    for ((k1, v1), (k2, v2)) in as_object(&ipld).iter().zip(as_object(&data_de_ipld).iter()) {
        assert_eq!(k1, k2);
        assert_eq!(v1, v2);
    }

    assert_eq!(ipld, data_de_ipld);
}

#[test]
fn unit_struct_not_supported() {
    let unit_array = vec![UnitStruct, UnitStruct, UnitStruct];
    let ipld = ipld_core::serde::to_ipld(unit_array);
    assert!(ipld.is_err());
}

#[derive(Debug, Deserialize, Serialize)]
struct SmallStruct {
    spam: u32,
    eggs: u32,
}

#[derive(Debug, PartialEq, Clone, Deserialize_tuple, Serialize_tuple)]
#[serde(deny_unknown_fields)]
struct StructWithTupleSerialization {
    spam: u32,
    eggs: u32,
    #[serde(default)]
    ham: String,
}

#[test]
fn struct_with_tuple_representation() {
    let st = StructWithTupleSerialization {
        spam: 42,
        eggs: 23,
        ham: "smoked".to_string(),
    };

    let ipld = ipld_core::serde::to_ipld(st.clone()).unwrap();
    println!("{:?}", ipld);

    let data_ser = serde_ipld_dagcbor::to_vec(&ipld).unwrap();
    println!("{:?}", data_ser);
    let data_de_ipld: Ipld = serde_ipld_dagcbor::from_slice(&data_ser).unwrap();

    let strt: StructWithTupleSerialization = ipld_core::serde::from_ipld(data_de_ipld).unwrap();
    assert_eq!(st, strt);

    let data_ser = vec![0x82, 0x17, 0x18, 0x2a]; // two element array [23,42]
    let data_de_ipld: Ipld = serde_ipld_dagcbor::from_slice(&data_ser).unwrap();
    let strt: StructWithTupleSerialization = ipld_core::serde::from_ipld(data_de_ipld).unwrap();
    assert_eq!(
        strt,
        StructWithTupleSerialization {
            spam: 23,
            eggs: 42,
            ham: "".to_string(),
        }
    );

    let data_ser = vec![0x84, 0x17, 0x18, 0x2a, 0x64, 0xf0, 0x9f, 0x91, 0x8c, 0xf4]; // [23,42,"👌",false]
    let data_de_ipld: Ipld = serde_ipld_dagcbor::from_slice(&data_ser).unwrap();
    let err =
        ipld_core::serde::from_ipld::<StructWithTupleSerialization>(data_de_ipld).unwrap_err();
    assert_eq!(
        err.to_string(),
        "serde error: The type failed to consume the entire sequence: 1 items remaining"
    );
}
