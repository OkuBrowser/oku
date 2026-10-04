use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct DhcpState {
    pub v4: Option<bool>,
    pub v6: Option<bool>,
}

impl DhcpState {
    fn merge_missing(&mut self, other: DhcpState) {
        if self.v4.is_none() {
            self.v4 = other.v4;
        }
        if self.v6.is_none() {
            self.v6 = other.v6;
        }
    }
}

pub(crate) fn dhcp_state(iface_name: &str, ifindex: u32) -> DhcpState {
    let mut state = systemd_networkd_dhcp_state(ifindex);
    state.merge_missing(systemd_lease_dhcp_state(ifindex));
    state.merge_missing(network_manager_dhcp_state(iface_name));
    state
}

fn systemd_networkd_dhcp_state(ifindex: u32) -> DhcpState {
    if ifindex == 0 {
        return DhcpState::default();
    }
    let path = PathBuf::from(format!("/run/systemd/netif/links/{ifindex}"));
    let Ok(content) = fs::read_to_string(path) else {
        return DhcpState::default();
    };
    parse_systemd_networkd_link(&content)
}

fn systemd_lease_dhcp_state(ifindex: u32) -> DhcpState {
    if ifindex == 0 {
        return DhcpState::default();
    }
    let path = PathBuf::from(format!("/run/systemd/netif/leases/{ifindex}"));
    match fs::read_to_string(path) {
        Ok(_) => DhcpState {
            v4: Some(true),
            v6: None,
        },
        Err(_) => DhcpState::default(),
    }
}

fn network_manager_dhcp_state(iface_name: &str) -> DhcpState {
    for dir in [
        "/run/NetworkManager/system-connections",
        "/etc/NetworkManager/system-connections",
    ] {
        let state = network_manager_dir_dhcp_state(Path::new(dir), iface_name);
        if state.v4.is_some() || state.v6.is_some() {
            return state;
        }
    }
    DhcpState::default()
}

fn network_manager_dir_dhcp_state(dir: &Path, iface_name: &str) -> DhcpState {
    let Ok(entries) = fs::read_dir(dir) else {
        return DhcpState::default();
    };
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_file() {
            continue;
        }
        let Ok(content) = fs::read_to_string(entry.path()) else {
            continue;
        };
        let state = parse_network_manager_connection(&content, iface_name);
        if state.v4.is_some() || state.v6.is_some() {
            return state;
        }
    }
    DhcpState::default()
}

fn parse_systemd_networkd_link(content: &str) -> DhcpState {
    let mut inferred = DhcpState::default();
    let mut explicit = DhcpState::default();

    for line in content.lines() {
        let Some((key, value)) = split_key_value(line) else {
            continue;
        };
        if key.starts_with("DHCP4_CLIENT_") {
            inferred.v4 = Some(true);
            continue;
        }
        if key.starts_with("DHCP6_CLIENT_") {
            inferred.v6 = Some(true);
            continue;
        }
        if key == "DHCP" {
            explicit = parse_systemd_dhcp_value(value);
        }
    }

    let mut result = inferred;
    if explicit.v4.is_some() {
        result.v4 = explicit.v4;
    }
    if explicit.v6.is_some() {
        result.v6 = explicit.v6;
    }
    result
}

fn parse_systemd_dhcp_value(value: &str) -> DhcpState {
    match value.trim().to_ascii_lowercase().as_str() {
        "yes" | "true" | "both" => DhcpState {
            v4: Some(true),
            v6: Some(true),
        },
        "ipv4" => DhcpState {
            v4: Some(true),
            v6: Some(false),
        },
        "ipv6" => DhcpState {
            v4: Some(false),
            v6: Some(true),
        },
        "no" | "false" | "none" => DhcpState {
            v4: Some(false),
            v6: Some(false),
        },
        _ => DhcpState::default(),
    }
}

fn parse_network_manager_connection(content: &str, iface_name: &str) -> DhcpState {
    let mut section = "";
    let mut connection_matches = false;
    let mut ipv4_method = None;
    let mut ipv6_method = None;

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            section = name.trim();
            continue;
        }
        let Some((key, value)) = split_key_value(line) else {
            continue;
        };
        match (section, key) {
            ("connection", "interface-name") => {
                connection_matches = value == iface_name;
            }
            ("ipv4", "method") => {
                ipv4_method = Some(value);
            }
            ("ipv6", "method") => {
                ipv6_method = Some(value);
            }
            _ => {}
        }
    }

    if !connection_matches {
        return DhcpState::default();
    }

    DhcpState {
        v4: ipv4_method.and_then(parse_network_manager_ipv4_method),
        v6: ipv6_method.and_then(parse_network_manager_ipv6_method),
    }
}

fn parse_network_manager_ipv4_method(method: &str) -> Option<bool> {
    match method.trim().to_ascii_lowercase().as_str() {
        "auto" => Some(true),
        "manual" | "disabled" | "link-local" | "shared" => Some(false),
        _ => None,
    }
}

fn parse_network_manager_ipv6_method(method: &str) -> Option<bool> {
    match method.trim().to_ascii_lowercase().as_str() {
        "dhcp" => Some(true),
        "manual" | "disabled" | "link-local" => Some(false),
        _ => None,
    }
}

fn split_key_value(line: &str) -> Option<(&str, &str)> {
    let (key, value) = line.split_once('=')?;
    Some((key.trim(), value.trim()))
}

#[cfg(test)]
mod tests {
    use super::{
        DhcpState, parse_network_manager_connection, parse_network_manager_ipv4_method,
        parse_network_manager_ipv6_method, parse_systemd_networkd_link,
    };

    #[test]
    fn parses_systemd_networkd_dhcp_values() {
        assert_eq!(
            parse_systemd_networkd_link("ADMIN_STATE=configured\nDHCP=yes\n"),
            DhcpState {
                v4: Some(true),
                v6: Some(true),
            }
        );
        assert_eq!(
            parse_systemd_networkd_link("DHCP=ipv4\n"),
            DhcpState {
                v4: Some(true),
                v6: Some(false),
            }
        );
        assert_eq!(
            parse_systemd_networkd_link("DHCP=ipv6\n"),
            DhcpState {
                v4: Some(false),
                v6: Some(true),
            }
        );
    }

    #[test]
    fn detects_systemd_runtime_client_keys() {
        assert_eq!(
            parse_systemd_networkd_link("DHCP4_CLIENT_ADDRESS=192.0.2.10\n"),
            DhcpState {
                v4: Some(true),
                v6: None,
            }
        );
        assert_eq!(
            parse_systemd_networkd_link("DHCP6_CLIENT_DUID=00:01:00:01\n"),
            DhcpState {
                v4: None,
                v6: Some(true),
            }
        );
    }

    #[test]
    fn explicit_systemd_dhcp_value_overrides_runtime_keys() {
        assert_eq!(
            parse_systemd_networkd_link("DHCP4_CLIENT_ADDRESS=192.0.2.10\nDHCP=no\n"),
            DhcpState {
                v4: Some(false),
                v6: Some(false),
            }
        );
    }

    #[test]
    fn parses_network_manager_connection_for_matching_interface() {
        let content = "\
[connection]
id=Wired
interface-name=eth0

[ipv4]
method=auto

[ipv6]
method=dhcp
";
        assert_eq!(
            parse_network_manager_connection(content, "eth0"),
            DhcpState {
                v4: Some(true),
                v6: Some(true),
            }
        );
        assert_eq!(
            parse_network_manager_connection(content, "wlan0"),
            DhcpState::default()
        );
    }

    #[test]
    fn parses_network_manager_ipv4_methods() {
        assert_eq!(parse_network_manager_ipv4_method("auto"), Some(true));
        assert_eq!(parse_network_manager_ipv4_method("manual"), Some(false));
        assert_eq!(parse_network_manager_ipv4_method("disabled"), Some(false));
        assert_eq!(parse_network_manager_ipv4_method("unknown"), None);
    }

    #[test]
    fn parses_network_manager_ipv6_methods() {
        assert_eq!(parse_network_manager_ipv6_method("dhcp"), Some(true));
        assert_eq!(parse_network_manager_ipv6_method("auto"), None);
        assert_eq!(parse_network_manager_ipv6_method("manual"), Some(false));
        assert_eq!(parse_network_manager_ipv6_method("disabled"), Some(false));
        assert_eq!(parse_network_manager_ipv6_method("link-local"), Some(false));
    }
}
