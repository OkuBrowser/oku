/*
    This file is part of Oku.

    Oku is free software: you can redistribute it and/or modify
    it under the terms of the GNU Affero General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    Oku is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU Affero General Public License for more details.

    You should have received a copy of the GNU Affero General Public License
    along with Oku.  If not, see <https://www.gnu.org/licenses/>.
*/

// #![feature(doc_cfg)]
#![allow(clippy::needless_doctest_main)]
#![doc(
    html_logo_url = "https://github.com/Dirout/oku/raw/master/branding/logo-filled.svg",
    html_favicon_url = "https://github.com/Dirout/oku/raw/master/branding/logo-filled.svg"
)]
pub mod config;
pub mod history;
pub mod replica_item;
pub mod suggestion_item;
pub mod widgets;
pub mod window_util;

use config::Config;
use directories_next::ProjectDirs;
use directories_next::UserDirs;
use fuser::BackgroundSession;
use gio::prelude::*;
use glib_macros::clone;
use gtk::prelude::GtkApplicationExt;
use history::HistoryManager;
use ipfs::Ipfs;
use ipfs::Keypair;
use ipfs::UninitializedIpfsNoop as UninitializedIpfs;
use oku_fs::fs::OkuFs;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::OnceLock;
use tokio::runtime::Handle;
use tracing::error;
use tracing::info;
use webkit2gtk::URISchemeRequest;
use webkit2gtk::WebContext;
use window_util::ipfs_scheme_handler;
use window_util::ipns_scheme_handler;
use window_util::node_scheme_handler;
use window_util::oku_scheme_handler;

#[macro_use]
extern crate lazy_static;

lazy_static! {
    /// The platform-specific directories intended for Oku's use
    static ref PROJECT_DIRECTORIES: ProjectDirs =
        ProjectDirs::from("com", "github.dirout", "oku").unwrap();
    /// The platform-specific directory where Oku caches data
    static ref CACHE_DIR: PathBuf = PROJECT_DIRECTORIES.cache_dir().to_path_buf();
    /// The platform-specific directory where Oku stores user data
    static ref DATA_DIR: PathBuf = PROJECT_DIRECTORIES.data_dir().to_path_buf();
    /// The platform-specific directories containing user files
    static ref USER_DIRECTORIES: UserDirs = UserDirs::new().unwrap();
    /// The platform-specific directory where users store pictures
    static ref PICTURES_DIR: PathBuf = USER_DIRECTORIES.picture_dir().unwrap().to_path_buf();
    /// The platform-specific directory where the Oku file system is mounted
    static ref MOUNT_DIR: PathBuf = DATA_DIR.join("mount");
    /// The platform-specific file path where Oku settings are stored
    static ref CONFIG_DIR: PathBuf = DATA_DIR.join("config.toml");
    static ref HISTORY_DIR: PathBuf = DATA_DIR.join("history");
    static ref CONFIG: Arc<Mutex<Config>> = Arc::new(Mutex::new(Config::load_or_default()));
    static ref HISTORY_MANAGER: Arc<Mutex<HistoryManager>> = Arc::new(Mutex::new(HistoryManager::load_sessions_or_create().unwrap()));
}

static NODE: OnceLock<OkuFs> = OnceLock::new();

/// The current release version number of Oku
const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");

async fn create_web_context() -> (WebContext, Option<BackgroundSession>, Ipfs) {
    let (node, mount_handle) = create_oku_client().await;
    NODE.get_or_init(|| node.clone());
    let ipfs = create_ipfs_client().await;

    let web_context = WebContext::builder().build();
    web_context.register_uri_scheme(
        "oku",
        clone!(move |request: &URISchemeRequest| {
            oku_scheme_handler(request);
        }),
    );
    web_context.register_uri_scheme(
        "hive",
        clone!(move |request: &URISchemeRequest| {
            node_scheme_handler(request);
        }),
    );
    web_context.register_uri_scheme(
        "ipns",
        clone!(
            #[strong]
            ipfs,
            move |request: &URISchemeRequest| {
                ipns_scheme_handler(&ipfs, request);
            }
        ),
    );
    web_context.register_uri_scheme(
        "ipfs",
        clone!(
            #[strong]
            ipfs,
            move |request: &URISchemeRequest| {
                ipfs_scheme_handler(&ipfs, request);
            }
        ),
    );
    (web_context, mount_handle, ipfs)
}

async fn create_oku_client() -> (OkuFs, Option<BackgroundSession>) {
    let node = OkuFs::start(&Handle::current()).await.unwrap();
    let node_clone = node.clone();
    let _ = std::fs::remove_dir_all(MOUNT_DIR.to_path_buf());
    let _ = std::fs::create_dir_all(MOUNT_DIR.to_path_buf());
    (node_clone, node.mount(MOUNT_DIR.to_path_buf()).ok())
}

async fn create_ipfs_client() -> Ipfs {
    let keypair = Keypair::generate_ed25519();
    // let local_peer_id = keypair.public().to_peer_id();

    // Initialize the repo and start a daemon
    let ipfs: Ipfs = UninitializedIpfs::new()
        .with_default()
        .set_keypair(&keypair)
        .add_listening_addr("/ip4/0.0.0.0/tcp/0".parse().unwrap())
        .with_mdns()
        .with_relay(true)
        .with_relay_server(Default::default())
        .with_upnp()
        .with_rendezvous_server()
        .listen_as_external_addr()
        .fd_limit(ipfs::FDLimit::Max)
        .start()
        .await
        .unwrap();

    ipfs.default_bootstrap().await.unwrap();
    ipfs.bootstrap().await.unwrap();

    ipfs
}

/// The main function of Oku
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("oku=trace")
        .pretty()
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    let (shutdown_send, mut shutdown_recv) = tokio::sync::mpsc::unbounded_channel();

    let application = libadwaita::Application::builder()
        .application_id("com.github.dirout.oku")
        .build();

    let (web_context, mount_handle, ipfs) = create_web_context().await;
    application.connect_activate(clone!(
        #[weak]
        application,
        #[weak]
        web_context,
        move |_| {
            let style_provder = gtk::CssProvider::default();
            crate::widgets::window::Window::new(&application, &style_provder, &web_context, false);
            gtk::style_context_add_provider_for_display(
                &gdk::Display::default().unwrap(),
                &style_provder,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
            let ctx = glib::MainContext::default();
            ctx.spawn_local(clone!(
                #[weak]
                application,
                async move {
                    tokio::signal::ctrl_c().await.unwrap();
                    application.quit();
                }
            ));
        }
    ));
    application.connect_window_added(clone!(move |_, window| {
        if let Some(node) = NODE.get() {
            let mut rx = node.replica_sender.subscribe();
            let window: widgets::window::Window = window.clone().downcast().unwrap();
            let ctx = glib::MainContext::default();
            ctx.spawn_local_with_priority(
                glib::source::Priority::HIGH,
                clone!(
                    #[weak]
                    window,
                    async move {
                        loop {
                            rx.borrow_and_update();
                            info!("Replicas updated … ");
                            window.replicas_updated();
                            match rx.changed().await {
                                Ok(_) => continue,
                                Err(e) => {
                                    error!("{}", e);
                                    break;
                                }
                            }
                        }
                    }
                ),
            );
        }
    }));
    application.connect_window_removed(clone!(
        #[weak]
        application,
        #[strong]
        shutdown_send,
        move |_, _| {
            if application.windows().len() == 0 {
                shutdown_send.send(()).unwrap();
            }
        }
    ));
    application.connect_shutdown(clone!(move |_| {
        shutdown_send.send(()).unwrap();
    }));
    application.set_accels_for_action("win.previous", &["<Alt>Left", "<Alt>KP_Left"]);
    application.set_accels_for_action("win.next", &["<Alt>Right", "<Alt>KP_Right"]);
    application.set_accels_for_action("win.reload", &["<Ctrl>r", "F5"]);
    application.set_accels_for_action("win.new-tab", &["<Ctrl>t"]);
    application.set_accels_for_action("win.close-tab", &["<Ctrl>w"]);
    application.set_accels_for_action("win.zoom-in", &["<Ctrl>plus"]);
    application.set_accels_for_action("win.zoom-out", &["<Ctrl>minus"]);
    application.set_accels_for_action("win.reset-zoom", &["<Ctrl>0"]);
    application.set_accels_for_action("win.find", &["<Ctrl>f"]);
    application.set_accels_for_action("win.print", &["<Ctrl>p"]);
    application.set_accels_for_action("win.fullscreen", &["F11"]);
    application.set_accels_for_action("win.save", &["<Ctrl>s"]);
    application.set_accels_for_action("win.new", &["<Ctrl>n"]);
    application.set_accels_for_action("win.new-private", &["<Ctrl><Shift>p"]);
    application.run();

    let _ = shutdown_recv.recv().await;
    if let Some(mount_handle) = mount_handle {
        mount_handle.join();
    }
    ipfs.exit_daemon().await;
    application.quit();
    std::process::exit(0)
}
