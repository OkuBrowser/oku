use alloc::vec::Vec;
#[cfg(feature = "std")]
use std::io::{self, Write};

const HEX_DIGITS: [u8; 16] = *b"0123456789ABCDEF";

#[inline]
pub(crate) fn write_hex_to_vec(e: u8, output: &mut Vec<u8>) {
    output.reserve(6);

    let length = output.len();

    // SAFETY: 6 bytes have just been reserved, and the length is only updated after all of them are initialized.
    // Writing through a raw pointer never creates a reference to the uninitialized spare capacity.
    unsafe {
        let p = output.as_mut_ptr().add(length);

        p.write(b'&');
        p.add(1).write(b'#');
        p.add(2).write(b'x');
        p.add(3).write(HEX_DIGITS[(e >> 4) as usize]);
        p.add(4).write(HEX_DIGITS[(e & 0xF) as usize]);
        p.add(5).write(b';');

        output.set_len(length + 6);
    }
}

#[cfg(feature = "std")]
#[inline]
pub(crate) fn write_hex_to_writer<W: Write>(e: u8, output: &mut W) -> Result<(), io::Error> {
    let entity =
        [b'&', b'#', b'x', HEX_DIGITS[(e >> 4) as usize], HEX_DIGITS[(e & 0xF) as usize], b';'];

    output.write_all(&entity)
}

#[inline]
pub(crate) fn write_html_entity_to_vec(e: u8, output: &mut Vec<u8>) {
    match e {
        b'&' => output.extend_from_slice(b"&amp;"),
        b'<' => output.extend_from_slice(b"&lt;"),
        b'>' => output.extend_from_slice(b"&gt;"),
        b'"' => output.extend_from_slice(b"&quot;"),
        _ => write_hex_to_vec(e, output),
    }
}

#[cfg(feature = "std")]
#[inline]
pub(crate) fn write_html_entity_to_writer<W: Write>(
    e: u8,
    output: &mut W,
) -> Result<(), io::Error> {
    match e {
        b'&' => output.write_all(b"&amp;"),
        b'<' => output.write_all(b"&lt;"),
        b'>' => output.write_all(b"&gt;"),
        b'"' => output.write_all(b"&quot;"),
        _ => write_hex_to_writer(e, output),
    }
}

#[inline]
pub(crate) fn write_char_to_vec(c: char, output: &mut Vec<u8>) {
    match c.len_utf8() {
        1 => output.push(c as u8),
        _ => {
            let mut buffer = [0u8; 4];

            output.extend_from_slice(c.encode_utf8(&mut buffer).as_bytes());
        },
    }
}

#[cfg(feature = "std")]
#[inline]
pub(crate) fn write_char_to_writer<W: Write>(c: char, output: &mut W) -> Result<(), io::Error> {
    let mut buffer = [0u8; 4];
    let length = c.encode_utf8(&mut buffer).len();

    output.write_all(&buffer[..length])
}
