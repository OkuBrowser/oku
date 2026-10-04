use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use clap::Parser;
use comfy_table::{presets::NOTHING, Cell, CellAlignment, Table};
use iroh_gossip::proto::sim::{
    BootstrapMode, NetworkConfig, RoundStats, RoundStatsAvg, RoundStatsDiff, Simulator,
    SimulatorConfig,
};
use n0_error::{Result, StackResultExt, StdResultExt};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use tracing::{error_span, info, warn};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[allow(clippy::enum_variant_names)]
enum Simulation {
    /// A single sender broadcasts a single message per round.
    GossipSingle,
    /// Each round a different sender is chosen at random, and broadcasts a single message
    GossipMulti,
    /// Each round, all peers broadcast a single message simultaneously.
    GossipAll,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ScenarioDescription {
    sim: Simulation,
    nodes: u32,
    #[serde(default)]
    bootstrap: BootstrapMode,
    #[serde(default = "defaults::rounds")]
    rounds: u32,
    config: Option<NetworkConfig>,
}

impl ScenarioDescription {
    pub fn label(&self) -> String {
        let &ScenarioDescription {
            sim,
            nodes,
            rounds,
            config: _,
            bootstrap: _,
        } = &self;
        format!("{sim:?}-n{nodes}-r{rounds}")
    }
}

mod defaults {
    pub fn rounds() -> u32 {
        30
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct SimConfig {
    seeds: Vec<u64>,
    config: Option<NetworkConfig>,
    scenario: Vec<ScenarioDescription>,
}

#[derive(Debug, Parser)]
struct Cli {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Debug, Parser)]
enum Command {
    /// Run simulations
    Run {
        #[clap(short, long)]
        config_path: PathBuf,
        #[clap(short, long)]
        out_dir: Option<PathBuf>,
        #[clap(short, long)]
        baseline: Option<PathBuf>,
        #[clap(short, long)]
        single_threaded: bool,
        #[clap(short, long)]
        filter: Vec<String>,
    },
    /// Compare simulation runs
    Compare {
        baseline: PathBuf,
        current: PathBuf,
        #[clap(short, long)]
        filter: Vec<String>,
    },
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args: Cli = Cli::parse();
    match args.command {
        Command::Run {
            config_path,
            out_dir,
            baseline,
            single_threaded,
            filter,
        } => {
            let config_text = std::fs::read_to_string(&config_path)
                .with_std_context(|_| format!("read config {}", config_path.display()))?;
            let config: SimConfig = toml::from_str(&config_text).std_context("parse config")?;

            let base_config = config.config.unwrap_or_default();
            info!("base config: {base_config:?}");
            let seeds = config.seeds;
            let mut scenarios = config.scenario;
            for scenario in scenarios.iter_mut() {
                scenario.config.get_or_insert_with(|| base_config.clone());
            }

            if let Some(out_dir) = out_dir.as_ref() {
                std::fs::create_dir_all(out_dir)
                    .with_std_context(|_| format!("create output dir {}", out_dir.display()))?;
            }

            let filter_fn = |s: &ScenarioDescription| {
                let label = s.label();
                if filter.is_empty() {
                    true
                } else {
                    filter.iter().any(|x| x == &label)
                }
            };

            let results: Result<Vec<_>> = if !single_threaded {
                scenarios
                    .into_par_iter()
                    .filter(filter_fn)
                    .map(|scenario| run_and_save_simulation(scenario, &seeds, out_dir.as_ref()))
                    .collect()
            } else {
                scenarios
                    .into_iter()
                    .filter(filter_fn)
                    .map(|scenario| run_and_save_simulation(scenario, &seeds, out_dir.as_ref()))
                    .collect()
            };
            let mut results = results?;
            results.sort_by_key(|a| a.scenario.label());
            for result in results {
                print_result(&result);
            }
            if let (Some(baseline), Some(out_dir)) = (baseline, out_dir) {
                compare_dirs(baseline, out_dir, filter)?;
            }
        }
        Command::Compare {
            baseline,
            current,
            filter,
        } => {
            compare_dirs(baseline, current, filter)?;
        }
    }

    Ok(())
}

fn run_and_save_simulation(
    scenario: ScenarioDescription,
    seeds: &[u64],
    out_dir: Option<impl AsRef<Path>>,
) -> Result<SimulationResults> {
    let label = scenario.label();

    if let Some(out_dir) = out_dir.as_ref() {
        let path = out_dir.as_ref().join(format!("{label}.config.toml"));
        let encoded = toml::to_string(&scenario).std_context("encode scenario")?;
        std::fs::write(&path, encoded)
            .with_std_context(|_| format!("write scenario {}", &path.display()))?;
    }

    let result = run_simulation(seeds, scenario);

    if let Some(out_dir) = out_dir.as_ref() {
        let path = out_dir.as_ref().join(format!("{label}.results.json"));
        let encoded = serde_json::to_string(&result).std_context("encode results")?;
        std::fs::write(&path, encoded)
            .with_std_context(|_| format!("write results {}", path.display()))?;
    }

    Ok(result)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct SimulationResults {
    scenario: ScenarioDescription,
    /// Maps seeds to results
    results: HashMap<u64, RoundStatsAvg>,
    average: Option<RoundStatsAvg>,
}

impl SimulationResults {
    fn load_from_file(path: impl AsRef<Path>) -> Result<Self> {
        let s = std::fs::read_to_string(path.as_ref())
            .with_std_context(|_| format!("read results {}", path.as_ref().display()))?;
        let out = serde_json::from_str(&s).std_context("decode results")?;
        Ok(out)
    }
}

fn run_simulation(seeds: &[u64], scenario: ScenarioDescription) -> SimulationResults {
    let mut results = HashMap::new();
    let network_config = scenario.config.clone().unwrap_or_default();
    for &seed in seeds {
        let span = error_span!("sim", name=%scenario.label(), %seed);
        let _guard = span.enter();

        let sim_config = SimulatorConfig {
            rng_seed: seed,
            peers: scenario.nodes as usize,
            ..Default::default()
        };
        let bootstrap = scenario.bootstrap.clone();
        let mut simulator = Simulator::new(sim_config, network_config.clone());
        info!("start");
        let outcome = simulator.bootstrap(bootstrap);

        if outcome.has_peers_with_no_neighbors() {
            warn!("not all nodes active after bootstrap: {outcome:?}");
        } else {
            info!("bootstrapped, all nodes active");
        }
        let result = match scenario.sim {
            Simulation::GossipSingle => BigSingle.run(simulator, scenario.rounds as usize),
            Simulation::GossipMulti => BigMulti.run(simulator, scenario.rounds as usize),
            Simulation::GossipAll => BigAll.run(simulator, scenario.rounds as usize),
        };
        info!("done");
        results.insert(seed, result);
    }

    let stats: Vec<_> = results.values().cloned().collect();
    let average = if !stats.is_empty() {
        let avg = RoundStatsAvg::avg(&stats);
        Some(avg)
    } else {
        None
    };
    SimulationResults {
        average,
        results,
        scenario,
    }
}

fn print_result(r: &SimulationResults) {
    let seeds = r.results.len();
    println!("{} with {seeds} seeds", r.scenario.label());
    let Some(avg) = r.average.as_ref() else {
        println!("no results, simulation did not complete");
        return;
    };
    let mut table = Table::new();
    let header = ["", "RMR", "LDH", "missed", "duration"]
        .into_iter()
        .map(|s| Cell::new(s).set_alignment(CellAlignment::Right));
    table
        .load_preset(NOTHING)
        .set_header(header)
        .add_row(fmt_round("mean", &avg.mean))
        .add_row(fmt_round("max", &avg.max))
        .add_row(fmt_round("min", &avg.min));
    println!("{table}");
    if avg.max.missed > 0.0 {
        println!("WARN: Messages were missed!")
    }
    println!();
}

trait Scenario {
    fn run(self, sim: Simulator, rounds: usize) -> RoundStatsAvg;
}

struct BigSingle;
impl Scenario for BigSingle {
    fn run(self, mut simulator: Simulator, rounds: usize) -> RoundStatsAvg {
        let from = simulator.random_peer();
        for i in 0..rounds {
            let message = format!("m{i}").into_bytes().into();
            let messages = vec![(from, message)];
            simulator.gossip_round(messages);
        }
        simulator.round_stats_average()
    }
}

struct BigMulti;
impl Scenario for BigMulti {
    fn run(self, mut simulator: Simulator, rounds: usize) -> RoundStatsAvg {
        for i in 0..rounds {
            let from = simulator.random_peer();
            let message = format!("m{i}").into_bytes().into();
            let messages = vec![(from, message)];
            simulator.gossip_round(messages);
        }
        simulator.round_stats_average()
    }
}

struct BigAll;
impl Scenario for BigAll {
    fn run(self, mut simulator: Simulator, rounds: usize) -> RoundStatsAvg {
        let messages_per_peer = 1;
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
        simulator.round_stats_average()
    }
}

fn compare_dirs(baseline_dir: PathBuf, current_path: PathBuf, filter: Vec<String>) -> Result<()> {
    let mut paths = vec![];
    for entry in std::fs::read_dir(&current_path)
        .with_std_context(|_| format!("read directory {}", current_path.display()))?
        .filter_map(Result::ok)
        .filter(|x| x.path().is_file())
    {
        let current_file = entry.path().to_owned();
        let Some(filename) = current_file.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        let Some(basename) = filename.strip_suffix(".results.json") else {
            continue;
        };
        if !filter.is_empty() && !filter.iter().any(|x| x == basename) {
            continue;
        }
        let baseline_file = baseline_dir.join(filename);
        if !baseline_file.exists() {
            println!("skip {filename} (not in baseline)");
        }
        paths.push((basename.to_string(), baseline_file, current_file));
    }
    paths.sort();
    for (basename, baseline_file, current_file) in paths {
        println!("comparing {basename}");
        if let Err(err) = compare_files(&baseline_file, &current_file) {
            println!("  skip (reason: {err:#}");
        }
    }
    Ok(())
}

fn compare_files(baseline: impl AsRef<Path>, current: impl AsRef<Path>) -> Result<()> {
    let baseline =
        SimulationResults::load_from_file(baseline.as_ref()).context("failed to load baseline")?;
    let current =
        SimulationResults::load_from_file(current.as_ref()).context("failed to load current")?;
    compare_results(baseline, current);
    Ok(())
}

fn compare_results(baseline: SimulationResults, current: SimulationResults) {
    match (baseline.average, current.average) {
        (None, Some(_avg)) => {
            println!("baseline run did not finish");
        }
        (Some(_avg), None) => {
            println!("current run did not finish");
        }
        (None, None) => println!("both runs did not finish"),
        (Some(baseline), Some(current)) => {
            let diff = baseline.diff(&current);
            let mut table = Table::new();
            let header = ["", "RMR", "LDH", "missed", "duration"]
                .into_iter()
                .map(|s| Cell::new(s).set_alignment(CellAlignment::Right));
            table
                .load_preset(NOTHING)
                .set_header(header)
                .add_row(fmt_diff_round("mean", &diff.mean))
                .add_row(fmt_diff_round("max", &diff.max))
                .add_row(fmt_diff_round("min", &diff.min));
            println!("{table}");
        }
    }
}

fn fmt_round(label: &str, round: &RoundStats) -> Vec<Cell> {
    [
        label.to_string(),
        format!("{:.2}", round.rmr),
        format!("{:.2}", round.ldh),
        format!("{:.2}", round.missed),
        format!("{}ms", round.duration.as_millis()),
    ]
    .into_iter()
    .map(|s| Cell::new(s).set_alignment(CellAlignment::Right))
    .collect()
}
fn fmt_diff_round(label: &str, round: &RoundStatsDiff) -> Vec<String> {
    vec![
        label.to_string(),
        fmt_percent(round.rmr),
        fmt_percent(round.ldh),
        fmt_percent(round.missed),
        fmt_percent(round.duration),
    ]
}

fn fmt_percent(diff: f32) -> String {
    format!("{:>+10.2}%", diff * 100.)
}
