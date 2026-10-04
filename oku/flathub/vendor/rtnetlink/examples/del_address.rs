// SPDX-License-Identifier: MIT

use std::{
    env,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
};

use futures_util::stream::TryStreamExt;
use ipnetwork::IpNetwork;
use rtnetlink::{new_connection, AddressMessageBuilder, Error, Handle};

#[tokio::main]
async fn main() -> Result<(), ()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        usage();
        return Ok(());
    }

    let link_name = &args[1];
    let ip: IpNetwork = args[2].parse().unwrap_or_else(|_| {
        eprintln!("invalid address");
        std::process::exit(1);
    });

    let (connection, handle, _) = new_connection().unwrap();
    tokio::spawn(connection);

    if let Err(e) = del_address(link_name, ip, handle.clone()).await {
        eprintln!("{e}");
    }
    Ok(())
}

async fn del_address(
    link_name: &str,
    ip: IpNetwork,
    handle: Handle,
) -> Result<(), Error> {
    let mut links = handle
        .link()
        .get()
        .match_name(link_name.to_string())
        .execute();
    if let Some(link) = links.try_next().await? {
        let index = link.header.index;
        let address = ip.ip();
        let prefix_len = ip.prefix();
        let message = match address {
            IpAddr::V4(address) => AddressMessageBuilder::<Ipv4Addr>::new()
                .index(index)
                .address(address, prefix_len)
                .build(),
            IpAddr::V6(address) => AddressMessageBuilder::<Ipv6Addr>::new()
                .index(index)
                .address(address, prefix_len)
                .build(),
        };
        handle.address().del(message).execute().await?
    }
    Ok(())
}

fn usage() {
    eprintln!(
        "usage:
    cargo run --example del_address -- <link_name> <ip_address>

Note that you need to run this program as root. Instead of running cargo as root,
build the example normally:

    cd rtnetlink ; cargo build --example del_address

Then find the binary in the target directory:

    cd ../target/debug/example ; sudo ./del_address <link_name> <ip_address>"
    );
}
