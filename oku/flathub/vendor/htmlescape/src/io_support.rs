use std::io::{Write, Read, Error, ErrorKind};
use std::io;
use std::fmt;

pub fn write_char<W: Write>(writer: &mut W, c: char) -> io::Result<()> {
    let mut buf = [0u8;4];
    let utf8 = encode_char_utf8(c, &mut buf);
    writer.write_all(utf8)
}

fn encode_char_utf8<'a>(c: char, buf: &'a mut [u8]) -> &'a [u8] {
    let c = c as u32;
    if c <= 0x7f {
        buf[0] = c as u8;
        &buf[..1]
    } else if c <= 0x7ff {
        buf[1] = 0b10000000 | (c & 0b00111111) as u8;
        buf[0] = 0b11000000 | ((c >> 6) & 0b00011111) as u8;
        &buf[..2]
    } else if c <= 0xffff {
        buf[2] = 0b10000000 | (c & 0b00111111) as u8;
        buf[1] = 0b10000000 | ((c >> 6) & 0b00111111) as u8;
        buf[0] = 0b11100000 | ((c >> 12) & 0b00001111) as u8;
        &buf[..3]
    } else {
        buf[3] = 0b10000000 | (c & 0b00111111) as u8;
        buf[2] = 0b10000000 | ((c >> 6) & 0b00111111) as u8;
        buf[1] = 0b10000000 | ((c >> 12) & 0b00111111) as u8;
        buf[0] = 0b11110000 | ((c >> 18) & 0b00000111) as u8;
        &buf[..4]
    }
}

fn utf8_char_bytes(first: u8) -> usize {
    if first & 0b10000000 == 0 {
        1
    } else if first & 0b11100000 == 0b11000000 {
        2
    } else if first & 0b11110000 == 0b11100000 {
        3
    } else if first & 0b11111000 == 0b11110000 {
        4
    } else {
        0
    }
}


// `Chars` code copied and modified from `std`
// The reason for doing this is that when using Chars from std, read_one_byte() isn't inlined.
// This is very painful when the backing Reader is just a Cursor over a byte array,
//  and read_one_byte() should be much more than return buf[idx++] (+ check for end ofc.).

pub struct Chars<R> {
    inner: R
}

pub fn chars<R: Read>(reader: R) -> Chars<R> {
    Chars { inner: reader }
}

#[derive(Debug)]
pub enum CharsError {
    /// Variant representing that the underlying stream was read successfully
    /// but it did not contain valid utf8 data.
    NotUtf8,

    /// Variant representing that an I/O error occurred.
    Other(Error),
}

impl<R: Read> Iterator for Chars<R> {
    type Item = Result<char, CharsError>;

    fn next(&mut self) -> Option<Result<char, CharsError>> {
        let first_byte = match read_a_byte(&mut self.inner) {
            None => return None,
            Some(Ok(b)) => b,
            Some(Err(e)) => return Some(Err(CharsError::Other(e))),
        };
        let width = utf8_char_bytes(first_byte);
        if width == 1 { return Some(Ok(first_byte as char)) }
        if width == 0 { return Some(Err(CharsError::NotUtf8)) }
        let mut buf = [first_byte, 0, 0, 0];
        {
            let mut start = 1;
            while start < width {
                match self.inner.read(&mut buf[start..width]) {
                    Ok(0) => return Some(Err(CharsError::NotUtf8)),
                    Ok(n) => start += n,
                    Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
                    Err(e) => return Some(Err(CharsError::Other(e))),
                }
            }
        }
        Some(match ::std::str::from_utf8(&buf[..width]).ok() {
            Some(s) => Ok(s.chars().next().unwrap()),
            None => Err(CharsError::NotUtf8),
        })
    }
}

fn read_a_byte<R: Read>(reader: &mut R) -> Option<io::Result<u8>> {
    let mut buf = [0];
    loop {
        return match reader.read(&mut buf) {
            Ok(0) => None,
            Ok(..) => Some(Ok(buf[0])),
            Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(e) => Some(Err(e)),
        };
    }
}

impl fmt::Display for CharsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            CharsError::NotUtf8 => {
                "byte stream did not contain valid utf8".fmt(f)
            }
            CharsError::Other(ref e) => e.fmt(f),
        }
    }
}

#[cfg(test)]
mod test {

    use super::encode_char_utf8;

    #[test]
    fn test_encode_char_utf8() {
        do_test_encode_char_utf8('$', &[0x24]);
        do_test_encode_char_utf8('¢', &[0xc2, 0xa2]);
        do_test_encode_char_utf8('€', &[0xe2, 0x82, 0xac]);
        do_test_encode_char_utf8('\u{10348}', &[0xf0, 0x90, 0x8d, 0x88]);
    }

    fn do_test_encode_char_utf8(c: char, expected: &[u8]) {
        let mut buf = [0u8;4];
        let utf8 = encode_char_utf8(c, &mut buf);
        assert_eq!(utf8, expected);
    }
}
