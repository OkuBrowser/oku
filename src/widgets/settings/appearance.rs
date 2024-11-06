use super::core::Settings;
use crate::config::enums::{ColourScheme, Palette};
use glib::{closure, Object};
use gtk::glib;
use gtk::subclass::prelude::*;
use libadwaita::{prelude::*, StyleManager};

impl Settings {
    pub fn setup_appearance_bindings(
        &self,
        style_manager: &StyleManager,
        window: &crate::widgets::window::Window,
    ) {
        let imp = self.imp();

        let config = imp.config.imp();

        // Update configuration from window's perspective
        let window_imp = window.imp();
        imp.config
            .bind_property("colour-per-domain", &window_imp.config, "colour-per-domain")
            .bidirectional()
            .build();
        imp.config
            .bind_property("colour-scheme", &window_imp.config, "colour-scheme")
            .bidirectional()
            .build();
        imp.config
            .bind_property("palette", &window_imp.config, "palette")
            .bidirectional()
            .build();

        // Set initial UI
        imp.domain_colour_row.set_active(config.colour_per_domain());
        imp.colour_scheme_row
            .set_selected(match config.colour_scheme() {
                ColourScheme::Default => 0 as u32,
                ColourScheme::ForceLight => 1 as u32,
                ColourScheme::PreferLight => 2 as u32,
                ColourScheme::PreferDark => 3 as u32,
                ColourScheme::ForceDark => 4 as u32,
            });
        imp.palette_row.set_selected(match config.palette() {
            Palette::None => 0 as u32,
            Palette::Blue => 1 as u32,
            Palette::Green => 2 as u32,
            Palette::Yellow => 3 as u32,
            Palette::Orange => 4 as u32,
            Palette::Red => 5 as u32,
            Palette::Purple => 6 as u32,
            Palette::Brown => 7 as u32,
        });

        // Colour per domain
        imp.config.property_expression("colour-per-domain").bind(
            &imp.domain_colour_row,
            "active",
            gtk::Widget::NONE,
        );
        imp.config
            .property_expression("colour-per-domain")
            .chain_closure::<bool>(closure!(|_: Option<Object>, x: bool| { !x }))
            .bind(&imp.palette_row, "sensitive", gtk::Widget::NONE);
        imp.domain_colour_row.property_expression("active").bind(
            &imp.config,
            "colour-per-domain",
            gtk::Widget::NONE,
        );

        // Colour scheme
        imp.config
            .bind_property("colour-scheme", style_manager, "color-scheme")
            .transform_to(move |_, x: ColourScheme| Some(libadwaita::ColorScheme::from(x.into())))
            .transform_from(move |_, x: libadwaita::ColorScheme| Some(ColourScheme::from(x)))
            .bidirectional()
            .build();
        imp.config
            .bind_property("colour-scheme", &imp.colour_scheme_row, "selected")
            .transform_to(move |_, x: ColourScheme| {
                Some(match x {
                    ColourScheme::Default => 0 as u32,
                    ColourScheme::ForceLight => 1 as u32,
                    ColourScheme::PreferLight => 2 as u32,
                    ColourScheme::PreferDark => 3 as u32,
                    ColourScheme::ForceDark => 4 as u32,
                })
            })
            .transform_from(|_, x: u32| {
                Some(match x {
                    0 => ColourScheme::Default,
                    1 => ColourScheme::ForceLight,
                    2 => ColourScheme::PreferLight,
                    3 => ColourScheme::PreferDark,
                    4 => ColourScheme::ForceDark,
                    _ => ColourScheme::Default,
                })
            })
            .bidirectional()
            .build();

        // Colour palette
        imp.config
            .bind_property("palette", &imp.palette_row, "selected")
            .transform_to(move |_, x: Palette| {
                Some(match x {
                    Palette::None => 0 as u32,
                    Palette::Blue => 1 as u32,
                    Palette::Green => 2 as u32,
                    Palette::Yellow => 3 as u32,
                    Palette::Orange => 4 as u32,
                    Palette::Red => 5 as u32,
                    Palette::Purple => 6 as u32,
                    Palette::Brown => 7 as u32,
                })
            })
            .transform_from(|_, x: u32| {
                Some(match x {
                    0 => Palette::None,
                    1 => Palette::Blue,
                    2 => Palette::Green,
                    3 => Palette::Yellow,
                    4 => Palette::Orange,
                    5 => Palette::Red,
                    6 => Palette::Purple,
                    7 => Palette::Brown,
                    _ => Palette::None,
                })
            })
            .bidirectional()
            .build();
    }

    pub fn setup_appearance_group(
        &self,
        style_manager: &StyleManager,
        window: &crate::widgets::window::Window,
    ) {
        let imp = self.imp();

        self.setup_colour_scheme_row();
        self.setup_domain_colour_row();
        self.setup_palette_row();
        self.setup_appearance_bindings(&style_manager, &window);

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
    }

    pub fn setup_domain_colour_row(&self) {
        let imp = self.imp();

        imp.domain_colour_row.set_title("Colour cycling");
        imp.domain_colour_row
            .set_subtitle("Change the browser colour for different sites");
    }
}
