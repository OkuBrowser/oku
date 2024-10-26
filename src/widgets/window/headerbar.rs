use super::*;
use crate::MOUNT_DIR;
use glib::clone;
use gtk::glib;
use gtk::subclass::prelude::*;
use libadwaita::prelude::*;

impl Window {
    pub fn setup_left_headerbar(&self) {
        self.setup_navigation_buttons();
        let imp = self.imp();

        // Refresh button
        imp.refresh_button.set_can_focus(true);
        imp.refresh_button.set_receives_default(true);
        imp.refresh_button.set_icon_name("view-refresh");

        // Add Tab button
        imp.add_tab_button.set_can_focus(true);
        imp.add_tab_button.set_receives_default(true);
        imp.add_tab_button.set_icon_name("tab-new");

        // Sidebar button
        imp.sidebar_button.set_can_focus(true);
        imp.sidebar_button.set_receives_default(true);
        imp.sidebar_button.set_icon_name("library-symbolic");

        // Left header buttons
        imp.left_header_buttons.append(&imp.navigation_buttons);
        imp.left_header_buttons.append(&imp.refresh_button);
        imp.left_header_buttons.append(&imp.add_tab_button);
        imp.left_header_buttons.append(&imp.sidebar_button);
    }

    pub fn setup_right_headerbar(&self) {
        let imp = self.imp();

        // Overview button
        imp.overview_button.set_can_focus(true);
        imp.overview_button.set_receives_default(true);
        imp.overview_button.set_view(Some(&imp.tab_view));

        // Note button
        imp.note_button.set_can_focus(true);
        imp.note_button.set_receives_default(true);
        imp.note_button.set_icon_name("note-symbolic");

        // Find button
        imp.find_button.set_can_focus(true);
        imp.find_button.set_receives_default(true);
        imp.find_button.set_icon_name("edit-find");

        // Replica menu button
        imp.replicas_button.set_can_focus(true);
        imp.replicas_button.set_receives_default(true);
        imp.replicas_button.set_icon_name("file-cabinet-symbolic");

        // Menu button
        imp.menu_button.set_can_focus(true);
        imp.menu_button.set_receives_default(true);
        imp.menu_button.set_icon_name("document-properties");

        imp.right_header_buttons.append(&imp.overview_button);
        imp.right_header_buttons.append(&imp.note_button);
        imp.right_header_buttons.append(&imp.find_button);
        imp.right_header_buttons.append(&imp.replicas_button);
        imp.right_header_buttons.append(&imp.menu_button);
    }

    pub fn setup_headerbar(&self) {
        self.setup_left_headerbar();
        self.setup_right_headerbar();
        let imp = self.imp();
        // HeaderBar
        imp.headerbar.set_can_focus(true);
        imp.headerbar.set_title_widget(Some(&imp.nav_entry));
        imp.headerbar.pack_start(&imp.left_header_buttons);
        imp.headerbar.pack_end(&imp.right_header_buttons);
    }

    pub fn setup_overview_button_clicked(&self) {
        let imp = self.imp();

        imp.overview_button.connect_clicked(clone!(
            #[weak(rename_to = tab_overview)]
            imp.tab_overview,
            move |_| {
                tab_overview.set_open(!tab_overview.is_open());
            }
        ));
    }

    pub fn setup_replicas_button_clicked(&self) {
        let imp = self.imp();

        imp.replicas_button.connect_clicked(clone!(move |_| {
            let _ = open::that_detached(MOUNT_DIR.to_path_buf());
        }));
    }
}
