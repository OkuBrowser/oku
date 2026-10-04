//! Browser (wasm) interface enumeration.
//!
//! Browsers expose a single bit of connectivity information,
//! `navigator.onLine`. We model it as one placeholder interface and never have
//! a default route or home router to report.

use js_sys::{JsString, Reflect};

use super::{DefaultRouteDetails, HomeRouter, IFF_UP, Interface, State};
use crate::ip::LocalAddresses;

/// The name of the single placeholder interface we report in the browser.
pub(crate) const BROWSER_INTERFACE: &str = "browserif";

/// Reads `globalThis.navigator.onLine`, defaulting to `true` when unavailable.
fn navigator_online() -> bool {
    fn read() -> Option<bool> {
        let navigator = Reflect::get(
            js_sys::global().as_ref(),
            JsString::from("navigator").as_ref(),
        )
        .ok()?;
        let online = Reflect::get(&navigator, JsString::from("onLine").as_ref()).ok()?;
        online.as_bool()
    }

    match read() {
        Some(v) => v,
        None => {
            tracing::warn!("navigator.onLine unavailable, assuming up");
            true
        }
    }
}

pub(super) async fn get_state() -> State {
    let is_up = navigator_online();
    tracing::debug!(onLine = is_up, "Fetched globalThis.navigator.onLine");

    let iface = Interface {
        name: BROWSER_INTERFACE.to_string(),
        index: 0,
        flags: if is_up { IFF_UP } else { 0 },
        mac_addr: None,
        addrs: Vec::new(),
    };

    State {
        interfaces: [(BROWSER_INTERFACE.to_string(), iface)]
            .into_iter()
            .collect(),
        local_addresses: LocalAddresses::default(),
        have_v6: false,
        have_v4: false,
        is_expensive: false,
        default_route_interface: Some(BROWSER_INTERFACE.to_string()),
        last_unsuspend: None,
    }
}

pub(super) async fn default_route() -> Option<DefaultRouteDetails> {
    Some(DefaultRouteDetails {
        interface_name: BROWSER_INTERFACE.to_string(),
    })
}

pub(super) fn home_router() -> Option<HomeRouter> {
    None
}
