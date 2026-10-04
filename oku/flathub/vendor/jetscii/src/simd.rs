// # Warning
//
// Everything in this module assumes that the SSE 4.2 feature is available.

use std::{cmp::min, slice};

#[cfg(target_arch = "x86")]
use std::arch::x86 as target_arch;
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64 as target_arch;

use self::target_arch::{
    __m128i, _mm_cmpestri, _mm_cmpestrm, _mm_extract_epi16, _mm_loadu_si128,
    _SIDD_CMP_EQUAL_ORDERED,
};

include!(concat!(env!("OUT_DIR"), "/src/simd_macros.rs"));

const BYTES_PER_OPERATION: usize = 16;

union TransmuteToSimd {
    simd: __m128i,
    bytes: [u8; 16],
}

trait PackedCompareControl {
    fn needle(&self) -> __m128i;
    fn needle_len(&self) -> i32;
}

#[inline]
#[target_feature(enable = "sse4.2")]
unsafe fn find_small<C, const CONTROL_BYTE: i32>(packed: PackedCompare<C, CONTROL_BYTE>, haystack: &[u8]) -> Option<usize>
where
    C: PackedCompareControl,
{
    let mut tail = [0u8; 16];
    core::ptr::copy_nonoverlapping(haystack.as_ptr(), tail.as_mut_ptr(), haystack.len());
    let haystack = &tail[..haystack.len()];
    debug_assert!(haystack.len() < ::std::i32::MAX as usize);
    packed.cmpestri(haystack.as_ptr(), haystack.len() as i32)
}

/// The PCMPxSTRx instructions always read 16 bytes worth of
/// data. Although the instructions handle unaligned memory access
/// just fine, they might attempt to read off the end of a page
/// and into a protected area.
///
/// To handle this case, we read in 16-byte aligned chunks with
/// respect to the *end* of the byte slice. This makes the
/// complicated part in searching the leftover bytes at the
/// beginning of the byte slice.
#[inline]
#[target_feature(enable = "sse4.2")]
unsafe fn find<C, const CONTROL_BYTE: i32>(packed: PackedCompare<C, CONTROL_BYTE>, mut haystack: &[u8]) -> Option<usize>
where
    C: PackedCompareControl,
{
    // FIXME: EXPLAIN SAFETY

    if haystack.is_empty() {
        return None;
    }

    if haystack.len() < 16 {
        return find_small(packed, haystack);
    }

    let mut offset = 0;

    if let Some(misaligned) = Misalignment::new(haystack) {
        if let Some(location) = packed.cmpestrm(misaligned.leading, misaligned.leading_junk) {
            // Since the masking operation covers an entire
            // 16-byte chunk, we have to see if the match occurred
            // somewhere *after* our data
            if location < haystack.len() {
                return Some(location);
            }
        }

        haystack = &haystack[misaligned.bytes_until_alignment..];
        offset += misaligned.bytes_until_alignment;
    }

    // TODO: try removing the 16-byte loop and check the disasm
    let n_complete_chunks = haystack.len() / BYTES_PER_OPERATION;

    // Getting the pointer once before the loop avoids the
    // overhead of manipulating the length of the slice inside the
    // loop.
    let mut haystack_ptr = haystack.as_ptr();
    let mut chunk_offset = 0;
    for _ in 0..n_complete_chunks {
        if let Some(location) = packed.cmpestri(haystack_ptr, BYTES_PER_OPERATION as i32) {
            return Some(offset + chunk_offset + location);
        }

        haystack_ptr = haystack_ptr.offset(BYTES_PER_OPERATION as isize);
        chunk_offset += BYTES_PER_OPERATION;
    }
    haystack = &haystack[chunk_offset..];
    offset += chunk_offset;

    // No data left to search
    if haystack.is_empty() {
        return None;
    }

    find_small(packed, haystack).map(|loc| loc + offset)
}

struct PackedCompare<T, const CONTROL_BYTE: i32>(T);
impl<T, const CONTROL_BYTE: i32> PackedCompare<T, CONTROL_BYTE>
where
    T: PackedCompareControl,
{
    #[inline]
    #[target_feature(enable = "sse4.2")]
    unsafe fn cmpestrm(&self, haystack: &[u8], leading_junk: usize) -> Option<usize> {
        // TODO: document why this is ok
        let haystack = _mm_loadu_si128(haystack.as_ptr() as *const __m128i);

        let mask = _mm_cmpestrm(
            self.0.needle(),
            self.0.needle_len(),
            haystack,
            BYTES_PER_OPERATION as i32,
            CONTROL_BYTE,
        );
        let mask = _mm_extract_epi16(mask, 0) as u16;

        if mask.trailing_zeros() < 16 {
            let mut mask = mask;
            // Byte: 7 6 5 4 3 2 1 0
            // Str : &[0, 1, 2, 3, ...]
            //
            // Bit-0 corresponds to Str-0; shifting to the right
            // removes the parts of the string that don't belong to
            // us.
            mask >>= leading_junk;
            // The first 1, starting from Bit-0 and going to Bit-7,
            // denotes the position of the first match.
            if mask == 0 {
                // All of our matches were before the slice started
                None
            } else {
                let first_match = mask.trailing_zeros() as usize;
                debug_assert!(first_match < 16);
                Some(first_match)
            }
        } else {
            None
        }
    }

    #[inline]
    #[target_feature(enable = "sse4.2")]
    unsafe fn cmpestri(&self, haystack: *const u8, haystack_len: i32) -> Option<usize> {
        debug_assert!(
            (1..=16).contains(&haystack_len),
            "haystack_len was {}",
            haystack_len,
        );

        // TODO: document why this is ok
        let haystack = _mm_loadu_si128(haystack as *const __m128i);

        let location = _mm_cmpestri(
            self.0.needle(),
            self.0.needle_len(),
            haystack,
            haystack_len,
            CONTROL_BYTE,
        );

        if location < 16 {
            Some(location as usize)
        } else {
            None
        }
    }
}

#[derive(Debug)]
struct Misalignment<'a> {
    leading: &'a [u8],
    leading_junk: usize,
    bytes_until_alignment: usize,
}

impl<'a> Misalignment<'a> {
    /// # Cases
    ///
    /// 0123456789ABCDEF
    /// |--|                < 1.
    ///       |--|          < 2.
    ///             |--|    < 3.
    ///             |----|  < 4.
    ///
    /// 1. The input slice is aligned.
    /// 2. The input slice is unaligned and is completely within the 16-byte chunk.
    /// 3. The input slice is unaligned and touches the boundary of the 16-byte chunk.
    /// 4. The input slice is unaligned and crosses the boundary of the 16-byte chunk.
    #[inline]
    fn new(haystack: &[u8]) -> Option<Self> {
        let aligned_start = ((haystack.as_ptr() as usize) & !0xF) as *const u8;

        // If we are already aligned, there's nothing to do
        if aligned_start == haystack.as_ptr() {
            return None;
        }

        let aligned_end = unsafe { aligned_start.offset(BYTES_PER_OPERATION as isize) };

        let leading_junk = haystack.as_ptr() as usize - aligned_start as usize;
        let leading_len = min(haystack.len() + leading_junk, BYTES_PER_OPERATION);

        let leading = unsafe { slice::from_raw_parts(aligned_start, leading_len) };

        let bytes_until_alignment = if leading_len == BYTES_PER_OPERATION {
            aligned_end as usize - haystack.as_ptr() as usize
        } else {
            haystack.len()
        };

        Some(Misalignment {
            leading,
            leading_junk,
            bytes_until_alignment,
        })
    }
}

pub struct Bytes {
    needle: __m128i,
    needle_len: i32,
}

impl Bytes {
    pub /* const */ fn new(bytes: [u8; 16], needle_len: i32) -> Self {
        Bytes {
            needle: unsafe { TransmuteToSimd { bytes }.simd },
            needle_len,
        }
    }

    #[inline]
    #[target_feature(enable = "sse4.2")]
    pub unsafe fn find(&self, haystack: &[u8]) -> Option<usize> {
        find(PackedCompare::<_, 0>(self), haystack)
    }
}

impl<'b> PackedCompareControl for &'b Bytes {
    fn needle(&self) -> __m128i {
        self.needle
    }
    fn needle_len(&self) -> i32 {
        self.needle_len
    }
}

pub struct ByteSubstring<'a> {
    complete_needle: &'a [u8],
    needle: __m128i,
    needle_len: i32,
}

impl<'a> ByteSubstring<'a> {
    pub /* const */ fn new(needle: &'a[u8]) -> Self {
        use std::cmp;

        let mut simd_needle = [0; 16];
        let len = cmp::min(simd_needle.len(), needle.len());
        simd_needle[..len].copy_from_slice(&needle[..len]);
        ByteSubstring {
            complete_needle: needle,
            needle: unsafe { TransmuteToSimd { bytes: simd_needle }.simd },
            needle_len: len as i32,
        }
    }

    #[cfg(feature = "pattern")]
    pub fn needle_len(&self) -> usize {
        self.complete_needle.len()
    }

    #[inline]
    #[target_feature(enable = "sse4.2")]
    pub unsafe fn find(&self, haystack: &[u8]) -> Option<usize> {
        let mut offset = 0;

        while let Some(idx) = find(PackedCompare::<_, _SIDD_CMP_EQUAL_ORDERED>(self), &haystack[offset..]) {
            let abs_offset = offset + idx;
            // Found a match, but is it really?
            if haystack[abs_offset..].starts_with(self.complete_needle) {
                return Some(abs_offset);
            }

            // Skip past this false positive
            offset += idx + 1;
        }

        None
    }
}

impl<'a, 'b> PackedCompareControl for &'b ByteSubstring<'a> {
    fn needle(&self) -> __m128i {
        self.needle
    }
    fn needle_len(&self) -> i32 {
        self.needle_len
    }
}

#[cfg(test)]
mod test {
    use proptest::prelude::*;
    use std::{fmt, str};
    use memmap::MmapMut;
    use region::Protection;

    use super::*;

    lazy_static! {
        static ref SPACE: Bytes = simd_bytes!(b' ');
        static ref XML_DELIM_3: Bytes = simd_bytes!(b'<', b'>', b'&');
        static ref XML_DELIM_5: Bytes = simd_bytes!(b'<', b'>', b'&', b'\'', b'"');
    }

    trait SliceFindPolyfill<T> {
        fn find_any(&self, needles: &[T]) -> Option<usize>;
        fn find_seq(&self, needle: &[T]) -> Option<usize>;
    }

    impl<T> SliceFindPolyfill<T> for [T]
    where
        T: PartialEq,
    {
        fn find_any(&self, needles: &[T]) -> Option<usize> {
            self.iter().position(|c| needles.contains(c))
        }

        fn find_seq(&self, needle: &[T]) -> Option<usize> {
            (0..self.len()).find(|&l| self[l..].starts_with(needle))
        }
    }

    struct Haystack {
        data: Vec<u8>,
        start: usize,
    }

    impl Haystack {
        fn without_start(&self) -> &[u8] {
            &self.data
        }

        fn with_start(&self) -> &[u8] {
            &self.data[self.start..]
        }
    }

    // Knowing the address of the data can be important
    impl fmt::Debug for Haystack {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.debug_struct("Haystack")
                .field("data", &self.data)
                .field("(addr)", &self.data.as_ptr())
                .field("start", &self.start)
                .finish()
        }
    }

    /// Creates a set of bytes and an offset inside them. Allows
    /// checking arbitrary memory offsets, not just where the
    /// allocator placed a value.
    fn haystack() -> BoxedStrategy<Haystack> {
        any::<Vec<u8>>()
            .prop_flat_map(|data| {
                let len = 0..=data.len();
                (Just(data), len)
            })
            .prop_map(|(data, start)| Haystack { data, start })
            .boxed()
    }

    #[derive(Debug)]
    struct Needle {
        data: [u8; 16],
        len: usize,
    }

    impl Needle {
        fn as_slice(&self) -> &[u8] {
            &self.data[..self.len]
        }
    }

    /// Creates an array and the number of valid values
    fn needle() -> BoxedStrategy<Needle> {
        (any::<[u8; 16]>(), 0..=16_usize)
            .prop_map(|(data, len)| Needle { data, len })
            .boxed()
    }

    proptest! {
        #[test]
        fn works_as_find_does_for_up_to_and_including_16_bytes(
            (haystack, needle) in (haystack(), needle())
        ) {
           let haystack = haystack.without_start();

           let us = unsafe { Bytes::new(needle.data, needle.len as i32).find(haystack) };
           let them = haystack.find_any(needle.as_slice());
           assert_eq!(us, them);
        }

        #[test]
        fn works_as_find_does_for_various_memory_offsets(
            (needle, haystack) in (needle(), haystack())
        ) {
            let haystack = haystack.with_start();

            let us = unsafe { Bytes::new(needle.data, needle.len as i32).find(haystack) };
            let them = haystack.find_any(needle.as_slice());
            assert_eq!(us, them);
        }
    }

    #[test]
    fn can_search_for_null_bytes() {
        unsafe {
            let null = simd_bytes!(b'\0');
            assert_eq!(Some(1), null.find(b"a\0"));
            assert_eq!(Some(0), null.find(b"\0"));
            assert_eq!(None, null.find(b""));
        }
    }

    #[test]
    fn can_search_in_null_bytes() {
        unsafe {
            let a = simd_bytes!(b'a');
            assert_eq!(Some(1), a.find(b"\0a"));
            assert_eq!(None, a.find(b"\0"));
        }
    }

    #[test]
    fn space_is_found() {
        unsafe {
            // Since the algorithm operates on 16-byte chunks, it's
            // important to cover tests around that boundary. Since 16
            // isn't that big of a number, we might as well do all of
            // them.

            assert_eq!(Some(0), SPACE.find(b" "));
            assert_eq!(Some(1), SPACE.find(b"0 "));
            assert_eq!(Some(2), SPACE.find(b"01 "));
            assert_eq!(Some(3), SPACE.find(b"012 "));
            assert_eq!(Some(4), SPACE.find(b"0123 "));
            assert_eq!(Some(5), SPACE.find(b"01234 "));
            assert_eq!(Some(6), SPACE.find(b"012345 "));
            assert_eq!(Some(7), SPACE.find(b"0123456 "));
            assert_eq!(Some(8), SPACE.find(b"01234567 "));
            assert_eq!(Some(9), SPACE.find(b"012345678 "));
            assert_eq!(Some(10), SPACE.find(b"0123456789 "));
            assert_eq!(Some(11), SPACE.find(b"0123456789A "));
            assert_eq!(Some(12), SPACE.find(b"0123456789AB "));
            assert_eq!(Some(13), SPACE.find(b"0123456789ABC "));
            assert_eq!(Some(14), SPACE.find(b"0123456789ABCD "));
            assert_eq!(Some(15), SPACE.find(b"0123456789ABCDE "));
            assert_eq!(Some(16), SPACE.find(b"0123456789ABCDEF "));
            assert_eq!(Some(17), SPACE.find(b"0123456789ABCDEFG "));
        }
    }

    #[test]
    fn space_not_found() {
        unsafe {
            // Since the algorithm operates on 16-byte chunks, it's
            // important to cover tests around that boundary. Since 16
            // isn't that big of a number, we might as well do all of
            // them.

            assert_eq!(None, SPACE.find(b""));
            assert_eq!(None, SPACE.find(b"0"));
            assert_eq!(None, SPACE.find(b"01"));
            assert_eq!(None, SPACE.find(b"012"));
            assert_eq!(None, SPACE.find(b"0123"));
            assert_eq!(None, SPACE.find(b"01234"));
            assert_eq!(None, SPACE.find(b"012345"));
            assert_eq!(None, SPACE.find(b"0123456"));
            assert_eq!(None, SPACE.find(b"01234567"));
            assert_eq!(None, SPACE.find(b"012345678"));
            assert_eq!(None, SPACE.find(b"0123456789"));
            assert_eq!(None, SPACE.find(b"0123456789A"));
            assert_eq!(None, SPACE.find(b"0123456789AB"));
            assert_eq!(None, SPACE.find(b"0123456789ABC"));
            assert_eq!(None, SPACE.find(b"0123456789ABCD"));
            assert_eq!(None, SPACE.find(b"0123456789ABCDE"));
            assert_eq!(None, SPACE.find(b"0123456789ABCDEF"));
            assert_eq!(None, SPACE.find(b"0123456789ABCDEFG"));
        }
    }

    #[test]
    fn works_on_nonaligned_beginnings() {
        unsafe {
            // We have special code for strings that don't lie on 16-byte
            // boundaries. Since allocation seems to happen on these
            // boundaries by default, let's walk around a bit.

            let s = b"0123456789ABCDEF ".to_vec();

            assert_eq!(Some(16), SPACE.find(&s[0..]));
            assert_eq!(Some(15), SPACE.find(&s[1..]));
            assert_eq!(Some(14), SPACE.find(&s[2..]));
            assert_eq!(Some(13), SPACE.find(&s[3..]));
            assert_eq!(Some(12), SPACE.find(&s[4..]));
            assert_eq!(Some(11), SPACE.find(&s[5..]));
            assert_eq!(Some(10), SPACE.find(&s[6..]));
            assert_eq!(Some(9), SPACE.find(&s[7..]));
            assert_eq!(Some(8), SPACE.find(&s[8..]));
            assert_eq!(Some(7), SPACE.find(&s[9..]));
            assert_eq!(Some(6), SPACE.find(&s[10..]));
            assert_eq!(Some(5), SPACE.find(&s[11..]));
            assert_eq!(Some(4), SPACE.find(&s[12..]));
            assert_eq!(Some(3), SPACE.find(&s[13..]));
            assert_eq!(Some(2), SPACE.find(&s[14..]));
            assert_eq!(Some(1), SPACE.find(&s[15..]));
            assert_eq!(Some(0), SPACE.find(&s[16..]));
            assert_eq!(None, SPACE.find(&s[17..]));
        }
    }

    #[test]
    fn misalignment_does_not_cause_a_false_positive_before_start() {
        const AAAA: u8 = 0x01;

        let needle = Needle {
            data: [
                AAAA, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            ],
            len: 1,
        };
        let haystack = Haystack {
            data: vec![
                AAAA, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00,
            ],
            start: 1,
        };

        let haystack = haystack.with_start();

        // Needs to trigger the misalignment code
        assert_ne!(0, (haystack.as_ptr() as usize) % 16);
        // There are 64 bits in the mask and we check to make sure the
        // result is less than the haystack
        assert!(haystack.len() > 64);

        let us = unsafe { Bytes::new(needle.data, needle.len as i32).find(haystack) };
        assert_eq!(None, us);
    }

    #[test]
    fn xml_delim_3_is_found() {
        unsafe {
            assert_eq!(Some(0), XML_DELIM_3.find(b"<"));
            assert_eq!(Some(0), XML_DELIM_3.find(b">"));
            assert_eq!(Some(0), XML_DELIM_3.find(b"&"));
            assert_eq!(None, XML_DELIM_3.find(b""));
        }
    }

    #[test]
    fn xml_delim_5_is_found() {
        unsafe {
            assert_eq!(Some(0), XML_DELIM_5.find(b"<"));
            assert_eq!(Some(0), XML_DELIM_5.find(b">"));
            assert_eq!(Some(0), XML_DELIM_5.find(b"&"));
            assert_eq!(Some(0), XML_DELIM_5.find(b"'"));
            assert_eq!(Some(0), XML_DELIM_5.find(b"\""));
            assert_eq!(None, XML_DELIM_5.find(b""));
        }
    }

    proptest! {
        #[test]
        fn works_as_find_does_for_byte_substrings(
            (needle, haystack) in (any::<Vec<u8>>(), any::<Vec<u8>>())
        ) {
            let us = unsafe {
                let s = ByteSubstring::new(&needle);
                s.find(&haystack)
            };
            let them = haystack.find_seq(&needle);
            assert_eq!(us, them);
        }
    }

    #[test]
    fn byte_substring_is_found() {
        unsafe {
            let substr = ByteSubstring::new(b"zz");
            assert_eq!(Some(0), substr.find(b"zz"));
            assert_eq!(Some(1), substr.find(b"0zz"));
            assert_eq!(Some(2), substr.find(b"01zz"));
            assert_eq!(Some(3), substr.find(b"012zz"));
            assert_eq!(Some(4), substr.find(b"0123zz"));
            assert_eq!(Some(5), substr.find(b"01234zz"));
            assert_eq!(Some(6), substr.find(b"012345zz"));
            assert_eq!(Some(7), substr.find(b"0123456zz"));
            assert_eq!(Some(8), substr.find(b"01234567zz"));
            assert_eq!(Some(9), substr.find(b"012345678zz"));
            assert_eq!(Some(10), substr.find(b"0123456789zz"));
            assert_eq!(Some(11), substr.find(b"0123456789Azz"));
            assert_eq!(Some(12), substr.find(b"0123456789ABzz"));
            assert_eq!(Some(13), substr.find(b"0123456789ABCzz"));
            assert_eq!(Some(14), substr.find(b"0123456789ABCDzz"));
            assert_eq!(Some(15), substr.find(b"0123456789ABCDEzz"));
            assert_eq!(Some(16), substr.find(b"0123456789ABCDEFzz"));
            assert_eq!(Some(17), substr.find(b"0123456789ABCDEFGzz"));
        }
    }

    #[test]
    fn byte_substring_is_not_found() {
        unsafe {
            let substr = ByteSubstring::new(b"zz");
            assert_eq!(None, substr.find(b""));
            assert_eq!(None, substr.find(b"0"));
            assert_eq!(None, substr.find(b"01"));
            assert_eq!(None, substr.find(b"012"));
            assert_eq!(None, substr.find(b"0123"));
            assert_eq!(None, substr.find(b"01234"));
            assert_eq!(None, substr.find(b"012345"));
            assert_eq!(None, substr.find(b"0123456"));
            assert_eq!(None, substr.find(b"01234567"));
            assert_eq!(None, substr.find(b"012345678"));
            assert_eq!(None, substr.find(b"0123456789"));
            assert_eq!(None, substr.find(b"0123456789A"));
            assert_eq!(None, substr.find(b"0123456789AB"));
            assert_eq!(None, substr.find(b"0123456789ABC"));
            assert_eq!(None, substr.find(b"0123456789ABCD"));
            assert_eq!(None, substr.find(b"0123456789ABCDE"));
            assert_eq!(None, substr.find(b"0123456789ABCDEF"));
            assert_eq!(None, substr.find(b"0123456789ABCDEFG"));
        }
    }

    #[test]
    fn byte_substring_has_false_positive() {
        unsafe {
            // The PCMPESTRI instruction will mark the "a" before "ab" as
            // a match because it cannot look beyond the 16 byte window
            // of the haystack. We need to double-check any match to
            // ensure it completely matches.

            let substr = ByteSubstring::new(b"ab");
            assert_eq!(Some(16), substr.find(b"aaaaaaaaaaaaaaaaab"))
            //   this "a" is a false positive ~~~~~~~~~~~~~~~^
        };
    }

    #[test]
    fn byte_substring_needle_is_longer_than_16_bytes() {
        unsafe {
            let needle = b"0123456789abcdefg";
            let haystack = b"0123456789abcdefgh";
            assert_eq!(Some(0), ByteSubstring::new(needle).find(haystack));
        }
    }

    fn with_guarded_string(value: &str, f: impl FnOnce(&str)) {
        // Allocate a string that ends directly before a
        // read-protected page.

        let page_size = region::page::size();
        assert!(value.len() <= page_size);

        // Map two rw-accessible pages of anonymous memory
        let mut mmap = MmapMut::map_anon(2 * page_size).unwrap();

        let (first_page, second_page) = mmap.split_at_mut(page_size);

        // Prohibit any access to the second page, so that any attempt
        // to read or write it would trigger a segfault
        unsafe {
            region::protect(second_page.as_ptr(), page_size, Protection::NONE).unwrap();
        }

        // Copy bytes to the end of the first page
        let dest = &mut first_page[page_size - value.len()..];
        dest.copy_from_slice(value.as_bytes());
        f(unsafe { str::from_utf8_unchecked(dest) });
    }

    #[test]
    fn works_at_page_boundary() {
        // PCMPxSTRx instructions are known to read 16 bytes at a
        // time. This behaviour may cause accidental segfaults by
        // reading past the page boundary.
        //
        // For now, this test failing crashes the whole test
        // suite. This could be fixed by setting a custom signal
        // handler, though Rust lacks such facilities at the moment.

        // Allocate a 16-byte string at page boundary.  To verify this
        // test, set protect=false to prevent segfaults.
        with_guarded_string("0123456789abcdef", |text| {
            // Will search for the last char
            let needle = simd_bytes!(b'f');

            // Check all suffixes of our 16-byte string
            for offset in 0..text.len() {
                let tail = &text[offset..];
                unsafe {
                    assert_eq!(Some(tail.len() - 1), needle.find(tail.as_bytes()));
                }
            }
        });
    }

    #[test]
    fn does_not_access_memory_after_haystack_when_haystack_is_multiple_of_16_bytes_and_no_match() {
        // For now, this test failing crashes the whole test
        // suite. This could be fixed by setting a custom signal
        // handler, though Rust lacks such facilities at the moment.
        with_guarded_string("0123456789abcdef", |text| {
            // Will search for a char not present
            let needle = simd_bytes!(b'z');

            unsafe {
                assert_eq!(None, needle.find(text.as_bytes()));
            }
        });
    }
}
