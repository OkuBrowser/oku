// SPDX-License-Identifier: MIT

use rtnetlink::{
    new_connection,
    packet_route::link::{NetkitMode, NetkitPolicy, NetkitScrub},
    LinkNetkit,
};

#[tokio::main]
async fn main() -> Result<(), String> {
    let (connection, handle, _) = new_connection().unwrap();
    tokio::spawn(connection);

    // Create a fully-configured netkit pair in L3 mode
    // This example shows all available configuration options
    handle
        .link()
        .add(
            LinkNetkit::new("netkit0", "netkit0-peer", NetkitMode::L3)
                .policy(NetkitPolicy::Pass) // Forward packets normally on primary
                .peer_policy(NetkitPolicy::Pass) // Forward packets normally on peer
                .scrub(NetkitScrub::Default) // Apply default packet scrubbing
                .peer_scrub(NetkitScrub::Default) // Apply scrubbing on peer
                .headroom(256) // Reserve 256 bytes of headroom
                .tailroom(128) // Reserve 128 bytes of tailroom
                .up() // Bring the interface up
                .build(),
        )
        .execute()
        .await
        .map_err(|e| format!("{e}"))?;

    println!("Created netkit pair: netkit0 <-> netkit0-peer");
    println!("  Mode: L3 (IP mode)");
    println!("  Policy: Pass (Forward)");
    println!("  Scrub: Default");
    println!("  Headroom: 256 bytes");
    println!("  Tailroom: 128 bytes");

    Ok(())
}
