use glib::{clone, Properties};
use gtk::glib;
use gtk::subclass::prelude::*;
use libadwaita::subclass::{dialog::AdwDialogImpl, preferences_dialog::PreferencesDialogImpl};
use libadwaita::{prelude::*, StyleManager};
use std::cell::Cell;

#[derive(Default, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Clone, Copy)]
#[non_exhaustive]
pub enum ColourScheme {
    #[default]
    Default,
    ForceLight,
    PreferLight,
    PreferDark,
    ForceDark,
    __Unknown(i32),
}

impl ColourScheme {
    pub fn to_adw_scheme(&self) -> libadwaita::ColorScheme {
        match self {
            Self::Default => libadwaita::ColorScheme::Default,
            Self::ForceLight => libadwaita::ColorScheme::ForceLight,
            Self::PreferLight => libadwaita::ColorScheme::PreferLight,
            Self::PreferDark => libadwaita::ColorScheme::PreferDark,
            Self::ForceDark => libadwaita::ColorScheme::ForceDark,
            Self::__Unknown(i) => libadwaita::ColorScheme::__Unknown(*i),
        }
    }
}

pub mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::Settings)]
    pub struct Settings {
        pub(crate) colour_scheme: Cell<ColourScheme>,
        pub(crate) main_page: libadwaita::PreferencesPage,
        pub(crate) appearance_group: libadwaita::PreferencesGroup,
        pub(crate) colour_scheme_row: libadwaita::ComboRow,
        pub(crate) colour_scheme_selection: gtk::SingleSelection,
        pub(crate) colour_scheme_list: gtk::StringList,
    }

    impl Settings {}

    #[glib::object_subclass]
    impl ObjectSubclass for Settings {
        const NAME: &'static str = "OkuSettings";
        type Type = super::Settings;
        type ParentType = libadwaita::PreferencesDialog;
    }

    impl ObjectImpl for Settings {
        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            Self::derived_set_property(self, id, value, pspec)
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            Self::derived_property(self, id, pspec)
        }
    }
    impl WidgetImpl for Settings {}
    impl PreferencesDialogImpl for Settings {}
    impl AdwDialogImpl for Settings {}
}

glib::wrapper! {
    pub struct Settings(ObjectSubclass<imp::Settings>)
    @extends libadwaita::PreferencesDialog, libadwaita::Dialog, gtk::Widget;
}

impl Settings {
    pub fn new(app: &libadwaita::Application, window: &super::window::Window) -> Self {
        let this: Self = glib::Object::builder::<Self>().build();

        this.set_title("Settings");

        let imp = this.imp();

        let style_manager = app.style_manager();
        style_manager.set_color_scheme(imp.colour_scheme.take().to_adw_scheme());

        this.setup_main_page();
        this.setup_colour_scheme_signal(&style_manager);
        this.set_visible(true);
        this.present(Some(window));

        this
    }

    pub fn setup_main_page(&self) {
        let imp = self.imp();

        self.setup_appearance_group();

        imp.main_page.add(&imp.appearance_group);
        self.add(&imp.main_page);
    }

    pub fn setup_appearance_group(&self) {
        let imp = self.imp();

        self.setup_colour_scheme_row();

        imp.appearance_group.set_title("Appearance");
        imp.appearance_group
            .set_description(Some("Preferences regarding the browser's look &amp; feel."));
        imp.appearance_group.add(&imp.colour_scheme_row);
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

        imp.colour_scheme_row.set_title("Colour Scheme");
        imp.colour_scheme_row
            .set_model(imp.colour_scheme_selection.model().as_ref());
    }

    pub fn setup_colour_scheme_signal(&self, style_manager: &StyleManager) {
        let imp = self.imp();

        imp.colour_scheme_row.connect_selected_notify(clone!(
            #[weak(rename_to = colour_scheme_row)]
            imp.colour_scheme_row,
            #[weak(rename_to = colour_scheme_list)]
            imp.colour_scheme_list,
            #[weak]
            imp,
            #[weak]
            style_manager,
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
                imp.colour_scheme.replace(selected_colour_scheme);
                style_manager.set_color_scheme(imp.colour_scheme.take().to_adw_scheme());
            }
        ));
    }
}
