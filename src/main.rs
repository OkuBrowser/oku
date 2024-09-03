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
pub mod widgets;
pub mod window_util;

use config::Config;
use directories_next::ProjectDirs;
use directories_next::UserDirs;
use fuser::BackgroundSession;
use gio::prelude::*;
use glib_macros::clone;
use gtk::prelude::GtkApplicationExt;
use ipfs::Ipfs;
use ipfs::Keypair;
use ipfs::UninitializedIpfsNoop as UninitializedIpfs;
use oku_fs::fs::OkuFs;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::runtime::Handle;
use webkit2gtk::URISchemeRequest;
use webkit2gtk::WebContext;
use window_util::ipfs_scheme_handler;
use window_util::ipns_scheme_handler;
use window_util::node_scheme_handler;

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
    static ref HISTORY_DIR: PathBuf = DATA_DIR.join(".history");
    static ref CONFIG: Arc<Mutex<Config>> = Arc::new(Mutex::new(Config::load_or_default()));
}

/// The current release version number of Oku
const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");

async fn create_web_context() -> (WebContext, BackgroundSession, Ipfs) {
    let (node, mount_handle) = create_oku_client().await;
    let ipfs = create_ipfs_client().await;

    let web_context = WebContext::builder().build();
    web_context.register_uri_scheme(
        "hive",
        clone!(
            #[strong]
            node,
            move |request: &URISchemeRequest| {
                node_scheme_handler(&node, request);
            }
        ),
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

async fn create_oku_client() -> (OkuFs, BackgroundSession) {
    let node = OkuFs::start(&Handle::current()).await.unwrap();
    let node_clone = node.clone();
    let _ = std::fs::remove_dir_all(MOUNT_DIR.to_path_buf());
    let _ = std::fs::create_dir_all(MOUNT_DIR.to_path_buf());
    (node_clone, node.mount(MOUNT_DIR.to_path_buf()).unwrap())
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
            crate::widgets::window::Window::new(&application, &web_context, None);
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
    application.run();

    let _ = shutdown_recv.recv().await;
    mount_handle.join();
    ipfs.exit_daemon().await;
    application.quit();
    std::process::exit(0)
}

// Create a new functional & graphical browser window
//
// # Arguments
//
// * `application` - The application data representing Oku
//
// * `matches` - The launch arguments passed to Oku
/*
fn new_window(application: &gtk::Application, matches: VariantDict) {
    if gtk::init().is_err() {
        println!("Failed to initialize GTK.");
        return;
    }

    let initial_url: String;
    match matches.lookup_value("url", Some(VariantTy::new("s").unwrap()))
    {
        Some(url) => {
            initial_url = url.to_string()[1..url.to_string().len()-1].to_string();
        }
        None => {
            initial_url = "about:blank".to_owned();
        }
    }

    let is_private = matches.contains("private");
    let verbose = matches.contains("verbose");
    let native = true;

    let glade_src = include_str!("oku.glade");
    let builder = gtk::Builder::from_string(glade_src);

    let window: gtk::ApplicationWindow = builder.object("window").unwrap();
    window.set_application(Some(application));
    window.set_title(Some("Oku"));
    window.set_default_size(1920, 1080);

    let downloads_button: gtk::Button = builder.object("downloads_button").unwrap();
    let downloads_popover: gtk::Popover = builder.object("downloads_popover").unwrap();

    let find_button: gtk::Button = builder.object("find_button").unwrap();
    let find_popover: gtk::Popover = builder.object("find_popover").unwrap();
    let previous_find_button: gtk::Button = builder.object("previous_find_button").unwrap();
    let next_find_button: gtk::Button = builder.object("next_find_button").unwrap();
    let find_case_insensitive: gtk::ToggleButton = builder.object("find_case_insensitive").unwrap();
    let find_at_word_starts: gtk::ToggleButton = builder.object("find_at_word_starts").unwrap();
    let find_treat_medial_capital_as_word_start: gtk::ToggleButton = builder.object("find_treat_medial_capital_as_word_start").unwrap();
    let find_backwards: gtk::ToggleButton = builder.object("find_backwards").unwrap();
    let find_wrap_around: gtk::ToggleButton = builder.object("find_wrap_around").unwrap();
    let find_search_entry: gtk::SearchEntry = builder.object("find_search_entry").unwrap();
    let current_match_label: gtk::Label = builder.object("current_match_label").unwrap();
    let total_matches_label: gtk::Label = builder.object("total_matches_label").unwrap();

    let menu_button: gtk::Button = builder.object("menu_button").unwrap();
    let menu: gtk::Popover = builder.object("menu").unwrap();

    let back_button: gtk::Button = builder.object("back_button").unwrap();
    let forward_button: gtk::Button = builder.object("forward_button").unwrap();
    let refresh_button: gtk::Button = builder.object("refresh_button").unwrap();
    let add_tab: gtk::Button = builder.object("add_tab").unwrap();

    let tabs: libadwaita::TabBar = builder.object("tabs").unwrap();
    let tab_view: libadwaita::TabView = libadwaita::TabView::new();
    let nav_entry: gtk::Entry = builder.object("nav_entry").unwrap();

    let zoomout_button: gtk::Button = builder.object("zoomout_button").unwrap();
    let zoomin_button: gtk::Button = builder.object("zoomin_button").unwrap();
    let zoomreset_button: gtk::Button = builder.object("zoomreset_button").unwrap();
    let fullscreen_button: gtk::Button = builder.object("fullscreen_button").unwrap();
    let screenshot_button: gtk::Button = builder.object("screenshot_button").unwrap();
    let new_window_button: gtk::Button = builder.object("new_window_button").unwrap();
    let _history_button: gtk::Button = builder.object("history_button").unwrap();
    let about_button: gtk::Button = builder.object("about_button").unwrap();

    tabs.set_view(Some(&tab_view));

    let tab_view = tabs.view().unwrap();

    if tab_view.n_pages() == 0 {
        create_initial_tab(&tabs, &nav_entry,initial_url.to_owned(), verbose, is_private, native)
    }

    tab_view.connect_pages_notify(
        clone!(@weak nav_entry, @weak builder, @weak tabs, @weak window => move |_| {
            let web_view = get_view(&tabs);
            update_nav_bar(&nav_entry, &web_view);
            window.set_title(Some(&web_view.title().unwrap_or_else(|| glib::GString::from("Oku")).to_string()));
        }),
    );

    nav_entry.connect_activate(clone!(@weak tabs, @weak nav_entry, @weak builder, @weak window => move |_| {
        let web_view = get_view(&tabs);
        connect(&nav_entry, &web_view);
    }));

    add_tab.connect_clicked(clone!(@weak tabs, @weak nav_entry, @weak builder => move |_| {
        let _tab_view = tabs.view().unwrap();
        let _web_view = new_tab_page(&tabs, &nav_entry, verbose, is_private, native);
    }));

    back_button.connect_clicked(
        clone!(@weak tabs, @weak nav_entry, @weak builder => move |_| {
            let web_view = get_view(&tabs);
            web_view.go_back()
        }),
    );

    forward_button.connect_clicked(
        clone!(@weak tabs, @weak nav_entry, @weak builder => move |_| {
            let web_view = get_view(&tabs);
            web_view.go_forward()
        }),
    );

    refresh_button.connect_clicked(
        clone!(@weak tabs, @weak nav_entry, @weak builder => move |_| {
            let web_view = get_view(&tabs);
            web_view.reload_bypass_cache()
        }),
    );

    downloads_button.connect_clicked(
        clone!(@weak tabs, @weak nav_entry, @weak builder => move |_| {
            downloads_popover.popup();
        }),
    );

    find_button.connect_clicked(
        clone!(@weak tabs, @weak nav_entry, @weak builder, @weak find_popover => move |_| {
            find_popover.popup();
        }),
    );
    find_search_entry.connect_search_changed(
        clone!(@weak tabs, @weak nav_entry, @weak builder, @weak find_search_entry, @weak find_popover => move |_| {
            let web_view = get_view(&tabs);
            let find_controller = web_view.find_controller().unwrap();
            let mut find_options = webkit2gtk::FindOptions::empty();
            find_options.set(webkit2gtk::FindOptions::CASE_INSENSITIVE, find_case_insensitive.is_active());
            find_options.set(webkit2gtk::FindOptions::AT_WORD_STARTS, find_at_word_starts.is_active());
            find_options.set(webkit2gtk::FindOptions::TREAT_MEDIAL_CAPITAL_AS_WORD_START, find_treat_medial_capital_as_word_start.is_active());
            find_options.set(webkit2gtk::FindOptions::BACKWARDS, find_backwards.is_active());
            find_options.set(webkit2gtk::FindOptions::WRAP_AROUND, find_wrap_around.is_active());
            let max_match_count = find_controller.max_match_count();
            find_controller.count_matches(&find_search_entry.text(), find_options.bits(), max_match_count);
            find_controller.search(&find_search_entry.text(), find_options.bits(), max_match_count);
            find_controller.connect_counted_matches(clone!(@weak web_view, @weak find_controller, @weak find_search_entry, @weak total_matches_label => move |_, total_matches| {
                if total_matches < u32::MAX
                {
                    total_matches_label.set_text(&total_matches.to_string());
                }
            }));
            find_search_entry.connect_activate(clone!(@weak web_view, @weak find_controller, @weak find_search_entry, @weak current_match_label, @weak total_matches_label => move |_| {
                find_controller.search_next();
                let mut current_match: u32 = current_match_label.text().parse().unwrap();
                let total_matches: u32 = total_matches_label.text().parse().unwrap();
                current_match += 1;
                if current_match > total_matches
                {
                    current_match = 1;
                }
                current_match_label.set_text(&current_match.to_string());
                find_search_entry.set_tooltip_text(Some(&format!("{} / {} matches", current_match, total_matches)));
            }));
            next_find_button.connect_clicked(clone!(@weak web_view, @weak find_controller, @weak find_search_entry, @weak current_match_label, @weak total_matches_label => move |_| {
                find_controller.search_next();
                let mut current_match: u32 = current_match_label.text().parse().unwrap();
                let total_matches: u32 = total_matches_label.text().parse().unwrap();
                current_match += 1;
                if current_match > total_matches
                {
                    current_match = 1;
                }
                current_match_label.set_text(&current_match.to_string());
                find_search_entry.set_tooltip_text(Some(&format!("{} / {} matches", current_match, total_matches)));
            }));
            previous_find_button.connect_clicked(clone!(@weak web_view, @weak find_controller, @weak find_search_entry, @weak current_match_label, @weak total_matches_label => move |_| {
                find_controller.search_previous();
                let mut current_match: u32 = current_match_label.text().parse().unwrap();
                let total_matches: u32 = total_matches_label.text().parse().unwrap();
                current_match -= 1;
                if current_match < 1
                {
                    current_match = 1;
                }
                current_match_label.set_text(&current_match.to_string());
                find_search_entry.set_tooltip_text(Some(&format!("{} / {} matches", current_match, total_matches)));
            }));
            find_popover.connect_closed(
                clone!(@weak tabs, @weak nav_entry, @weak builder, @weak current_match_label, @weak total_matches_label => move |_| {
                    find_controller.search_finish();
                    current_match_label.set_text("0");
                    total_matches_label.set_text("0");
                }),
            );
        }),
    );

    menu_button.connect_clicked(
        clone!(@weak tabs, @weak nav_entry, @weak builder => move |_| {
            menu.popup();
        }),
    );

    about_button.connect_clicked(
        clone!(@weak tabs, @weak nav_entry, @weak window => move |_| {
            new_about_dialog(&window.application().unwrap())
        }),
    );

    zoomin_button.connect_clicked(
        clone!(@weak tabs, @weak nav_entry, @weak builder => move |_| {
            let web_view = get_view(&tabs);
            let current_zoom_level = web_view.zoom_level();
            web_view.set_zoom_level(current_zoom_level + 0.1);
        }),
    );

    zoomout_button.connect_clicked(
        clone!(@weak tabs, @weak nav_entry, @weak builder => move |_| {
            let web_view = get_view(&tabs);
            let current_zoom_level = web_view.zoom_level();
            web_view.set_zoom_level(current_zoom_level - 0.1);
        }),
    );

    zoomreset_button.connect_clicked(
        clone!(@weak tabs, @weak nav_entry, @weak builder => move |_| {
            let web_view = get_view(&tabs);
            web_view.set_zoom_level(1.0);
        }),
    );

    fullscreen_button.connect_clicked(
        clone!(@weak tabs, @weak nav_entry, @weak builder => move |_| {
            let web_view = get_view(&tabs);
            web_view.run_javascript("document.documentElement.webkitRequestFullscreen();", gio::NONE_CANCELLABLE, move |_| {

            })
        }),
    );

    screenshot_button.connect_clicked(
        clone!(@weak tabs, @weak nav_entry, @weak builder => move |_| {
            let web_view = get_view(&tabs);
            web_view.snapshot(webkit2gtk::SnapshotRegion::FullDocument, webkit2gtk::SnapshotOptions::all(), gio::NONE_CANCELLABLE, move |snapshot| {
                let snapshot_surface = cairo::ImageSurface::try_from(snapshot.unwrap()).unwrap();
                let mut writer = File::create(format!("{}/{}.png", PICTURES_DIR.to_owned(), Utc::now())).unwrap();
                snapshot_surface.write_to_png(&mut writer).unwrap();
            });
        }),
    );

    new_window_button.connect_clicked(
        clone!(@weak tabs, @weak nav_entry, @weak builder, @weak window => move |_| {
            matches.remove("url");
            new_window(&window.application().unwrap(), matches.to_owned())
        }),
    );

    window.show();
}
*/
