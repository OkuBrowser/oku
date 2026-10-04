use std::io::{self, Write, BufRead, Cursor};
use std::char;
use self::DecodeState::*;
use self::DecodeErrKind::*;
use io_support::{self, write_char, CharsError};
use entities::*;

#[derive(Debug)]
pub enum DecodeErrKind {
    /// A non-existent named entity was referenced.
    /// Example: &thisentitydoesnotexist
    UnknownEntity,

    /// A numerical escape sequence (&# or &#x) containing an invalid character.
    /// Examples: `&#32a`, `&#xfoo`
    MalformedNumEscape,

    /// A numerical escape sequence (&# or &#x) resolved to an invalid unicode code point.
    /// Example: `&#xffffff`
    InvalidCharacter,

    /// The input ended prematurely (ie. inside an unterminated named entity sequence).
    PrematureEnd,

    /// An IO error occured.
    IoError(io::Error),

    /// The supplied Reader produces invalid UTF-8.
    EncodingError,
}

impl PartialEq for DecodeErrKind {
    fn eq(&self, other: &DecodeErrKind) -> bool {
        match (self, other) {
            (&UnknownEntity, &UnknownEntity) => true,
            (&MalformedNumEscape, &MalformedNumEscape) => true,
            (&InvalidCharacter, &InvalidCharacter) => true,
            (&PrematureEnd, &PrematureEnd) => true,
            (&IoError(_), &IoError(_)) => true,
            (&EncodingError, &EncodingError) => true,
            _ => false
        }
    }
}

impl Eq for DecodeErrKind {}

/// Error from decoding a entity-encoded string.
#[derive(Debug, Eq, PartialEq)]
pub struct DecodeErr {
    /// Number of characters read from the input before encountering an error
    pub position: usize,
    /// Type of error
    pub kind: DecodeErrKind
}

#[derive(PartialEq, Eq)]
enum DecodeState {
    Normal,
    Entity,
    Named,
    Numeric,
    Hex,
    Dec
}

macro_rules! try_parse(
    ($parse:expr, $pos:expr) => (
        match $parse {
            Err(reason) => return Err(DecodeErr{ position: $pos, kind: reason}),
            Ok(res) => res
        }
    ););

macro_rules! try_dec_io(
    ($io:expr, $pos:expr) => (
        match $io {
            Err(e) => return Err(DecodeErr{ position: $pos, kind: IoError(e)}),
            Ok(res) => res
        }
    ););

/// Decodes an entity-encoded string from a reader to a writer.
///
/// Similar to `decode_html`, except reading from a reader rather than a string, and
/// writing to a writer rather than returning a `String`.
///
/// # Arguments
/// - `reader` - UTF-8 encoded data is read from here.
/// - `writer` - UTF8- decoded data is written to here.
///
/// # Errors
/// Errors can be caused by IO errors, `reader` producing invalid UTF-8, or by syntax errors.
pub fn decode_html_rw<R: BufRead, W: Write>(reader: R, writer: &mut W) -> Result<(), DecodeErr> {
    let mut state: DecodeState = Normal;
    let mut pos = 0;
    let mut good_pos = 0;
    let mut buf = String::with_capacity(8);
    for c in io_support::chars(reader) {
        let c = match c {
            Err(e) => {
                let kind = match e {
                    CharsError::NotUtf8   => EncodingError,
                    CharsError::Other(io) => IoError(io)
                };
                return Err(DecodeErr{ position: pos, kind: kind });
            }
            Ok(c) => c
        };
        match state {
            Normal if c == '&' => state = Entity,
            Normal => try_dec_io!(write_char(writer, c), good_pos),
            Entity if c == '#' => state = Numeric,
            Entity if c == ';' => return Err(DecodeErr{ position: good_pos, kind: UnknownEntity }),
            Entity => {
                state = Named;
                buf.push(c);
            }
            Named if c == ';' => {
                state = Normal;
                let ch = try_parse!(decode_named_entity(&buf), good_pos);
                try_dec_io!(write_char(writer, ch), good_pos);
                buf.clear();
            }
            Named => buf.push(c),
            Numeric if is_digit(c) => {
                state = Dec;
                buf.push(c);
            }
            Numeric if c == 'x' => state = Hex,
            Dec if c == ';' => {
                state = Normal;
                let ch = try_parse!(decode_numeric(&buf, 10), good_pos);
                try_dec_io!(write_char(writer, ch), good_pos);
                buf.clear();
            }
            Hex if c == ';' => {
                state = Normal;
                let ch = try_parse!(decode_numeric(&buf, 16), good_pos);
                try_dec_io!(write_char(writer, ch), good_pos);
                buf.clear();
            }
            Hex if is_hex_digit(c) => buf.push(c),
            Dec if is_digit(c) => buf.push(c),
            Numeric | Hex | Dec => return Err(DecodeErr{ position: good_pos, kind: MalformedNumEscape}),
        }
        pos += 1;
        if state == Normal {
            good_pos = pos;
        }
    }
    if state != Normal {
        Err(DecodeErr{ position: good_pos, kind: PrematureEnd})
    } else {
        Ok(())
    }
}

/// Decodes an entity-encoded string.
///
/// Decodes an entity encoded string, replacing HTML entities (`&amp;`, `&#20;` ...) with the
/// the corresponding character. Case matters for named entities, ie. `&Amp;` is invalid.
/// Case does not matter for hex entities, so `&#x2E;` and `&#x2e;` are treated the same.
///
/// # Arguments
/// - `s` - Entity-encoded string to decode.
///
/// # Failure
/// The function will fail if input string contains invalid named entities (eg. `&nosuchentity;`),
/// invalid hex entities (eg. `&#xRT;`), invalid decimal entities (eg. `&#-1;), unclosed entities
/// (`s == "&amp hej och hå"`) or otherwise malformed entities.
///
/// This function will never return errors with `kind` set to `IoError` or `EncodingError`.
pub fn decode_html(s: &str) -> Result<String, DecodeErr> {
    let mut writer = Vec::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut reader = Cursor::new(bytes);
    let res = decode_html_rw(&mut reader, &mut writer);
    match res {
        Ok(_) => Ok(String::from_utf8(writer).unwrap()),
        Err(err) => Err(err)
    }
}

fn is_digit(c: char) -> bool { c >= '0' && c <= '9' }

fn is_hex_digit(c: char) -> bool {
    is_digit(c) || (c >= 'a' && c <= 'f') || (c >= 'A' && c <= 'F')
}

fn decode_named_entity(entity: &str) -> Result<char, DecodeErrKind> {
    match NAMED_ENTITIES.binary_search_by(|&(ent, _)| ent.cmp(entity)) {
        Err(..) => Err(UnknownEntity),
        Ok(idx) => {
            let (_, c) = NAMED_ENTITIES[idx];
            Ok(c)
        }
    }
}

fn decode_numeric(esc: &str, radix: u32) -> Result<char, DecodeErrKind> {
    match u32::from_str_radix(esc, radix) {
        Ok(n) => match char::from_u32(n) {
            Some(c) => Ok(c),
            None => Err(InvalidCharacter)
        },
        Err(..) => Err(MalformedNumEscape)
    }
}

