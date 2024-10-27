use super::*;
use crate::widgets;
use glib::clone;
use gtk::glib;
use gtk::subclass::prelude::*;
use libadwaita::prelude::*;

impl Window {
    pub fn setup_note_button_clicked(&self) {
        let imp = self.imp();

        imp.note_button.connect_clicked(clone!(
            #[weak(rename_to = this)]
            self,
            move |_| {
                widgets::note_editor::NoteEditor::new(Some(&this), None);
            }
        ));
    }
}
