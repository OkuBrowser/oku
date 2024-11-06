use super::*;
use crate::config::enums::Palette;
use gtk::subclass::prelude::*;
use std::hash::{Hash, Hasher};
use webkit2gtk::prelude::WebViewExt;

impl Window {
    pub fn update_color(
        &self,
        web_view: &webkit2gtk::WebView,
        style_manager: &libadwaita::StyleManager,
    ) {
        let imp = self.imp();

        let config = imp.config.imp();
        if !config.colour_per_domain() {
            imp.style_provider.borrow().load_from_string("");
            if config.palette() != Palette::None {
                self.update_from_palette(&web_view, &style_manager, &config.palette());
            }
        } else {
            self.update_domain_color(&web_view, &style_manager);
        }
    }

    pub fn update_from_palette(
        &self,
        web_view: &webkit2gtk::WebView,
        style_manager: &libadwaita::StyleManager,
        palette_colour: &Palette,
    ) {
        let imp = self.imp();

        let hue = palette_colour.hue();
        let is_dark = style_manager.is_dark();
        let stylesheet = if is_dark {
            format!(
                "
                    @define-color view_bg_color hsl({hue}, 20%, 8%);
                    @define-color view_fg_color hsl({hue}, 100%, 98%);
                    @define-color window_bg_color hsl({hue}, 20%, 8%);
                    @define-color window_fg_color hsl({hue}, 100%, 98%);
                    @define-color dialog_bg_color hsl({hue}, 20%, 8%);
                    @define-color dialog_fg_color hsl({hue}, 100%, 98%);
                    @define-color popover_bg_color hsl({hue}, 20%, 8%);
                    @define-color popover_fg_color hsl({hue}, 100%, 98%);
                    @define-color card_bg_color hsl({hue}, 20%, 8%);
                    @define-color card_fg_color hsl({hue}, 100%, 98%);
                    @define-color headerbar_bg_color hsl({hue}, 80%, 10%);
                    @define-color headerbar_fg_color hsl({hue}, 100%, 98%);
                    @define-color sidebar_bg_color hsl({hue}, 80%, 10%);
                    @define-color sidebar_fg_color hsl({hue}, 100%, 98%);
                "
            )
        } else {
            format!(
                "
                    @define-color view_bg_color hsl({hue}, 100%, 99%);
                    @define-color view_fg_color hsl({hue}, 100%, 12%);
                    @define-color window_bg_color hsl({hue}, 100%, 99%);
                    @define-color window_fg_color hsl({hue}, 100%, 12%);
                    @define-color dialog_bg_color hsl({hue}, 100%, 99%);
                    @define-color dialog_fg_color hsl({hue}, 100%, 12%);
                    @define-color popover_bg_color hsl({hue}, 100%, 99%);
                    @define-color popover_fg_color hsl({hue}, 100%, 12%);
                    @define-color card_bg_color hsl({hue}, 100%, 99%);
                    @define-color card_fg_color hsl({hue}, 100%, 12%);
                    @define-color headerbar_bg_color hsl({hue}, 100%, 96%);
                    @define-color headerbar_fg_color hsl({hue}, 100%, 12%);
                    @define-color sidebar_bg_color hsl({hue}, 100%, 96%);
                    @define-color sidebar_fg_color hsl({hue}, 100%, 12%);
                "
            )
        };
        let rgba = gdk::RGBA::parse(if is_dark {
            format!("hsl({hue}, 20%, 8%)")
        } else {
            format!("hsl({hue}, 100%, 99%)")
        })
        .unwrap_or(if is_dark {
            gdk::RGBA::BLACK
        } else {
            gdk::RGBA::WHITE
        });
        web_view.set_background_color(&rgba);

        imp.style_provider.borrow().load_from_string(&stylesheet);
    }

    /// Adapted from Geopard (https://github.com/ranfdev/Geopard)
    pub fn update_domain_color(
        &self,
        web_view: &webkit2gtk::WebView,
        style_manager: &libadwaita::StyleManager,
    ) {
        let imp = self.imp();

        let url = web_view.uri().unwrap_or("about:blank".into());
        let parsed_url = url::Url::parse(&url);
        let domain = parsed_url
            .as_ref()
            .map(|u| u.domain())
            .ok()
            .flatten()
            .unwrap_or(&url)
            .to_string();

        let hash = {
            let mut s = std::collections::hash_map::DefaultHasher::new();
            domain.hash(&mut s);
            s.finish()
        };

        let hue = hash % 360;
        let is_dark = style_manager.is_dark();
        let stylesheet = if is_dark {
            format!(
                "
                    @define-color view_bg_color hsl({hue}, 20%, 8%);
                    @define-color view_fg_color hsl({hue}, 100%, 98%);
                    @define-color window_bg_color hsl({hue}, 20%, 8%);
                    @define-color window_fg_color hsl({hue}, 100%, 98%);
                    @define-color dialog_bg_color hsl({hue}, 20%, 8%);
                    @define-color dialog_fg_color hsl({hue}, 100%, 98%);
                    @define-color popover_bg_color hsl({hue}, 20%, 8%);
                    @define-color popover_fg_color hsl({hue}, 100%, 98%);
                    @define-color card_bg_color hsl({hue}, 20%, 8%);
                    @define-color card_fg_color hsl({hue}, 100%, 98%);
                    @define-color headerbar_bg_color hsl({hue}, 80%, 10%);
                    @define-color headerbar_fg_color hsl({hue}, 100%, 98%);
                    @define-color sidebar_bg_color hsl({hue}, 80%, 10%);
                    @define-color sidebar_fg_color hsl({hue}, 100%, 98%);
                "
            )
        } else {
            format!(
                "
                    @define-color view_bg_color hsl({hue}, 100%, 99%);
                    @define-color view_fg_color hsl({hue}, 100%, 12%);
                    @define-color window_bg_color hsl({hue}, 100%, 99%);
                    @define-color window_fg_color hsl({hue}, 100%, 12%);
                    @define-color dialog_bg_color hsl({hue}, 100%, 99%);
                    @define-color dialog_fg_color hsl({hue}, 100%, 12%);
                    @define-color popover_bg_color hsl({hue}, 100%, 99%);
                    @define-color popover_fg_color hsl({hue}, 100%, 12%);
                    @define-color card_bg_color hsl({hue}, 100%, 99%);
                    @define-color card_fg_color hsl({hue}, 100%, 12%);
                    @define-color headerbar_bg_color hsl({hue}, 100%, 96%);
                    @define-color headerbar_fg_color hsl({hue}, 100%, 12%);
                    @define-color sidebar_bg_color hsl({hue}, 100%, 96%);
                    @define-color sidebar_fg_color hsl({hue}, 100%, 12%);
                "
            )
        };
        let rgba = gdk::RGBA::parse(if is_dark {
            format!("hsl({hue}, 20%, 8%)")
        } else {
            format!("hsl({hue}, 100%, 99%)")
        })
        .unwrap_or(if is_dark {
            gdk::RGBA::BLACK
        } else {
            gdk::RGBA::WHITE
        });
        web_view.set_background_color(&rgba);

        imp.style_provider.borrow().load_from_string(&stylesheet);
    }
}
