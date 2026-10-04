//! DNS resolver lookup for Apple mobile targets.

use std::net::IpAddr;

use objc2_core_foundation::{CFArray, CFDictionary, CFPropertyList, CFRetained, CFString, Type};
use objc2_system_configuration::{
    SCDynamicStore, kSCDynamicStoreDomainState, kSCDynamicStorePropNetPrimaryService, kSCEntNetDNS,
    kSCEntNetIPv4, kSCPropNetDNSServerAddresses,
};

fn new_dynamic_store() -> Option<CFRetained<SCDynamicStore>> {
    let name = CFString::from_str("netdev");
    // SAFETY:
    // We do not register a callback, and pass a null context pointer.
    unsafe { SCDynamicStore::new(None, &name, None, std::ptr::null_mut()) }
}

fn cast_property_list<T: Type>(value: CFRetained<CFPropertyList>) -> CFRetained<T> {
    // SAFETY:
    // Callers only use this after determining the expected dynamic-store value shape.
    unsafe {
        let raw = CFRetained::into_raw(value);
        CFRetained::from_raw(raw.cast())
    }
}

fn parse_dns_server_addresses(
    dict: CFRetained<CFDictionary<CFString, CFArray<CFString>>>,
) -> Vec<IpAddr> {
    let key = unsafe { kSCPropNetDNSServerAddresses };
    let Some(addresses) = dict.get(key) else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for address in addresses.iter() {
        if let Ok(ip) = address.to_string().parse::<IpAddr>() {
            if !out.contains(&ip) {
                out.push(ip);
            }
        }
    }
    out
}

fn dns_servers_for_key(store: &SCDynamicStore, key: &CFString) -> Vec<IpAddr> {
    let Some(value) = SCDynamicStore::value(Some(store), key) else {
        return Vec::new();
    };
    let dict: CFRetained<CFDictionary<CFString, CFArray<CFString>>> = cast_property_list(value);
    parse_dns_server_addresses(dict)
}

fn global_dns_servers(store: &SCDynamicStore) -> Vec<IpAddr> {
    let key = SCDynamicStore::key_create_network_global_entity(
        None,
        unsafe { kSCDynamicStoreDomainState },
        unsafe { kSCEntNetDNS },
    );
    dns_servers_for_key(store, &key)
}

fn primary_service_id(store: &SCDynamicStore) -> Option<CFRetained<CFString>> {
    let key = SCDynamicStore::key_create_network_global_entity(
        None,
        unsafe { kSCDynamicStoreDomainState },
        unsafe { kSCEntNetIPv4 },
    );
    let value = SCDynamicStore::value(Some(store), &key)?;
    let dict: CFRetained<CFDictionary<CFString, CFString>> = cast_property_list(value);
    dict.get(unsafe { kSCDynamicStorePropNetPrimaryService })
}

fn primary_service_dns_servers(store: &SCDynamicStore) -> Vec<IpAddr> {
    let Some(service_id) = primary_service_id(store) else {
        return Vec::new();
    };
    let key = SCDynamicStore::key_create_network_service_entity(
        None,
        unsafe { kSCDynamicStoreDomainState },
        &service_id,
        Some(unsafe { kSCEntNetDNS }),
    );
    dns_servers_for_key(store, &key)
}

pub fn get_system_dns_conf() -> Vec<IpAddr> {
    let Some(store) = new_dynamic_store() else {
        return Vec::new();
    };

    let global = global_dns_servers(&store);
    if !global.is_empty() {
        return global;
    }

    primary_service_dns_servers(&store)
}
