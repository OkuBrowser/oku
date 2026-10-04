use std::fs::read_to_string;
use std::net::IpAddr;
use std::net::ToSocketAddrs;

#[cfg_attr(target_os = "ios", allow(dead_code))]
pub fn get_system_dns_conf() -> Vec<IpAddr> {
    const PATH_RESOLV_CONF: &str = "/etc/resolv.conf";
    let r = read_to_string(PATH_RESOLV_CONF);
    match r {
        Ok(content) => {
            let conf_lines: Vec<&str> = content.trim().split('\n').collect();
            let mut dns_servers = Vec::new();
            for line in conf_lines {
                let fields: Vec<&str> = line.split_whitespace().collect();
                if fields.len() >= 2 {
                    // field [0]: Configuration type (e.g., "nameserver", "domain", "search")
                    // field [1]: Corresponding value (e.g., IP address, domain name)
                    if fields[0] == "nameserver" {
                        let sock_addr = format!("{}:53", fields[1]);
                        if let Ok(mut addrs) = sock_addr.to_socket_addrs() {
                            if let Some(addr) = addrs.next() {
                                dns_servers.push(addr.ip());
                            }
                        } else {
                            eprintln!("Invalid IP address format: {}", fields[1]);
                        }
                    }
                }
            }
            dns_servers
        }
        Err(_) => Vec::new(),
    }
}
