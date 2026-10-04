//! This module implements things that are related to the computer, rather than a specific adapter.

use std::string::String;

use windows_registry::LOCAL_MACHINE;
use windows_result::HRESULT;

use crate::error::*;

/// Returns the DNS suffix search list for the network connection used by the computer.
pub fn get_search_list() -> Result<Vec<String>> {
    match LOCAL_MACHINE
        .get_string("SYSTEM\\CurrentControlSet\\Services\\Tcpip\\Parameters\\SearchList")
    {
        Ok(search_list) => Ok(search_list.split(',').map(|s| s.to_owned()).collect()),
        Err(err) if err.code() == E_FILE_NOT_FOUND => Ok(vec![]),
        Err(err) => Err(err.into()),
    }
}

/// Returns the computer domain name (if any).
/// Returns `None` if the computer does not belong to a domain.
pub fn get_domain() -> Result<Option<String>> {
    match LOCAL_MACHINE.get_string("SYSTEM\\CurrentControlSet\\Services\\Tcpip\\Parameters\\Domain")
    {
        Ok(domain) => Ok((!domain.is_empty()).then_some(domain)),
        Err(err) if err.code() == E_FILE_NOT_FOUND => Ok(None),
        Err(err) => Err(err.into()),
    }
}

/// Returns `true` if the computer is configured to use the round robin strategy.
/// Otherwise, returns `false`.
pub fn is_round_robin_enabled() -> Result<bool> {
    match LOCAL_MACHINE.get_u32("SYSTEM\\CurrentControlSet\\Services\\DNS\\Parameters\\RoundRobin")
    {
        Ok(value) => Ok(value != 0),
        Err(err) if err.code() == E_FILE_NOT_FOUND => Ok(true), // The default is 1 according to msdn
        Err(err) => Err(err.into()),
    }
}

const E_FILE_NOT_FOUND: HRESULT = HRESULT(0x80070002_u32 as i32);
