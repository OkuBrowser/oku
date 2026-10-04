//! Network.framework helpers for Apple mobile targets.

#![allow(non_camel_case_types)]

use std::collections::HashMap;
use std::ffi::CStr;
use std::net::IpAddr;
use std::os::raw::{c_char, c_void};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use block2::RcBlock;
use dispatch2::{
    DispatchQueue, DispatchQueueGlobalPriority, DispatchRetained, GlobalQueueIdentifier,
};

use crate::interface::types::InterfaceType;
use crate::net::device::NetworkDevice;

#[derive(Debug, Clone)]
pub(crate) struct NWInterface {
    pub bsd_name: String,
    pub if_type: InterfaceType,
    #[cfg(feature = "gateway")]
    pub index: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NWPathStatus {
    Invalid,
    Satisfied,
    Unsatisfied,
    Satisfiable,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) struct NWPathSnapshot {
    pub status: NWPathStatus,
    pub interfaces: Vec<NWInterface>,
    pub gateways: NetworkDevice,
    pub has_ipv4: bool,
    pub has_ipv6: bool,
    pub has_dns: bool,
    pub is_expensive: bool,
    pub is_constrained: bool,
}

impl NWPathSnapshot {
    pub fn interface_map(&self) -> HashMap<String, NWInterface> {
        let mut map = HashMap::with_capacity(self.interfaces.len());
        for iface in &self.interfaces {
            map.insert(iface.bsd_name.clone(), iface.clone());
        }
        map
    }

    #[cfg(feature = "gateway")]
    pub fn first_non_loopback_interface_index(&self) -> Option<u32> {
        self.interfaces
            .iter()
            .find(|iface| iface.if_type != InterfaceType::Loopback)
            .map(|iface| iface.index)
    }
}

impl Default for NWPathSnapshot {
    fn default() -> Self {
        Self {
            status: NWPathStatus::Invalid,
            interfaces: Vec::new(),
            gateways: NetworkDevice::new(),
            has_ipv4: false,
            has_ipv6: false,
            has_dns: false,
            is_expensive: false,
            is_constrained: false,
        }
    }
}

#[link(name = "Network", kind = "framework")]
unsafe extern "C" {}

type nw_path_monitor_t = *mut c_void;
type nw_path_t = *mut c_void;
type nw_interface_t = *mut c_void;
type nw_endpoint_t = *mut c_void;

#[allow(dead_code)]
#[repr(C)]
#[derive(Clone, Copy, Debug)]
enum nw_interface_type_t {
    other = 0,
    wifi = 1,
    cellular = 2,
    wired = 3,
    loopback = 4,
}

#[allow(dead_code)]
#[repr(C)]
#[derive(Clone, Copy, Debug)]
enum nw_path_status_t {
    invalid = 0,
    satisfied = 1,
    unsatisfied = 2,
    satisfiable = 3,
}

#[allow(dead_code)]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum nw_endpoint_type_t {
    invalid = 0,
    address = 1,
    host = 2,
    bonjour_service = 3,
    url = 4,
}

unsafe extern "C" {
    fn nw_path_monitor_create() -> nw_path_monitor_t;
    fn nw_path_monitor_set_queue(monitor: nw_path_monitor_t, queue: *mut c_void);
    fn nw_path_monitor_set_update_handler(monitor: nw_path_monitor_t, handler: *mut c_void);
    fn nw_path_monitor_start(monitor: nw_path_monitor_t);
    fn nw_path_monitor_cancel(monitor: nw_path_monitor_t);

    fn nw_path_get_status(path: nw_path_t) -> nw_path_status_t;
    fn nw_path_has_ipv4(path: nw_path_t) -> bool;
    fn nw_path_has_ipv6(path: nw_path_t) -> bool;
    fn nw_path_has_dns(path: nw_path_t) -> bool;
    fn nw_path_is_expensive(path: nw_path_t) -> bool;
    fn nw_path_is_constrained(path: nw_path_t) -> bool;
    fn nw_path_enumerate_interfaces(path: nw_path_t, enumerate_block: *mut c_void);
    fn nw_path_enumerate_gateways(path: nw_path_t, enumerate_block: *mut c_void);

    fn nw_interface_get_name(interface: nw_interface_t) -> *const c_char;
    fn nw_interface_get_type(interface: nw_interface_t) -> nw_interface_type_t;
    #[cfg(feature = "gateway")]
    fn nw_interface_get_index(interface: nw_interface_t) -> u32;

    fn nw_endpoint_get_type(endpoint: nw_endpoint_t) -> nw_endpoint_type_t;
    fn nw_endpoint_copy_address_string(endpoint: nw_endpoint_t) -> *mut c_char;

    fn nw_release(obj: *mut c_void);
}

fn map_interface_type(t: nw_interface_type_t) -> InterfaceType {
    match t {
        nw_interface_type_t::wifi => InterfaceType::Wireless80211,
        nw_interface_type_t::cellular => InterfaceType::Wwanpp,
        nw_interface_type_t::wired => InterfaceType::Ethernet,
        nw_interface_type_t::loopback => InterfaceType::Loopback,
        nw_interface_type_t::other => InterfaceType::Unknown,
    }
}

fn map_path_status(status: nw_path_status_t) -> NWPathStatus {
    match status {
        nw_path_status_t::satisfied => NWPathStatus::Satisfied,
        nw_path_status_t::unsatisfied => NWPathStatus::Unsatisfied,
        nw_path_status_t::satisfiable => NWPathStatus::Satisfiable,
        nw_path_status_t::invalid => NWPathStatus::Invalid,
    }
}

fn parse_gateway_ip(endpoint: nw_endpoint_t) -> Option<IpAddr> {
    if endpoint.is_null() {
        return None;
    }

    let ty = unsafe { nw_endpoint_get_type(endpoint) };
    if ty != nw_endpoint_type_t::address {
        return None;
    }

    let address = unsafe { nw_endpoint_copy_address_string(endpoint) };
    if address.is_null() {
        return None;
    }

    let ip = unsafe { CStr::from_ptr(address) }
        .to_string_lossy()
        .parse::<IpAddr>()
        .ok();

    unsafe {
        libc::free(address.cast());
    }

    ip
}

fn collect_path_snapshot(path: nw_path_t) -> NWPathSnapshot {
    let mut snapshot = NWPathSnapshot {
        status: map_path_status(unsafe { nw_path_get_status(path) }),
        has_ipv4: unsafe { nw_path_has_ipv4(path) },
        has_ipv6: unsafe { nw_path_has_ipv6(path) },
        has_dns: unsafe { nw_path_has_dns(path) },
        is_expensive: unsafe { nw_path_is_expensive(path) },
        is_constrained: unsafe { nw_path_is_constrained(path) },
        ..NWPathSnapshot::default()
    };

    let interfaces: Arc<Mutex<Vec<NWInterface>>> = Arc::new(Mutex::new(Vec::new()));
    let interfaces_ref = Arc::clone(&interfaces);
    type InterfaceEnumBlock = dyn Fn(*mut c_void) -> u8 + 'static;
    let interface_block: RcBlock<InterfaceEnumBlock> = RcBlock::new(move |iface_ptr| -> u8 {
        let iface = iface_ptr as nw_interface_t;
        if iface.is_null() {
            return 1;
        }

        let name_ptr = unsafe { nw_interface_get_name(iface) };
        if name_ptr.is_null() {
            return 1;
        }

        let name = unsafe { CStr::from_ptr(name_ptr) }
            .to_string_lossy()
            .into_owned();
        let if_type = map_interface_type(unsafe { nw_interface_get_type(iface) });
        #[cfg(feature = "gateway")]
        let index = unsafe { nw_interface_get_index(iface) };

        let nw_interface = NWInterface {
            bsd_name: name,
            if_type,
            #[cfg(feature = "gateway")]
            index,
        };

        interfaces_ref.lock().unwrap().push(nw_interface);

        1
    });

    unsafe {
        nw_path_enumerate_interfaces(
            path,
            RcBlock::<InterfaceEnumBlock>::as_ptr(&interface_block) as *mut c_void,
        );
    }
    snapshot.interfaces = std::mem::take(&mut *interfaces.lock().unwrap());

    let gateways: Arc<Mutex<NetworkDevice>> = Arc::new(Mutex::new(NetworkDevice::new()));
    let gateways_ref = Arc::clone(&gateways);
    type GatewayEnumBlock = dyn Fn(*mut c_void) -> u8 + 'static;
    let gateway_block: RcBlock<GatewayEnumBlock> = RcBlock::new(move |endpoint_ptr| -> u8 {
        if let Some(ip) = parse_gateway_ip(endpoint_ptr as nw_endpoint_t) {
            let mut gateway = gateways_ref.lock().unwrap();
            match ip {
                IpAddr::V4(ipv4) => {
                    if !gateway.ipv4.contains(&ipv4) {
                        gateway.ipv4.push(ipv4);
                    }
                }
                IpAddr::V6(ipv6) => {
                    if !gateway.ipv6.contains(&ipv6) {
                        gateway.ipv6.push(ipv6);
                    }
                }
            }
        }

        1
    });

    unsafe {
        nw_path_enumerate_gateways(
            path,
            RcBlock::<GatewayEnumBlock>::as_ptr(&gateway_block) as *mut c_void,
        );
    }
    snapshot.gateways = gateways.lock().unwrap().clone();

    snapshot
}

type UpdateHandler = dyn Fn(*mut c_void) + 'static;

/// Capture a single snapshot of the current default Network.framework path.
pub(crate) fn current_path_snapshot() -> Option<NWPathSnapshot> {
    let shared: Arc<(Mutex<Option<NWPathSnapshot>>, Condvar)> =
        Arc::new((Mutex::new(None), Condvar::new()));
    let shared_ref = Arc::clone(&shared);

    let queue = DispatchQueue::global_queue(GlobalQueueIdentifier::Priority(
        DispatchQueueGlobalPriority::Default,
    ));

    let update_block: RcBlock<UpdateHandler> = RcBlock::new(move |path_ptr: *mut c_void| {
        if path_ptr.is_null() {
            return;
        }

        let snapshot = collect_path_snapshot(path_ptr as nw_path_t);
        let (lock, cv) = &*shared_ref;
        let mut guard = lock.lock().unwrap();
        if guard.is_none() {
            *guard = Some(snapshot);
            cv.notify_one();
        }
    });

    unsafe {
        let monitor = nw_path_monitor_create();
        if monitor.is_null() {
            return None;
        }

        let queue_ptr = DispatchRetained::<DispatchQueue>::as_ptr(&queue)
            .as_ptr()
            .cast::<c_void>();
        let update_ptr = RcBlock::<UpdateHandler>::as_ptr(&update_block) as *mut c_void;

        nw_path_monitor_set_queue(monitor, queue_ptr);
        nw_path_monitor_set_update_handler(monitor, update_ptr);
        nw_path_monitor_start(monitor);

        let (lock, cv) = &*shared;
        let guard = lock.lock().unwrap();
        let mut guard = if guard.is_none() {
            cv.wait_timeout(guard, Duration::from_millis(800))
                .unwrap()
                .0
        } else {
            guard
        };
        let snapshot = guard.take();

        nw_path_monitor_cancel(monitor);
        nw_release(monitor);

        snapshot
    }
}
