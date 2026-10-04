use std::{collections::BTreeMap, iter};

use ipld_core::{
    cid::Cid,
    codec::{Codec, Links},
    ipld,
    ipld::Ipld,
};
use ipld_dagpb::{DagPbCodec, PbNode};

fn gen_ipld_node() -> Ipld {
    let cid = Cid::try_from("bafkreie74tgmnxqwojhtumgh5dzfj46gi4mynlfr7dmm7duwzyvnpw7h7m").unwrap();
    let pb_link = ipld!({
        "Hash": cid,
        "Name": "block",
        "Tsize": 13,
    });

    let links: Vec<Ipld> = vec![pb_link.clone(), pb_link];
    let mut pb_node = BTreeMap::<String, Ipld>::new();
    pb_node.insert("Data".to_string(), b"Here is some data\n".to_vec().into());
    pb_node.insert("Links".to_string(), links.into());
    pb_node.into()
}

#[test]
fn test_codec_ipld_encode_decode() {
    let ipld = gen_ipld_node();
    let bytes = DagPbCodec::encode_to_vec(&ipld).unwrap();
    let ipld2 = DagPbCodec::decode_from_slice(&bytes).unwrap();
    assert_eq!(ipld, ipld2);
}

#[test]
fn test_codec_pbnode_encode_decode() {
    let ipld = gen_ipld_node();
    let bytes = DagPbCodec::encode_to_vec(&ipld).unwrap();
    let pb_node: PbNode = DagPbCodec::decode_from_slice(&bytes).unwrap();
    let bytes2 = DagPbCodec::encode_to_vec(&pb_node).unwrap();
    assert_eq!(bytes, bytes2);
}

#[test]
fn test_codec_links() {
    let ipld = gen_ipld_node();
    let bytes = DagPbCodec::encode_to_vec(&ipld).unwrap();
    let links = DagPbCodec::links(&bytes).unwrap().collect::<Vec<_>>();
    let cid = Cid::try_from("bafkreie74tgmnxqwojhtumgh5dzfj46gi4mynlfr7dmm7duwzyvnpw7h7m").unwrap();
    let expected = iter::repeat_n(cid, 2).collect::<Vec<_>>();
    assert_eq!(links, expected);
}
