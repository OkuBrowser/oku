use std::collections::HashMap;

use n0_error::{e, stack_error};
use serde::Deserialize;
use tracing::warn;
use wmi::{FilterValue, WMIConnection};

use super::DefaultRouteDetails;
pub(super) use super::netdev_impl::{get_state, home_router};

/// API Docs: <https://learn.microsoft.com/en-us/previous-versions/windows/desktop/wmiiprouteprov/win32-ip4routetable>
#[derive(Deserialize, Debug)]
#[allow(non_camel_case_types, non_snake_case)]
struct Win32_IP4RouteTable {
    Name: String,
}

#[stack_error(derive, add_meta, std_sources, from_sources)]
#[non_exhaustive]
pub enum Error {
    #[allow(dead_code)] // not sure why we have this here?
    #[error("IO")]
    Io { source: std::io::Error },
    #[error("not route found")]
    NoRoute {},
    #[error("WMI")]
    Wmi { source: wmi::WMIError },
}

fn get_default_route() -> Result<DefaultRouteDetails, Error> {
    let wmi_con = WMIConnection::new()?;

    let query: HashMap<_, _> = [("Destination".into(), FilterValue::Str("0.0.0.0"))].into();
    let route: Win32_IP4RouteTable = wmi_con
        .filtered_query(&query)?
        .drain(..)
        .next()
        .ok_or_else(|| e!(Error::NoRoute))?;

    Ok(DefaultRouteDetails {
        interface_name: route.Name,
    })
}

pub async fn default_route() -> Option<DefaultRouteDetails> {
    // WMI uses COM which can deadlock on a tokio worker thread.
    match tokio::task::spawn_blocking(get_default_route).await {
        Ok(Ok(route)) => Some(route),
        Ok(Err(err)) => {
            warn!("failed to retrieve default route: {:#?}", err);
            None
        }
        Err(err) => {
            warn!("default route task panicked: {:#?}", err);
            None
        }
    }
}
