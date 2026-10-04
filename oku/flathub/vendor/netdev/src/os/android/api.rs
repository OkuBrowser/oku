use crate::stats::counters::InterfaceStats;
use jni::JavaVM;
use jni::objects::{JByteArray, JObject, JObjectArray, JString, JValue};
use std::collections::{HashMap, HashSet};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::panic::{self, PanicHookInfo};
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

#[derive(Clone, Debug, Default)]
pub(crate) struct InterfaceExtras {
    pub(crate) transmit_speed: Option<u64>,
    pub(crate) receive_speed: Option<u64>,
    pub(crate) auto_negotiate: Option<bool>,
    pub(crate) dhcp_v4_enabled: Option<bool>,
    pub(crate) dhcp_v6_enabled: Option<bool>,
    pub(crate) stats: Option<InterfaceStats>,
    #[cfg(feature = "gateway")]
    pub(crate) dns_servers: Vec<IpAddr>,
}

pub(crate) fn get_interface_stats(name: &str) -> Option<InterfaceStats> {
    with_android_env(|env, _| {
        let name = env.new_string(name).ok()?;
        let rx_bytes = call_static_long(
            env,
            "android/net/TrafficStats",
            "getRxBytes",
            "(Ljava/lang/String;)J",
            &[JValue::Object(name.as_ref())],
        )?;
        let tx_bytes = call_static_long(
            env,
            "android/net/TrafficStats",
            "getTxBytes",
            "(Ljava/lang/String;)J",
            &[JValue::Object(name.as_ref())],
        )?;

        if rx_bytes < 0 || tx_bytes < 0 {
            return None;
        }

        Some(InterfaceStats {
            rx_bytes: rx_bytes as u64,
            tx_bytes: tx_bytes as u64,
            timestamp: Some(SystemTime::now()),
        })
    })
}

pub(crate) fn collect_interface_extras(if_names: &[String]) -> HashMap<String, InterfaceExtras> {
    let mut extras = HashMap::new();

    with_android_env(|env, context| {
        populate_traffic_stats(env, if_names, &mut extras);

        let mut wifi_ifaces = HashSet::new();
        populate_connectivity_extras(env, context, &mut extras, &mut wifi_ifaces);
        populate_wifi_speed(env, context, &mut extras, &wifi_ifaces);

        Some(())
    });

    extras
}

fn with_android_env<T, F>(f: F) -> Option<T>
where
    F: FnOnce(&mut jni::AttachGuard<'_>, &JObject<'static>) -> Option<T>,
{
    let ctx = try_android_context()?;
    let vm = unsafe { JavaVM::from_raw(ctx.vm().cast()) }.ok()?;
    let mut env = vm.attach_current_thread().ok()?;
    let context = unsafe { JObject::from_raw(ctx.context().cast()) };
    if is_null(&env, &context) {
        return None;
    }
    f(&mut env, &context)
}

fn try_android_context() -> Option<ndk_context::AndroidContext> {
    static ANDROID_CONTEXT_AVAILABLE: OnceLock<bool> = OnceLock::new();

    if *ANDROID_CONTEXT_AVAILABLE.get_or_init(try_init_android_context) {
        Some(ndk_context::android_context())
    } else {
        None
    }
}

fn try_init_android_context() -> bool {
    static PANIC_HOOK_GUARD: Mutex<()> = Mutex::new(());

    let Ok(_guard) = PANIC_HOOK_GUARD.lock() else {
        return false;
    };
    let previous_hook = panic::take_hook();
    panic::set_hook(Box::new(silent_panic_hook));
    let result = panic::catch_unwind(ndk_context::android_context).is_ok();
    panic::set_hook(previous_hook);
    result
}

fn silent_panic_hook(_info: &PanicHookInfo<'_>) {}

fn populate_traffic_stats(
    env: &mut jni::AttachGuard<'_>,
    if_names: &[String],
    extras: &mut HashMap<String, InterfaceExtras>,
) {
    for if_name in if_names {
        if let Some(stats) = get_traffic_stats(env, if_name) {
            extras.entry(if_name.clone()).or_default().stats = Some(stats);
        }
    }
}

fn populate_connectivity_extras(
    env: &mut jni::AttachGuard<'_>,
    context: &JObject<'static>,
    extras: &mut HashMap<String, InterfaceExtras>,
    wifi_ifaces: &mut HashSet<String>,
) {
    let Some(connectivity_manager) = get_system_service(env, context, "CONNECTIVITY_SERVICE")
    else {
        return;
    };
    if is_null(env, &connectivity_manager) {
        return;
    }

    let Some(networks_obj) = call_object_method(
        env,
        &connectivity_manager,
        "getAllNetworks",
        "()[Landroid/net/Network;",
        &[],
    ) else {
        return;
    };
    if is_null(env, &networks_obj) {
        return;
    }

    let networks = JObjectArray::from(networks_obj);
    let Ok(length) = env.get_array_length(&networks) else {
        return;
    };

    let transport_wifi = get_static_int(
        env,
        "android/net/NetworkCapabilities",
        "TRANSPORT_WIFI",
        "I",
    );

    for index in 0..length {
        let Ok(network) = env.get_object_array_element(&networks, index) else {
            clear_pending_exception(env);
            continue;
        };
        if is_null(env, &network) {
            continue;
        }

        let Some(link_properties) = call_object_method(
            env,
            &connectivity_manager,
            "getLinkProperties",
            "(Landroid/net/Network;)Landroid/net/LinkProperties;",
            &[JValue::Object(network.as_ref())],
        ) else {
            continue;
        };
        if is_null(env, &link_properties) {
            continue;
        }

        let Some(if_name) = call_string_method(
            env,
            &link_properties,
            "getInterfaceName",
            "()Ljava/lang/String;",
            &[],
        ) else {
            continue;
        };

        #[cfg(feature = "gateway")]
        if let Some(dns_servers) = get_dns_servers(env, &link_properties) {
            extras.entry(if_name.clone()).or_default().dns_servers = dns_servers;
        }

        if has_dhcp_v4(env, &link_properties) {
            extras.entry(if_name.clone()).or_default().dhcp_v4_enabled = Some(true);
        }

        if let Some(transport_wifi) = transport_wifi
            && network_has_transport(env, &connectivity_manager, &network, transport_wifi)
        {
            wifi_ifaces.insert(if_name);
        }
    }
}

fn populate_wifi_speed(
    env: &mut jni::AttachGuard<'_>,
    context: &JObject<'static>,
    extras: &mut HashMap<String, InterfaceExtras>,
    wifi_ifaces: &HashSet<String>,
) {
    if wifi_ifaces.is_empty() {
        return;
    }

    let Some(wifi_manager) = get_system_service(env, context, "WIFI_SERVICE") else {
        return;
    };
    if is_null(env, &wifi_manager) {
        return;
    }

    let Some(wifi_info) = call_object_method(
        env,
        &wifi_manager,
        "getConnectionInfo",
        "()Landroid/net/wifi/WifiInfo;",
        &[],
    ) else {
        return;
    };
    if is_null(env, &wifi_info) {
        return;
    }

    let tx_speed = call_int_method(env, &wifi_info, "getTxLinkSpeedMbps", "()I", &[])
        .filter(|speed| *speed > 0)
        .map(|speed| (speed as u64) * 1_000_000);
    let rx_speed = call_int_method(env, &wifi_info, "getRxLinkSpeedMbps", "()I", &[])
        .filter(|speed| *speed > 0)
        .map(|speed| (speed as u64) * 1_000_000);
    let link_speed = call_int_method(env, &wifi_info, "getLinkSpeed", "()I", &[])
        .filter(|speed| *speed > 0)
        .map(|speed| (speed as u64) * 1_000_000);

    for if_name in wifi_ifaces {
        let extra = extras.entry(if_name.clone()).or_default();
        if extra.transmit_speed.is_none() {
            extra.transmit_speed = tx_speed.or(link_speed);
        }
        if extra.receive_speed.is_none() {
            extra.receive_speed = rx_speed.or(link_speed);
        }
    }
}

fn get_traffic_stats(env: &mut jni::AttachGuard<'_>, if_name: &str) -> Option<InterfaceStats> {
    let if_name = env.new_string(if_name).ok()?;
    let rx_bytes = call_static_long(
        env,
        "android/net/TrafficStats",
        "getRxBytes",
        "(Ljava/lang/String;)J",
        &[JValue::Object(if_name.as_ref())],
    )?;
    let tx_bytes = call_static_long(
        env,
        "android/net/TrafficStats",
        "getTxBytes",
        "(Ljava/lang/String;)J",
        &[JValue::Object(if_name.as_ref())],
    )?;

    if rx_bytes < 0 || tx_bytes < 0 {
        return None;
    }

    Some(InterfaceStats {
        rx_bytes: rx_bytes as u64,
        tx_bytes: tx_bytes as u64,
        timestamp: Some(SystemTime::now()),
    })
}

fn get_system_service(
    env: &mut jni::AttachGuard<'_>,
    context: &JObject<'static>,
    field_name: &str,
) -> Option<JObject<'static>> {
    let service = env
        .get_static_field("android/content/Context", field_name, "Ljava/lang/String;")
        .ok()?
        .l()
        .ok()?;
    let service_obj = call_object_method(
        env,
        context,
        "getSystemService",
        "(Ljava/lang/String;)Ljava/lang/Object;",
        &[JValue::Object(service.as_ref())],
    )?;
    Some(unsafe { JObject::from_raw(service_obj.into_raw()) })
}

#[cfg(feature = "gateway")]
fn get_dns_servers(
    env: &mut jni::AttachGuard<'_>,
    link_properties: &JObject<'_>,
) -> Option<Vec<IpAddr>> {
    let dns_list = call_object_method(
        env,
        link_properties,
        "getDnsServers",
        "()Ljava/util/List;",
        &[],
    )?;
    if is_null(env, &dns_list) {
        return Some(Vec::new());
    }

    let size = call_int_method(env, &dns_list, "size", "()I", &[])? as i32;
    let mut dns_servers = Vec::new();

    for index in 0..size {
        let Some(entry) = call_object_method(
            env,
            &dns_list,
            "get",
            "(I)Ljava/lang/Object;",
            &[JValue::Int(index)],
        ) else {
            continue;
        };
        if let Some(ip) = inet_address_to_ip(env, &entry)
            && !dns_servers.contains(&ip)
        {
            dns_servers.push(ip);
        }
    }

    Some(dns_servers)
}

fn has_dhcp_v4(env: &mut jni::AttachGuard<'_>, link_properties: &JObject<'_>) -> bool {
    let Some(server) = call_object_method(
        env,
        link_properties,
        "getDhcpServerAddress",
        "()Ljava/net/Inet4Address;",
        &[],
    ) else {
        return false;
    };
    !is_null(env, &server)
}

fn network_has_transport(
    env: &mut jni::AttachGuard<'_>,
    connectivity_manager: &JObject<'_>,
    network: &JObject<'_>,
    transport: i32,
) -> bool {
    let Some(capabilities) = call_object_method(
        env,
        connectivity_manager,
        "getNetworkCapabilities",
        "(Landroid/net/Network;)Landroid/net/NetworkCapabilities;",
        &[JValue::Object(network.as_ref())],
    ) else {
        return false;
    };
    if is_null(env, &capabilities) {
        return false;
    }

    call_bool_method(
        env,
        &capabilities,
        "hasTransport",
        "(I)Z",
        &[JValue::Int(transport)],
    )
    .unwrap_or(false)
}

fn inet_address_to_ip(
    env: &mut jni::AttachGuard<'_>,
    inet_address: &JObject<'_>,
) -> Option<IpAddr> {
    let addr = call_object_method(env, inet_address, "getAddress", "()[B", &[])?;
    if is_null(env, &addr) {
        return None;
    }

    let bytes = env.convert_byte_array(&JByteArray::from(addr)).ok()?;
    match bytes.as_slice() {
        [a, b, c, d] => Some(IpAddr::V4(Ipv4Addr::new(*a, *b, *c, *d))),
        bytes if bytes.len() == 16 => {
            let mut octets = [0u8; 16];
            octets.copy_from_slice(bytes);
            Some(IpAddr::V6(Ipv6Addr::from(octets)))
        }
        _ => None,
    }
}

fn call_object_method(
    env: &mut jni::AttachGuard<'_>,
    obj: &JObject<'_>,
    name: &str,
    sig: &str,
    args: &[JValue],
) -> Option<JObject<'static>> {
    let value = env.call_method(obj, name, sig, args);
    match value {
        Ok(value) => value
            .l()
            .ok()
            .map(|obj| unsafe { JObject::from_raw(obj.into_raw()) }),
        Err(_) => {
            clear_pending_exception(env);
            None
        }
    }
}

fn call_string_method(
    env: &mut jni::AttachGuard<'_>,
    obj: &JObject<'_>,
    name: &str,
    sig: &str,
    args: &[JValue],
) -> Option<String> {
    let value = call_object_method(env, obj, name, sig, args)?;
    if is_null(env, &value) {
        return None;
    }

    let value = JString::from(value);
    env.get_string(&value).ok().map(|s| s.into())
}

fn call_int_method(
    env: &mut jni::AttachGuard<'_>,
    obj: &JObject<'_>,
    name: &str,
    sig: &str,
    args: &[JValue],
) -> Option<i32> {
    let value = env.call_method(obj, name, sig, args);
    match value {
        Ok(value) => value.i().ok(),
        Err(_) => {
            clear_pending_exception(env);
            None
        }
    }
}

fn call_bool_method(
    env: &mut jni::AttachGuard<'_>,
    obj: &JObject<'_>,
    name: &str,
    sig: &str,
    args: &[JValue],
) -> Option<bool> {
    let value = env.call_method(obj, name, sig, args);
    match value {
        Ok(value) => value.z().ok(),
        Err(_) => {
            clear_pending_exception(env);
            None
        }
    }
}

fn call_static_long(
    env: &mut jni::AttachGuard<'_>,
    class: &str,
    name: &str,
    sig: &str,
    args: &[JValue],
) -> Option<i64> {
    let value = env.call_static_method(class, name, sig, args);
    match value {
        Ok(value) => value.j().ok(),
        Err(_) => {
            clear_pending_exception(env);
            None
        }
    }
}

fn get_static_int(
    env: &mut jni::AttachGuard<'_>,
    class: &str,
    name: &str,
    sig: &str,
) -> Option<i32> {
    let value = env.get_static_field(class, name, sig);
    match value {
        Ok(value) => value.i().ok(),
        Err(_) => {
            clear_pending_exception(env);
            None
        }
    }
}

fn is_null(env: &jni::AttachGuard<'_>, obj: &JObject<'_>) -> bool {
    let null = JObject::null();
    env.is_same_object(obj, &null).unwrap_or(true)
}

fn clear_pending_exception(env: &jni::AttachGuard<'_>) {
    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }
}
