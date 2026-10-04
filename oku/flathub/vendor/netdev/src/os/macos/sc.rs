//! SystemConfiguration::NetworkInterface-related types and functions for macOS.

use crate::interface::types::InterfaceType;
use mac_addr::MacAddr;
use objc2_core_foundation::{CFArray, CFRetained};
use objc2_system_configuration::SCNetworkInterface;
use std::collections::HashMap;
use std::io::Cursor;

const SC_NWIF_PATH: &str = "/Library/Preferences/SystemConfiguration/NetworkInterfaces.plist";
const SC_PREFS_PATH: &str = "/Library/Preferences/SystemConfiguration/preferences.plist";

#[derive(Clone, Debug, Default)]
pub(crate) struct SCInterface {
    #[allow(dead_code)]
    pub bsd_name: String,
    #[allow(dead_code)]
    pub mac: Option<MacAddr>,
    pub friendly_name: Option<String>,
    pub sc_type: Option<String>,
    #[allow(dead_code)]
    pub active: Option<bool>,
    pub dhcp_v4_enabled: Option<bool>,
    pub dhcp_v6_enabled: Option<bool>,
}

impl SCInterface {
    pub fn if_type(&self) -> Option<InterfaceType> {
        self.sc_type
            .as_ref()
            .map(|sc_type| map_sc_interface_type(sc_type))
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

fn sc_network_interfaces_all() -> CFRetained<CFArray<SCNetworkInterface>> {
    let untyped_ifaces: CFRetained<CFArray> = SCNetworkInterface::all();

    // SAFETY:
    // SCNetworkInterfaceCopyAll() returns a CFArray whose elements are SCNetworkInterfaceRef.
    unsafe {
        let raw = CFRetained::into_raw(untyped_ifaces);
        CFRetained::from_raw(raw.cast())
    }
}

/// Build a map of `BSD name -> SCInterface` using SystemConfiguration.
pub(crate) fn get_sc_interface_map() -> HashMap<String, SCInterface> {
    let mut if_map = HashMap::new();
    let sc_interfaces: CFRetained<CFArray<SCNetworkInterface>> = sc_network_interfaces_all();
    for sc_iface in sc_interfaces.iter() {
        // Key by BSD interface name (e.g. "en0", "bridge0").
        let Some(bsd_name) = sc_iface.bsd_name() else {
            continue;
        };

        let name = bsd_name.to_string();

        let sc_if_type: Option<String> = sc_iface.interface_type().map(|s| s.to_string());

        let friendly_name = sc_iface.localized_display_name().map(|s| s.to_string());

        let mac: Option<MacAddr> = sc_iface
            .hardware_address_string()
            .map(|mac_str| MacAddr::from_hex_format(&mac_str.to_string()));

        if_map.insert(
            name.clone(),
            SCInterface {
                bsd_name: name,
                friendly_name,
                sc_type: sc_if_type,
                mac,
                active: None,
                dhcp_v4_enabled: None,
                dhcp_v6_enabled: None,
            },
        );
    }
    if_map
}

fn load_sc_interfaces_plist_map(bytes: &[u8]) -> HashMap<String, SCInterface> {
    let mut map = HashMap::new();

    let v = match plist::Value::from_reader(Cursor::new(bytes)) {
        Ok(v) => v,
        Err(_) => return map,
    };

    let dict = match v.into_dictionary() {
        Some(d) => d,
        None => return map,
    };

    let interfaces = match dict.get("Interfaces").and_then(|v| v.as_array()) {
        Some(a) => a,
        None => return map,
    };

    for it in interfaces {
        let d = match it.as_dictionary() {
            Some(d) => d,
            None => continue,
        };

        let bsd = match d.get("BSD Name").and_then(|v| v.as_string()) {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => continue,
        };

        let friendly_name = d
            .get("SCNetworkInterfaceInfo")
            .and_then(|v| v.as_dictionary())
            .and_then(|info| info.get("UserDefinedName"))
            .and_then(|v| v.as_string())
            .map(|s| s.to_string());

        let sc_type = d
            .get("SCNetworkInterfaceType")
            .and_then(|v| v.as_string())
            .map(|s| s.to_string());

        let active = d.get("Active").and_then(|v| v.as_boolean());

        let mac = d
            .get("IOMACAddress")
            .and_then(|v| v.as_data())
            .and_then(|data| {
                if data.len() == 6 {
                    let mut a = [0u8; 6];
                    a.copy_from_slice(data);
                    Some(MacAddr::from_octets(a))
                } else {
                    None
                }
            });

        map.insert(
            bsd.clone(),
            SCInterface {
                bsd_name: bsd,
                friendly_name,
                sc_type,
                mac,
                active,
                dhcp_v4_enabled: None,
                dhcp_v6_enabled: None,
            },
        );
    }

    map
}

fn load_sc_preferences_plist_map(bytes: &[u8]) -> HashMap<String, SCInterface> {
    let mut map = HashMap::new();

    let v = match plist::Value::from_reader(Cursor::new(bytes)) {
        Ok(v) => v,
        Err(_) => return map,
    };
    let dict = match v.as_dictionary() {
        Some(d) => d,
        None => return map,
    };
    let services = match dict.get("NetworkServices").and_then(|v| v.as_dictionary()) {
        Some(d) => d,
        None => return map,
    };

    for service_id in current_set_service_order(dict) {
        if let Some(service) = services.get(&service_id).and_then(|v| v.as_dictionary()) {
            insert_service_metadata(&mut map, service);
        }
    }

    for service in services.values().filter_map(|v| v.as_dictionary()) {
        insert_service_metadata(&mut map, service);
    }

    map
}

fn current_set_service_order(dict: &plist::Dictionary) -> Vec<String> {
    let Some(current_set) = dict.get("CurrentSet").and_then(|v| v.as_string()) else {
        return Vec::new();
    };
    let Some(set_id) = current_set.strip_prefix("/Sets/") else {
        return Vec::new();
    };

    dict.get("Sets")
        .and_then(|v| v.as_dictionary())
        .and_then(|sets| sets.get(set_id))
        .and_then(|v| v.as_dictionary())
        .and_then(|set| set.get("Network"))
        .and_then(|v| v.as_dictionary())
        .and_then(|network| network.get("Global"))
        .and_then(|v| v.as_dictionary())
        .and_then(|global| global.get("IPv4"))
        .and_then(|v| v.as_dictionary())
        .and_then(|ipv4| ipv4.get("ServiceOrder"))
        .and_then(|v| v.as_array())
        .map(|order| {
            order
                .iter()
                .filter_map(|v| v.as_string().map(ToOwned::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

fn insert_service_metadata(map: &mut HashMap<String, SCInterface>, service: &plist::Dictionary) {
    let Some(interface) = service.get("Interface").and_then(|v| v.as_dictionary()) else {
        return;
    };
    let Some(bsd_name) = interface.get("DeviceName").and_then(|v| v.as_string()) else {
        return;
    };
    if bsd_name.is_empty() || map.contains_key(bsd_name) {
        return;
    }

    let friendly_name = service
        .get("UserDefinedName")
        .and_then(|v| v.as_string())
        .or_else(|| interface.get("UserDefinedName").and_then(|v| v.as_string()))
        .map(ToOwned::to_owned);
    let sc_type = interface
        .get("Hardware")
        .and_then(|v| v.as_string())
        .or_else(|| interface.get("Type").and_then(|v| v.as_string()))
        .map(ToOwned::to_owned);
    let dhcp_v4_enabled = service
        .get("IPv4")
        .and_then(|v| v.as_dictionary())
        .and_then(|ipv4| ipv4.get("ConfigMethod"))
        .and_then(|v| v.as_string())
        .and_then(map_ipv4_config_method_to_dhcp_v4);
    let dhcp_v6_enabled = service
        .get("IPv6")
        .and_then(|v| v.as_dictionary())
        .and_then(|ipv6| ipv6.get("ConfigMethod"))
        .and_then(|v| v.as_string())
        .and_then(map_ipv6_config_method_to_dhcp_v6);

    map.insert(
        bsd_name.to_string(),
        SCInterface {
            bsd_name: bsd_name.to_string(),
            mac: None,
            friendly_name,
            sc_type,
            active: None,
            dhcp_v4_enabled,
            dhcp_v6_enabled,
        },
    );
}

fn map_ipv4_config_method_to_dhcp_v4(method: &str) -> Option<bool> {
    match method {
        "DHCP" => Some(true),
        "Manual" | "Off" => Some(false),
        _ => None,
    }
}

fn map_ipv6_config_method_to_dhcp_v6(method: &str) -> Option<bool> {
    match method {
        "Manual" | "LinkLocal" | "Off" => Some(false),
        _ => None,
    }
}

fn merge_sc_interface_maps(
    mut base: HashMap<String, SCInterface>,
    overlay: HashMap<String, SCInterface>,
) -> HashMap<String, SCInterface> {
    for (name, overlay_iface) in overlay {
        let entry = base.entry(name).or_insert_with(|| SCInterface {
            bsd_name: overlay_iface.bsd_name.clone(),
            ..SCInterface::default()
        });

        if entry.mac.is_none() {
            entry.mac = overlay_iface.mac;
        }
        if overlay_iface.friendly_name.is_some() {
            entry.friendly_name = overlay_iface.friendly_name;
        }
        if overlay_iface.sc_type.is_some() {
            entry.sc_type = overlay_iface.sc_type;
        }
        if entry.active.is_none() {
            entry.active = overlay_iface.active;
        }
        if overlay_iface.dhcp_v4_enabled.is_some() {
            entry.dhcp_v4_enabled = overlay_iface.dhcp_v4_enabled;
        }
        if overlay_iface.dhcp_v6_enabled.is_some() {
            entry.dhcp_v6_enabled = overlay_iface.dhcp_v6_enabled;
        }
    }
    base
}

/// Read and parse the NetworkInterfaces.plist file into a map of `BSD name -> SCInterface`.
pub(crate) fn read_sc_interfaces_plist_map() -> std::io::Result<HashMap<String, SCInterface>> {
    let bytes = std::fs::read(SC_NWIF_PATH)?;
    Ok(load_sc_interfaces_plist_map(&bytes))
}

/// Read macOS SystemConfiguration plist files into a map of `BSD name -> SCInterface`.
pub(crate) fn read_sc_plist_interface_map() -> std::io::Result<HashMap<String, SCInterface>> {
    let preferences = std::fs::read(SC_PREFS_PATH)
        .map(|bytes| load_sc_preferences_plist_map(&bytes))
        .unwrap_or_default();

    match read_sc_interfaces_plist_map() {
        Ok(interfaces) => Ok(merge_sc_interface_maps(interfaces, preferences)),
        Err(_err) if !preferences.is_empty() => Ok(preferences),
        Err(err) => Err(err),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        load_sc_preferences_plist_map, map_ipv4_config_method_to_dhcp_v4,
        map_ipv6_config_method_to_dhcp_v6,
    };

    #[test]
    fn maps_ipv4_config_methods_to_dhcp_v4_state() {
        assert_eq!(map_ipv4_config_method_to_dhcp_v4("DHCP"), Some(true));
        assert_eq!(map_ipv4_config_method_to_dhcp_v4("Manual"), Some(false));
        assert_eq!(map_ipv4_config_method_to_dhcp_v4("Off"), Some(false));
        assert_eq!(map_ipv4_config_method_to_dhcp_v4("Automatic"), None);
        assert_eq!(map_ipv4_config_method_to_dhcp_v4("Unknown"), None);
    }

    #[test]
    fn maps_ipv6_config_methods_to_dhcp_v6_state() {
        assert_eq!(map_ipv6_config_method_to_dhcp_v6("Automatic"), None);
        assert_eq!(map_ipv6_config_method_to_dhcp_v6("Manual"), Some(false));
        assert_eq!(map_ipv6_config_method_to_dhcp_v6("LinkLocal"), Some(false));
        assert_eq!(map_ipv6_config_method_to_dhcp_v6("Off"), Some(false));
        assert_eq!(map_ipv6_config_method_to_dhcp_v6("Unknown"), None);
    }

    #[test]
    fn parses_preferences_network_service_metadata() {
        let plist = br#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CurrentSet</key>
    <string>/Sets/SET</string>
    <key>NetworkServices</key>
    <dict>
        <key>SERVICE</key>
        <dict>
            <key>IPv4</key>
            <dict>
                <key>ConfigMethod</key>
                <string>DHCP</string>
            </dict>
            <key>Interface</key>
            <dict>
                <key>DeviceName</key>
                <string>en1</string>
                <key>Hardware</key>
                <string>AirPort</string>
                <key>UserDefinedName</key>
                <string>Wi-Fi</string>
            </dict>
            <key>UserDefinedName</key>
            <string>Wi-Fi</string>
            <key>IPv6</key>
            <dict>
                <key>ConfigMethod</key>
                <string>Automatic</string>
            </dict>
        </dict>
    </dict>
    <key>Sets</key>
    <dict>
        <key>SET</key>
        <dict>
            <key>Network</key>
            <dict>
                <key>Global</key>
                <dict>
                    <key>IPv4</key>
                    <dict>
                        <key>ServiceOrder</key>
                        <array>
                            <string>SERVICE</string>
                        </array>
                    </dict>
                </dict>
            </dict>
        </dict>
    </dict>
</dict>
</plist>"#;

        let map = load_sc_preferences_plist_map(plist);
        let iface = map.get("en1").unwrap();
        assert_eq!(iface.friendly_name.as_deref(), Some("Wi-Fi"));
        assert_eq!(iface.sc_type.as_deref(), Some("AirPort"));
        assert_eq!(iface.dhcp_v4_enabled, Some(true));
        assert_eq!(iface.dhcp_v6_enabled, None);
    }
}
