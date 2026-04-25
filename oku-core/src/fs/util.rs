use bytes::Bytes;
use iroh_docs::DocTicket;
use log::error;
use miette::IntoDiagnostic;
use path_clean::PathClean;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use std::ffi::CString;
use std::path::PathBuf;

/// Cleans a path and ensures it begins with the root.
///
/// # Arguments
///
/// * `path` - The path to normalise.
///
/// # Returns
///
/// The given path, prefixed with `/` if missing, and `.` & `..` components processed.
pub fn normalise_path(path: &PathBuf) -> PathBuf {
    PathBuf::from("/").join(path).clean()
}

/// Converts a path to a key for an entry in a file system replica.
///
/// # Arguments
///
/// * `path` - The path to convert to a key.
///
/// # Returns
///
/// A null-terminated byte string representing the path.
pub fn path_to_entry_key(path: &PathBuf) -> Bytes {
    let path = normalise_path(path);
    let mut path_bytes = path.into_os_string().into_encoded_bytes();
    path_bytes.push(b'\0');
    path_bytes.into()
}

/// Converts a key of a replica entry into a path within a replica.
///
/// # Arguments
///
/// * `key` - The replica entry key, being a null-terminated byte string.
///
/// # Returns
///
/// A path pointing to the file with the key.
pub fn entry_key_to_path(key: &[u8]) -> miette::Result<PathBuf> {
    Ok(PathBuf::from(
        CString::from_vec_with_nul(key.to_vec())
            .into_diagnostic()?
            .into_string()
            .into_diagnostic()?,
    ))
}

/// Converts a path to a key prefix for entries in a file system replica.
///
/// # Arguments
///
/// * `path` - The path to convert to a key prefix.
///
/// # Returns
///
/// A byte string representing the path, without a null byte at the end.
pub fn path_to_entry_prefix(path: &PathBuf) -> Bytes {
    let path = normalise_path(path);
    let path_bytes = path.into_os_string().into_encoded_bytes();
    path_bytes.into()
}

/// Format bytes as a base32-encoded lowercase string.
///
/// # Arguments
///
/// * `bytes` - The bytes to encode.
///
/// # Return
///
/// The bytes encoded as a lowercase string, represented in base32.
pub fn fmt(bytes: impl AsRef<[u8]>) -> String {
    let mut text = data_encoding::BASE32_NOPAD.encode(bytes.as_ref());
    text.make_ascii_lowercase();
    text
}

/// Format first ten bytes of a byte list as a base32-encoded lowercase string.
///
/// # Arguments
///
/// * `bytes` - The byte list to encode.
///
/// # Return
///
/// The first ten bytes encoded as a lowercase string, represented in base32.
pub fn fmt_short(bytes: impl AsRef<[u8]>) -> String {
    let len = bytes.as_ref().len().min(10);
    let mut text = data_encoding::BASE32_NOPAD.encode(&bytes.as_ref()[..len]);
    text.make_ascii_lowercase();
    text
}

/// Parse a string as a base32-encoded byte array of length `N`.
///
/// # Arguments
///
/// * `input` - The string to parse.
///
/// # Returns
///
/// An array of bytes of length `N`.
pub fn parse_array<const N: usize>(input: &str) -> miette::Result<[u8; N]> {
    data_encoding::BASE32_NOPAD
        .decode(input.to_ascii_uppercase().as_bytes())
        .into_diagnostic()?
        .try_into()
        .map_err(|_| {
            miette::miette!(
                "Unable to parse {input} as a base32-encoded byte array of length {N} … "
            )
        })
}

/// Parse a string either as a hex-encoded or base32-encoded byte array of length `LEN`.
///
/// # Arguments
///
/// * `input` - The string to parse.
///
/// # Returns
///
/// An array of bytes of length `LEN`.
pub fn parse_array_hex_or_base32<const LEN: usize>(input: &str) -> miette::Result<[u8; LEN]> {
    let mut bytes = [0u8; LEN];
    if input.len() == LEN * 2 {
        hex::decode_to_slice(input, &mut bytes).into_diagnostic()?;
        Ok(bytes)
    } else {
        Ok(parse_array(input)?)
    }
}

/// Merge multiple tickets into one, returning `None` if no tickets were given.
///
/// # Arguments
///
/// * `tickets` - A vector of tickets to merge.
///
/// # Returns
///
/// `None` if no tickets were given, or a ticket with a merged capability and merged list of nodes.
pub fn merge_tickets(tickets: &Vec<DocTicket>) -> Option<DocTicket> {
    let ticket_parts: Vec<_> = tickets
        .par_iter()
        .map(|ticket| ticket.capability.clone())
        .zip(tickets.par_iter().map(|ticket| ticket.nodes.clone()))
        .collect();
    ticket_parts
        .into_iter()
        .reduce(|mut merged_tickets, next_ticket| {
            if let Err(e) = merged_tickets.0.merge(next_ticket.0) {
                error!("{e}");
            }
            merged_tickets.1.extend_from_slice(&next_ticket.1);
            merged_tickets
        })
        .map(|mut merged_tickets| {
            merged_tickets.1.sort_unstable();
            merged_tickets.1.dedup();
            DocTicket {
                capability: merged_tickets.0,
                nodes: merged_tickets.1,
            }
        })
}
