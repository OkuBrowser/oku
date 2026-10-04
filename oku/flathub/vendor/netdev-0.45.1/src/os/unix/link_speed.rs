#![cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd"
))]
#![allow(non_camel_case_types)]

use libc::{AF_INET, IFNAMSIZ, SOCK_DGRAM, close, ioctl, socket};
use std::ffi::CString;

#[cfg(any(target_os = "macos", target_os = "ios"))]
use macos_subtypes::{ifm_subtype, map_subtype_to_bps};

#[cfg(target_os = "freebsd")]
use freebsd_subtypes::{ifm_subtype, map_subtype_to_bps};

#[cfg(target_os = "openbsd")]
use openbsd_subtypes::{ifm_subtype, map_subtype_to_bps};

#[cfg(target_os = "netbsd")]
use netbsd_subtypes::{ifm_subtype, map_subtype_to_bps};

/// Returns the unix/BSD/Apple network interface link speed in bps for the given interface name.
pub(crate) fn get_link_speed(iface_name: &str) -> std::io::Result<LinkSpeed> {
    Ok(get_ifmediareq(iface_name)?.into())
}

/// Returns the ifmediareq struct for the given interface name.
fn get_ifmediareq(iface_name: &str) -> std::io::Result<ifmediareq> {
    let name = CString::new(iface_name)?;
    let bytes = name.as_bytes_with_nul();

    let mut ifmr: ifmediareq = unsafe { std::mem::zeroed() };
    ifmr.ifm_name[..bytes.len()].copy_from_slice(&bytes);

    let sock = unsafe { socket(AF_INET, SOCK_DGRAM, 0) };
    if sock < 0 {
        return Err(std::io::Error::last_os_error());
    }

    let ret = unsafe { ioctl(sock, SIOCGIFXMEDIA, &mut ifmr) };
    unsafe {
        close(sock);
    }

    if ret < 0 {
        return Err(std::io::Error::last_os_error());
    }

    Ok(ifmr)
}

pub(crate) struct LinkSpeed {
    pub bps: Option<u64>,
    pub auto_negotiate: bool,
}

impl From<ifmediareq> for LinkSpeed {
    fn from(value: ifmediareq) -> Self {
        let current_subtype = ifm_subtype(value.ifm_current);
        let active_subtype = ifm_subtype(value.ifm_active);

        // According to the documentation the current subtype should be IFM_MANUAL (1) if the
        // link speed is set manually. However, at least on macOS, current equals active if
        // it was set manually. Testing for IFM_AUTO seems to work though.
        let auto_negotiate = current_subtype == IFM_AUTO;

        Self {
            bps: map_subtype_to_bps(active_subtype).ok(),
            auto_negotiate,
        }
    }
}

// OpenBSD uses uint64_t instead of int for their ifmediareq struct, see
// https://github.com/openbsd/src/blob/master/sys/net/if.h#L456 .
// Other BSD and Apple targets use int. Furthermore, OpenBSD uses an u64 TMASK
// and OpenBSD and NetBSD use different values for the subtypes.
#[cfg(target_os = "openbsd")]
type ifmediareq_int = u64;

#[cfg(not(target_os = "openbsd"))]
type ifmediareq_int = i32;

#[repr(C)]
struct ifmediareq {
    ifm_name: [u8; IFNAMSIZ],
    ifm_current: ifmediareq_int,
    ifm_mask: ifmediareq_int,
    ifm_status: ifmediareq_int,
    ifm_active: ifmediareq_int,
    ifm_count: i32,
    ifm_ulist: *mut ifmediareq_int,
}

// ifmediareq can be obtained via calling ioctl with the address SIOCGIFMEDIA or SIOCGIFXMEDIA.
// The version without the 'X' seems to be the older one, which maybe doesn't work with the
// extended types. However, some BSD variants (OpenBSD and NetBSD) don't define SIOCGIFXMEDIA.
// This code uses SIOCGIFXMEDIA where available, otherwise SIOCGIFMEDIA.

#[cfg(any(target_os = "macos", target_os = "ios"))]
// #define SIOCGIFXMEDIA   _IOWR('i', 72, struct ifmediareq)
const SIOCGIFXMEDIA: u64 = 0xc02c6948;
// #define SIOCGIFMEDIA    _IOWR('i', 56, struct ifmediareq)
// const SIOCGIFMEDIA: u64 = 0xc02c6938;

#[cfg(all(target_os = "freebsd", target_pointer_width = "64"))]
// https://github.com/freebsd/freebsd-src/blob/master/sys/sys/sockio.h#L139
// #define	SIOCGIFXMEDIA	_IOWR('i', 139, struct ifmediareq)
const SIOCGIFXMEDIA: u64 = 0xc030698b;

#[cfg(all(target_os = "freebsd", target_arch = "x86"))]
const SIOCGIFXMEDIA: u32 = 0xc030698b;

#[cfg(all(target_os = "freebsd", target_arch = "arm"))]
// FreeBSD armv7 uses a 64-bit time_t and has a distinct ioctl value from i386.
const SIOCGIFXMEDIA: u32 = 0xc028698b;

#[cfg(all(
    target_os = "freebsd",
    not(target_pointer_width = "64"),
    not(target_arch = "x86"),
    not(target_arch = "arm")
))]
// Fallback for unknown 32-bit FreeBSD architectures to keep builds working.
const SIOCGIFXMEDIA: u32 = 0xc028698b;

#[cfg(any(target_os = "openbsd", target_os = "netbsd"))]
// OpenBSD and netbsd don't define SIOCGIFXMEDIA.
// Still using SIOCGIFXMEDIA as the variable name here.
// https://github.com/openbsd/src/blob/master/sys/sys/sockio.h#L70
// https://github.com/NetBSD/src/blob/trunk/sys/sys/sockio.h#L94
// #define	SIOCGIFMEDIA	_IOWR('i', 56, struct ifmediareq)
const SIOCGIFXMEDIA: u64 = 0xc02c6938;

// The following constants are the same across all BSD variants and Apple targets.

const IFM_AUTO: i32 = 0;
//const IFM_MANUAL: i32 = 1;
//const IFM_NONE: i32 = 2;

#[cfg(any(target_os = "freebsd", target_os = "netbsd"))]
const IFM_ETHER: i32 = 0x00000020;

const IFM_10_T: i32 = 3; // 10BaseT - RJ45
const IFM_10_2: i32 = 4; // 10Base2 - Thinnet
const IFM_10_5: i32 = 5; // 10Base5 - AUI
const IFM_100_TX: i32 = 6; // 100BaseTX - RJ45
const IFM_100_FX: i32 = 7; // 100BaseFX - Fiber
const IFM_100_T4: i32 = 8; // 100BaseT4 - 4 pair cat 3
const IFM_100_VG: i32 = 9; // 100VG-AnyLAN
const IFM_100_T2: i32 = 10; // 100BaseT2
const IFM_1000_SX: i32 = 11; // 1000BaseSX - multi-mode fiber
const IFM_10_STP: i32 = 12; // 10BaseT over shielded TP
const IFM_10_FL: i32 = 13; // 10baseFL - Fiber
const IFM_1000_LX: i32 = 14; // 1000baseLX - single-mode fiber
const IFM_1000_CX: i32 = 15; // 1000baseCX - 150ohm STP
const IFM_1000_T: i32 = 16; // 1000baseT - 4 pair cat 5
const IFM_HPNA_1: i32 = 17; // HomePNA 1.0 (1Mb/s)

// The following constants are different on every BSD variant and macOS

#[cfg(any(target_os = "macos", target_os = "ios"))]
mod macos_subtypes {
    // https://github.com/apple/darwin-xnu/blob/main/bsd/net/if_media.h
    // MacOSX.sdk/usr/include/net/if_media.h

    const IFM_TMASK_COMPAT: i32 = 0x0000001f;
    const IFM_TMASK_EXT: i32 = 0x000f0000;
    const IFM_TMASK_EXT_SHIFT: i32 = 11;
    const IFM_TMASK: i32 = IFM_TMASK_COMPAT | IFM_TMASK_EXT;

    pub(crate) fn ifm_subtype(x: i32) -> i32 {
        x & IFM_TMASK
    }

    const fn ifm_x(x: i32) -> i32 {
        ((x) & IFM_TMASK_COMPAT)
            | (((x) & (IFM_TMASK_EXT >> IFM_TMASK_EXT_SHIFT)) << IFM_TMASK_EXT_SHIFT)
    }

    const IFM_10G_SR: i32 = 18; // 10GbaseSR - multi-mode fiber
    const IFM_10G_LR: i32 = 19; // 10GbaseLR - single-mode fiber
    const IFM_10G_CX4: i32 = 20; // 10GbaseCX4 - copper
    const IFM_10G_T: i32 = 21; // 10GbaseT - 4 pair cat 6
    const IFM_2500_T: i32 = 22; // 2500baseT - 4 pair cat 5
    const IFM_5000_T: i32 = 23; // 5000baseT - 4 pair cat 5
    const IFM_1000_CX_SGMII: i32 = 24; // 1000Base-CX-SGMII
    const IFM_1000_KX: i32 = 25; // 1000Base-KX backplane
    const IFM_10G_KX4: i32 = 26; // 10GBase-KX4 backplane
    const IFM_10G_KR: i32 = 27; // 10GBase-KR backplane
    const IFM_10G_CR1: i32 = 28; // 10GBase-CR1 Twinax splitter
    const IFM_10G_ER: i32 = 29; // 10GBase-ER
    const IFM_20G_KR2: i32 = 30; // 20GBase-KR2 backplane

    const IFM_2500_SX: i32 = ifm_x(32); // 2500BaseSX - multi-mode fiber
    const IFM_10G_TWINAX: i32 = ifm_x(33); // 10GBase Twinax copper
    const IFM_10G_TWINAX_LONG: i32 = ifm_x(34); // 10GBase Twinax Long copper
    const IFM_10G_LRM: i32 = ifm_x(35); // 10GBase-LRM 850nm Multi-mode
    const IFM_2500_KX: i32 = ifm_x(36); // 2500Base-KX backplane
    const IFM_40G_CR4: i32 = ifm_x(37); // 40GBase-CR4
    const IFM_40G_SR4: i32 = ifm_x(38); // 40GBase-SR4
    const IFM_50G_PCIE: i32 = ifm_x(39); // 50G Ethernet over PCIE
    const IFM_25G_PCIE: i32 = ifm_x(40); // 25G Ethernet over PCIE
    const IFM_1000_SGMII: i32 = ifm_x(41); // 1G media interface
    const IFM_10G_SFI: i32 = ifm_x(42); // 10G media interface
    const IFM_40G_XLPPI: i32 = ifm_x(43); // 40G media interface
    const IFM_40G_LR4: i32 = ifm_x(44); // 40GBase-LR4
    const IFM_40G_KR4: i32 = ifm_x(45); // 40GBase-KR4
    const IFM_100G_CR4: i32 = ifm_x(47); // 100GBase-CR4
    const IFM_100G_SR4: i32 = ifm_x(48); // 100GBase-SR4
    const IFM_100G_KR4: i32 = ifm_x(49); // 100GBase-KR4
    const IFM_100G_LR4: i32 = ifm_x(50); // 100GBase-LR4
    const IFM_56G_R4: i32 = ifm_x(51); // 56GBase-R4
    const IFM_100_T: i32 = ifm_x(52); // 100BaseT - RJ45
    const IFM_25G_CR: i32 = ifm_x(53); // 25GBase-CR
    const IFM_25G_KR: i32 = ifm_x(54); // 25GBase-KR
    const IFM_25G_SR: i32 = ifm_x(55); // 25GBase-SR
    const IFM_50G_CR2: i32 = ifm_x(56); // 50GBase-CR2
    const IFM_50G_KR2: i32 = ifm_x(57); // 50GBase-KR2
    const IFM_25G_LR: i32 = ifm_x(58); // 25GBase-LR
    const IFM_10G_AOC: i32 = ifm_x(59); // 10G active optical cable
    const IFM_25G_ACC: i32 = ifm_x(60); // 25G active copper cable
    const IFM_25G_AOC: i32 = ifm_x(61); // 25G active optical cable
    const IFM_100_SGMII: i32 = ifm_x(62); // 100M media interface
    const IFM_2500_X: i32 = ifm_x(63); // 2500BaseX
    const IFM_5000_KR: i32 = ifm_x(64); // 5GBase-KR backplane
    const IFM_25G_T: i32 = ifm_x(65); // 25GBase-T - RJ45
    const IFM_25G_CR_S: i32 = ifm_x(66); // 25GBase-CR (short)
    const IFM_25G_CR1: i32 = ifm_x(67); // 25GBase-CR1 DA cable
    const IFM_25G_KR_S: i32 = ifm_x(68); // 25GBase-KR (short)
    const IFM_5000_KR_S: i32 = ifm_x(69); // 5GBase-KR backplane (short)
    const IFM_5000_KR1: i32 = ifm_x(70); // 5GBase-KR backplane
    const IFM_25G_AUI: i32 = ifm_x(71); // 25G-AUI-C2C (chip to chip)
    const IFM_40G_XLAUI: i32 = ifm_x(72); // 40G-XLAUI
    const IFM_40G_XLAUI_AC: i32 = ifm_x(73); // 40G active copper/optical
    const IFM_40G_ER4: i32 = ifm_x(74); // 40GBase-ER4
    const IFM_50G_SR2: i32 = ifm_x(75); // 50GBase-SR2
    const IFM_50G_LR2: i32 = ifm_x(76); // 50GBase-LR2
    const IFM_50G_LAUI2_AC: i32 = ifm_x(77); // 50G active copper/optical
    const IFM_50G_LAUI2: i32 = ifm_x(78); // 50G-LAUI2
    const IFM_50G_AUI2_AC: i32 = ifm_x(79); // 50G active copper/optical
    const IFM_50G_AUI2: i32 = ifm_x(80); // 50G-AUI2
    const IFM_50G_CP: i32 = ifm_x(81); // 50GBase-CP
    const IFM_50G_SR: i32 = ifm_x(82); // 50GBase-SR
    const IFM_50G_LR: i32 = ifm_x(83); // 50GBase-LR
    const IFM_50G_FR: i32 = ifm_x(84); // 50GBase-FR
    const IFM_50G_KR_PAM4: i32 = ifm_x(85); // 50GBase-KR PAM4
    const IFM_25G_KR1: i32 = ifm_x(86); // 25GBase-KR1
    const IFM_50G_AUI1_AC: i32 = ifm_x(87); // 50G active copper/optical
    const IFM_50G_AUI1: i32 = ifm_x(88); // 50G-AUI1
    const IFM_100G_CAUI4_AC: i32 = ifm_x(89); // 100G-CAUI4 active copper/optical
    const IFM_100G_CAUI4: i32 = ifm_x(90); // 100G-CAUI4
    const IFM_100G_AUI4_AC: i32 = ifm_x(91); // 100G-AUI4 active copper/optical
    const IFM_100G_AUI4: i32 = ifm_x(92); // 100G-AUI4
    const IFM_100G_CR_PAM4: i32 = ifm_x(93); // 100GBase-CR PAM4
    const IFM_100G_KR_PAM4: i32 = ifm_x(94); // 100GBase-CR PAM4
    const IFM_100G_CP2: i32 = ifm_x(95); // 100GBase-CP2
    const IFM_100G_SR2: i32 = ifm_x(96); // 100GBase-SR2
    const IFM_100G_DR: i32 = ifm_x(97); // 100GBase-DR
    const IFM_100G_KR2_PAM4: i32 = ifm_x(98); // 100GBase-KR2 PAM4
    const IFM_100G_CAUI2_AC: i32 = ifm_x(99); // 100G-CAUI2 active copper/optical
    const IFM_100G_CAUI2: i32 = ifm_x(100); // 100G-CAUI2
    const IFM_100G_AUI2_AC: i32 = ifm_x(101); // 100G-AUI2 active copper/optical
    const IFM_100G_AUI2: i32 = ifm_x(102); // 100G-AUI2
    const IFM_200G_CR4_PAM4: i32 = ifm_x(103); // 200GBase-CR4 PAM4
    const IFM_200G_SR4: i32 = ifm_x(104); // 200GBase-SR4
    const IFM_200G_FR4: i32 = ifm_x(105); // 200GBase-FR4
    const IFM_200G_LR4: i32 = ifm_x(106); // 200GBase-LR4
    const IFM_200G_DR4: i32 = ifm_x(107); // 200GBase-DR4
    const IFM_200G_KR4_PAM4: i32 = ifm_x(108); // 200GBase-KR4 PAM4
    const IFM_200G_AUI4_AC: i32 = ifm_x(109); // 200G-AUI4 active copper/optical
    const IFM_200G_AUI4: i32 = ifm_x(110); // 200G-AUI4
    const IFM_200G_AUI8_AC: i32 = ifm_x(111); // 200G-AUI8 active copper/optical
    const IFM_200G_AUI8: i32 = ifm_x(112); // 200G-AUI8
    const IFM_400G_FR8: i32 = ifm_x(113); // 400GBase-FR8
    const IFM_400G_LR8: i32 = ifm_x(114); // 400GBase-LR8
    const IFM_400G_DR4: i32 = ifm_x(115); // 400GBase-DR4
    const IFM_400G_AUI8_AC: i32 = ifm_x(116); // 400G-AUI8 active copper/optical
    const IFM_400G_AUI8: i32 = ifm_x(117); // 400G-AUI8

    pub(crate) fn map_subtype_to_bps(subtype: i32) -> std::io::Result<u64> {
        use crate::os::unix::link_speed;
        match subtype {
            link_speed::IFM_HPNA_1 => Ok(1 * 1e6 as u64),
            link_speed::IFM_10_T
            | link_speed::IFM_10_2
            | link_speed::IFM_10_5
            | link_speed::IFM_10_STP
            | link_speed::IFM_10_FL => Ok(10 * 1e6 as u64),
            link_speed::IFM_100_TX
            | link_speed::IFM_100_FX
            | link_speed::IFM_100_T4
            | link_speed::IFM_100_VG
            | link_speed::IFM_100_T2
            | IFM_100_T
            | IFM_100_SGMII => Ok(100 * 1e6 as u64),
            link_speed::IFM_1000_SX
            | link_speed::IFM_1000_LX
            | link_speed::IFM_1000_CX
            | link_speed::IFM_1000_T
            | IFM_1000_CX_SGMII
            | IFM_1000_KX
            | IFM_1000_SGMII => Ok(1000 * 1e6 as u64),
            IFM_2500_T | IFM_2500_SX | IFM_2500_KX | IFM_2500_X => Ok(2500 * 1e6 as u64),
            IFM_5000_T | IFM_5000_KR | IFM_5000_KR_S | IFM_5000_KR1 => Ok(5000 * 1e6 as u64),
            IFM_10G_SR | IFM_10G_LR | IFM_10G_CX4 | IFM_10G_T | IFM_10G_KX4 | IFM_10G_KR
            | IFM_10G_CR1 | IFM_10G_ER | IFM_10G_TWINAX | IFM_10G_TWINAX_LONG | IFM_10G_LRM
            | IFM_10G_SFI | IFM_10G_AOC => Ok(10 * 1e9 as u64),
            IFM_20G_KR2 => Ok(20 * 1e9 as u64),
            IFM_25G_PCIE | IFM_25G_CR | IFM_25G_KR | IFM_25G_SR | IFM_25G_LR | IFM_25G_ACC
            | IFM_25G_AOC | IFM_25G_T | IFM_25G_CR_S | IFM_25G_CR1 | IFM_25G_KR_S | IFM_25G_AUI
            | IFM_25G_KR1 => Ok(25 * 1e9 as u64),
            IFM_40G_CR4 | IFM_40G_SR4 | IFM_40G_XLPPI | IFM_40G_LR4 | IFM_40G_KR4
            | IFM_40G_XLAUI | IFM_40G_XLAUI_AC | IFM_40G_ER4 => Ok(40 * 1e9 as u64),
            IFM_50G_PCIE | IFM_50G_CR2 | IFM_50G_KR2 | IFM_50G_SR2 | IFM_50G_LR2
            | IFM_50G_LAUI2_AC | IFM_50G_LAUI2 | IFM_50G_AUI2_AC | IFM_50G_AUI2 | IFM_50G_CP
            | IFM_50G_SR | IFM_50G_LR | IFM_50G_FR | IFM_50G_KR_PAM4 | IFM_50G_AUI1_AC
            | IFM_50G_AUI1 => Ok(50 * 1e9 as u64),
            IFM_56G_R4 => Ok(56 * 1e9 as u64),
            IFM_100G_CR4 | IFM_100G_SR4 | IFM_100G_KR4 | IFM_100G_LR4 | IFM_100G_CAUI4_AC
            | IFM_100G_CAUI4 | IFM_100G_AUI4_AC | IFM_100G_AUI4 | IFM_100G_CR_PAM4
            | IFM_100G_KR_PAM4 | IFM_100G_CP2 | IFM_100G_SR2 | IFM_100G_DR | IFM_100G_KR2_PAM4
            | IFM_100G_CAUI2_AC | IFM_100G_CAUI2 | IFM_100G_AUI2_AC | IFM_100G_AUI2 => {
                Ok(100 * 1e9 as u64)
            }
            IFM_200G_CR4_PAM4 | IFM_200G_SR4 | IFM_200G_FR4 | IFM_200G_LR4 | IFM_200G_DR4
            | IFM_200G_KR4_PAM4 | IFM_200G_AUI4_AC | IFM_200G_AUI4 | IFM_200G_AUI8_AC
            | IFM_200G_AUI8 => Ok(200 * 1e9 as u64),
            IFM_400G_FR8 | IFM_400G_LR8 | IFM_400G_DR4 | IFM_400G_AUI8_AC | IFM_400G_AUI8 => {
                Ok(400 * 1e9 as u64)
            }
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!(
                    "subtype {} does not map to a known link speed value",
                    subtype
                ),
            )),
        }
    }
}

#[cfg(target_os = "freebsd")]
mod freebsd_subtypes {
    // https://github.com/freebsd/freebsd-src/blob/master/sys/net/if_media.h

    const IFM_NMASK: i32 = 0x000000e0;
    const IFM_TMASK: i32 = 0x0000001f;
    const IFM_ETH_XTYPE: i32 = 0x00007800;
    const IFM_ETH_XSHIFT: i32 = 6;

    pub(crate) fn ifm_subtype(x: i32) -> i32 {
        // #define	IFM_SUBTYPE(x)	\
        //   (IFM_TYPE(x) == IFM_ETHER ? IFM_ETHER_SUBTYPE_GET(x) : ((x) & IFM_TMASK))
        // #define	IFM_ETHER_SUBTYPE_GET(x) ((x) & (IFM_TMASK|IFM_ETH_XTYPE))
        if x & IFM_NMASK == crate::os::unix::link_speed::IFM_ETHER {
            x & (IFM_TMASK | IFM_ETH_XTYPE)
        } else {
            x & IFM_TMASK
        }
    }

    const fn ifm_x(x: i32) -> i32 {
        ((x) & IFM_TMASK) | (((x) & (IFM_ETH_XTYPE >> IFM_ETH_XSHIFT)) << IFM_ETH_XSHIFT)
    }

    const IFM_10G_LR: i32 = 18; // 10GBase-LR 1310nm Single-mode
    const IFM_10G_SR: i32 = 19; // 10GBase-SR 850nm Multi-mode
    const IFM_10G_CX4: i32 = 20; // 10GBase CX4 copper
    const IFM_2500_SX: i32 = 21; // 2500BaseSX - multi-mode fiber
    const IFM_10G_TWINAX: i32 = 22; // 10GBase Twinax copper
    const IFM_10G_TWINAX_LONG: i32 = 23; // 10GBase Twinax Long copper
    const IFM_10G_LRM: i32 = 24; // 10GBase-LRM 850nm Multi-mode
    const IFM_UNKNOWN: i32 = 25; // media types not defined yet
    const IFM_10G_T: i32 = 26; // 10GBase-T - RJ45
    const IFM_40G_CR4: i32 = 27; // 40GBase-CR4
    const IFM_40G_SR4: i32 = 28; // 40GBase-SR4
    const IFM_40G_LR4: i32 = 29; // 40GBase-LR4
    const IFM_1000_KX: i32 = 30; // 1000Base-KX backplane

    const IFM_10G_KX4: i32 = ifm_x(32); // 10GBase-KX4 backplane
    const IFM_10G_KR: i32 = ifm_x(33); // 10GBase-KR backplane
    const IFM_10G_CR1: i32 = ifm_x(34); // 10GBase-CR1 Twinax splitter
    const IFM_20G_KR2: i32 = ifm_x(35); // 20GBase-KR2 backplane
    const IFM_2500_KX: i32 = ifm_x(36); // 2500Base-KX backplane
    const IFM_2500_T: i32 = ifm_x(37); // 2500Base-T - RJ45 (NBaseT)
    const IFM_5000_T: i32 = ifm_x(38); // 5000Base-T - RJ45 (NBaseT)
    const IFM_50G_PCIE: i32 = ifm_x(39); // 50G Ethernet over PCIE
    const IFM_25G_PCIE: i32 = ifm_x(40); // 25G Ethernet over PCIE
    const IFM_1000_SGMII: i32 = ifm_x(41); // 1G media interface
    const IFM_10G_SFI: i32 = ifm_x(42); // 10G media interface
    const IFM_40G_XLPPI: i32 = ifm_x(43); // 40G media interface
    const IFM_1000_CX_SGMII: i32 = ifm_x(44); // 1000Base-CX-SGMII
    const IFM_40G_KR4: i32 = ifm_x(45); // 40GBase-KR4
    const IFM_10G_ER: i32 = ifm_x(46); // 10GBase-ER
    const IFM_100G_CR4: i32 = ifm_x(47); // 100GBase-CR4
    const IFM_100G_SR4: i32 = ifm_x(48); // 100GBase-SR4
    const IFM_100G_KR4: i32 = ifm_x(49); // 100GBase-KR4
    const IFM_100G_LR4: i32 = ifm_x(50); // 100GBase-LR4
    const IFM_56G_R4: i32 = ifm_x(51); // 56GBase-R4
    const IFM_100_T: i32 = ifm_x(52); // 100BaseT - RJ45
    const IFM_25G_CR: i32 = ifm_x(53); // 25GBase-CR
    const IFM_25G_KR: i32 = ifm_x(54); // 25GBase-KR
    const IFM_25G_SR: i32 = ifm_x(55); // 25GBase-SR
    const IFM_50G_CR2: i32 = ifm_x(56); // 50GBase-CR2
    const IFM_50G_KR2: i32 = ifm_x(57); // 50GBase-KR2
    const IFM_25G_LR: i32 = ifm_x(58); // 25GBase-LR
    const IFM_10G_AOC: i32 = ifm_x(59); // 10G active optical cable
    const IFM_25G_ACC: i32 = ifm_x(60); // 25G active copper cable
    const IFM_25G_AOC: i32 = ifm_x(61); // 25G active optical cable
    const IFM_100_SGMII: i32 = ifm_x(62); // 100M media interface
    const IFM_2500_X: i32 = ifm_x(63); // 2500BaseX
    const IFM_5000_KR: i32 = ifm_x(64); // 5GBase-KR backplane
    const IFM_25G_T: i32 = ifm_x(65); // 25GBase-T - RJ45
    const IFM_25G_CR_S: i32 = ifm_x(66); // 25GBase-CR (short)
    const IFM_25G_CR1: i32 = ifm_x(67); // 25GBase-CR1 DA cable
    const IFM_25G_KR_S: i32 = ifm_x(68); // 25GBase-KR (short)
    const IFM_5000_KR_S: i32 = ifm_x(69); // 5GBase-KR backplane (short)
    const IFM_5000_KR1: i32 = ifm_x(70); // 5GBase-KR backplane
    const IFM_25G_AUI: i32 = ifm_x(71); // 25G-AUI-C2C (chip to chip)
    const IFM_40G_XLAUI: i32 = ifm_x(72); // 40G-XLAUI
    const IFM_40G_XLAUI_AC: i32 = ifm_x(73); // 40G active copper/optical
    const IFM_40G_ER4: i32 = ifm_x(74); // 40GBase-ER4
    const IFM_50G_SR2: i32 = ifm_x(75); // 50GBase-SR2
    const IFM_50G_LR2: i32 = ifm_x(76); // 50GBase-LR2
    const IFM_50G_LAUI2_AC: i32 = ifm_x(77); // 50G active copper/optical
    const IFM_50G_LAUI2: i32 = ifm_x(78); // 50G-LAUI2
    const IFM_50G_AUI2_AC: i32 = ifm_x(79); // 50G active copper/optical
    const IFM_50G_AUI2: i32 = ifm_x(80); // 50G-AUI2
    const IFM_50G_CP: i32 = ifm_x(81); // 50GBase-CP
    const IFM_50G_SR: i32 = ifm_x(82); // 50GBase-SR
    const IFM_50G_LR: i32 = ifm_x(83); // 50GBase-LR
    const IFM_50G_FR: i32 = ifm_x(84); // 50GBase-FR
    const IFM_50G_KR_PAM4: i32 = ifm_x(85); // 50GBase-KR PAM4
    const IFM_25G_KR1: i32 = ifm_x(86); // 25GBase-KR1
    const IFM_50G_AUI1_AC: i32 = ifm_x(87); // 50G active copper/optical
    const IFM_50G_AUI1: i32 = ifm_x(88); // 50G-AUI1
    const IFM_100G_CAUI4_AC: i32 = ifm_x(89); // 100G-CAUI4 active copper/optical
    const IFM_100G_CAUI4: i32 = ifm_x(90); // 100G-CAUI4
    const IFM_100G_AUI4_AC: i32 = ifm_x(91); // 100G-AUI4 active copper/optical
    const IFM_100G_AUI4: i32 = ifm_x(92); // 100G-AUI4
    const IFM_100G_CR_PAM4: i32 = ifm_x(93); // 100GBase-CR PAM4
    const IFM_100G_KR_PAM4: i32 = ifm_x(94); // 100GBase-CR PAM4
    const IFM_100G_CP2: i32 = ifm_x(95); // 100GBase-CP2
    const IFM_100G_SR2: i32 = ifm_x(96); // 100GBase-SR2
    const IFM_100G_DR: i32 = ifm_x(97); // 100GBase-DR
    const IFM_100G_KR2_PAM4: i32 = ifm_x(98); // 100GBase-KR2 PAM4
    const IFM_100G_CAUI2_AC: i32 = ifm_x(99); // 100G-CAUI2 active copper/optical
    const IFM_100G_CAUI2: i32 = ifm_x(100); // 100G-CAUI2
    const IFM_100G_AUI2_AC: i32 = ifm_x(101); // 100G-AUI2 active copper/optical
    const IFM_100G_AUI2: i32 = ifm_x(102); // 100G-AUI2
    const IFM_200G_CR4_PAM4: i32 = ifm_x(103); // 200GBase-CR4 PAM4
    const IFM_200G_SR4: i32 = ifm_x(104); // 200GBase-SR4
    const IFM_200G_FR4: i32 = ifm_x(105); // 200GBase-FR4
    const IFM_200G_LR4: i32 = ifm_x(106); // 200GBase-LR4
    const IFM_200G_DR4: i32 = ifm_x(107); // 200GBase-DR4
    const IFM_200G_KR4_PAM4: i32 = ifm_x(108); // 200GBase-KR4 PAM4
    const IFM_200G_AUI4_AC: i32 = ifm_x(109); // 200G-AUI4 active copper/optical
    const IFM_200G_AUI4: i32 = ifm_x(110); // 200G-AUI4
    const IFM_200G_AUI8_AC: i32 = ifm_x(111); // 200G-AUI8 active copper/optical
    const IFM_200G_AUI8: i32 = ifm_x(112); // 200G-AUI8
    const IFM_400G_FR8: i32 = ifm_x(113); // 400GBase-FR8
    const IFM_400G_LR8: i32 = ifm_x(114); // 400GBase-LR8
    const IFM_400G_DR4: i32 = ifm_x(115); // 400GBase-DR4
    const IFM_400G_AUI8_AC: i32 = ifm_x(116); // 400G-AUI8 active copper/optical
    const IFM_400G_AUI8: i32 = ifm_x(117); // 400G-AUI8
    const IFM_50G_KR4: i32 = ifm_x(118); // 50GBase-KR4
    const IFM_40G_LM4: i32 = ifm_x(119); // 40GBase-LM4

    pub(crate) fn map_subtype_to_bps(subtype: i32) -> std::io::Result<u64> {
        use crate::os::unix::link_speed;
        match subtype {
            link_speed::IFM_HPNA_1 => Ok(1 * 1e6 as u64),
            link_speed::IFM_10_T
            | link_speed::IFM_10_2
            | link_speed::IFM_10_5
            | link_speed::IFM_10_STP
            | link_speed::IFM_10_FL => Ok(10 * 1e6 as u64),
            link_speed::IFM_100_TX
            | link_speed::IFM_100_FX
            | link_speed::IFM_100_T4
            | link_speed::IFM_100_VG
            | link_speed::IFM_100_T2
            | IFM_100_T
            | IFM_100_SGMII => Ok(100 * 1e6 as u64),
            link_speed::IFM_1000_SX
            | link_speed::IFM_1000_LX
            | link_speed::IFM_1000_CX
            | link_speed::IFM_1000_T
            | IFM_1000_CX_SGMII
            | IFM_1000_KX
            | IFM_1000_SGMII => Ok(1000 * 1e6 as u64),
            IFM_2500_T | IFM_2500_SX | IFM_2500_KX | IFM_2500_X => Ok(2500 * 1e6 as u64),
            IFM_5000_T | IFM_5000_KR | IFM_5000_KR_S | IFM_5000_KR1 => Ok(5000 * 1e6 as u64),
            IFM_10G_SR | IFM_10G_LR | IFM_10G_CX4 | IFM_10G_T | IFM_10G_KX4 | IFM_10G_KR
            | IFM_10G_CR1 | IFM_10G_ER | IFM_10G_TWINAX | IFM_10G_TWINAX_LONG | IFM_10G_LRM
            | IFM_10G_SFI | IFM_10G_AOC => Ok(10 * 1e9 as u64),
            IFM_20G_KR2 => Ok(20 * 1e9 as u64),
            IFM_25G_PCIE | IFM_25G_CR | IFM_25G_KR | IFM_25G_SR | IFM_25G_LR | IFM_25G_ACC
            | IFM_25G_AOC | IFM_25G_T | IFM_25G_CR_S | IFM_25G_CR1 | IFM_25G_KR_S | IFM_25G_AUI
            | IFM_25G_KR1 => Ok(25 * 1e9 as u64),
            IFM_40G_CR4 | IFM_40G_SR4 | IFM_40G_XLPPI | IFM_40G_LR4 | IFM_40G_KR4
            | IFM_40G_XLAUI | IFM_40G_XLAUI_AC | IFM_40G_ER4 | IFM_40G_LM4 => Ok(40 * 1e9 as u64),
            IFM_50G_PCIE | IFM_50G_CR2 | IFM_50G_KR2 | IFM_50G_SR2 | IFM_50G_LR2
            | IFM_50G_LAUI2_AC | IFM_50G_LAUI2 | IFM_50G_AUI2_AC | IFM_50G_AUI2 | IFM_50G_CP
            | IFM_50G_SR | IFM_50G_LR | IFM_50G_FR | IFM_50G_KR_PAM4 | IFM_50G_AUI1_AC
            | IFM_50G_AUI1 | IFM_50G_KR4 => Ok(50 * 1e9 as u64),
            IFM_56G_R4 => Ok(56 * 1e9 as u64),
            IFM_100G_CR4 | IFM_100G_SR4 | IFM_100G_KR4 | IFM_100G_LR4 | IFM_100G_CAUI4_AC
            | IFM_100G_CAUI4 | IFM_100G_AUI4_AC | IFM_100G_AUI4 | IFM_100G_CR_PAM4
            | IFM_100G_KR_PAM4 | IFM_100G_CP2 | IFM_100G_SR2 | IFM_100G_DR | IFM_100G_KR2_PAM4
            | IFM_100G_CAUI2_AC | IFM_100G_CAUI2 | IFM_100G_AUI2_AC | IFM_100G_AUI2 => {
                Ok(100 * 1e9 as u64)
            }
            IFM_200G_CR4_PAM4 | IFM_200G_SR4 | IFM_200G_FR4 | IFM_200G_LR4 | IFM_200G_DR4
            | IFM_200G_KR4_PAM4 | IFM_200G_AUI4_AC | IFM_200G_AUI4 | IFM_200G_AUI8_AC
            | IFM_200G_AUI8 => Ok(200 * 1e9 as u64),
            IFM_400G_FR8 | IFM_400G_LR8 | IFM_400G_DR4 | IFM_400G_AUI8_AC | IFM_400G_AUI8 => {
                Ok(400 * 1e9 as u64)
            }
            IFM_UNKNOWN | _ => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!(
                    "subtype {} does not map to a known link speed value",
                    subtype
                ),
            )),
        }
    }
}

#[cfg(target_os = "openbsd")]
mod openbsd_subtypes {
    // https://github.com/openbsd/src/blob/master/sys/net/if_media.h

    const IFM_TMASK: u64 = 0x00000000000000ff;

    pub(crate) fn ifm_subtype(x: i32) -> u64 {
        (x as u64) & IFM_TMASK
    }

    const IFM_10G_LR: i32 = 18; // 10GBase-LR - single-mode fiber
    const IFM_10G_SR: i32 = 19; // 10GBase-SR - multi-mode fiber
    const IFM_10G_CX4: i32 = 20; // 10GBase-CX4 - copper
    const IFM_2500_SX: i32 = 21; // 2500baseSX - multi-mode fiber
    const IFM_10G_T: i32 = 22; // 10GbaseT cat 6
    const IFM_10G_SFP_CU: i32 = 23; // 10G SFP+ direct attached cable
    const IFM_10G_LRM: i32 = 24; // 10GBase-LRM 850nm Multi-mode
    const IFM_40G_CR4: i32 = 25; // 40GBase-CR4
    const IFM_40G_SR4: i32 = 26; // 40GBase-SR4
    const IFM_40G_LR4: i32 = 27; // 40GBase-LR4
    const IFM_1000_KX: i32 = 28; // 1000Base-KX backplane
    const IFM_10G_KX4: i32 = 29; // 10GBase-KX4 backplane
    const IFM_10G_KR: i32 = 30; // 10GBase-KR backplane
    const IFM_10G_CR1: i32 = 31; // 10GBase-CR1 Twinax splitter
    const IFM_20G_KR2: i32 = 32; // 20GBase-KR2 backplane
    const IFM_2500_KX: i32 = 33; // 2500Base-KX backplane
    const IFM_2500_T: i32 = 34; // 2500Base-T - RJ45 (NBaseT)
    const IFM_5000_T: i32 = 35; // 5000Base-T - RJ45 (NBaseT)
    const IFM_1000_SGMII: i32 = 36; // 1G media interface
    const IFM_10G_SFI: i32 = 37; // 10G media interface
    const IFM_40G_XLPPI: i32 = 38; // 40G media interface
    const IFM_1000_CX_SGMII: i32 = 39; // 1000Base-CX-SGMII
    const IFM_40G_KR4: i32 = 40; // 40GBase-KR4
    const IFM_10G_ER: i32 = 41; // 10GBase-ER
    const IFM_100G_CR4: i32 = 42; // 100GBase-CR4
    const IFM_100G_SR4: i32 = 43; // 100GBase-SR4
    const IFM_100G_KR4: i32 = 44; // 100GBase-KR4
    const IFM_100G_LR4: i32 = 45; // 100GBase-LR4
    const IFM_56G_R4: i32 = 46; // 56GBase-R4
    const IFM_25G_CR: i32 = 47; // 25GBase-CR
    const IFM_25G_KR: i32 = 48; // 25GBase-KR
    const IFM_25G_SR: i32 = 49; // 25GBase-SR
    const IFM_50G_CR2: i32 = 50; // 50GBase-CR2
    const IFM_50G_KR2: i32 = 51; // 50GBase-KR2
    const IFM_25G_LR: i32 = 52; // 25GBase-LR
    const IFM_25G_ER: i32 = 53; // 25GBase-ER
    const IFM_10G_AOC: i32 = 54; // 10G Active Optical Cable
    const IFM_25G_AOC: i32 = 55; // 25G Active Optical Cable
    const IFM_40G_AOC: i32 = 56; // 40G Active Optical Cable
    const IFM_100G_AOC: i32 = 57; // 100G Active Optical Cable

    pub(crate) fn map_subtype_to_bps(subtype: i32) -> std::io::Result<u64> {
        use crate::os::unix::link_speed;
        match subtype {
            link_speed::IFM_HPNA_1 => Ok(1 * 1e6 as u64),
            link_speed::IFM_10_T
            | link_speed::IFM_10_2
            | link_speed::IFM_10_5
            | link_speed::IFM_10_STP
            | link_speed::IFM_10_FL => Ok(10 * 1e6 as u64),
            link_speed::IFM_100_TX
            | link_speed::IFM_100_FX
            | link_speed::IFM_100_T4
            | link_speed::IFM_100_VG
            | link_speed::IFM_100_T2 => Ok(100 * 1e6 as u64),
            link_speed::IFM_1000_SX
            | link_speed::IFM_1000_LX
            | link_speed::IFM_1000_CX
            | link_speed::IFM_1000_T
            | IFM_1000_CX_SGMII
            | IFM_1000_KX
            | IFM_1000_SGMII => Ok(1000 * 1e6 as u64),
            IFM_2500_SX | IFM_2500_KX | IFM_2500_T => Ok(2500 * 1e6 as u64),
            IFM_5000_T => Ok(5000 * 1e6 as u64),
            IFM_10G_LR | IFM_10G_SR | IFM_10G_CX4 | IFM_10G_T | IFM_10G_SFP_CU | IFM_10G_LRM
            | IFM_10G_KX4 | IFM_10G_KR | IFM_10G_CR1 | IFM_10G_SFI | IFM_10G_ER | IFM_10G_AOC => {
                Ok(10 * 1e9 as u64)
            }
            IFM_20G_KR2 => Ok(20 * 1e9 as u64),
            IFM_25G_CR | IFM_25G_KR | IFM_25G_SR | IFM_25G_LR | IFM_25G_ER | IFM_25G_AOC => {
                Ok(25 * 1e9 as u64)
            }
            IFM_40G_CR4 | IFM_40G_SR4 | IFM_40G_LR4 | IFM_40G_XLPPI | IFM_40G_KR4 | IFM_40G_AOC => {
                Ok(40 * 1e9 as u64)
            }
            IFM_50G_CR2 | IFM_50G_KR2 => Ok(50 * 1e9 as u64),
            IFM_56G_R4 => Ok(56 * 1e9 as u64),
            IFM_100G_CR4 | IFM_100G_SR4 | IFM_100G_KR4 | IFM_100G_LR4 | IFM_100G_AOC => {
                Ok(100 * 1e9 as u64)
            }
            // => Ok(200 * 1e9 as u64),
            // => Ok(400 * 1e9 as u64),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!(
                    "subtype {} does not map to a known link speed value",
                    subtype
                ),
            )),
        }
    }
}

#[cfg(target_os = "netbsd")]
mod netbsd_subtypes {
    // https://github.com/NetBSD/src/blob/trunk/sys/net/if_media.h

    const IFM_NMASK: i32 = 0x000000e0;
    const IFM_TMASK: i32 = 0x0000001f;
    const _IFM_ETH_XTMASK: i32 = 0x0000e000;
    const IFM_ETH_XSHIFT: i32 = 13 - 5;

    pub(crate) fn ifm_subtype(x: i32) -> i32 {
        // #define	IFM_SUBTYPE(x)	(IFM_TYPE(x) == IFM_ETHER ?			      \
        // 	    IFM_ETHER_SUBTYPE_GET(x) : ((x) & IFM_TMASK))
        // #define IFM_ETHER_SUBTYPE_GET(x) ((x) & (IFM_TMASK | _IFM_ETH_XTMASK))
        if (x & IFM_NMASK) == crate::os::unix::link_speed::IFM_ETHER {
            x & (IFM_TMASK | _IFM_ETH_XTMASK)
        } else {
            x & IFM_TMASK
        }
    }

    const fn ifm_ex(x: i32) -> i32 {
        ((x) & IFM_TMASK) | (((x) & (_IFM_ETH_XTMASK >> IFM_ETH_XSHIFT)) << IFM_ETH_XSHIFT)
    }

    const IFM_10G_LR: i32 = 18; // 10GbaseLR - single-mode fiber
    const IFM_10G_SR: i32 = 19; // 10GBase-SR 850nm Multi-mode
    const IFM_10G_CX4: i32 = 20; // 10GBase CX4 copper
    const IFM_2500_SX: i32 = 21; // 2500baseSX - multi-mode fiber
    const IFM_1000_BX10: i32 = 22; // 1000base-BX10
    const IFM_10G_TWINAX: i32 = 23; // 10GBase Twinax copper
    const IFM_10G_TWINAX_LONG: i32 = 24; // 10GBase Twinax Long copper
    const IFM_10G_LRM: i32 = 25; // 10GBase-LRM 850nm Multi-mode
    const IFM_10G_T: i32 = 26; // 10GBase-T - RJ45
    const IFM_1000_KX: i32 = 27; // 1000base-KX backplane
    const IFM_2500_KX: i32 = 28; // 2500base-KX backplane
    const IFM_2500_T: i32 = 29; // 2500base-T - RJ45
    const IFM_5000_T: i32 = 30; // 5Gbase-T - RJ45
    const IFM_1000_SGMII: i32 = ifm_ex(32); // 1G SGMII
    const IFM_5000_KR: i32 = ifm_ex(33); // 5GBASE-KR backplane
    const IFM_10G_AOC: i32 = ifm_ex(34); // 10G active optical cable
    const IFM_10G_CR1: i32 = ifm_ex(35); // 10GBASE-CR1 Twinax splitter
    const IFM_10G_ER: i32 = ifm_ex(36); // 10GBASE-ER
    const IFM_10G_KR: i32 = ifm_ex(37); // 10GBASE-KR backplane
    const IFM_10G_KX4: i32 = ifm_ex(38); // 10GBASE-KX4 backplane
    const IFM_10G_LX4: i32 = ifm_ex(39); // 10GBASE-LX4
    const IFM_10G_SFI: i32 = ifm_ex(40); // 10G SFI
    const IFM_10G_ZR: i32 = ifm_ex(41); // 10GBASE-ZR
    const IFM_20G_KR2: i32 = ifm_ex(42); // 20GBASE-KR2 backplane
    const IFM_25G_AOC: i32 = ifm_ex(43); // 25G active optical cable
    const IFM_25G_AUI: i32 = ifm_ex(44); // 25G-AUI-C2C (chip to chip)
    const IFM_25G_CR: i32 = ifm_ex(45); // 25GBASE-CR (twinax)
    const IFM_25G_ACC: i32 = ifm_ex(46); // 25GBASE-ACC
    const IFM_25G_CR_S: i32 = ifm_ex(47); // 25GBASE-CR-S (CR short)
    const IFM_25G_ER: i32 = ifm_ex(48); // 25GBASE-ER
    const IFM_25G_KR: i32 = ifm_ex(49); // 25GBASE-KR
    const IFM_25G_KR_S: i32 = ifm_ex(50); // 25GBASE-KR-S (KR short)
    const IFM_25G_LR: i32 = ifm_ex(51); // 25GBASE-LR
    const IFM_25G_SR: i32 = ifm_ex(52); // 25GBASE-SR
    const IFM_25G_T: i32 = ifm_ex(53); // 25GBASE-T - RJ45
    const IFM_40G_AOC: i32 = ifm_ex(54); // 40G Active Optical Cable
    const IFM_40G_CR4: i32 = ifm_ex(55); // 40GBASE-CR4
    const IFM_40G_ER4: i32 = ifm_ex(56); // 40GBASE-ER4
    const IFM_40G_FR: i32 = ifm_ex(57); // 40GBASE-FR
    const IFM_40G_KR4: i32 = ifm_ex(58); // 40GBASE-KR4
    const IFM_40G_LR4: i32 = ifm_ex(59); // 40GBASE-LR4
    const IFM_40G_SR4: i32 = ifm_ex(60); // 40GBASE-SR4
    const IFM_40G_T: i32 = ifm_ex(61); // 40GBASE-T
    const IFM_40G_XLPPI: i32 = ifm_ex(62); // 40G XLPPI
    const IFM_50G_AUI1: i32 = ifm_ex(63); // 50GAUI-1
    const IFM_50G_AUI2: i32 = ifm_ex(64); // 50GAUI-2
    const IFM_50G_CR: i32 = ifm_ex(65); // 50GBASE-CR
    const IFM_50G_CR2: i32 = ifm_ex(66); // 50GBASE-CR2
    const IFM_50G_FR: i32 = ifm_ex(67); // 50GBASE-FR
    const IFM_50G_KR: i32 = ifm_ex(68); // 50GBASE-KR
    const IFM_50G_KR2: i32 = ifm_ex(69); // 50GBASE-KR2
    const IFM_50G_LAUI2: i32 = ifm_ex(70); // 50GLAUI-2
    const IFM_50G_LR: i32 = ifm_ex(71); // 50GBASE-LR
    const IFM_50G_SR: i32 = ifm_ex(73); // 50GBASE-SR
    const IFM_50G_SR2: i32 = ifm_ex(74); // 50GBASE-SR2
    const IFM_56G_R4: i32 = ifm_ex(75); // 56GBASE-R4
    const IFM_100G_CR2: i32 = ifm_ex(76); // 100GBASE-CR2 (CP2?)
    const IFM_100G_CR4: i32 = ifm_ex(77); // 100GBASE-CR4
    const IFM_100G_CR10: i32 = ifm_ex(78); // 100GBASE-CR10
    const IFM_100G_DR: i32 = ifm_ex(79); // 100GBASE-DR
    const IFM_100G_ER4: i32 = ifm_ex(80); // 100GBASE-ER4
    const IFM_100G_KP4: i32 = ifm_ex(81); // 100GBASE-KP4
    const IFM_100G_KR2: i32 = ifm_ex(82); // 100GBASE-KR2
    const IFM_100G_KR4: i32 = ifm_ex(83); // 100GBASE-KR4
    const IFM_100G_LR4: i32 = ifm_ex(84); // 100GBASE-LR4
    const IFM_100G_SR2: i32 = ifm_ex(85); // 100GBASE-SR2
    const IFM_100G_SR4: i32 = ifm_ex(86); // 100GBASE-SR4
    const IFM_100G_SR10: i32 = ifm_ex(87); // 100GBASE-SR10
    const IFM_200G_CR2: i32 = ifm_ex(88); // 200GBASE-CR2
    const IFM_200G_CR4: i32 = ifm_ex(89); // 200GBASE-CR4
    const IFM_200G_DR4: i32 = ifm_ex(90); // 200GBASE-DR4
    const IFM_200G_FR4: i32 = ifm_ex(91); // 200GBASE-FR4
    const IFM_200G_KR2: i32 = ifm_ex(92); // 200GBASE-KR2
    const IFM_200G_KR4: i32 = ifm_ex(93); // 200GBASE-KR4
    const IFM_200G_LR4: i32 = ifm_ex(94); // 200GBASE-LR4
    const IFM_200G_SR4: i32 = ifm_ex(95); // 200GBASE-SR4
    const IFM_400G_CR4: i32 = ifm_ex(96); // 400GBASE-CR4
    const IFM_400G_DR4: i32 = ifm_ex(97); // 400GBASE-DR4
    const IFM_400G_FR8: i32 = ifm_ex(98); // 400GBASE-FR8
    const IFM_400G_KR4: i32 = ifm_ex(99); // 400GBASE-KR4
    const IFM_400G_LR8: i32 = ifm_ex(100); // 400GBASE-LR8
    const IFM_400G_SR16: i32 = ifm_ex(101); // 400GBASE-SR16
    const IFM_100G_ACC: i32 = ifm_ex(102); // 100GBASE-ACC
    const IFM_100G_AOC: i32 = ifm_ex(103); // 100GBASE-AOC
    const IFM_100G_FR: i32 = ifm_ex(104); // 100GBASE-FR
    const IFM_100G_LR: i32 = ifm_ex(105); // 100GBASE-LR
    const IFM_200G_ER4: i32 = ifm_ex(106); // 200GBASE-ER4
    const IFM_400G_ER8: i32 = ifm_ex(107); // 400GBASE-ER8
    const IFM_400G_FR4: i32 = ifm_ex(108); // 400GBASE-FR4
    const IFM_400G_LR4: i32 = ifm_ex(109); // 400GBASE-LR4
    const IFM_400G_SR4_2: i32 = ifm_ex(110); // 400GBASE-SR4.2
    const IFM_400G_SR8: i32 = ifm_ex(111); // 400GBASE-SR8

    pub(crate) fn map_subtype_to_bps(subtype: i32) -> std::io::Result<u64> {
        use crate::os::unix::link_speed;
        match subtype {
            link_speed::IFM_HPNA_1 => Ok(1 * 1e6 as u64),
            link_speed::IFM_10_T
            | link_speed::IFM_10_2
            | link_speed::IFM_10_5
            | link_speed::IFM_10_STP
            | link_speed::IFM_10_FL => Ok(10 * 1e6 as u64),
            link_speed::IFM_100_TX
            | link_speed::IFM_100_FX
            | link_speed::IFM_100_T4
            | link_speed::IFM_100_VG
            | link_speed::IFM_100_T2 => Ok(100 * 1e6 as u64),
            link_speed::IFM_1000_SX
            | link_speed::IFM_1000_LX
            | link_speed::IFM_1000_CX
            | link_speed::IFM_1000_T
            | IFM_1000_BX10
            | IFM_1000_KX
            | IFM_1000_SGMII => Ok(1000 * 1e6 as u64),
            IFM_2500_SX | IFM_2500_KX | IFM_2500_T => Ok(2500 * 1e6 as u64),
            IFM_5000_T | IFM_5000_KR => Ok(5000 * 1e6 as u64),
            IFM_10G_LR | IFM_10G_SR | IFM_10G_CX4 | IFM_10G_TWINAX | IFM_10G_TWINAX_LONG
            | IFM_10G_LRM | IFM_10G_T | IFM_10G_AOC | IFM_10G_CR1 | IFM_10G_ER | IFM_10G_KR
            | IFM_10G_KX4 | IFM_10G_LX4 | IFM_10G_SFI | IFM_10G_ZR => Ok(10 * 1e9 as u64),
            IFM_20G_KR2 => Ok(20 * 1e9 as u64),
            IFM_25G_AOC | IFM_25G_AUI | IFM_25G_CR | IFM_25G_ACC | IFM_25G_CR_S | IFM_25G_ER
            | IFM_25G_KR | IFM_25G_KR_S | IFM_25G_LR | IFM_25G_SR | IFM_25G_T => {
                Ok(25 * 1e9 as u64)
            }
            IFM_40G_AOC | IFM_40G_CR4 | IFM_40G_ER4 | IFM_40G_FR | IFM_40G_KR4 | IFM_40G_LR4
            | IFM_40G_SR4 | IFM_40G_T | IFM_40G_XLPPI => Ok(40 * 1e9 as u64),
            IFM_50G_AUI1 | IFM_50G_AUI2 | IFM_50G_CR | IFM_50G_CR2 | IFM_50G_FR | IFM_50G_KR
            | IFM_50G_KR2 | IFM_50G_LAUI2 | IFM_50G_LR | IFM_50G_SR | IFM_50G_SR2 => {
                Ok(50 * 1e9 as u64)
            }
            IFM_56G_R4 => Ok(56 * 1e9 as u64),
            IFM_100G_CR2 | IFM_100G_CR4 | IFM_100G_CR10 | IFM_100G_DR | IFM_100G_ER4
            | IFM_100G_KP4 | IFM_100G_KR2 | IFM_100G_KR4 | IFM_100G_LR4 | IFM_100G_SR2
            | IFM_100G_SR4 | IFM_100G_SR10 | IFM_100G_ACC | IFM_100G_AOC | IFM_100G_FR
            | IFM_100G_LR => Ok(100 * 1e9 as u64),
            IFM_200G_CR2 | IFM_200G_CR4 | IFM_200G_DR4 | IFM_200G_FR4 | IFM_200G_KR2
            | IFM_200G_KR4 | IFM_200G_LR4 | IFM_200G_SR4 | IFM_200G_ER4 => Ok(200 * 1e9 as u64),
            IFM_400G_CR4 | IFM_400G_DR4 | IFM_400G_FR8 | IFM_400G_KR4 | IFM_400G_LR8
            | IFM_400G_SR16 | IFM_400G_ER8 | IFM_400G_FR4 | IFM_400G_LR4 | IFM_400G_SR4_2
            | IFM_400G_SR8 => Ok(400 * 1e9 as u64),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!(
                    "subtype {} does not map to a known link speed value",
                    subtype
                ),
            )),
        }
    }
}
