//! Dummy compile target for simple no-panic evaluation

use asn1_der::{DerObject, Sink};

fn main() {
    /// An ASN.1-DER encoded integer `7`
    const INT7: &'static [u8] = b"\x02\x01\x07";

    // Decode an arbitrary DER object
    let object = DerObject::decode(INT7).expect("Failed to decode object");

    // Encode an arbitrary DER object
    let (mut buf, mut buf_len) = ([0; 4], 0);
    let mut sink = buf.iter_mut().counting_sink(&mut buf_len);
    object.encode(&mut sink).expect("Failed to encode object");

    // Simple integer testing
    #[cfg(feature = "native_types")]
    {
        use asn1_der::typed::{DerDecodable, DerEncodable};

        // Decode a `u8`
        let number = u8::decode(INT7).expect("Failed to decode number");
        assert_eq!(number, 7);

        // Encode a new `u8`
        let (mut buf, mut buf_len) = ([0; 4], 0);
        let mut sink = buf.iter_mut().counting_sink(&mut buf_len);
        7u8.encode(&mut sink).expect("Failed to encode number");
    }
}
