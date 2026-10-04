// SPDX-License-Identifier: MIT

use std::env;

use ipnetwork::IpNetwork;
use netlink_packet_route::route::MplsLabel;
use rtnetlink::{new_connection, Error, Handle, RouteMessageBuilder};

#[tokio::main]
async fn main() -> Result<(), ()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        usage();
        return Ok(());
    }

    let input_label = args[1]
        .parse::<u32>()
        .map(|label| MplsLabel {
            label,
            traffic_class: 0,
            bottom_of_stack: true,
            ttl: 0,
        })
        .unwrap_or_else(|_| {
            eprintln!("invalid MPLS input label");
            std::process::exit(1);
        });

    let gateway: IpNetwork = args[2].parse().unwrap_or_else(|_| {
        eprintln!("invalid gateway");
        std::process::exit(1);
    });

    let output_label = args[3]
        .parse::<u32>()
        .map(|label| MplsLabel {
            label,
            traffic_class: 0,
            bottom_of_stack: true,
            ttl: 0,
        })
        .unwrap_or_else(|_| {
            eprintln!("invalid MPLS output label");
            std::process::exit(1);
        });

    let (connection, handle, _) = new_connection().unwrap();
    tokio::spawn(connection);

    if let Err(e) =
        add_route_mpls(input_label, &gateway, output_label, handle.clone())
            .await
    {
        eprintln!("{e}");
    } else {
        println!("Route has been added");
    }
    Ok(())
}

async fn add_route_mpls(
    input_label: MplsLabel,
    gateway: &IpNetwork,
    output_label: MplsLabel,
    handle: Handle,
) -> Result<(), Error> {
    let route = RouteMessageBuilder::<MplsLabel>::new()
        .label(input_label)
        .via(gateway.ip().into())
        .output_mpls(vec![output_label])
        .build();
    handle.route().add(route).execute().await?;
    Ok(())
}

fn usage() {
    eprintln!(
        "\
usage:
    cargo run --example add_route_mpls -- <input_label> <gateway> <output_label>

Note that you need to run this program as root:

    env CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER='sudo -E' \\
        cargo run --example add_route_mpls -- <input_label> <gateway> \
        <output_label>"
    );
}
