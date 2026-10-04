use std::iter;

use ipld_core::{
    cid::Cid,
    codec::{Codec, Links},
    ipld,
    ipld::Ipld,
};
use serde_ipld_dagjson::codec::DagJsonCodec;

#[test]
fn test_codec_encode() {
    let data = "hello world!".to_string();
    let expected = br#""hello world!""#;

    let encoded = DagJsonCodec::encode_to_vec(&data).unwrap();
    assert_eq!(encoded, expected);
}

#[test]
fn test_codec_decode() {
    let data = br#""hello world!""#;
    let expected = "hello world!".to_string();

    let decoded: String = DagJsonCodec::decode_from_slice(data).unwrap();
    assert_eq!(decoded, expected);
}

#[test]
fn test_codec_links() {
    let cid = Cid::try_from("bafkreibme22gw2h7y2h7tg2fhqotaqjucnbc24deqo72b6mkl2egezxhvy").unwrap();
    let data: Ipld = ipld!({"some": {"nested": cid}, "or": [cid, cid], "foo": true});
    let expected = iter::repeat_n(cid, 3).collect::<Vec<_>>();
    let encoded = DagJsonCodec::encode_to_vec(&data).unwrap();

    let links = DagJsonCodec::links(&encoded).unwrap().collect::<Vec<_>>();
    assert_eq!(links, expected);
}
