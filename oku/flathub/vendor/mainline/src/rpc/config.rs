use std::{
    net::{Ipv4Addr, SocketAddrV4},
    time::Duration,
};

use super::{ServerSettings, DEFAULT_REQUEST_TIMEOUT};

#[derive(Debug, Clone)]
/// Dht Configurations
pub struct Config {
    /// Bootstrap nodes
    ///
    /// Defaults to [super::DEFAULT_BOOTSTRAP_NODES]
    pub bootstrap: Option<Vec<SocketAddrV4>>,
    /// Explicit port to listen on.
    ///
    /// Defaults to None
    pub port: Option<u16>,
    /// UDP socket request timeout duration.
    ///
    /// The longer this duration is, the longer queries take until they are deemeed "done".
    /// The shortet this duration is, the more responses from busy nodes we miss out on,
    /// which affects the accuracy of queries trying to find closest nodes to a target.
    ///
    /// Defaults to [DEFAULT_REQUEST_TIMEOUT]
    pub request_timeout: Duration,
    /// Server to respond to incoming Requests
    pub server_settings: ServerSettings,
    /// Whether or not to start in server mode from the get go.
    ///
    /// Defaults to false where it will run in [Adaptive mode](https://github.com/pubky/mainline?tab=readme-ov-file#adaptive-mode).
    pub server_mode: bool,
    /// A known public IPv4 address for this node to generate
    /// a secure node Id from according to [BEP_0042](https://www.bittorrent.org/beps/bep_0042.html)
    ///
    /// Defaults to None, where we depend on suggestions from responding nodes.
    pub public_ip: Option<Ipv4Addr>,
    /// Address to bind to.
    ///
    /// Defaults to 0.0.0.0 (all interfaces)
    pub bind_address: Option<Ipv4Addr>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bootstrap: None,
            port: None,
            request_timeout: DEFAULT_REQUEST_TIMEOUT,
            server_settings: Default::default(),
            server_mode: false,
            public_ip: None,
            bind_address: None,
        }
    }
}
