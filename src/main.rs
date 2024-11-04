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
    html_logo_url = "https://github.com/OkuBrowser/oku/raw/master/branding/logo-filled.svg",
    html_favicon_url = "https://github.com/OkuBrowser/oku/raw/master/branding/logo-filled.svg"
)]
pub mod bookmark_item;
pub mod config;
pub mod database;
pub mod history_item;
pub mod replica_item;
pub mod scheme_handlers;
pub mod suggestion_item;
pub mod vox_providers;
pub mod widgets;
pub mod window_util;

use config::Config;
use database::DATABASE;
use directories_next::ProjectDirs;
use directories_next::UserDirs;
use env_logger::Builder;
use gio::prelude::*;
use glib_macros::clone;
use gtk::prelude::GtkApplicationExt;
use ipfs::Ipfs;
use ipfs::Keypair;
use ipfs::UninitializedIpfsDefault as UninitializedIpfs;
use log::error;
use log::{info, LevelFilter};
use oku_fs::fs::OkuFs;
use oku_fs::fuser::BackgroundSession;
use scheme_handlers::util::handle_request;
use scheme_handlers::util::RequestScheme;
use scheme_handlers::util::SchemeRequest;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::sync::LazyLock;
use std::sync::Mutex;
use std::sync::OnceLock;
use tokio::runtime::Handle;
use webkit2gtk::prelude::WebViewExt;
use webkit2gtk::URISchemeRequest;
use webkit2gtk::WebContext;

#[macro_use]
extern crate lazy_static;

lazy_static! {
    /// The platform-specific directories intended for Oku's use
    static ref PROJECT_DIRECTORIES: ProjectDirs =
        ProjectDirs::from("io", "github.OkuBrowser","oku").unwrap();
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
    static ref CONFIG: Arc<Mutex<Config>> = Arc::new(Mutex::new(Config::load_or_default()));
    /// The current release version number of Oku
    static ref VERSION: &'static str = option_env!("CARGO_PKG_VERSION").unwrap();
}

static NODE: OnceLock<OkuFs> = OnceLock::new();
static HOME_REPLICA_SET: LazyLock<Arc<AtomicBool>> =
    LazyLock::new(|| Arc::new(AtomicBool::new(false)));

async fn create_web_context() -> (WebContext, Option<BackgroundSession>, Ipfs) {
    let (node, mount_handle) = create_oku_client().await;
    NODE.get_or_init(|| node.clone());
    let ipfs = create_ipfs_client().await;

    let web_context = WebContext::builder().build();
    web_context.register_uri_scheme(
        "oku",
        clone!(
            #[strong]
            ipfs,
            move |request: &URISchemeRequest| {
                handle_request(
                    ipfs.clone(),
                    SchemeRequest(request.clone()),
                    RequestScheme::Oku,
                )
            }
        ),
    );
    web_context.register_uri_scheme(
        "hive",
        clone!(
            #[strong]
            ipfs,
            move |request: &URISchemeRequest| {
                handle_request(
                    ipfs.clone(),
                    SchemeRequest(request.clone()),
                    RequestScheme::Hive,
                )
            }
        ),
    );
    web_context.register_uri_scheme(
        "ipns",
        clone!(
            #[strong]
            ipfs,
            move |request: &URISchemeRequest| {
                handle_request(
                    ipfs.clone(),
                    SchemeRequest(request.clone()),
                    RequestScheme::Ipns,
                )
            }
        ),
    );
    web_context.register_uri_scheme(
        "ipfs",
        clone!(
            #[strong]
            ipfs,
            move |request: &URISchemeRequest| {
                handle_request(
                    ipfs.clone(),
                    SchemeRequest(request.clone()),
                    RequestScheme::Ipfs,
                )
            }
        ),
    );
    web_context.register_uri_scheme(
        "view-source",
        clone!(
            #[strong]
            ipfs,
            move |request: &URISchemeRequest| {
                handle_request(
                    ipfs.clone(),
                    SchemeRequest(request.clone()),
                    RequestScheme::ViewSource,
                )
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
    match gio::Resource::load("resources.gresource") {
        Ok(res) => gio::resources_register(&res),
        Err(e) => error!("{}", e),
    }

    let mut builder = Builder::new();
    builder.filter(Some("oku"), LevelFilter::Trace);
    builder.filter(Some("oku_fs"), LevelFilter::Trace);
    builder.format_module_path(true);
    builder.init();

    let (shutdown_send, mut shutdown_recv) = tokio::sync::mpsc::unbounded_channel();

    let application = libadwaita::Application::builder()
        .application_id("io.github.OkuBrowser.oku")
        .flags(gio::ApplicationFlags::HANDLES_OPEN)
        .version(VERSION.to_string())
        .build();
    application.add_main_option(
        "new-window",
        b'n'.into(),
        glib::OptionFlags::NONE,
        glib::OptionArg::String,
        "New window",
        None,
    );
    application.add_main_option(
        "new-private-window",
        b'p'.into(),
        glib::OptionFlags::NONE,
        glib::OptionArg::String,
        "New private window",
        None,
    );

    let (web_context, mount_handle, ipfs) = create_web_context().await;
    application.connect_activate(clone!(move |application| {
        let ctx = glib::MainContext::default();
        ctx.spawn_local(clone!(
            #[weak]
            application,
            async move {
                tokio::signal::ctrl_c().await.unwrap();
                application.quit();
            }
        ));
    }));
    application.connect_handle_local_options(clone!(move |application, dict| {
        if !(dict.contains("new-window") || dict.contains("new-private-window")) {
            if let Ok(_) = application.register(Some(&gio::Cancellable::new())) {
                application.open(&[], "false,true");
            }
            return -1;
        }
        match dict.lookup::<String>("new-window").unwrap() {
            Some(initial_uri) => {
                if let Ok(_) = application.register(Some(&gio::Cancellable::new())) {
                    let file = gio::File::for_uri(&initial_uri);
                    application.open(&[file], "false,false");
                }
            }
            None => (),
        };
        match dict.lookup::<String>("new-private-window").unwrap() {
            Some(initial_uri) => {
                if let Ok(_) = application.register(Some(&gio::Cancellable::new())) {
                    let file = gio::File::for_uri(&initial_uri);
                    application.open(&[file], "true,false");
                }
            }
            None => (),
        };
        -1
    }));
    application.connect_open(clone!(
        #[weak]
        web_context,
        move |application, files, hint| {
            application.activate();
            let style_provider = gtk::CssProvider::default();
            gtk::style_context_add_provider_for_display(
                &gdk::Display::default().unwrap(),
                &style_provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
            let (is_private_str, no_arguments_str) =
                hint.split_once(",").unwrap_or(("false", "true"));
            let is_private = is_private_str.parse().unwrap_or(false);
            let no_arguments = no_arguments_str.parse().unwrap_or(false);
            let new_window = match no_arguments {
                true => match application.windows().last() {
                    Some(window) => window.clone().downcast().unwrap(),
                    None => crate::widgets::window::Window::new(
                        &application,
                        &style_provider,
                        &web_context,
                        is_private,
                    ),
                },
                false => crate::widgets::window::Window::new(
                    &application,
                    &style_provider,
                    &web_context,
                    is_private,
                ),
            };
            let mut files = files.to_vec();
            files.sort_unstable_by_key(|x| x.uri());
            files.dedup_by_key(|x| x.uri());
            for file_index in 0..files.len() {
                let file = &files[file_index];
                let new_view = if file_index == 0 {
                    new_window.get_view()
                } else {
                    new_window.new_tab_page(&web_context, None, None).0
                };
                new_view.load_uri(&file.uri());
            }
        }
    ));
    application.connect_window_added(clone!(move |_, window| {
        let window: widgets::window::Window = window.clone().downcast().unwrap();
        let ctx = glib::MainContext::default();
        let mut bookmark_rx = DATABASE.bookmark_sender.subscribe();
        ctx.spawn_local_with_priority(
            glib::source::Priority::HIGH,
            clone!(
                #[weak]
                window,
                async move {
                    loop {
                        bookmark_rx.borrow_and_update();
                        info!("Bookmarks updated … ");
                        window.bookmarks_updated();
                        match bookmark_rx.changed().await {
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
        let mut history_rx = DATABASE.history_sender.subscribe();
        ctx.spawn_local_with_priority(
            glib::source::Priority::HIGH,
            clone!(
                #[weak]
                window,
                async move {
                    loop {
                        history_rx.borrow_and_update();
                        info!("History updated … ");
                        window.history_updated();
                        match history_rx.changed().await {
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

        if let Some(node) = NODE.get() {
            let mut replica_rx = node.replica_sender.subscribe();
            ctx.spawn_local_with_priority(
                glib::source::Priority::HIGH,
                clone!(
                    #[weak]
                    window,
                    async move {
                        loop {
                            replica_rx.borrow_and_update();
                            info!("Replicas updated … ");
                            window.replicas_updated();
                            HOME_REPLICA_SET
                                .store(node.home_replica().await.is_some(), Ordering::Relaxed);
                            match replica_rx.changed().await {
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
    application.set_accels_for_action(
        "win.previous",
        &["<Alt>Left", "<Alt>KP_Left", "<Ctrl>bracketleft"],
    );
    application.set_accels_for_action(
        "win.next",
        &["<Alt>Right", "<Alt>KP_Right", "<Ctrl>bracketright"],
    );
    application.set_accels_for_action("win.reload", &["<Ctrl>r", "F5"]);
    application.set_accels_for_action("win.new-tab", &["<Ctrl>t"]);
    application.set_accels_for_action("win.close-tab", &["<Ctrl>w"]);
    application.set_accels_for_action("win.zoom-in", &["<Ctrl><Shift>plus"]);
    application.set_accels_for_action("win.zoom-out", &["<Ctrl>minus"]);
    application.set_accels_for_action("win.reset-zoom", &["<Ctrl>0"]);
    application.set_accels_for_action("win.find", &["<Ctrl>f"]);
    application.set_accels_for_action("win.print", &["<Ctrl>p"]);
    application.set_accels_for_action("win.fullscreen", &["F11"]);
    application.set_accels_for_action("win.save", &["<Ctrl>s"]);
    application.set_accels_for_action("win.new", &["<Ctrl>n"]);
    application.set_accels_for_action("win.new-private", &["<Ctrl><Shift>p"]);
    application.set_accels_for_action("win.go-home", &["<Alt>Home"]);
    application.set_accels_for_action("win.stop-loading", &["Escape"]);
    application.set_accels_for_action("win.reload-bypass", &["<Ctrl><Shift>r", "<Shift>F5"]);
    application.set_accels_for_action("win.next-find", &["<Ctrl>g"]);
    application.set_accels_for_action("win.previous-find", &["<Ctrl><Shift>g"]);
    application.set_accels_for_action("win.screenshot", &["<Ctrl><Shift>s"]);
    application.set_accels_for_action("win.settings", &["<Ctrl>comma"]);
    application.set_accels_for_action("win.view-source", &["<Ctrl>u"]);
    application.set_accels_for_action("win.shortcuts", &["<Ctrl><Shift>question"]);
    application.set_accels_for_action("win.open-file", &["<Ctrl>o"]);
    application.set_accels_for_action("win.inspector", &["<Ctrl><Shift>i", "F12"]);
    application.set_accels_for_action("win.close-window", &["<Ctrl>q", "<Ctrl><Shift>w"]);
    application.set_accels_for_action("win.library", &["<Ctrl>d"]);
    application.set_accels_for_action("win.next-tab", &["<Ctrl>Page_Down", "<Ctrl>Tab"]);
    application.set_accels_for_action("win.previous-tab", &["<Ctrl>Page_Up", "<Ctrl><Shift>Tab"]);
    application.set_accels_for_action("win.current-tab-left", &["<Ctrl><Shift>Page_Up"]);
    application.set_accels_for_action("win.current-tab-right", &["<Ctrl><Shift>Page_Down"]);
    application.set_accels_for_action("win.duplicate-current-tab", &["<Ctrl><Shift>k"]);
    application.set_accels_for_action("win.tab-overview", &["<Ctrl><Shift>o"]);
    application.run();

    let _ = shutdown_recv.recv().await;
    if let Some(mount_handle) = mount_handle {
        mount_handle.join();
    }
    ipfs.exit_daemon().await;
    application.quit();
    std::process::exit(0)
}
