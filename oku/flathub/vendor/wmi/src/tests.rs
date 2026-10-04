use crate::{WMIConnection, WMIError, WMIResult};

pub mod fixtures {
    use super::*;

    pub fn wmi_con() -> WMIConnection {
        WMIConnection::new().unwrap()
    }
}

pub fn start_test_program() {
    std::process::Command::new("C:\\Windows\\System32\\cmd.exe")
        .args(["timeout", "1"])
        .spawn()
        .expect("failed to run test program");
}

pub fn ignore_access_denied(result: WMIResult<()>) -> WMIResult<()> {
    use windows::Win32::System::Wmi::WBEM_E_ACCESS_DENIED;

    if let Err(e) = result {
        if let WMIError::HResultError { hres } = e {
            if hres != WBEM_E_ACCESS_DENIED.0 {
                return Err(e);
            }
        }
    }
    Ok(())
}
