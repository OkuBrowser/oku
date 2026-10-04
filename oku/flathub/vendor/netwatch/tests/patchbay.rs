//! Patchbay network simulation tests for netwatch.
//!
//! These tests use the [`patchbay`] crate to create virtual network topologies
//! in Linux user namespaces, testing netwatch's interface and route detection
//! under various network conditions.
//!
//! To run:
//! ```sh
//! cargo make patchbay
//! ```

#![cfg(all(target_os = "linux", not(skip_patchbay)))]

use netwatch::interfaces::State;
use patchbay::{IpSupport, Lab};
use testresult::TestResult;

/// Init the user namespace before any threads are spawned.
#[ctor::ctor(unsafe)]
fn userns_ctor() {
    patchbay::init_userns().expect("failed to init userns");
}

/// Creates a new lab with a single device connected to a router.
///
/// `ip_support` is the IP support of the router to which the device is connected.
///
/// Returns the [`State`] for the device.
async fn state_for_routed_device(ip_support: IpSupport) -> TestResult<State> {
    let lab = Lab::new().await?;
    let router = lab
        .add_router("router")
        .ip_support(ip_support)
        .build()
        .await?;
    let device = lab.add_device("device").uplink(router.id()).build().await?;
    let state = device.spawn(|_| State::new())?.await?;
    Ok(state)
}

/// Netwatch detects a default route on a v4-only network.
#[tokio::test]
async fn default_route_v4_only() -> TestResult {
    let state = state_for_routed_device(IpSupport::V4Only).await?;

    assert!(state.have_v4, "should have v4");
    assert!(!state.have_v6, "should not have v6");
    assert_eq!(state.default_route_interface.as_deref(), Some("eth0"));
    Ok(())
}

/// Netwatch detects a default route on a v6-only network.
#[tokio::test]
async fn default_route_v6_only() -> TestResult {
    let state = state_for_routed_device(IpSupport::V6Only).await?;

    assert!(!state.have_v4, "should not have v4");
    assert!(state.have_v6, "should have v6");
    assert_eq!(state.default_route_interface.as_deref(), Some("eth0"));
    Ok(())
}

/// Netwatch detects a default route on a dual-stack network.
#[tokio::test]
async fn default_route_dual_stack() -> TestResult {
    let state = state_for_routed_device(IpSupport::DualStack).await?;

    assert!(state.have_v4, "should have v4");
    assert!(state.have_v6, "should have v6");
    assert_eq!(state.default_route_interface.as_deref(), Some("eth0"));
    Ok(())
}

/// After replugging from a v4 router to a v6 router, netwatch detects the new
/// default route.
#[tokio::test]
async fn default_route_after_replug_v4_to_v6() -> TestResult {
    let lab = Lab::new().await?;
    let v4_router = lab
        .add_router("v4")
        .ip_support(IpSupport::V4Only)
        .build()
        .await?;
    let v6_router = lab
        .add_router("v6")
        .ip_support(IpSupport::V6Only)
        .build()
        .await?;
    let device = lab
        .add_device("device")
        .uplink(v4_router.id())
        .build()
        .await?;

    // Verify the initial v4 state.
    let state = device.spawn(|_| State::new())?.await?;
    assert!(state.have_v4, "should have v4");
    assert!(!state.have_v6, "should not have v6");
    assert_eq!(state.default_route_interface.as_deref(), Some("eth0"));

    // Replug from the v4 router to the v6 router.
    device.iface("eth0").unwrap().replug(v6_router.id()).await?;

    let state = device.spawn(|_| State::new())?.await?;
    assert!(!state.have_v4, "should not have v4");
    assert!(state.have_v6, "should have v6");
    assert_eq!(state.default_route_interface.as_deref(), Some("eth0"));

    Ok(())
}
