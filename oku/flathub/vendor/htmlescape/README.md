# A HTML entity encoding library for Rust
[![Build Status](https://travis-ci.org/veddan/rust-htmlescape.png?branch=master)](https://travis-ci.org/veddan/rust-htmlescape)

## Example usage
All example assume a `extern crate htmlescape;` and `use htmlescape::{relevant functions here};` is present.

###Encoding
`htmlescape::encode_minimal()` encodes an input string using a minimal set of HTML entities.

```rust
let title = "Cats & dogs";
let tag = format!("<title>{}</title>", encode_minimal(title));
assert_eq!(tag.as_slice(), "<title>Cats &amp; dogs</title>");
```

There is also a `htmlescape::encode_attribute()` function for encoding strings that are to be used
as html attribute values.

###Decoding
`htmlescape::decode_html()` decodes an encoded string, replacing HTML entities with the
corresponding characters. Named, hex, and decimal entities are supported. A `Result` value is
returned, with either the decoded string in `Ok`, or an error in `Err`.

```rust
let encoded = "Cats&#x20;&amp;&#32;dogs";
let decoded = match decode_html(encoded) {
  Err(reason) => panic!("Error {:?} at character {}", reason.kind, reason.position),
  Ok(s) => s
};
assert_eq!(decoded.as_slice(), "Cats & dogs");
```

###Avoiding allocations
Both the encoding and decoding functions are available in forms that take a `Writer` for output rather
than returning an `String`. These version can be used to avoid allocation and copying if the returned
`String` was just going to be written to a `Writer` anyway.
