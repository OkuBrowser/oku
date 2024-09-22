use crate::config::{ColourScheme, Palette};
use crate::{CONFIG, NODE};
use glib::clone;
use gtk::glib;
use gtk::subclass::prelude::*;
use libadwaita::subclass::{dialog::AdwDialogImpl, preferences_dialog::PreferencesDialogImpl};
use libadwaita::{prelude::*, StyleManager};
use oku_fs::config::OkuFsRelayConnectionConfig;
use tracing::error;

pub mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct Settings {
        pub(crate) main_page: libadwaita::PreferencesPage,
        pub(crate) appearance_group: libadwaita::PreferencesGroup,
        pub(crate) colour_scheme_row: libadwaita::ComboRow,
        pub(crate) colour_scheme_selection: gtk::SingleSelection,
        pub(crate) colour_scheme_list: gtk::StringList,
        pub(crate) domain_colour_row: libadwaita::SwitchRow,
        pub(crate) palette_row: libadwaita::ComboRow,
        pub(crate) palette_selection: gtk::SingleSelection,
        pub(crate) palette_list: gtk::StringList,
        pub(crate) node_group: libadwaita::PreferencesGroup,
        pub(crate) node_relay_row: libadwaita::EntryRow,
        pub(crate) node_relay_attempts_row: libadwaita::SpinRow,
    }

    impl Settings {}

    #[glib::object_subclass]
    impl ObjectSubclass for Settings {
        const NAME: &'static str = "OkuSettings";
        type Type = super::Settings;
        type ParentType = libadwaita::PreferencesDialog;
    }

    impl ObjectImpl for Settings {}
    impl WidgetImpl for Settings {}
    impl PreferencesDialogImpl for Settings {}
    impl AdwDialogImpl for Settings {}
}

glib::wrapper! {
    pub struct Settings(ObjectSubclass<imp::Settings>)
    @extends libadwaita::PreferencesDialog, libadwaita::Dialog, gtk::Widget;
}

pub fn apply_appearance_config(style_manager: &StyleManager, window: &super::window::Window) {
    let config = CONFIG.lock().unwrap();
    style_manager.set_color_scheme(config.colour_scheme().to_adw_scheme());
    config.save();
    drop(config);
    let web_view = window.get_view();
    window.update_color(&web_view, &style_manager);
}

impl Settings {
    pub fn new(app: &libadwaita::Application, window: &super::window::Window) -> Self {
        let this: Self = glib::Object::builder::<Self>().build();
        let imp = this.imp();
        this.set_title("Settings");

        let config = CONFIG.lock().unwrap();
        imp.domain_colour_row.set_active(config.colour_per_domain());
        drop(config);

        let style_manager = app.style_manager();

        let config = CONFIG.lock().unwrap();
        style_manager.set_color_scheme(config.colour_scheme().to_adw_scheme());
        drop(config);

        this.setup_main_page(&style_manager, &window);
        this.setup_colour_scheme_signal(&style_manager, &window);
        this.setup_palette_signal(&style_manager, &window);

        this.set_visible(true);
        this.present(Some(window));

        this
    }

    pub fn save(&self) {
        let config = CONFIG.lock().unwrap();
        config.save()
    }

    pub fn setup_main_page(&self, style_manager: &StyleManager, window: &super::window::Window) {
        let imp = self.imp();

        self.setup_appearance_group(&style_manager, &window);
        self.setup_node_group();

        imp.main_page.add(&imp.appearance_group);
        imp.main_page.add(&imp.node_group);
        self.add(&imp.main_page);
    }

    pub fn apply_node_config(&self) -> miette::Result<()> {
        let imp = self.imp();

        if let Some(node) = NODE.get() {
            if imp.node_relay_row.text().trim().is_empty() {
                node.config.set_relay_connection_config(None)?;
                imp.node_relay_attempts_row.set_sensitive(false);
            } else {
                match node.config.relay_connection_config()? {
                    Some(relay_connection_config) => {
                        relay_connection_config
                            .set_relay_address(imp.node_relay_row.text().trim().to_string())?;
                        imp.node_relay_attempts_row.set_sensitive(true);
                        relay_connection_config.set_relay_connection_attempts(
                            imp.node_relay_attempts_row.value() as i64,
                        )?;
                    }
                    None => {
                        node.config.set_relay_connection_config(Some(
                            OkuFsRelayConnectionConfig::new(
                                imp.node_relay_row.text().trim().to_string(),
                                imp.node_relay_attempts_row.value() as i64,
                            ),
                        ))?;
                        imp.node_relay_attempts_row.set_sensitive(true);
                    }
                }
            }
        }
        Ok(())
    }

    pub fn setup_node_group(&self) {
        let imp = self.imp();

        self.setup_node_relay_row();
        self.setup_node_relay_attempts_row();

        imp.node_group.set_title("Node");
        imp.node_group.set_description(Some(
            "Configuration of the browser's peer-to-peer capabilities",
        ));
        imp.node_group.add(&imp.node_relay_row);
        imp.node_group.add(&imp.node_relay_attempts_row);
    }

    pub fn setup_node_relay_row(&self) {
        let imp = self.imp();

        imp.node_relay_row.set_title("Relay address");
        imp.node_relay_row.set_tooltip_text(Some(
            "An address to a relay server to perform hole punching.",
        ));
        imp.node_relay_row.set_show_apply_button(true);
        let initial_value = match NODE.get() {
            None => String::new(),
            Some(node) => match node.config.relay_connection_config() {
                Ok(relay_connection_config) => match relay_connection_config {
                    Some(relay_connection_config) => {
                        match relay_connection_config.relay_address() {
                            Ok(relay_address) => relay_address,
                            Err(e) => {
                                error!("{}", e);
                                String::new()
                            }
                        }
                    }
                    None => String::new(),
                },
                Err(e) => {
                    error!("{}", e);
                    String::new()
                }
            },
        };
        imp.node_relay_row.set_text(&initial_value);
        if imp.node_relay_row.text().trim().is_empty() {
            imp.node_relay_attempts_row.set_sensitive(false)
        } else {
            imp.node_relay_attempts_row.set_sensitive(true)
        }
        imp.node_relay_row.connect_apply(clone!(
            #[weak(rename_to = this)]
            self,
            move |_node_relay_row| {
                match this.apply_node_config() {
                    Ok(_) => (),
                    Err(e) => error!("{}", e),
                }
            }
        ));
    }

    pub fn setup_node_relay_attempts_row(&self) {
        let imp = self.imp();

        imp.node_relay_attempts_row
            .set_title("Maximum relay connection attempts");
        imp.node_relay_attempts_row
            .set_subtitle("Number of times node should re-attempt connecting to relay");
        let initial_value = match NODE.get() {
            None => 0,
            Some(node) => match node.config.relay_connection_config() {
                Ok(relay_connection_config) => match relay_connection_config {
                    Some(relay_connection_config) => {
                        match relay_connection_config.relay_connection_attempts() {
                            Ok(relay_connection_attempts) => relay_connection_attempts,
                            Err(e) => {
                                error!("{}", e);
                                0
                            }
                        }
                    }
                    None => 0,
                },
                Err(e) => {
                    error!("{}", e);
                    0
                }
            },
        };
        imp.node_relay_attempts_row.configure(
            Some(&gtk::Adjustment::new(
                initial_value as f64,
                0.0,
                i64::MAX as f64,
                1.0,
                10.0,
                0.0,
            )),
            1.0,
            0,
        );
        imp.node_relay_attempts_row.connect_value_notify(clone!(
            #[weak(rename_to = this)]
            self,
            move |_node_relay_attempts_row| {
                match this.apply_node_config() {
                    Ok(_) => (),
                    Err(e) => error!("{}", e),
                }
            }
        ));
    }

    pub fn setup_appearance_group(
        &self,
        style_manager: &StyleManager,
        window: &super::window::Window,
    ) {
        let imp = self.imp();

        self.setup_colour_scheme_row();
        self.setup_domain_colour_row(&style_manager, &window);
        self.setup_palette_row();

        imp.appearance_group.set_title("Appearance");
        imp.appearance_group
            .set_description(Some("Preferences regarding the browser's look &amp; feel"));
        imp.appearance_group.add(&imp.colour_scheme_row);
        imp.appearance_group.add(&imp.domain_colour_row);
        imp.appearance_group.add(&imp.palette_row);
    }

    pub fn setup_colour_scheme_row(&self) {
        let imp = self.imp();

        imp.colour_scheme_list.append("Automatic");
        imp.colour_scheme_list.append("Force Light");
        imp.colour_scheme_list.append("Prefer Light");
        imp.colour_scheme_list.append("Prefer Dark");
        imp.colour_scheme_list.append("Force Dark");
        imp.colour_scheme_selection
            .set_model(Some(&imp.colour_scheme_list));

        imp.colour_scheme_row.set_title("Colour scheme");
        imp.colour_scheme_row
            .set_subtitle("Whether the browser should be light or dark");
        imp.colour_scheme_row
            .set_model(imp.colour_scheme_selection.model().as_ref());

        let config = CONFIG.lock().unwrap();
        let initial_position = match config.colour_scheme() {
            ColourScheme::Default => 0,
            ColourScheme::ForceLight => 1,
            ColourScheme::PreferLight => 2,
            ColourScheme::PreferDark => 3,
            ColourScheme::ForceDark => 4,
            ColourScheme::__Unknown(_i) => 0,
        };
        drop(config);
        imp.colour_scheme_row.set_selected(initial_position);
    }

    pub fn setup_colour_scheme_signal(
        &self,
        style_manager: &StyleManager,
        window: &super::window::Window,
    ) {
        let imp = self.imp();

        imp.colour_scheme_row.connect_selected_notify(clone!(
            #[weak(rename_to = colour_scheme_row)]
            imp.colour_scheme_row,
            #[weak(rename_to = colour_scheme_list)]
            imp.colour_scheme_list,
            #[weak]
            style_manager,
            #[weak]
            window,
            move |_| {
                let selected_string = colour_scheme_list
                    .string(colour_scheme_row.selected())
                    .unwrap();
                let selected_colour_scheme = match selected_string.as_str() {
                    "Automatic" => ColourScheme::Default,
                    "Force Light" => ColourScheme::ForceLight,
                    "Prefer Light" => ColourScheme::PreferLight,
                    "Prefer Dark" => ColourScheme::PreferDark,
                    "Force Dark" => ColourScheme::ForceDark,
                    _ => ColourScheme::Default,
                };
                let config = CONFIG.lock().unwrap();
                config.set_colour_scheme(selected_colour_scheme);
                drop(config);
                apply_appearance_config(&style_manager, &window);
            }
        ));
    }

    pub fn setup_palette_row(&self) {
        let imp = self.imp();

        imp.palette_list.append("None");
        imp.palette_list.append("Blue");
        imp.palette_list.append("Green");
        imp.palette_list.append("Yellow");
        imp.palette_list.append("Orange");
        imp.palette_list.append("Red");
        imp.palette_list.append("Purple");
        imp.palette_list.append("Brown");
        imp.palette_selection.set_model(Some(&imp.palette_list));

        imp.palette_row.set_title("Browser colour");
        imp.palette_row
            .set_subtitle("The colour of the browser window");
        imp.palette_row
            .set_model(imp.palette_selection.model().as_ref());

        let config = CONFIG.lock().unwrap();
        let initial_position = match config.palette() {
            Palette::None => 0,
            Palette::Blue => 1,
            Palette::Green => 2,
            Palette::Yellow => 3,
            Palette::Orange => 4,
            Palette::Red => 5,
            Palette::Purple => 6,
            Palette::Brown => 7,
        };
        drop(config);
        imp.palette_row.set_selected(initial_position);
        imp.palette_row
            .set_sensitive(!imp.domain_colour_row.is_active());
    }

    pub fn setup_palette_signal(
        &self,
        style_manager: &StyleManager,
        window: &super::window::Window,
    ) {
        let imp = self.imp();

        imp.palette_row.connect_selected_notify(clone!(
            #[weak(rename_to = palette_row)]
            imp.palette_row,
            #[weak(rename_to = palette_list)]
            imp.palette_list,
            #[weak]
            style_manager,
            #[weak]
            window,
            move |_| {
                let selected_string = palette_list.string(palette_row.selected()).unwrap();
                let selected_colour = match selected_string.as_str() {
                    "None" => Palette::None,
                    "Blue" => Palette::Blue,
                    "Green" => Palette::Green,
                    "Yellow" => Palette::Yellow,
                    "Orange" => Palette::Orange,
                    "Red" => Palette::Red,
                    "Purple" => Palette::Purple,
                    "Brown" => Palette::Brown,
                    _ => Palette::None,
                };
                let config = CONFIG.lock().unwrap();
                config.set_palette(selected_colour);
                drop(config);
                apply_appearance_config(&style_manager, &window);
            }
        ));
    }

    pub fn setup_domain_colour_row(
        &self,
        style_manager: &StyleManager,
        window: &super::window::Window,
    ) {
        let imp = self.imp();

        imp.domain_colour_row.set_title("Colour cycling");
        imp.domain_colour_row
            .set_subtitle("Change the browser colour for different sites");
        imp.domain_colour_row.connect_active_notify(clone!(
            #[weak]
            imp,
            #[weak]
            style_manager,
            #[weak]
            window,
            move |domain_colour_row| {
                let config = CONFIG.lock().unwrap();
                config.set_colour_per_domain(domain_colour_row.is_active());
                drop(config);
                imp.palette_row
                    .set_sensitive(!domain_colour_row.is_active());
                apply_appearance_config(&style_manager, &window);
            }
        ));
    }
}
