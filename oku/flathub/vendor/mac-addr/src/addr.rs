#[cfg(all(not(feature = "std"), feature = "alloc"))]
extern crate alloc;

#[cfg(all(not(feature = "std"), feature = "alloc"))]
use alloc::format;

use crate::error::ParseMacAddrError;
use core::fmt;
use core::str::FromStr;

#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc as alloc_mod;
#[cfg(feature = "std")]
use std as alloc_mod;

#[cfg(any(feature = "std", feature = "alloc"))]
use alloc_mod::string::String;

/// 48-bit MAC address (IEEE EUI-48).
#[repr(C)]
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Default, Debug)]
pub struct MacAddr(pub u8, pub u8, pub u8, pub u8, pub u8, pub u8);

impl MacAddr {
    /// Constructs a new [`MacAddr`] from six octets.
    #[inline]
    pub fn new(a: u8, b: u8, c: u8, d: u8, e: u8, f: u8) -> MacAddr {
        MacAddr(a, b, c, d, e, f)
    }

    /// Constructs from a `[u8; 6]` array.
    #[inline]
    pub fn from_octets(octets: [u8; 6]) -> MacAddr {
        MacAddr(
            octets[0], octets[1], octets[2], octets[3], octets[4], octets[5],
        )
    }

    /// Returns the 6 octets backing this address.
    #[inline]
    pub fn octets(&self) -> [u8; 6] {
        [self.0, self.1, self.2, self.3, self.4, self.5]
    }

    /// Returns a colon-separated lowercase hex string (`xx:xx:xx:xx:xx:xx`).
    #[cfg(any(feature = "std", feature = "alloc"))]
    #[inline]
    pub fn address(&self) -> String {
        format!(
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            self.0, self.1, self.2, self.3, self.4, self.5
        )
    }

    /// Returns the all-zeros address.
    #[inline]
    pub fn zero() -> MacAddr {
        MacAddr(0, 0, 0, 0, 0, 0)
    }

    /// Returns the broadcast address (`ff:ff:ff:ff:ff:ff`).
    #[inline]
    pub fn broadcast() -> MacAddr {
        MacAddr(0xff, 0xff, 0xff, 0xff, 0xff, 0xff)
    }

    /// Parses a fixed-width hex string (`xx:xx:xx:xx:xx:xx`).
    #[inline]
    pub fn from_hex_format(hex_mac_addr: &str) -> MacAddr {
        if hex_mac_addr.len() != 17 {
            return MacAddr::zero();
        }
        let mut fields = hex_mac_addr.split(':');
        let o1 = u8::from_str_radix(fields.next().unwrap_or_default(), 16).unwrap_or(0);
        let o2 = u8::from_str_radix(fields.next().unwrap_or_default(), 16).unwrap_or(0);
        let o3 = u8::from_str_radix(fields.next().unwrap_or_default(), 16).unwrap_or(0);
        let o4 = u8::from_str_radix(fields.next().unwrap_or_default(), 16).unwrap_or(0);
        let o5 = u8::from_str_radix(fields.next().unwrap_or_default(), 16).unwrap_or(0);
        let o6 = u8::from_str_radix(fields.next().unwrap_or_default(), 16).unwrap_or(0);
        MacAddr(o1, o2, o3, o4, o5, o6)
    }

    #[inline]
    pub fn is_broadcast(&self) -> bool {
        self.0 == 0xff
            && self.1 == 0xff
            && self.2 == 0xff
            && self.3 == 0xff
            && self.4 == 0xff
            && self.5 == 0xff
    }

    /// Returns `true` if the address is multicast.
    #[inline]
    pub fn is_multicast(&self) -> bool {
        self.0 & 0x01 == 0x01
    }

    /// Returns `true` if the address is unicast.
    #[inline]
    pub fn is_unicast(&self) -> bool {
        !self.is_multicast() && !self.is_broadcast()
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
}

impl fmt::Display for MacAddr {
    /// Lowercase hex with `:` separators.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let _ = write!(
            f,
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            self.0, self.1, self.2, self.3, self.4, self.5
        );
        Ok(())
    }
}

impl fmt::LowerHex for MacAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            self.0, self.1, self.2, self.3, self.4, self.5
        )
    }
}

impl fmt::UpperHex for MacAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            self.0, self.1, self.2, self.3, self.4, self.5
        )
    }
}

impl FromStr for MacAddr {
    type Err = ParseMacAddrError;

    fn from_str(s: &str) -> Result<MacAddr, ParseMacAddrError> {
        let mut parts = [0u8; 6];
        let mut i = 0;
        for split in s.split(':') {
            if i == 6 {
                return Err(ParseMacAddrError::TooManyComponents);
            }
            match u8::from_str_radix(split, 16) {
                Ok(b) if !split.is_empty() => parts[i] = b,
                _ => return Err(ParseMacAddrError::InvalidComponent),
            }
            i += 1;
        }
        if i == 6 {
            Ok(MacAddr(
                parts[0], parts[1], parts[2], parts[3], parts[4], parts[5],
            ))
        } else {
            Err(ParseMacAddrError::TooFewComponents)
        }
    }
}

#[cfg(feature = "serde")]
impl Serialize for MacAddr {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            serializer.collect_str(self)
        } else {
            serializer.serialize_bytes(&[self.0, self.1, self.2, self.3, self.4, self.5])
        }
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for MacAddr {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct MacAddrVisitor;
        impl<'de> de::Visitor<'de> for MacAddrVisitor {
            type Value = MacAddr;

            fn visit_str<E: de::Error>(self, value: &str) -> Result<MacAddr, E> {
                value.parse().map_err(E::custom)
            }
            fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<MacAddr, E> {
                if v.len() == 6 {
                    Ok(MacAddr::new(v[0], v[1], v[2], v[3], v[4], v[5]))
                } else {
                    Err(E::invalid_length(v.len(), &self))
                }
            }
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("either a string MAC address or 6-byte array")
            }
        }

        if deserializer.is_human_readable() {
            deserializer.deserialize_str(MacAddrVisitor)
        } else {
            deserializer.deserialize_bytes(MacAddrVisitor)
        }
    }
}

impl From<[u8; 6]> for MacAddr {
    #[inline]
    fn from(v: [u8; 6]) -> Self {
        MacAddr::from_octets(v)
    }
}

impl From<MacAddr> for [u8; 6] {
    #[inline]
    fn from(m: MacAddr) -> Self {
        m.octets()
    }
}

impl TryFrom<&[u8]> for MacAddr {
    type Error = ();

    #[inline]
    fn try_from(s: &[u8]) -> Result<Self, Self::Error> {
        if s.len() == 6 {
            Ok(MacAddr::new(s[0], s[1], s[2], s[3], s[4], s[5]))
        } else {
            Err(())
        }
    }
}

impl AsRef<[u8; 6]> for MacAddr {
    /// # Safety
    /// This is a plain `repr(Rust)` tuple struct of six `u8`s.
    /// Reinterpreting its memory as `[u8; 6]` is layout-compatible for all stable Rust targets.
    #[inline]
    fn as_ref(&self) -> &[u8; 6] {
        unsafe { &*(self as *const MacAddr as *const [u8; 6]) }
    }
}
