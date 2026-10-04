//! Tests that use the [`iroh_gossip::proto::sim::Simulator`].

use std::{env, fmt, str::FromStr, time::Duration};

use iroh_gossip::proto::{
    sim::{BootstrapMode, LatencyConfig, NetworkConfig, Simulator, SimulatorConfig},
    Config,
};

#[test]
// #[traced_test]
fn big_hyparview() {
    tracing_subscriber::fmt::try_init().ok();
    let mut proto = Config::default();
    proto.membership.shuffle_interval = Duration::from_secs(5);
    let config = SimulatorConfig::from_env();
    let bootstrap = BootstrapMode::default();
    let network_config = NetworkConfig {
        proto,
        latency: LatencyConfig::default(),
    };
    let mut simulator = Simulator::new(config, network_config);
    simulator.bootstrap(bootstrap);
    let state = simulator.report();
    println!("{state}");
    assert!(!state.has_peers_with_no_neighbors());
}

#[test]
// #[traced_test]
fn big_multiple_sender() {
    tracing_subscriber::fmt::try_init().ok();

    let network_config = NetworkConfig::default();
    let config = SimulatorConfig::from_env();
    let bootstrap = BootstrapMode::default();
    let mut simulator = Simulator::new(config, network_config);

    simulator.bootstrap(bootstrap);

    let rounds = read_var("ROUNDS", 30);
    for i in 0..rounds {
        let from = simulator.random_peer();
        let message = format!("m{i}").into_bytes().into();
        let messages = vec![(from, message)];
        simulator.gossip_round(messages);
    }
    let avg = simulator.round_stats_average().mean;
    println!(
        "average with {} peers after {} rounds:\n{}",
        simulator.peer_count(),
        rounds,
        avg
    );
    println!("{}", simulator.report());
    assert!(avg.ldh < 18.);
    assert!(avg.rmr < 1.);
    assert_eq!(avg.missed, 0.0);
}

#[test]
// #[traced_test]
fn big_single_sender() {
    tracing_subscriber::fmt::try_init().ok();

    let network_config = NetworkConfig::default();

    let config = SimulatorConfig::from_env();
    let bootstrap = BootstrapMode::default();
    let rounds = read_var("ROUNDS", 30);
    let mut simulator = Simulator::new(config, network_config);
    simulator.bootstrap(bootstrap);
    let from = simulator.random_peer();
    for i in 0..rounds {
        let message = format!("m{i}").into_bytes().into();
        let messages = vec![(from, message)];
        simulator.gossip_round(messages);
    }
    let avg = simulator.round_stats_average().mean;
    println!(
        "average with {} peers after {} rounds:\n{}",
        simulator.peer_count(),
        rounds,
        avg
    );
    println!("{}", simulator.report());
    assert!(avg.ldh < 15.);
    assert!(avg.rmr < 0.2);
    assert_eq!(avg.missed, 0.0);
}

#[test]
// #[traced_test]
fn big_burst() {
    tracing_subscriber::fmt::try_init().ok();
    let network_config = NetworkConfig::default();
    let config = SimulatorConfig::from_env();
    let bootstrap = BootstrapMode::default();
    let rounds = read_var("ROUNDS", 5);

    let mut simulator = Simulator::new(config, network_config);
    simulator.bootstrap(bootstrap);
    let messages_per_peer = read_var("MESSAGES_PER_PEER", 1);
    for i in 0..rounds {
        let mut messages = vec![];
        for id in simulator.network.peer_ids() {
            for j in 0..messages_per_peer {
                let message: bytes::Bytes = format!("{i}:{j}.{id}").into_bytes().into();
                messages.push((id, message));
            }
        }
        simulator.gossip_round(messages);
    }
    let avg = simulator.round_stats_average().mean;
    println!(
        "average with {} peers after {} rounds:\n{}",
        simulator.peer_count(),
        rounds,
        avg
    );
    println!("{}", simulator.report());
    assert!(avg.ldh < 30.);
    assert!(avg.rmr < 3.);
    assert_eq!(avg.missed, 0.0);
}

fn read_var<T: FromStr<Err: fmt::Display + fmt::Debug>>(name: &str, default: T) -> T {
    env::var(name)
        .map(|x| {
            x.parse()
                .unwrap_or_else(|_| panic!("Failed to parse environment variable {name}"))
        })
        .unwrap_or(default)
}
