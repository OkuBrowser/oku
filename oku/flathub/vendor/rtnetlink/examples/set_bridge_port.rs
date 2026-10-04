// SPDX-License-Identifier: MIT

use futures_util::stream::TryStreamExt;
use rtnetlink::{
    new_connection, packet_route::link::LinkAttribute, Handle, LinkBridge,
    LinkBridgePort, LinkDummy, LinkUnspec,
};

async fn create_bridge_and_get_index(handle: &Handle) -> Result<u32, String> {
    handle
        .link()
        .add(LinkBridge::new("my-bridge0").up().build())
        .execute()
        .await
        .map_err(|e| format!("{e}"))?;

    let mut bridge_links = handle
        .link()
        .get()
        .match_name("my-bridge0".to_string())
        .execute();
    if let Some(bridge_link) =
        bridge_links.try_next().await.map_err(|e| format!("{e}"))?
    {
        Ok(bridge_link.header.index)
    } else {
        Err("failed to find my-bridge0".into())
    }
}

async fn create_dummy_and_attach_to_bridge(
    handle: &Handle,
    bridge_index: u32,
) -> Result<u32, String> {
    handle
        .link()
        .add(LinkDummy::new("my-dummy0").build())
        .execute()
        .await
        .map_err(|e| format!("{e}"))?;

    handle
        .link()
        .set(
            LinkUnspec::new_with_name("my-dummy0")
                .controller(bridge_index)
                .down()
                .build(),
        )
        .execute()
        .await
        .map_err(|e| format!("{e}"))?;

    let mut dummy_links = handle
        .link()
        .get()
        .match_name("my-dummy0".to_string())
        .execute();
    if let Some(dummy_link) =
        dummy_links.try_next().await.map_err(|e| format!("{e}"))?
    {
        Ok(dummy_link.header.index)
    } else {
        Err("failed to find my-dummy0".into())
    }
}

async fn set_bridge_port(
    handle: &Handle,
    port_index: u32,
) -> Result<(), String> {
    let message = LinkBridgePort::new(port_index)
        .priority(10)
        .hairpin(true)
        .build();

    handle
        .link()
        .set_port(message)
        .execute()
        .await
        .map_err(|e| format!("{e}"))?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), String> {
    let (connection, handle, _) = new_connection().unwrap();
    tokio::spawn(connection);

    let bridge_index = create_bridge_and_get_index(&handle).await?;

    let port_index =
        create_dummy_and_attach_to_bridge(&handle, bridge_index).await?;
    set_bridge_port(&handle, port_index)
        .await
        .map_err(|e| e.to_string())?;

    let mut dummy_links = handle
        .link()
        .get()
        .match_name("my-dummy0".to_string())
        .execute();
    if let Some(dummy_link) =
        dummy_links.try_next().await.map_err(|e| format!("{e}"))?
    {
        for nla in dummy_link.attributes {
            if let LinkAttribute::LinkInfo(i) = &nla {
                println!("{:?}", i);
            }
        }
        Ok(())
    } else {
        Err("failed to find my-dummy0".into())
    }
}
