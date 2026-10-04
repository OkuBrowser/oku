use crate::context::WMIContext;
use crate::utils::WMIResult;
use log::debug;
use std::marker::PhantomData;
use windows::Win32::Foundation::CO_E_NOTINITIALIZED;
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, CoCreateInstance, CoSetProxyBlanket, EOAC_NONE, RPC_C_AUTHN_LEVEL,
    RPC_C_AUTHN_LEVEL_CALL, RPC_C_AUTHN_LEVEL_CONNECT, RPC_C_AUTHN_LEVEL_DEFAULT,
    RPC_C_AUTHN_LEVEL_NONE, RPC_C_AUTHN_LEVEL_PKT, RPC_C_AUTHN_LEVEL_PKT_INTEGRITY,
    RPC_C_AUTHN_LEVEL_PKT_PRIVACY, RPC_C_IMP_LEVEL_IMPERSONATE,
};
use windows::Win32::System::Rpc::{RPC_C_AUTHN_WINNT, RPC_C_AUTHZ_NONE};
use windows::Win32::System::Wmi::{
    IWbemContext, IWbemLocator, IWbemServices, WBEM_FLAG_CONNECT_USE_MAX_WAIT, WbemLocator,
};
use windows::core::BSTR;

/// Authentication level for a WMI connection proxy, passed to [`CoSetProxyBlanket`].
///
/// These variants correspond directly to the Windows `RPC_C_AUTHN_LEVEL_*` constants.
///
/// [`CoSetProxyBlanket`]: https://learn.microsoft.com/en-us/windows/win32/api/combaseapi/nf-combaseapi-cosetproxyblanket
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthLevel {
    /// `RPC_C_AUTHN_LEVEL_DEFAULT` – COM chooses the authentication level.
    Default,
    /// `RPC_C_AUTHN_LEVEL_NONE` – No authentication.
    None,
    /// `RPC_C_AUTHN_LEVEL_CONNECT` – Authenticate on connection.
    Connect,
    /// `RPC_C_AUTHN_LEVEL_CALL` – Authenticate at the beginning of each remote call.
    Call,
    /// `RPC_C_AUTHN_LEVEL_PKT` – Authenticate each packet.
    Pkt,
    /// `RPC_C_AUTHN_LEVEL_PKT_INTEGRITY` – Authenticate and verify that packet data is not modified.
    PktIntegrity,
    /// `RPC_C_AUTHN_LEVEL_PKT_PRIVACY` – Authenticate, verify, and encrypt each packet.
    PktPrivacy,
}

impl From<AuthLevel> for RPC_C_AUTHN_LEVEL {
    fn from(level: AuthLevel) -> Self {
        match level {
            AuthLevel::Default => RPC_C_AUTHN_LEVEL_DEFAULT,
            AuthLevel::None => RPC_C_AUTHN_LEVEL_NONE,
            AuthLevel::Connect => RPC_C_AUTHN_LEVEL_CONNECT,
            AuthLevel::Call => RPC_C_AUTHN_LEVEL_CALL,
            AuthLevel::Pkt => RPC_C_AUTHN_LEVEL_PKT,
            AuthLevel::PktIntegrity => RPC_C_AUTHN_LEVEL_PKT_INTEGRITY,
            AuthLevel::PktPrivacy => RPC_C_AUTHN_LEVEL_PKT_PRIVACY,
        }
    }
}

#[cfg(not(target_vendor = "win7"))]
fn init_security() -> windows_core::Result<()> {
    use windows::Win32::System::Com::CoInitializeSecurity;
    unsafe {
        CoInitializeSecurity(
            None,
            -1, // let COM choose.
            None,
            None,
            RPC_C_AUTHN_LEVEL_DEFAULT,
            RPC_C_IMP_LEVEL_IMPERSONATE,
            None,
            EOAC_NONE,
            None,
        )?;
    };

    Ok(())
}

/// ```compile_fail
/// let wmi = wmi::WMIConnection::new().unwrap();
/// _test_not_send(wmi);
/// ```
fn _test_not_send(_s: impl Send) {}

/// A connection to the local WMI provider.
///
/// <div class="warning">
///
/// If COM is uninitialized when a new WMI connection is created, it will be initialized (using [`CoIncrementMTAUsage`]),
/// and the [default security policy] will be set. COM will NOT be uninitialized when the connection is dropped.
///
/// If this is not what you want, then you must initialize COM yourself. See [Hosting the Windows Runtime] for more.
///
/// </div>
///
/// [`CoIncrementMTAUsage`]: https://learn.microsoft.com/en-us/windows/win32/api/combaseapi/nf-combaseapi-coincrementmtausage
/// [default security policy]: https://learn.microsoft.com/en-us/windows/win32/wmisdk/setting-the-default-process-security-level-using-c-
/// [Hosting the Windows Runtime]: https://kennykerrca.wordpress.com/2018/03/24/cppwinrt-hosting-the-windows-runtime/
#[derive(Clone, Debug)]
pub struct WMIConnection {
    // Force the type to be `!Send`, as each thread must initialize COM and a separate connection.
    _phantom: PhantomData<*mut ()>,
    pub(crate) svc: IWbemServices,
    pub(crate) ctx: WMIContext,
}

impl WMIConnection {
    /// Creates a connection with a default `CIMV2` namespace path.
    pub fn new() -> WMIResult<Self> {
        Self::with_namespace_path("ROOT\\CIMV2")
    }

    /// Creates a connection with the given namespace path.
    ///
    /// ```edition2018
    /// # fn main() -> wmi::WMIResult<()> {
    /// # use wmi::*;
    /// # use serde::Deserialize;
    /// let wmi_con = WMIConnection::with_namespace_path("ROOT\\Microsoft\\Windows\\Storage")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_namespace_path(namespace_path: &str) -> WMIResult<Self> {
        let loc = create_locator_or_init()?;

        let ctx = WMIContext::new()?;
        let svc = create_services(&loc, namespace_path, None, None, None, &ctx.0)?;

        let this = Self {
            _phantom: PhantomData,
            svc,
            ctx,
        };

        this.set_proxy_blanket(AuthLevel::Call)?;
        Ok(this)
    }

    /// Sets the authentication level for this connection by calling [`CoSetProxyBlanket`].
    ///
    /// This directly maps to the Windows [`CoSetProxyBlanket`] function, which configures
    /// the proxy security settings on the WMI service interface.
    ///
    /// Some WMI namespaces require elevated authentication levels to access sensitive data.
    /// For example, `Win32_EncryptableVolume` (BitLocker) requires [`AuthLevel::PktPrivacy`]
    /// to ensure packet-level encryption during communication.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use wmi::*;
    /// # fn main() -> WMIResult<()> {
    /// let wmi_con = WMIConnection::with_namespace_path(
    ///     "ROOT\\CIMV2\\Security\\MicrosoftVolumeEncryption"
    /// )?;
    /// wmi_con.set_proxy_blanket(AuthLevel::PktPrivacy)?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`CoSetProxyBlanket`]: https://learn.microsoft.com/en-us/windows/win32/api/combaseapi/nf-combaseapi-cosetproxyblanket
    pub fn set_proxy_blanket(&self, auth_level: AuthLevel) -> WMIResult<()> {
        let auth_level = RPC_C_AUTHN_LEVEL::from(auth_level);
        debug!("Calling CoSetProxyBlanket with auth_level={}", auth_level.0);

        unsafe {
            CoSetProxyBlanket(
                &self.svc,
                RPC_C_AUTHN_WINNT,
                RPC_C_AUTHZ_NONE,
                None,
                auth_level,
                RPC_C_IMP_LEVEL_IMPERSONATE,
                None,
                EOAC_NONE,
            )?;
        }

        Ok(())
    }

    /// Creates a connection to a remote computer with a default `CIMV2` namespace path.
    /// See [`IWbemLocator::ConnectServer`](https://learn.microsoft.com/en-us/windows/win32/api/wbemcli/nf-wbemcli-iwbemlocator-connectserver).
    ///
    /// # Example
    /// ```no_run
    /// # use wmi::*;
    /// # fn main() -> WMIResult<()> {
    /// let wmi_con = WMIConnection::with_credentials(
    ///     "ServerName",         // Server name or IP address
    ///     Some("username"),
    ///     Some("password"),
    ///     Some("domain"),
    /// )?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_credentials(
        server: &str,
        username: Option<&str>,
        password: Option<&str>,
        domain: Option<&str>,
    ) -> WMIResult<Self> {
        Self::with_credentials_and_namespace(server, "ROOT\\CIMV2", username, password, domain)
    }

    /// Creates a connection to a remote computer with the given namespace path and credentials.
    ///
    /// # Example
    /// ```no_run
    /// # use wmi::*;
    /// # fn main() -> WMIResult<()> {
    /// let wmi_con = WMIConnection::with_credentials_and_namespace(
    ///     "ServerName",         // Server name or IP address
    ///     "ROOT\\CIMV2",        // Namespace path
    ///     Some("username"),
    ///     Some("password"),
    ///     Some("domain"),
    /// )?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_credentials_and_namespace(
        server: &str,
        namespace_path: &str,
        username: Option<&str>,
        password: Option<&str>,
        domain: Option<&str>,
    ) -> WMIResult<Self> {
        let loc = create_locator_or_init()?;

        // Build the full namespace path for remote connection
        let full_namespace = &format!(r"\\{}\{}", server, namespace_path);

        let ctx = WMIContext::new()?;
        let svc = create_services(&loc, full_namespace, username, password, domain, &ctx.0)?;

        let this = Self {
            _phantom: PhantomData,
            svc,
            ctx,
        };

        this.set_proxy_blanket(AuthLevel::PktPrivacy)?;
        Ok(this)
    }
}

fn create_locator() -> windows_core::Result<IWbemLocator> {
    debug!("Calling CoCreateInstance for CLSID_WbemLocator");

    let loc = unsafe { CoCreateInstance(&WbemLocator, None, CLSCTX_INPROC_SERVER)? };

    debug!("Got locator {:?}", loc);

    Ok(loc)
}

#[cfg(target_vendor = "win7")]
fn create_locator_or_init() -> windows_core::Result<IWbemLocator> {
    // `CoIncrementMTAUsage` is not available on Windows 7,
    // so it is the responsibly of the caller to initialize COM and the security layer,
    // using `CoInitializeEx` and `CoInitializeSecurity`, respectively.
    create_locator()
}

#[cfg(not(target_vendor = "win7"))]
fn create_locator_or_init() -> windows_core::Result<IWbemLocator> {
    use windows::Win32::Foundation::RPC_E_TOO_LATE;
    use windows::Win32::System::Com::CoIncrementMTAUsage;

    let loc_res = create_locator();
    match loc_res {
        // If COM is not initialized, initialize it and try again.
        // Based on [`load_factory`](https://github.com/microsoft/windows-rs/blob/945130accc25ac18a47054115e861ca704a37eb5/crates/libs/core/src/imp/factory_cache.rs#L73)
        // from the `windows-rs` crate.
        Err(err) if err.code() == CO_E_NOTINITIALIZED => {
            let _ = unsafe { CoIncrementMTAUsage() }?;
            let sec_result = init_security();

            // If security was initialized already, there's no need to return an error.
            if let Err(err) = &sec_result
                && err.code() != RPC_E_TOO_LATE
            {
                sec_result?;
            }

            create_locator()
        }
        loc_res => loc_res,
    }
}

fn create_services(
    loc: &IWbemLocator,
    namespace_path: &str,
    username: Option<&str>,
    password: Option<&str>,
    authority: Option<&str>,
    ctx: &IWbemContext,
) -> WMIResult<IWbemServices> {
    let namespace_path = BSTR::from(namespace_path);
    let user = BSTR::from(username.unwrap_or_default());
    let password = BSTR::from(password.unwrap_or_default());
    let authority = BSTR::from(authority.unwrap_or_default());

    let svc = unsafe {
        loc.ConnectServer(
            &namespace_path,
            &user,
            &password,
            &BSTR::new(),
            WBEM_FLAG_CONNECT_USE_MAX_WAIT.0,
            &authority,
            ctx,
        )?
    };

    Ok(svc)
}

#[allow(non_snake_case)]
#[allow(non_camel_case_types)]
#[cfg(test)]
mod tests {
    use rusty_fork::rusty_fork_test;

    use super::*;

    #[test]
    fn it_can_set_proxy_blanket() {
        let wmi_con = WMIConnection::new().expect("Failed to create WMI connection");

        // CoSetProxyBlanket doesn't require admin; calling it should always succeed.
        wmi_con
            .set_proxy_blanket(AuthLevel::PktPrivacy)
            .expect("set_proxy_blanket should succeed");
    }

    #[test]
    fn it_can_create_multiple_connections() {
        {
            let _ = WMIConnection::new();
        }
        {
            let _ = WMIConnection::new();
        }
    }

    #[test]
    fn it_can_connect_to_localhost_without_credentials() {
        // Connect to localhost with empty credentials
        let result = WMIConnection::with_credentials("localhost", None, None, None);

        // The connection should succeed
        assert!(
            result.is_ok(),
            "Failed to connect to localhost without credentials: {:?}",
            result.err()
        );
    }

    rusty_fork_test! {
        /// See https://github.com/ohadravid/wmi-rs/issues/136.
        #[test]
        fn it_can_run_as_thread_local_in_non_main_thread() {
            use crate::WMIConnection;

            thread_local! {
                static WMI: Option<WMIConnection> = {
                    let wmi = WMIConnection::new().unwrap();

                    Some(wmi)
                };
            }

            let thread = std::thread::spawn(|| {
                WMI.with(|_wmi| {
                    assert!(true);
                })
            });

            thread.join().unwrap();
        }
    }
}
