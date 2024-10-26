use super::*;
use glib::clone;
use gtk::glib;
use gtk::subclass::prelude::*;
use libadwaita::prelude::*;

impl Window {
    pub fn setup_note_popover(&self) {
        let imp = self.imp();

        imp.note_popover.set_child(Some(&imp.note_box));
        imp.note_popover.set_parent(&imp.note_button);
        imp.note_popover.set_autohide(true);
    }

    pub fn setup_note_button_clicked(&self) {
        let imp = self.imp();

        imp.note_button.connect_clicked(clone!(
            #[weak(rename_to = note_popover)]
            imp.note_popover,
            move |_| {
                note_popover.popup();
            }
        ));
    }
}
