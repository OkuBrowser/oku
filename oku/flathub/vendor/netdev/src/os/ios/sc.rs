//! SystemConfiguration helpers for Apple mobile targets.

use crate::interface::types::InterfaceType;
use objc2_core_foundation::{CFArray, CFDictionary, CFRetained, CFString};
use objc2_system_configuration::{
    SCNetworkInterface, SCNetworkProtocol, SCNetworkService, SCPreferences,
    kSCNetworkProtocolTypeIPv4, kSCNetworkProtocolTypeIPv6,
};
use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub(crate) struct SCInterface {
    pub friendly_name: Option<String>,
    pub sc_type: Option<String>,
    pub dhcp_v4_enabled: Option<bool>,
    pub dhcp_v6_enabled: Option<bool>,
}

impl SCInterface {
    pub fn if_type(&self) -> Option<InterfaceType> {
        self.sc_type.as_deref().map(map_sc_interface_type)
    }
}

fn map_sc_interface_type(type_id: &str) -> InterfaceType {
    match type_id {
        "Bridge" => InterfaceType::Bridge,
        "AirPort" => InterfaceType::Wireless80211,
        "Ethernet" => InterfaceType::Ethernet,
        "IEEE80211" => InterfaceType::Wireless80211,
        "Loopback" => InterfaceType::Loopback,
        "Modem" => InterfaceType::GenericModem,
        "PPP" => InterfaceType::Ppp,
        "WWAN" => InterfaceType::Wwanpp,
        _ => InterfaceType::Unknown,
    }
}

fn ipv4_method_to_dhcp(method: &str) -> Option<bool> {
    match method {
        "DHCP" => Some(true),
        "Manual" | "Off" => Some(false),
        _ => None,
    }
}

fn ipv6_method_to_dhcp(method: &str) -> Option<bool> {
    match method {
        "Manual" | "LinkLocal" | "Off" => Some(false),
        _ => None,
    }
}

fn cast_cf_array<T>(array: CFRetained<CFArray>) -> CFRetained<CFArray<T>> {
    // SAFETY:
    // The underlying CFArray is documented to contain only the requested type.
    unsafe {
        let raw = CFRetained::into_raw(array);
        CFRetained::from_raw(raw.cast())
    }
}

fn cast_cf_dictionary<V>(dict: CFRetained<CFDictionary>) -> CFRetained<CFDictionary<CFString, V>> {
    // SAFETY:
    // The callers only use this when the SystemConfiguration API documents
    // CFString keys and value types that match `V`.
    unsafe {
        let raw = CFRetained::into_raw(dict);
        CFRetained::from_raw(raw.cast())
    }
}

fn sc_network_interfaces_all() -> CFRetained<CFArray<SCNetworkInterface>> {
    cast_cf_array(SCNetworkInterface::all())
}

fn sc_network_services_all(prefs: &SCPreferences) -> Option<CFRetained<CFArray<SCNetworkService>>> {
    SCNetworkService::all(prefs).map(cast_cf_array)
}

fn protocol_config_method(
    service: &SCNetworkService,
    protocol_type: &CFString,
) -> Option<CFRetained<CFString>> {
    let protocol = service.protocol(protocol_type)?;
    let protocol: CFRetained<SCNetworkProtocol> = protocol;
    let config = protocol.configuration()?;
    let config: CFRetained<CFDictionary<CFString, CFString>> = cast_cf_dictionary(config);
    let key = CFString::from_str("ConfigMethod");
    config.get(&key)
}

/// Build a map of `BSD name -> SCInterface` using SystemConfiguration APIs.
pub(crate) fn get_sc_interface_map() -> HashMap<String, SCInterface> {
    let mut map = HashMap::new();

    for sc_iface in sc_network_interfaces_all().iter() {
        let Some(bsd_name) = sc_iface.bsd_name() else {
            continue;
        };

        map.insert(
            bsd_name.to_string(),
            SCInterface {
                friendly_name: sc_iface.localized_display_name().map(|s| s.to_string()),
                sc_type: sc_iface.interface_type().map(|s| s.to_string()),
                dhcp_v4_enabled: None,
                dhcp_v6_enabled: None,
            },
        );
    }

    let prefs_name = CFString::from_str("netdev");
    let Some(prefs) = SCPreferences::new(None, &prefs_name, None) else {
        return map;
    };

    let Some(services) = sc_network_services_all(&prefs) else {
        return map;
    };

    for service in services.iter() {
        let Some(sc_iface) = service.interface() else {
            continue;
        };
        let Some(bsd_name) = sc_iface.bsd_name() else {
            continue;
        };

        let entry = map.entry(bsd_name.to_string()).or_default();

        if entry.friendly_name.is_none() {
            entry.friendly_name = service
                .name()
                .map(|s| s.to_string())
                .or_else(|| sc_iface.localized_display_name().map(|s| s.to_string()));
        }

        if entry.sc_type.is_none() {
            entry.sc_type = sc_iface.interface_type().map(|s| s.to_string());
        }

        if entry.dhcp_v4_enabled.is_none() {
            entry.dhcp_v4_enabled =
                protocol_config_method(&service, unsafe { kSCNetworkProtocolTypeIPv4 })
                    .and_then(|method| ipv4_method_to_dhcp(&method.to_string()));
        }

        if entry.dhcp_v6_enabled.is_none() {
            entry.dhcp_v6_enabled =
                protocol_config_method(&service, unsafe { kSCNetworkProtocolTypeIPv6 })
                    .and_then(|method| ipv6_method_to_dhcp(&method.to_string()));
        }
    }

    map
}
