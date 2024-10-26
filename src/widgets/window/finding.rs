use super::*;
use glib::clone;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};
use libadwaita::prelude::*;
use webkit2gtk::prelude::WebViewExt;
use webkit2gtk::FindOptions;

impl Window {
    pub fn get_find_options(&self) -> FindOptions {
        let imp = self.imp();

        let mut find_options = FindOptions::empty();

        find_options.set(
            webkit2gtk::FindOptions::CASE_INSENSITIVE,
            imp.find_case_insensitive.is_active(),
        );
        find_options.set(
            webkit2gtk::FindOptions::AT_WORD_STARTS,
            imp.find_at_word_starts.is_active(),
        );
        find_options.set(
            webkit2gtk::FindOptions::TREAT_MEDIAL_CAPITAL_AS_WORD_START,
            imp.find_treat_medial_capital_as_word_start.is_active(),
        );
        find_options.set(
            webkit2gtk::FindOptions::BACKWARDS,
            imp.find_backwards.is_active(),
        );
        find_options.set(
            webkit2gtk::FindOptions::WRAP_AROUND,
            imp.find_wrap_around.is_active(),
        );

        find_options
    }

    pub fn setup_find_signals(&self) {
        let imp = self.imp();

        imp.find_search_entry.connect_search_changed(clone!(
            #[weak]
            imp,
            #[weak(rename_to = this)]
            self,
            move |find_search_entry| {
                let web_view = this.get_view();
                let find_controller = web_view.find_controller().unwrap();
                let find_options = this.get_find_options();
                find_controller.search(&find_search_entry.text(), find_options.bits(), u32::MAX);
                imp.find_search_entry.connect_activate(clone!(
                    #[weak]
                    find_controller,
                    move |_find_search_entry| find_controller.search_next()
                ));
                imp.find_search_entry.connect_next_match(clone!(
                    #[weak]
                    find_controller,
                    move |_find_search_entry| find_controller.search_next()
                ));
                imp.find_search_entry.connect_previous_match(clone!(
                    #[weak]
                    find_controller,
                    move |_find_search_entry| find_controller.search_previous()
                ));
                imp.find_search_entry.connect_stop_search(clone!(
                    #[weak]
                    imp,
                    #[weak]
                    find_controller,
                    move |_find_search_entry| {
                        imp.total_matches_label.set_text("");
                        find_controller.search_finish()
                    }
                ));
                imp.next_find_button.connect_clicked(clone!(
                    #[weak]
                    find_controller,
                    move |_next_find_button| find_controller.search_next()
                ));
                let action_next_find = gio::ActionEntry::builder("next-find")
                    .activate(clone!(
                        #[weak]
                        find_controller,
                        move |_window: &Self, _, _| find_controller.search_next()
                    ))
                    .build();
                this.add_action_entries([action_next_find]);
                imp.previous_find_button.connect_clicked(clone!(
                    #[weak]
                    find_controller,
                    move |_previous_find_button| find_controller.search_previous()
                ));
                let action_previous_find = gio::ActionEntry::builder("previous-find")
                    .activate(clone!(
                        #[weak]
                        find_controller,
                        move |_window: &Self, _, _| find_controller.search_previous()
                    ))
                    .build();
                this.add_action_entries([action_previous_find]);
                imp.find_case_insensitive.connect_clicked(clone!(
                    #[weak]
                    imp,
                    #[weak]
                    find_controller,
                    move |_find_case_insensitive| {
                        imp.total_matches_label.set_text("");
                        find_controller.search_finish()
                    }
                ));
                imp.find_at_word_starts.connect_clicked(clone!(
                    #[weak]
                    imp,
                    #[weak]
                    find_controller,
                    move |find_at_word_starts| {
                        if find_at_word_starts.is_active() {
                            imp.find_treat_medial_capital_as_word_start
                                .set_sensitive(true);
                        } else {
                            imp.find_treat_medial_capital_as_word_start
                                .set_sensitive(false);
                        }
                        imp.total_matches_label.set_text("");
                        find_controller.search_finish()
                    }
                ));
                imp.find_treat_medial_capital_as_word_start
                    .connect_clicked(clone!(
                        #[weak]
                        imp,
                        #[weak]
                        find_controller,
                        move |_find_treat_medial_capital_as_word_start| {
                            imp.total_matches_label.set_text("");
                            find_controller.search_finish()
                        }
                    ));
                imp.find_backwards.connect_clicked(clone!(
                    #[weak]
                    find_controller,
                    #[weak]
                    imp,
                    move |find_backwards| {
                        if find_backwards.is_active() {
                            imp.next_find_button.set_icon_name("go-up");
                            imp.previous_find_button.set_icon_name("go-down");
                        } else {
                            imp.next_find_button.set_icon_name("go-down");
                            imp.previous_find_button.set_icon_name("go-up");
                        }
                        imp.total_matches_label.set_text("");
                        find_controller.search_finish()
                    }
                ));
                imp.find_wrap_around.connect_clicked(clone!(
                    #[weak]
                    imp,
                    #[weak]
                    find_controller,
                    move |_find_wrap_around| {
                        imp.total_matches_label.set_text("");
                        find_controller.search_finish()
                    }
                ));
            }
        ));
    }

    pub fn setup_find_box(&self) {
        let imp = self.imp();

        self.setup_find_signals();

        imp.find_search_entry.set_can_focus(true);
        imp.find_search_entry.set_focusable(true);
        imp.find_search_entry.set_focus_on_click(true);
        imp.find_search_entry.set_editable(true);
        imp.find_search_entry.set_hexpand(true);
        imp.find_search_entry
            .set_placeholder_text(Some("Search in page … "));
        imp.find_search_entry
            .set_input_purpose(gtk::InputPurpose::Url);
        imp.find_search_entry.set_halign(gtk::Align::Fill);
        imp.find_search_entry.set_margin_start(4);
        imp.find_search_entry.set_margin_end(4);

        imp.find_middle_box.append(&imp.find_search_entry);
        imp.find_middle_box.append(&imp.total_matches_label);
        imp.find_middle_box.set_margin_start(2);
        imp.find_middle_box.set_margin_end(2);

        imp.previous_find_button.set_can_focus(true);
        imp.previous_find_button.set_receives_default(true);
        imp.previous_find_button.set_icon_name("go-up");
        imp.previous_find_button.add_css_class("linked");

        imp.next_find_button.set_can_focus(true);
        imp.next_find_button.set_receives_default(true);
        imp.next_find_button.set_icon_name("go-down");
        imp.next_find_button.add_css_class("linked");

        imp.find_buttons.append(&imp.previous_find_button);
        imp.find_buttons.append(&imp.next_find_button);
        imp.find_buttons.add_css_class("linked");
        imp.find_buttons.set_margin_start(2);
        imp.find_buttons.set_margin_end(2);

        imp.find_case_insensitive.set_can_focus(true);
        imp.find_case_insensitive.set_receives_default(true);
        imp.find_case_insensitive
            .set_icon_name("format-text-strikethrough");
        imp.find_case_insensitive.add_css_class("linked");
        imp.find_case_insensitive
            .set_tooltip_text(Some("Ignore case when searching"));

        imp.find_at_word_starts.set_can_focus(true);
        imp.find_at_word_starts.set_receives_default(true);
        imp.find_at_word_starts.set_icon_name("go-first");
        imp.find_at_word_starts.add_css_class("linked");
        imp.find_at_word_starts
            .set_tooltip_text(Some("Search text only at the start of words"));

        imp.find_treat_medial_capital_as_word_start
            .set_can_focus(true);
        imp.find_treat_medial_capital_as_word_start
            .set_receives_default(true);
        imp.find_treat_medial_capital_as_word_start
            .set_icon_name("format-text-underline");
        imp.find_treat_medial_capital_as_word_start
            .add_css_class("linked");
        imp.find_treat_medial_capital_as_word_start
            .set_tooltip_text(Some(
                "Treat capital letters in the middle of words as word start",
            ));
        imp.find_treat_medial_capital_as_word_start
            .set_sensitive(false);

        imp.find_backwards.set_can_focus(true);
        imp.find_backwards.set_receives_default(true);
        imp.find_backwards.set_icon_name("media-seek-backward");
        imp.find_backwards.add_css_class("linked");
        imp.find_backwards
            .set_tooltip_text(Some("Search backwards"));

        imp.find_wrap_around.set_can_focus(true);
        imp.find_wrap_around.set_receives_default(true);
        imp.find_wrap_around.set_icon_name("media-playlist-repeat");
        imp.find_wrap_around.add_css_class("linked");
        imp.find_wrap_around
            .set_tooltip_text(Some("Wrap around the document when searching"));

        imp.find_option_buttons.append(&imp.find_case_insensitive);
        imp.find_option_buttons.append(&imp.find_at_word_starts);
        imp.find_option_buttons
            .append(&imp.find_treat_medial_capital_as_word_start);
        imp.find_option_buttons.append(&imp.find_backwards);
        imp.find_option_buttons.append(&imp.find_wrap_around);
        imp.find_option_buttons.add_css_class("linked");
        imp.find_option_buttons.set_margin_start(2);
        imp.find_option_buttons.set_margin_end(2);

        imp.find_box.set_orientation(gtk::Orientation::Horizontal);
        imp.find_box.set_hexpand(true);
        imp.find_box.add_css_class("toolbar");
        imp.find_box.append(&imp.find_option_buttons);
        imp.find_box.append(&imp.find_middle_box);
        imp.find_box.append(&imp.find_buttons);
    }

    pub fn setup_find_popover(&self) {
        let imp = self.imp();

        self.setup_find_box();
        imp.find_popover.set_child(Some(&imp.find_box));
        imp.find_popover.set_parent(&imp.find_button);
        imp.find_popover.set_autohide(true);
    }

    pub fn setup_find_button_clicked(&self) {
        let imp = self.imp();

        imp.find_button.connect_clicked(clone!(
            #[weak(rename_to = find_popover)]
            imp.find_popover,
            move |_| {
                find_popover.popup();
            }
        ));
        let action_find = gio::ActionEntry::builder("find")
            .activate(clone!(
                #[weak(rename_to = find_popover)]
                imp.find_popover,
                move |window: &Self, _, _| {
                    if window.imp().tab_view.n_pages() == 0 {
                        return;
                    }
                    if !find_popover.is_visible() {
                        find_popover.popup();
                    } else {
                        find_popover.popdown();
                    }
                }
            ))
            .build();
        self.add_action_entries([action_find]);
    }
}
