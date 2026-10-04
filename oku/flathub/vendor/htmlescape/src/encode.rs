use std::io::{self, Write};
use std::char;
use io_support::{write_char};
use entities::*;

///
/// HTML entity-encode a string.
///
/// Entity-encodes a string with a minimal set of entities:
///
/// - `" -- &quot;`
/// - `& -- &amp;`
/// - `' -- &#x27;`
/// - `< -- &lt;`
/// - `> -- &gt;`
///
/// # Arguments
/// - `s` - The string to encode.
///
/// # Return value
/// The encoded string.
///
/// # Example
/// ~~~
/// let encoded = htmlescape::encode_minimal("<em>Hej!</em>");
/// assert_eq!(&encoded, "&lt;em&gt;Hej!&lt;/em&gt;");
/// ~~~
///
/// # Safety notes
/// Using the function to encode an untrusted string that is to be used as a HTML attribute value
/// may lead to XSS vulnerabilities. Consider the following example:
///
/// ~~~
/// let name = "dummy onmouseover=alert(/XSS/)";    // User input
/// let tag = format!("<option value={}>", htmlescape::encode_minimal(name));
/// // Here `tag` is    "<option value=dummy onmouseover=alert(/XSS/)>"
/// ~~~
///
/// Use `escape_attribute` for escaping HTML attribute values.
pub fn encode_minimal(s: &str) -> String {
    let mut writer = Vec::with_capacity((s.len()/3 + 1) * 4);
    match encode_minimal_w(s, &mut writer) {
        Err(_) => panic!(),
        Ok(_) => String::from_utf8(writer).expect("impossible invalid UTF-8 in output")
    }
}

///
/// HTML entity-encode a string to a writer.
///
/// Similar to `encode_minimal`, except that the output is written to a writer rather
/// than returned as a `String`.
///
/// # Arguments
/// - `s` - The string to encode.
/// - `writer` - Output is written to here.
pub fn encode_minimal_w<W: Write>(s: &str, writer: &mut W) -> io::Result<()> {
    for c in s.chars() {
        match get_entity(c) {
            None => try!(write_char(writer, c)),
            Some(entity) => try!(writer.write_all(entity.as_bytes()))
        }
    }
    Ok(())
}

///
/// HTML entity-encodes a string for use in attributes values.
///
/// Entity-encodes a string using an extensive set of entities, giving a string suitable for use
/// in HTML attribute values. All entities from `encode_minimal` are used, and further, all
/// non-alphanumeric ASCII characters are hex-encoded (`&#x__;`).
/// See the [OWASP XSS Prevention Cheat Sheet](
/// https://www.owasp.org/index.php/XSS_(Cross_Site_Scripting)_Prevention_Cheat_Sheet) for more
/// information on entity-encoding for attribute values.
///
/// # Arguments
/// - `s` - The string to encode.
///
/// # Return value
/// The encoded string.
///
/// # Example
/// ~~~
/// let encoded = htmlescape::encode_attribute("\"No\", he said.");
/// assert_eq!(&encoded, "&quot;No&quot;&#x2C;&#x20;he&#x20;said&#x2E;");
/// ~~~
pub fn encode_attribute(s: &str) -> String {
    let mut writer = Vec::with_capacity(s.len() * 3);
    match encode_attribute_w(s, &mut writer) {
        Err(_) => panic!(),
        Ok(_) => String::from_utf8(writer).unwrap()
    }
}

///
/// HTML entity-encodes a string, for use in attributes values, to a writer.
///
/// Similar to `encode_attribute`, except that the output is written to a writer rather
/// than returned as a `String`.
///
/// # Arguments
/// - `s` - The string to encode.
/// - `writer` - Output is written to here.
pub fn encode_attribute_w<W: Write>(s: &str, writer: &mut W) -> io::Result<()> {
    for c in s.chars() {
        let b = c as usize;
        let res = match get_entity(c) {
            Some(entity) => writer.write_all(entity.as_bytes()),
            None =>
                if b < 256 && (b > 127 || !is_ascii_alnum(c)) {
                    write_hex(writer, c)
                } else {
                    write_char(writer, c)
                }
        };
        try!(res);
    }
    Ok(())
}

fn get_entity(c: char) -> Option<&'static str> {
    match MINIMAL_ENTITIES.binary_search_by(|&(ec, _)| ec.cmp(&c) ) {
        Err(..) => None,
        Ok(idx) => {
            let (_, e) = MINIMAL_ENTITIES[idx];
            Some(e)
        }
    }
}

fn write_hex<W: Write>(writer: &mut W, c: char) -> io::Result<()> {
    let hex = b"0123456789ABCDEF";
    try!(writer.write(b"&#x"));
    let n = c as u8;
    let bytes = [hex[((n & 0xF0) >> 4) as usize],
                 hex[(n & 0x0F) as usize],
                 b';'];
    writer.write_all(&bytes)
}

fn is_ascii_alnum(c: char) -> bool {
    (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9')
}

