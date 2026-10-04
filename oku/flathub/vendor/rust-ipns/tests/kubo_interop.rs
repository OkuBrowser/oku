use libp2p_identity::PeerId;
use libp2p_identity::PublicKey;
use libp2p_identity::ed25519::PublicKey as Ed25519PublicKey;
use rust_ipns::Record;
use std::fs;

const PUBLIC_KEY_BYTES: [u8; 32] = [
    250, 155, 28, 240, 6, 251, 30, 167, 35, 45, 154, 154, 4, 215, 179, 84, 161, 68, 2, 178, 68, 47,
    140, 220, 75, 67, 224, 135, 51, 233, 181, 84,
];
const PUBLIC_KEY_BYTES_ZERO_SEQ: [u8; 32] = [
    9, 57, 103, 203, 179, 253, 77, 146, 11, 119, 113, 95, 23, 157, 189, 85, 95, 23, 181, 249, 48,
    226, 100, 124, 164, 163, 237, 105, 86, 156, 255, 180,
];

#[test]
fn parse_and_reconstruct_kubo_record() {
    // This IPNS record was created using Kubo (the Go IPFS node implementation)
    let original_record_bytes = fs::read("./tests/test.ipns").unwrap();
    let record = Record::decode(&original_record_bytes).unwrap();
    let public_key: PublicKey = Ed25519PublicKey::try_from_bytes(&PUBLIC_KEY_BYTES)
        .unwrap()
        .into();

    let peer_id = PeerId::from_public_key(&public_key);
    // these fixtures have a long-elapsed EOL, so full verify() rejects them as expired while the
    // signature/name binding still checks out.
    record.verify_signature(peer_id).unwrap();
    assert!(record.verify(peer_id).is_err());

    let new_record_bytes = record.encode().unwrap();
    assert_eq!(original_record_bytes, new_record_bytes);
}

#[test]
fn parse_and_reconstruct_kubo_record_seq_zero() {
    // This IPNS record was created using Kubo (the Go IPFS node implementation).
    // It contains a ZERO sequence value
    let original_record_bytes = fs::read("./tests/test_seq_zero.ipns").unwrap();
    let record = Record::decode(&original_record_bytes).unwrap();
    let public_key: PublicKey = Ed25519PublicKey::try_from_bytes(&PUBLIC_KEY_BYTES_ZERO_SEQ)
        .unwrap()
        .into();

    let peer_id = PeerId::from_public_key(&public_key);
    record.verify_signature(peer_id).unwrap();
    assert!(record.verify(peer_id).is_err());

    let new_record_bytes = record.encode().unwrap();
    assert_eq!(original_record_bytes, new_record_bytes);
}
