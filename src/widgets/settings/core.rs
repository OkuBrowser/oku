use crate::config::Config;
use gtk::glib;
use gtk::subclass::prelude::*;
use libadwaita::subclass::{dialog::AdwDialogImpl, preferences_dialog::PreferencesDialogImpl};
use libadwaita::{prelude::*, StyleManager};

pub mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct Settings {
        pub(crate) config: Config,
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

pub fn apply_appearance_config(
    style_manager: &StyleManager,
    window: &crate::widgets::window::Window,
) {
    style_manager.set_color_scheme(window.imp().config.imp().colour_scheme().into());
    let web_view = window.get_view();
    window.update_color(&web_view, &style_manager);
}

impl Settings {
    pub fn new(app: &libadwaita::Application, window: &crate::widgets::window::Window) -> Self {
        let this: Self = glib::Object::builder::<Self>().build();
        this.set_title("Settings");

        let style_manager = app.style_manager();

        this.setup_main_page(&style_manager, &window);

        this.set_visible(true);
        this.present(Some(window));

        this
    }

    pub fn setup_main_page(
        &self,
        style_manager: &StyleManager,
        window: &crate::widgets::window::Window,
    ) {
        let imp = self.imp();

        self.setup_appearance_group(&style_manager, &window);
        self.setup_okunet_group();

        imp.main_page.add(&imp.appearance_group);
        imp.main_page.add(&imp.okunet_group);
        self.add(&imp.main_page);
    }
}
