use std::sync::atomic::Ordering;

use crate::config::{ColourScheme, Palette};
use crate::window_util::get_window_from_widget;
use crate::{CONFIG, HOME_REPLICA_SET, NODE};
use glib::clone;
use gtk::glib;
use gtk::subclass::prelude::*;
use libadwaita::subclass::{dialog::AdwDialogImpl, preferences_dialog::PreferencesDialogImpl};
use libadwaita::{prelude::*, ResponseAppearance, StyleManager};
use log::error;

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
        pub(crate) okunet_group: libadwaita::PreferencesGroup,
        pub(crate) author_row: libadwaita::ActionRow,
        pub(crate) copy_author_button: gtk::Button,
        pub(crate) export_author_button: gtk::Button,
        pub(crate) import_author_button: gtk::Button,
        pub(crate) import_export_buttons: gtk::Box,
        pub(crate) author_buttons: gtk::Box,
        pub(crate) display_name_row: libadwaita::EntryRow,
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
        self.setup_okunet_group();

        imp.main_page.add(&imp.appearance_group);
        imp.main_page.add(&imp.okunet_group);
        self.add(&imp.main_page);
    }

    pub fn setup_okunet_group(&self) {
        let imp = self.imp();

        imp.display_name_row.set_title("Display name");
        imp.display_name_row.set_show_apply_button(true);

        imp.import_author_button
            .set_icon_name("system-switch-user-symbolic");
        imp.import_author_button.add_css_class("circular");
        imp.import_author_button.add_css_class("linked");
        imp.import_author_button
            .set_tooltip_text(Some("Import user credentials"));
        imp.import_author_button.set_valign(gtk::Align::Center);
        imp.import_author_button.connect_clicked(clone!(
            #[weak(rename_to = this)]
            self,
            move |_| {
                glib::spawn_future_local(async move {
                    if let Err(e) = this.import_user().await {
                        error!("{}", e);
                    }
                    this.initialise_okunet_information();
                });
            }
        ));

        imp.export_author_button.set_icon_name("user-info-symbolic");
        imp.export_author_button.add_css_class("circular");
        imp.export_author_button.add_css_class("linked");
        imp.export_author_button
            .set_tooltip_text(Some("Export user credentials"));
        imp.export_author_button.set_valign(gtk::Align::Center);
        imp.export_author_button.connect_clicked(clone!(
            #[weak(rename_to = this)]
            self,
            move |_| {
                glib::spawn_future_local(async move {
                    if let Err(e) = this.save_exported_user().await {
                        error!("{}", e);
                    }
                });
            }
        ));

        imp.copy_author_button.set_icon_name("copy-symbolic");
        imp.copy_author_button.add_css_class("circular");
        imp.copy_author_button.set_valign(gtk::Align::Center);
        imp.copy_author_button.connect_clicked(clone!(
            #[weak(rename_to = this)]
            self,
            #[weak]
            imp,
            move |_| {
                let clipboard = gdk::Display::default().unwrap().clipboard();
                let author_id = imp.author_row.subtitle().unwrap_or_default();
                clipboard.set_text(&author_id);
                let window = get_window_from_widget(&this);
                let app = window.application().unwrap();
                let notification = gio::Notification::new("Author ID copied");
                notification.set_body(Some(&format!(
                    "Author ID ({}) has been copied to the clipboard.",
                    author_id
                )));
                app.send_notification(None, &notification);
            }
        ));

        imp.import_export_buttons.append(&imp.import_author_button);
        imp.import_export_buttons.append(&imp.export_author_button);
        imp.import_export_buttons.add_css_class("linked");

        imp.author_buttons.set_spacing(4);
        imp.author_buttons.append(&imp.copy_author_button);
        imp.author_buttons.append(&imp.import_export_buttons);

        imp.author_row.set_title("Author ID");
        imp.author_row.add_css_class("property");
        imp.author_row.add_css_class("monospace");
        imp.author_row.set_subtitle_lines(1);
        imp.author_row.add_suffix(&imp.author_buttons);

        self.initialise_okunet_information();

        imp.okunet_group.set_title("OkuNet");
        imp.okunet_group
            .set_description(Some("Settings affecting the use of OkuNet"));
        imp.okunet_group.add(&imp.author_row);
        imp.okunet_group.add(&imp.display_name_row);
    }

    pub fn initialise_okunet_information(&self) {
        let imp = self.imp();

        if let Some(node) = NODE.get() {
            let ctx = glib::MainContext::default();
            ctx.spawn_local(clone!(
                #[weak]
                imp,
                async move {
                    match node.default_author().await {
                        Ok(author_id) => imp.author_row.set_subtitle(&author_id.to_string()),
                        Err(e) => error!("{}", e),
                    }
                }
            ));
            match HOME_REPLICA_SET.load(Ordering::Relaxed) {
                true => {
                    ctx.spawn_local(clone!(
                        #[weak]
                        imp,
                        async move {
                            if let Some(current_identity) = node.identity().await {
                                imp.display_name_row.set_text(&current_identity.name);
                            }
                        }
                    ));
                    imp.display_name_row
                        .connect_apply(clone!(move |display_name_row| {
                            ctx.spawn_local(clone!(
                                #[weak]
                                display_name_row,
                                async move {
                                    if let Err(e) = node
                                        .set_display_name(display_name_row.text().to_string())
                                        .await
                                    {
                                        error!("{}", e);
                                    }
                                }
                            ));
                        }));
                }
                false => {
                    imp.display_name_row.set_sensitive(false);
                }
            }
        }
    }

    pub async fn save_exported_user(&self) -> miette::Result<()> {
        let node = NODE
            .get()
            .ok_or(miette::miette!("Oku node has not yet started … "))?;
        let exported_user_toml = node.export_user_toml().await?;
        let dialog = libadwaita::AlertDialog::new(
            Some("Export user credentials?"),
            Some("Do not share your user credentials with anyone."),
        );
        dialog.add_responses(&[("cancel", "Cancel"), ("export", "Export")]);
        dialog.set_response_appearance("cancel", ResponseAppearance::Default);
        dialog.set_response_appearance("export", ResponseAppearance::Destructive);
        dialog.set_default_response(Some("cancel"));
        dialog.set_close_response("cancel");
        dialog.connect_response(
            None,
            clone!(
                #[weak(rename_to = this)]
                self,
                move |_, response| {
                    match response {
                        "cancel" => (),
                        "export" => {
                            let toml_filter = gtk::FileFilter::new();
                            toml_filter.add_pattern("*.toml");
                            let filter_store = gio::ListStore::new::<gtk::FileFilter>();
                            filter_store.append(&toml_filter);
                            let file_dialog = gtk::FileDialog::builder()
                                .accept_label("Export")
                                .initial_name("user.toml")
                                .filters(&filter_store)
                                .title("Select destination for exported user credentials.")
                                .build();
                            file_dialog.save(
                                Some(&get_window_from_widget(&this)),
                                Some(&gio::Cancellable::new()),
                                clone!(
                                    #[strong]
                                    exported_user_toml,
                                    move |destination| {
                                        let path = destination.ok().map(|x| x.path()).flatten();
                                        if let Some(path) = path {
                                            if let Err(e) = std::fs::write(path, exported_user_toml)
                                            {
                                                error!("{}", e);
                                            }
                                        }
                                    }
                                ),
                            )
                        }
                        _ => {
                            unreachable!()
                        }
                    }
                }
            ),
        );
        dialog.present(Some(self));
        Ok(())
    }

    pub async fn import_user(&self) -> miette::Result<()> {
        let node = NODE
            .get()
            .ok_or(miette::miette!("Oku node has not yet started … "))?;
        let dialog = libadwaita::AlertDialog::new(
            Some("Import user credentials?"),
            Some("This will overwrite your existing user credentials. This cannot be undone."),
        );
        dialog.add_responses(&[("cancel", "Cancel"), ("import", "Import")]);
        dialog.set_response_appearance("cancel", ResponseAppearance::Default);
        dialog.set_response_appearance("import", ResponseAppearance::Destructive);
        dialog.set_default_response(Some("cancel"));
        dialog.set_close_response("cancel");
        dialog.connect_response(
            None,
            clone!(
                #[weak(rename_to = this)]
                self,
                move |_, response| {
                    match response {
                        "cancel" => (),
                        "import" => {
                            let toml_filter = gtk::FileFilter::new();
                            toml_filter.add_pattern("*.toml");
                            let filter_store = gio::ListStore::new::<gtk::FileFilter>();
                            filter_store.append(&toml_filter);
                            let file_dialog = gtk::FileDialog::builder()
                                .accept_label("Import")
                                .initial_name("user.toml")
                                .filters(&filter_store)
                                .title("Select destination for exported user credentials.")
                                .build();
                            file_dialog.open(
                                Some(&get_window_from_widget(&this)),
                                Some(&gio::Cancellable::new()),
                                clone!(
                                    #[strong]
                                    node,
                                    move |destination| {
                                        let exported_user_toml = destination
                                            .ok()
                                            .map(|x| x.path())
                                            .flatten()
                                            .map(|x| std::fs::read_to_string(x).ok())
                                            .flatten();
                                        if let Some(exported_user_toml) = exported_user_toml {
                                            glib::spawn_future_local(clone!(async move {
                                                if let Err(e) =
                                                    node.import_user_toml(exported_user_toml).await
                                                {
                                                    error!("{}", e);
                                                }
                                            }));
                                        }
                                    }
                                ),
                            )
                        }
                        _ => {
                            unreachable!()
                        }
                    }
                }
            ),
        );
        dialog.present(Some(self));
        Ok(())
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
