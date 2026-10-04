use super::macros::linked_list_iter;
use crate::interface::state::OperState;
use std::ffi::CStr;
use windows_sys::Win32::NetworkManagement::IpHelper::{
    GAA_FLAG_INCLUDE_GATEWAYS, GetAdaptersAddresses, IP_ADAPTER_ADDRESSES_LH,
};
use windows_sys::Win32::Networking::WinSock::AF_UNSPEC;

/// Return the operational state of a given Windows interface by its adapter name (GUID string)
pub fn operstate(if_name: &str) -> OperState {
    let mut mem = Vec::<u8>::with_capacity(15000);
    let mut retries = 3;
    loop {
        let mut dwsize = mem.capacity() as u32;
        let ret = unsafe {
            GetAdaptersAddresses(
                AF_UNSPEC as u32,
                GAA_FLAG_INCLUDE_GATEWAYS,
                std::ptr::null_mut(),
                mem.as_mut_ptr().cast(),
                &mut dwsize,
            )
        };
        match ret {
            0 => {
                unsafe {
                    mem.set_len(dwsize as usize);
                }
                break;
            }
            111 if retries > 0 => {
                mem.reserve(dwsize as usize);
                retries -= 1;
            }
            _ => return OperState::Unknown,
        }
    }

    let ptr = mem.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH;
    for cur in unsafe { linked_list_iter!(&ptr) } {
        let adapter_name = unsafe {
            CStr::from_ptr(cur.AdapterName.cast())
                .to_string_lossy()
                .to_string()
        };
        if adapter_name == if_name {
            return match cur.OperStatus {
                1 => OperState::Up,
                2 => OperState::Down,
                3 => OperState::Testing,
                4 => OperState::Unknown,
                5 => OperState::Dormant,
                6 => OperState::NotPresent,
                7 => OperState::LowerLayerDown,
                _ => OperState::Unknown,
            };
        }
    }

    OperState::Unknown
}
