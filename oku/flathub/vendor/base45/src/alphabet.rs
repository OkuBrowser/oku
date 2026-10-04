/// Table of characters (by index). Currently unused.
pub const TABLE: [u8; 45] = *b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:";

/// Decode a byte to its index.
pub const fn decode(v: u8) -> Option<u8> {
    // table statements are fast, but use more space. Enabled by default.
    Some(match v {
        b'0' => 0,
        b'1' => 1,
        b'2' => 2,
        b'3' => 3,
        b'4' => 4,
        b'5' => 5,
        b'6' => 6,
        b'7' => 7,
        b'8' => 8,
        b'9' => 9,
        b'A' => 10,
        b'B' => 11,
        b'C' => 12,
        b'D' => 13,
        b'E' => 14,
        b'F' => 15,
        b'G' => 16,
        b'H' => 17,
        b'I' => 18,
        b'J' => 19,
        b'K' => 20,
        b'L' => 21,
        b'M' => 22,
        b'N' => 23,
        b'O' => 24,
        b'P' => 25,
        b'Q' => 26,
        b'R' => 27,
        b'S' => 28,
        b'T' => 29,
        b'U' => 30,
        b'V' => 31,
        b'W' => 32,
        b'X' => 33,
        b'Y' => 34,
        b'Z' => 35,
        b' ' => 36,
        b'$' => 37,
        b'%' => 38,
        b'*' => 39,
        b'+' => 40,
        b'-' => 41,
        b'.' => 42,
        b'/' => 43,
        b':' => 44,
        _ => return None,
    })
}

pub fn encode(n: u8) -> Option<u8> {
    //#[cfg(feature = "bin_table")]
    TABLE.get(n as usize).copied()
    // very fast, literal index pointer
}

pub const SIZE: u32 = 45;
pub const SIZE_SIZE: u32 = SIZE * SIZE;
