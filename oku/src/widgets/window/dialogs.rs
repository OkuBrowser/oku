use super::*;
use crate::{APP_ID, VERSION};
use gtk::prelude::GtkWindowExt;
use libadwaita::prelude::*;

fn make_shortcut(action_name: &str, accelerator: &str, title: &str) -> libadwaita::ShortcutsItem {
    let shortcut = libadwaita::ShortcutsItem::new(title, accelerator);
    shortcut.set_action_name(action_name);
    shortcut
}

impl Window {
    pub fn about_dialog(&self) {
        let about_dialog = libadwaita::AboutDialog::builder()
            .version(VERSION.to_string())
            .application_name("Oku")
            .developer_name("Emil Sayahi")
            .application_icon(APP_ID)
            .license_type(gtk::License::Agpl30)
            .issue_url("https://github.com/OkuBrowser/oku/issues")
            .website("https://okubrowser.github.io")
            .build();
        about_dialog.present(Some(self));
    }

    pub fn shortcuts_window(&self) {
        let previous = make_shortcut(
            "win.previous",
            "<Alt>Left <Alt>KP_Left <Ctrl>bracketleft",
            "Go back",
        );
        let next = make_shortcut(
            "win.next",
            "<Alt>Right <Alt>KP_Right <Ctrl>bracketright",
            "Go forward",
        );
        let reload = make_shortcut("win.reload", "<Ctrl>r F5", "Refresh page");
        let reload_bypass = make_shortcut(
            "win.reload-bypass",
            "<Ctrl><Shift>r <Shift>F5",
            "Refresh page, bypassing cache",
        );
        let new_tab = make_shortcut("win.new-tab", "<Ctrl>t", "Create new tab");
        let close_tab = make_shortcut("win.close-tab", "<Ctrl>w", "Close current tab");
        let zoom_in = make_shortcut("win.zoom-in", "<Ctrl><Shift>plus", "Zoom in");
        let zoom_out = make_shortcut("win.zoom-out", "<Ctrl>minus", "Zoom out");
        let reset_zoom = make_shortcut("win.reset-zoom", "<Ctrl>0", "Reset zoom level");
        let find = make_shortcut("win.find", "<Ctrl>f", "Find in page");
        let print = make_shortcut("win.print", "<Ctrl>p", "Print current page");
        let fullscreen = make_shortcut("win.fullscreen", "F11", "Toggle fullscreen");
        let save = make_shortcut("win.save", "<Ctrl>s", "Save current page");
        let new = make_shortcut("win.new", "<Ctrl>n", "New window");
        let new_private = make_shortcut("win.new-private", "<Ctrl><Shift>p", "New private window");
        let go_home = make_shortcut("win.go-home", "<Alt>Home", "Go home");
        let stop_loading = make_shortcut("win.stop-loading", "Escape", "Stop loading");
        let next_find = make_shortcut("win.next-find", "<Ctrl>g", "Find next result");
        let previous_find = make_shortcut(
            "win.previous-find",
            "<Ctrl><Shift>g",
            "Find previous result",
        );
        let screenshot = make_shortcut("win.screenshot", "<Ctrl><Shift>s", "Take screenshot");
        let settings = make_shortcut("win.settings", "<Ctrl>comma", "Open settings");
        let view_source = make_shortcut("win.view-source", "<Ctrl>u", "View page source");
        let shortcuts = make_shortcut(
            "win.shortcuts",
            "<Ctrl><Shift>question",
            "View keyboard shortcuts",
        );
        let open_file = make_shortcut("win.open-file", "<Ctrl>o", "Open file");
        let inspector = make_shortcut("win.inspector", "<Ctrl><Shift>i F12", "Show inspector");
        let close_window =
            make_shortcut("win.close-window", "<Ctrl>q <Ctrl><Shift>w", "Close window");
        let library = make_shortcut("win.library", "<Ctrl>d", "Toggle library");
        let next_tab = make_shortcut(
            "win.next-tab",
            "<Ctrl>Page_Down <Ctrl>Tab",
            "Switch to next tab",
        );
        let previous_tab = make_shortcut(
            "win.previous-tab",
            "<Ctrl>Page_Up <Ctrl><Shift>Tab",
            "Switch to previous tab",
        );
        let current_tab_left = make_shortcut(
            "win.current-tab-left",
            "<Ctrl><Shift>Page_Up",
            "Move current tab left",
        );
        let current_tab_right = make_shortcut(
            "win.current-tab-right",
            "<Ctrl><Shift>Page_Down",
            "Move current tab right",
        );
        let duplicate_current_tab = make_shortcut(
            "win.duplicate-current-tab",
            "<Ctrl><Shift>k",
            "Duplicate current tab",
        );
        let tab_overview =
            make_shortcut("win.tab-overview", "<Ctrl><Shift>o", "Toggle tab overview");
        let navigation_group = libadwaita::ShortcutsSection::new(Some("Navigation"));
        navigation_group.add(go_home);
        navigation_group.add(previous);
        navigation_group.add(next);
        navigation_group.add(stop_loading);
        navigation_group.add(reload);
        navigation_group.add(reload_bypass);
        navigation_group.add(open_file);
        let tabs_group = libadwaita::ShortcutsSection::new(Some("Tabs"));
        tabs_group.add(new_tab);
        tabs_group.add(close_tab);
        tabs_group.add(next_tab);
        tabs_group.add(previous_tab);
        tabs_group.add(current_tab_left);
        tabs_group.add(current_tab_right);
        tabs_group.add(duplicate_current_tab);
        tabs_group.add(tab_overview);
        let view_group = libadwaita::ShortcutsSection::new(Some("View"));
        view_group.add(zoom_in);
        view_group.add(zoom_out);
        view_group.add(reset_zoom);
        view_group.add(fullscreen);
        let find_group = libadwaita::ShortcutsSection::new(Some("Finding"));
        find_group.add(find);
        find_group.add(next_find);
        find_group.add(previous_find);
        let general_group = libadwaita::ShortcutsSection::new(Some("General"));
        general_group.add(print);
        general_group.add(save);
        general_group.add(screenshot);
        general_group.add(view_source);
        general_group.add(new);
        general_group.add(new_private);
        general_group.add(close_window);
        general_group.add(inspector);
        general_group.add(library);
        general_group.add(settings);
        general_group.add(shortcuts);
        let shortcuts_window = libadwaita::ShortcutsDialog::builder()
            .title("Shortcuts")
            .build();
        shortcuts_window.add(navigation_group);
        shortcuts_window.add(tabs_group);
        shortcuts_window.add(view_group);
        shortcuts_window.add(general_group);
        shortcuts_window.present(gtk::Widget::NONE);
    }
}
