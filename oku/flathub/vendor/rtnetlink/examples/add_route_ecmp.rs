// SPDX-License-Identifier: MIT

use std::{env, net::Ipv4Addr};

use ipnetwork::Ipv4Network;
use rtnetlink::{
    new_connection, Error, Handle, RouteMessageBuilder, RouteNextHopBuilder,
};

#[tokio::main]
async fn main() -> Result<(), ()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 7 {
        usage();
        return Ok(());
    }

    let iface_index = args[1].parse::<u32>().unwrap_or_else(|_| {
        eprintln!("invalid interface index");
        std::process::exit(1);
    });

    let dst = args[2].parse::<Ipv4Network>().unwrap_or_else(|_| {
        eprintln!("invalid destination");
        std::process::exit(1);
    });

    let via1 = args[3].parse::<Ipv4Addr>().unwrap_or_else(|_| {
        eprintln!("invalid via");
        std::process::exit(1);
    });

    let weight1 = args[4].parse::<u8>().unwrap_or_else(|_| {
        eprintln!("invalid weight");
        std::process::exit(1);
    });

    let via2 = args[5].parse::<Ipv4Addr>().unwrap_or_else(|_| {
        eprintln!("invalid via");
        std::process::exit(1);
    });

    let weight2 = args[6].parse::<u8>().unwrap_or_else(|_| {
        eprintln!("invalid weight");
        std::process::exit(1);
    });

    let (connection, handle, _) = new_connection().unwrap();
    tokio::spawn(connection);

    if let Err(e) =
        add_route_ecmp(iface_index, &dst, via1, weight1, via2, weight2, handle)
            .await
    {
        eprintln!("{e}");
    } else {
        println!("Route has been added");
    }
    Ok(())
}

async fn add_route_ecmp(
    iface_index: u32,
    dst: &Ipv4Network,
    via1: Ipv4Addr,
    weight1: u8,
    via2: Ipv4Addr,
    weight2: u8,
    handle: Handle,
) -> Result<(), Error> {
    let route = RouteMessageBuilder::<Ipv4Addr>::new()
        .destination_prefix(dst.ip(), dst.prefix())
        .multipath(vec![
            RouteNextHopBuilder::new_ipv4()
                .interface(iface_index)
                .weight(weight1)
                .via(via1.into())
                .unwrap()
                .build(),
            RouteNextHopBuilder::new_ipv4()
                .interface(iface_index)
                .weight(weight2)
                .via(via2.into())
                .unwrap()
                .build(),
        ])
        .build();
    handle.route().add(route).execute().await?;
    Ok(())
}

fn usage() {
    eprintln!(
        "\
usage:
    cargo run --example add_route_ecmp -- <iface_index> <dst> <via> <weight> \\
    <via> <weight>

Note that you need to run this program as root:

    env CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER='sudo' \\
        cargo run --example add_route_ecmp -- <iface_index> \\
        <dst> <via> <weight> <via> <weight>"
    );
}
