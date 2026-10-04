// SPDX-License-Identifier: MIT

use rtnetlink::{new_connection, packet_route::link::NetkitMode, LinkNetkit};

#[tokio::main]
async fn main() -> Result<(), String> {
    let (connection, handle, _) = new_connection().unwrap();
    tokio::spawn(connection);

    // Create a netkit pair in L3 mode
    handle
        .link()
        .add(LinkNetkit::new("netkit0", "netkit0-peer", NetkitMode::L3).build())
        .execute()
        .await
        .map_err(|e| format!("{e}"))
}
