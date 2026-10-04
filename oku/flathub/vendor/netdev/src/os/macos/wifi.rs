use objc2::rc::autoreleasepool;
use objc2_core_wlan::CWWiFiClient;
use objc2_foundation::NSString;

/// Returns the macOS Wi-Fi transmit rate in bps for the given interface name.
///
/// The CoreWLAN calls below produce autoreleased Obj-C/XPC objects
/// (`CWFRequestParameters`, `CWFInterface`, the `CWFXPCRequestProtocolCoreWLAN`
/// proxy). This is called from long-lived threads with no draining autorelease
/// pool (e.g. `netwatch`'s interface monitor re-enumerates on every network
/// event), so without an explicit pool those objects are added to a pool that
/// never drains and accumulate without bound — a steady multi-MB/hour macOS
/// leak. A scoped pool per call frees them immediately.
pub(crate) fn get_wifi_transmit_rate(iface_name: &str) -> Option<u64> {
    autoreleasepool(|_pool| {
        let client = unsafe { CWWiFiClient::sharedWiFiClient() };
        let name = NSString::from_str(iface_name);

        let wifi_iface = unsafe { client.interfaceWithName(Some(&name)) };
        wifi_iface.map(|i| {
            let transmit_rate = unsafe { i.transmitRate() };
            (transmit_rate * 1e6) as u64
        })
    })
}
