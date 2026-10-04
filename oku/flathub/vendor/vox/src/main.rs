use actix_files::NamedFile;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::{App, HttpServer};
use clap::{Parser, Subcommand};
use miette::IntoDiagnostic;
use mimalloc::MiMalloc;
use notify_debouncer_full::{new_debouncer, notify::RecursiveMode};
use std::net::Ipv4Addr;
use std::sync::mpsc::channel;
use std::sync::LazyLock;
use std::{path::PathBuf, time::Duration};
use tokio::time::sleep;
use tracing::{error, info, trace, Level};
use vox::fs_provider::FsProvider;
use vox::provider::{VoxProvider, VERSION};

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

static FS_PROVIDER: LazyLock<FsProvider> = LazyLock::new(|| FsProvider::new());

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    /// The level of log output; warnings, information, debugging messages, and trace logs.
    #[arg(short, long, action = clap::ArgAction::Count, default_value_t = 2, global = true)]
    verbosity: u8,
}
#[derive(Subcommand)]
enum Commands {
    /// Build the site.
    Build {
        /// An optional path to the site directory.
        #[arg(default_value = None)]
        path: Option<PathBuf>,
        /// Watch for changes.
        #[arg(short, long, default_value_t = false)]
        watch: bool,
        /// Visualise the DAG.
        #[arg(short = 'd', long, default_value_t = false)]
        visualise_dag: bool,
        /// Generate stylesheet for syntax highlighting.
        #[arg(short = 's', long, default_value_t = false)]
        generate_syntax_css: bool,
    },
    /// Serve the site.
    Serve {
        /// An optional path to the site directory.
        #[arg(default_value = None)]
        path: Option<PathBuf>,
        /// Watch for changes.
        #[arg(short, long, default_value_t = false)]
        watch: bool,
        /// The port to serve the site on.
        #[arg(short, long, default_value_t = 80)]
        port: u16,
        /// Visualise the DAG.
        #[arg(short = 'd', long, default_value_t = false)]
        visualise_dag: bool,
        /// Generate stylesheet for syntax highlighting.
        #[arg(short = 's', long, default_value_t = false)]
        generate_syntax_css: bool,
    },
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> miette::Result<()> {
    miette::set_panic_hook();
    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Build {
            path,
            watch,
            visualise_dag,
            generate_syntax_css,
        }) => {
            if let Some(path) = path {
                std::env::set_current_dir(path).into_diagnostic()?;
            }
            let verbosity_level = match cli.verbosity {
                0 => Level::ERROR,
                1 => Level::WARN,
                2 => Level::INFO,
                3 => Level::DEBUG,
                4 => Level::TRACE,
                _ => Level::TRACE,
            };
            let mut subscriber_builder = tracing_subscriber::fmt()
                .with_env_filter(format!("vox={}", verbosity_level))
                .pretty()
                .with_file(false)
                .with_line_number(false);
            if cli.verbosity >= 3 {
                subscriber_builder = subscriber_builder
                    .with_thread_ids(true)
                    .with_thread_names(true)
                    .with_file(true)
                    .with_line_number(true);
            }
            subscriber_builder.init();
            info!("Building … ");
            loop {
                let building = build(watch, visualise_dag, generate_syntax_css);
                match building {
                    Ok(_) => {
                        if !watch {
                            break;
                        }
                    }
                    Err(err) => {
                        error!("Building failed: {:#?}", err);
                        info!("Retrying in 5 seconds … ");
                        sleep(Duration::from_secs(5)).await;
                        continue;
                    }
                }
            }
        }
        Some(Commands::Serve {
            path,
            watch,
            port,
            visualise_dag,
            generate_syntax_css,
        }) => {
            if let Some(path) = path {
                std::env::set_current_dir(path).into_diagnostic()?;
            }
            let verbosity_level = match cli.verbosity {
                0 => Level::ERROR,
                1 => Level::WARN,
                2 => Level::INFO,
                3 => Level::DEBUG,
                4 => Level::TRACE,
                _ => Level::TRACE,
            };
            let mut subscriber_builder = tracing_subscriber::fmt()
                .with_env_filter(format!("vox={}", verbosity_level))
                .pretty()
                .with_file(false)
                .with_line_number(false);
            if cli.verbosity >= 3 {
                subscriber_builder = subscriber_builder
                    .with_thread_ids(true)
                    .with_thread_names(true)
                    .with_file(true)
                    .with_line_number(true);
            }
            subscriber_builder.init();
            let build_loop = tokio::spawn(async move {
                loop {
                    let building = build(watch, visualise_dag, generate_syntax_css);
                    match building {
                        Ok(_) => {
                            if !watch {
                                break;
                            }
                        }
                        Err(err) => {
                            error!("Building failed: {:#?}", err);
                            info!("Retrying in 5 seconds … ");
                            sleep(Duration::from_secs(5)).await;
                            continue;
                        }
                    }
                }
            });
            let serve_loop = tokio::spawn(async move {
                loop {
                    let serving = tokio::spawn(
                        HttpServer::new(|| {
                            let mut service = actix_files::Files::new("/", "output")
                                .prefer_utf8(true)
                                .use_hidden_files()
                                .use_etag(true)
                                .use_last_modified(true)
                                .show_files_listing()
                                .redirect_to_slash_directory();
                            service = service.index_file("index.html");
                            service = service.default_handler(|req: ServiceRequest| {
                                let (http_req, _payload) = req.into_parts();
                                async {
                                    let response = NamedFile::open("output/404.html")?
                                        .into_response(&http_req);
                                    Ok(ServiceResponse::new(http_req, response))
                                }
                            });
                            App::new().service(service)
                        })
                        .bind((Ipv4Addr::UNSPECIFIED, port))
                        .unwrap()
                        .run(),
                    );
                    println!("Serving on {}:{} … ", Ipv4Addr::UNSPECIFIED, port);
                    match serving.await.unwrap() {
                        Ok(_) => {}
                        Err(err) => {
                            error!("Serving failed: {:#?}", err);
                            info!("Retrying in 5 seconds … ");
                            sleep(Duration::from_secs(5)).await;
                            continue;
                        }
                    }
                }
            });
            tokio::spawn(async move {
                match tokio::signal::ctrl_c().await {
                    Ok(()) => {
                        info!("Exiting … ");
                        std::process::exit(0);
                    }
                    Err(err) => {
                        error!("Unable to listen for shutdown signal: {}", err);
                        std::process::exit(0);
                    }
                }
            });
            build_loop.await.into_diagnostic()?;
            serve_loop.await.into_diagnostic()?;
        }
        None => println!("Vox {}", VERSION),
    };
    Ok(())
}

fn build(watch: bool, visualise_dag: bool, generate_syntax_css: bool) -> miette::Result<()> {
    let parser = FS_PROVIDER.create_liquid_parser()?;
    let global = FS_PROVIDER.get_global_context()?;
    let (mut dag, mut pages, mut layouts) = FS_PROVIDER.generate_dag()?;

    // Write the initial site to the output directory.
    info!("Performing initial build … ");
    let (_updated_pages, updated_dag) = FS_PROVIDER.generate_site(
        &parser,
        &global.0,
        &global.1,
        &dag,
        &visualise_dag,
        &generate_syntax_css,
    )?;
    dag = updated_dag;

    // Watch for changes to the site.
    if watch {
        let current_path = std::env::current_dir().into_diagnostic()?;
        let output_path = current_path.join("output");
        let git_path = current_path.join(".git");
        let (sender, receiver) = channel();
        let mut debouncer =
            new_debouncer(Duration::from_secs(1), None, sender).into_diagnostic()?;
        info!("Watching {:?} … ", current_path);
        debouncer
            .watch(&current_path, RecursiveMode::Recursive)
            .into_diagnostic()?;

        while let Ok(events) = receiver.recv().into_diagnostic()? {
            // Changes to the output directory or version control are irrelevant.
            if !events.iter().any(|event| {
                event
                    .paths
                    .iter()
                    .any(|path| !path.starts_with(&output_path) && !path.starts_with(&git_path))
            }) {
                continue;
            }
            let global_or_snippets_changed = events.iter().any(|event| {
                event.paths.iter().any(|path| {
                    path.strip_prefix(&current_path)
                        .unwrap_or(path)
                        .starts_with("global.toml")
                        || path
                            .strip_prefix(&current_path)
                            .unwrap_or(path)
                            .starts_with("snippets/")
                })
            });
            trace!(
                "Changes detected: {:#?} … ",
                events
                    .into_iter()
                    .map(|event| (&event.paths)
                        .into_iter()
                        .map(|path| {
                            path.strip_prefix(&current_path)
                                .unwrap_or(&path)
                                .to_path_buf()
                        })
                        .collect::<Vec<_>>())
                    .collect::<Vec<_>>()
            );

            (dag, pages, layouts) = FS_PROVIDER.incremental_regeneration(
                &global_or_snippets_changed,
                &parser,
                &visualise_dag,
                &generate_syntax_css,
                &dag,
                &pages,
                &layouts,
            )?;
        }
    }
    Ok(())
}
