use super::*;
use crate::VERSION;
use gtk::prelude::GtkWindowExt;
use libadwaita::prelude::*;

impl Window {
    pub fn about_dialog(&self) {
        let about_dialog = libadwaita::AboutDialog::builder()
            .version(VERSION.to_string())
            .application_name("Oku")
            .developer_name("Emil Sayahi")
            .application_icon("io.github.OkuBrowser.oku")
            .license_type(gtk::License::Agpl30)
            .issue_url("https://github.com/OkuBrowser/oku/issues")
            .website("https://okubrowser.github.io")
            .build();
        about_dialog.present(Some(self));
    }

    pub fn shortcuts_window(&self) {
        let previous = gtk::ShortcutsShortcut::builder()
            .shortcut_type(gtk::ShortcutType::Accelerator)
            .action_name("win.previous")
            .accelerator("<Alt>Left <Alt>KP_Left <Ctrl>bracketleft")
            .title("Go back")
            .build();
        let next = gtk::ShortcutsShortcut::builder()
            .shortcut_type(gtk::ShortcutType::Accelerator)
            .action_name("win.next")
            .accelerator("<Alt>Right <Alt>KP_Right <Ctrl>bracketright")
            .title("Go forward")
            .build();
        let reload = gtk::ShortcutsShortcut::builder()
            .shortcut_type(gtk::ShortcutType::Accelerator)
            .action_name("win.reload")
            .accelerator("<Ctrl>r F5")
            .title("Refresh page")
            .build();
        let reload_bypass = gtk::ShortcutsShortcut::builder()
            .shortcut_type(gtk::ShortcutType::Accelerator)
            .action_name("win.reload-bypass")
            .accelerator("<Ctrl><Shift>r <Shift>F5")
            .title("Refresh page, bypassing cache")
            .build();
        let new_tab = gtk::ShortcutsShortcut::builder()
            .shortcut_type(gtk::ShortcutType::Accelerator)
            .action_name("win.new-tab")
            .accelerator("<Ctrl>t")
            .title("Create new tab")
            .build();
        let close_tab = gtk::ShortcutsShortcut::builder()
            .shortcut_type(gtk::ShortcutType::Accelerator)
            .action_name("win.close-tab")
            .accelerator("<Ctrl>w")
            .title("Close current tab")
            .build();
        let zoom_in = gtk::ShortcutsShortcut::builder()
            .shortcut_type(gtk::ShortcutType::Accelerator)
            .action_name("win.zoom-in")
            .accelerator("<Ctrl><Shift>plus")
            .title("Zoom in")
            .build();
        let zoom_out = gtk::ShortcutsShortcut::builder()
            .shortcut_type(gtk::ShortcutType::Accelerator)
            .action_name("win.zoom-out")
            .accelerator("<Ctrl>minus")
            .title("Zoom out")
            .build();
        let reset_zoom = gtk::ShortcutsShortcut::builder()
            .shortcut_type(gtk::ShortcutType::Accelerator)
            .action_name("win.reset-zoom")
            .accelerator("<Ctrl>0")
            .title("Reset zoom level")
            .build();
        let find = gtk::ShortcutsShortcut::builder()
            .shortcut_type(gtk::ShortcutType::Accelerator)
            .action_name("win.find")
            .accelerator("<Ctrl>f")
            .title("Find in page")
            .build();
        let print = gtk::ShortcutsShortcut::builder()
            .shortcut_type(gtk::ShortcutType::Accelerator)
            .action_name("win.print")
            .accelerator("<Ctrl>p")
            .title("Print current page")
            .build();
        let fullscreen = gtk::ShortcutsShortcut::builder()
            .shortcut_type(gtk::ShortcutType::Accelerator)
            .action_name("win.fullscreen")
            .accelerator("F11")
            .title("Toggle fullscreen")
            .build();
        let save = gtk::ShortcutsShortcut::builder()
            .shortcut_type(gtk::ShortcutType::Accelerator)
            .action_name("win.save")
            .accelerator("<Ctrl>s")
            .title("Save current page")
            .build();
        let new = gtk::ShortcutsShortcut::builder()
            .shortcut_type(gtk::ShortcutType::Accelerator)
            .action_name("win.new")
            .accelerator("<Ctrl>n")
            .title("New window")
            .build();
        let new_private = gtk::ShortcutsShortcut::builder()
            .shortcut_type(gtk::ShortcutType::Accelerator)
            .action_name("win.new-private")
            .accelerator("<Ctrl><Shift>p")
            .title("New private window")
            .build();
        let go_home = gtk::ShortcutsShortcut::builder()
            .shortcut_type(gtk::ShortcutType::Accelerator)
            .action_name("win.go-home")
            .accelerator("<Alt>Home")
            .title("Go home")
            .build();
        let stop_loading = gtk::ShortcutsShortcut::builder()
            .shortcut_type(gtk::ShortcutType::Accelerator)
            .action_name("win.stop-loading")
            .accelerator("Escape")
            .title("Stop loading")
            .build();
        let next_find = gtk::ShortcutsShortcut::builder()
            .shortcut_type(gtk::ShortcutType::Accelerator)
            .action_name("win.next-find")
            .accelerator("<Ctrl>g")
            .title("Find next result")
            .build();
        let previous_find = gtk::ShortcutsShortcut::builder()
            .shortcut_type(gtk::ShortcutType::Accelerator)
            .action_name("win.previous-find")
            .accelerator("<Ctrl><Shift>g")
            .title("Find previous result")
            .build();
        let screenshot = gtk::ShortcutsShortcut::builder()
            .shortcut_type(gtk::ShortcutType::Accelerator)
            .action_name("win.screenshot")
            .accelerator("<Ctrl><Shift>s")
            .title("Take screenshot")
            .build();
        let settings = gtk::ShortcutsShortcut::builder()
            .shortcut_type(gtk::ShortcutType::Accelerator)
            .action_name("win.settings")
            .accelerator("<Ctrl>comma")
            .title("Open settings")
            .build();
        let view_source = gtk::ShortcutsShortcut::builder()
            .shortcut_type(gtk::ShortcutType::Accelerator)
            .action_name("win.view-source")
            .accelerator("<Ctrl>u")
            .title("View page source")
            .build();
        let shortcuts = gtk::ShortcutsShortcut::builder()
            .shortcut_type(gtk::ShortcutType::Accelerator)
            .action_name("win.shortcuts")
            .accelerator("<Ctrl><Shift>question")
            .title("View keyboard shortcuts")
            .build();
        let open_file = gtk::ShortcutsShortcut::builder()
            .shortcut_type(gtk::ShortcutType::Accelerator)
            .action_name("win.open-file")
            .accelerator("<Ctrl>o")
            .title("Open file")
            .build();
        let inspector = gtk::ShortcutsShortcut::builder()
            .shortcut_type(gtk::ShortcutType::Accelerator)
            .action_name("win.inspector")
            .accelerator("<Ctrl><Shift>i F12")
            .title("Show inspector")
            .build();
        let close_window = gtk::ShortcutsShortcut::builder()
            .shortcut_type(gtk::ShortcutType::Accelerator)
            .action_name("win.close-window")
            .accelerator("<Ctrl>q <Ctrl><Shift>w")
            .title("Close window")
            .build();
        let library = gtk::ShortcutsShortcut::builder()
            .shortcut_type(gtk::ShortcutType::Accelerator)
            .action_name("win.library")
            .accelerator("<Ctrl>d")
            .title("Toggle library")
            .build();
        let next_tab = gtk::ShortcutsShortcut::builder()
            .shortcut_type(gtk::ShortcutType::Accelerator)
            .action_name("win.next-tab")
            .accelerator("<Ctrl>Page_Down <Ctrl>Tab")
            .title("Switch to next tab")
            .build();
        let previous_tab = gtk::ShortcutsShortcut::builder()
            .shortcut_type(gtk::ShortcutType::Accelerator)
            .action_name("win.previous-tab")
            .accelerator("<Ctrl>Page_Up <Ctrl><Shift>Tab")
            .title("Switch to previous tab")
            .build();
        let current_tab_left = gtk::ShortcutsShortcut::builder()
            .shortcut_type(gtk::ShortcutType::Accelerator)
            .action_name("win.current-tab-left")
            .accelerator("<Ctrl><Shift>Page_Up")
            .title("Move current tab left")
            .build();
        let current_tab_right = gtk::ShortcutsShortcut::builder()
            .shortcut_type(gtk::ShortcutType::Accelerator)
            .action_name("win.current-tab-right")
            .accelerator("<Ctrl><Shift>Page_Down")
            .title("Move current tab right")
            .build();
        let duplicate_current_tab = gtk::ShortcutsShortcut::builder()
            .shortcut_type(gtk::ShortcutType::Accelerator)
            .action_name("win.duplicate-current-tab")
            .accelerator("<Ctrl><Shift>k")
            .title("Duplicate current tab")
            .build();
        let tab_overview = gtk::ShortcutsShortcut::builder()
            .shortcut_type(gtk::ShortcutType::Accelerator)
            .action_name("win.tab-overview")
            .accelerator("<Ctrl><Shift>o")
            .title("Toggle tab overview")
            .build();
        let navigation_group = gtk::ShortcutsGroup::builder().title("Navigation").build();
        navigation_group.add_shortcut(&go_home);
        navigation_group.add_shortcut(&previous);
        navigation_group.add_shortcut(&next);
        navigation_group.add_shortcut(&stop_loading);
        navigation_group.add_shortcut(&reload);
        navigation_group.add_shortcut(&reload_bypass);
        navigation_group.add_shortcut(&open_file);
        let tabs_group = gtk::ShortcutsGroup::builder().title("Tabs").build();
        tabs_group.add_shortcut(&new_tab);
        tabs_group.add_shortcut(&close_tab);
        tabs_group.add_shortcut(&next_tab);
        tabs_group.add_shortcut(&previous_tab);
        tabs_group.add_shortcut(&current_tab_left);
        tabs_group.add_shortcut(&current_tab_right);
        tabs_group.add_shortcut(&duplicate_current_tab);
        tabs_group.add_shortcut(&tab_overview);
        let view_group = gtk::ShortcutsGroup::builder().title("View").build();
        view_group.add_shortcut(&zoom_in);
        view_group.add_shortcut(&zoom_out);
        view_group.add_shortcut(&reset_zoom);
        view_group.add_shortcut(&fullscreen);
        let find_group = gtk::ShortcutsGroup::builder().title("Finding").build();
        find_group.add_shortcut(&find);
        find_group.add_shortcut(&next_find);
        find_group.add_shortcut(&previous_find);
        let general_group = gtk::ShortcutsGroup::builder().title("General").build();
        general_group.add_shortcut(&print);
        general_group.add_shortcut(&save);
        general_group.add_shortcut(&screenshot);
        general_group.add_shortcut(&view_source);
        general_group.add_shortcut(&new);
        general_group.add_shortcut(&new_private);
        general_group.add_shortcut(&close_window);
        general_group.add_shortcut(&inspector);
        general_group.add_shortcut(&library);
        general_group.add_shortcut(&settings);
        general_group.add_shortcut(&shortcuts);
        let main_section = gtk::ShortcutsSection::builder().build();
        main_section.add_group(&navigation_group);
        main_section.add_group(&tabs_group);
        main_section.add_group(&view_group);
        main_section.add_group(&general_group);
        let shortcuts_window = gtk::ShortcutsWindow::builder()
            .title("Shortcuts")
            .modal(true)
            .build();
        shortcuts_window.add_section(&main_section);
        shortcuts_window.present();
    }
}
