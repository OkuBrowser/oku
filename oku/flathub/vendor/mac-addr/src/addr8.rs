#[cfg(all(not(feature = "std"), feature = "alloc"))]
extern crate alloc;

#[cfg(all(not(feature = "std"), feature = "alloc"))]
use alloc::format;

use crate::addr::MacAddr;
use crate::error::ParseMacAddrError;
use core::fmt;
use core::str::FromStr; // EUI-48

#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc as alloc_mod;
#[cfg(feature = "std")]
use std as alloc_mod;

#[cfg(any(feature = "std", feature = "alloc"))]
use alloc_mod::string::String;

/// 64-bit MAC-like address (IEEE EUI-64).
#[repr(C)]
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Default, Debug)]
pub struct MacAddr8(
    pub u8,
    pub u8,
    pub u8,
    pub u8,
    pub u8,
    pub u8,
    pub u8,
    pub u8,
);

impl MacAddr8 {
    /// Constructs a new [`MacAddr8`] from eight octets.
    #[inline]
    pub fn new(a: u8, b: u8, c: u8, d: u8, e: u8, f: u8, g: u8, h: u8) -> MacAddr8 {
        MacAddr8(a, b, c, d, e, f, g, h)
    }

    /// Constructs from a `[u8; 8]` array.
    #[inline]
    pub fn from_octets(octets: [u8; 8]) -> MacAddr8 {
        MacAddr8(
            octets[0], octets[1], octets[2], octets[3], octets[4], octets[5], octets[6], octets[7],
        )
    }

    /// Returns the 8 octets backing this address.
    #[inline]
    pub fn octets(&self) -> [u8; 8] {
        [
            self.0, self.1, self.2, self.3, self.4, self.5, self.6, self.7,
        ]
    }

    /// Returns a colon-separated lowercase hex string (`xx:xx:xx:xx:xx:xx:xx:xx`).
    #[cfg(any(feature = "std", feature = "alloc"))]
    #[inline]
    pub fn address(&self) -> String {
        format!(
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            self.0, self.1, self.2, self.3, self.4, self.5, self.6, self.7
        )
    }

    /// Returns `true` if the address is multicast.
    #[inline]
    pub fn is_multicast(&self) -> bool {
        self.0 & 0x01 == 0x01
    }

    /// Returns `true` if the address is unicast.
    #[inline]
    pub fn is_unicast(&self) -> bool {
        !self.is_multicast()
    }

    /// Returns `true` if the address is locally administered.
    #[inline]
    pub fn is_locally_administered(&self) -> bool {
        self.0 & 0x02 == 0x02
    }

    /// Returns `true` if the address is universally administered.
    #[inline]
    pub fn is_universal(&self) -> bool {
        !self.is_locally_administered()
    }

    /// Returns the OUI (first 3 octets).
    #[inline]
    pub fn oui(&self) -> [u8; 3] {
        [self.0, self.1, self.2]
    }

    /// Converts from EUI-48 (`MacAddr`) to EUI-64 by inserting `ff:fe`.
    /// Commonly used for IPv6 IID formation from MAC addresses.
    #[inline]
    pub fn from_eui48(mac: MacAddr) -> MacAddr8 {
        let [a, b, c, d, e, f] = mac.octets();
        MacAddr8(a, b, c, 0xff, 0xfe, d, e, f)
    }

    /// Converts to EUI-48 if the EUI-64 matches the `ff:fe` embedding pattern.
    #[inline]
    pub fn to_eui48(&self) -> Option<MacAddr> {
        if self.3 == 0xff && self.4 == 0xfe {
            Some(MacAddr::new(self.0, self.1, self.2, self.5, self.6, self.7))
        } else {
            None
        }
    }
}

impl fmt::Display for MacAddr8 {
    /// Lowercase hex with `:` separators.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let _ = write!(
            f,
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            self.0, self.1, self.2, self.3, self.4, self.5, self.6, self.7
        );
        Ok(())
    }
}

impl fmt::LowerHex for MacAddr8 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            self.0, self.1, self.2, self.3, self.4, self.5, self.6, self.7
        )
    }
}

impl fmt::UpperHex for MacAddr8 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            self.0, self.1, self.2, self.3, self.4, self.5, self.6, self.7
        )
    }
}

impl FromStr for MacAddr8 {
    type Err = ParseMacAddrError;

    fn from_str(s: &str) -> Result<MacAddr8, ParseMacAddrError> {
        let mut parts = [0u8; 8];
        let mut i = 0;
        for split in s.split(':') {
            if i == 8 {
                return Err(ParseMacAddrError::TooManyComponents);
            }
            match u8::from_str_radix(split, 16) {
                Ok(b) if !split.is_empty() => parts[i] = b,
                _ => return Err(ParseMacAddrError::InvalidComponent),
            }
            i += 1;
        }
        if i == 8 {
            Ok(MacAddr8(
                parts[0], parts[1], parts[2], parts[3], parts[4], parts[5], parts[6], parts[7],
            ))
        } else {
            Err(ParseMacAddrError::TooFewComponents)
        }
    }
}

#[cfg(feature = "serde")]
impl Serialize for MacAddr8 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            serializer.collect_str(self)
        } else {
            serializer.serialize_bytes(&[
                self.0, self.1, self.2, self.3, self.4, self.5, self.6, self.7,
            ])
        }
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for MacAddr8 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct MacAddr8Visitor;
        impl<'de> de::Visitor<'de> for MacAddr8Visitor {
            type Value = MacAddr8;

            fn visit_str<E: de::Error>(self, value: &str) -> Result<MacAddr8, E> {
                value.parse().map_err(E::custom)
            }
            fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<MacAddr8, E> {
                if v.len() == 8 {
                    Ok(MacAddr8::new(
                        v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7],
                    ))
                } else {
                    Err(E::invalid_length(v.len(), &self))
                }
            }
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("either a string EUI-64 address or 8-byte array")
            }
        }

        if deserializer.is_human_readable() {
            deserializer.deserialize_str(MacAddr8Visitor)
        } else {
            deserializer.deserialize_bytes(MacAddr8Visitor)
        }
    }
}

impl From<[u8; 8]> for MacAddr8 {
    #[inline]
    fn from(v: [u8; 8]) -> Self {
        MacAddr8::from_octets(v)
    }
}

impl From<MacAddr8> for [u8; 8] {
    #[inline]
    fn from(m: MacAddr8) -> Self {
        m.octets()
    }
}

impl TryFrom<&[u8]> for MacAddr8 {
    type Error = ();

    #[inline]
    fn try_from(s: &[u8]) -> Result<Self, Self::Error> {
        if s.len() == 8 {
            Ok(MacAddr8::new(
                s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7],
            ))
        } else {
            Err(())
        }
    }
}

impl AsRef<[u8; 8]> for MacAddr8 {
    /// # Safety
    /// This is a plain `repr(Rust)` tuple struct of eight `u8`s.
    /// Reinterpreting its memory as `[u8; 8]` is layout-compatible for all stable Rust targets.
    #[inline]
    fn as_ref(&self) -> &[u8; 8] {
        unsafe { &*(self as *const MacAddr8 as *const [u8; 8]) }
    }
}
