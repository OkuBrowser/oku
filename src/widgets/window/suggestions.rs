use super::*;
use crate::suggestion_item::SuggestionItem;
use crate::widgets;
use glib::clone;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};
use libadwaita::prelude::*;
use std::cell::Ref;
use std::rc::Rc;
use webkit2gtk::functions::uri_for_display;

impl Window {
    pub fn suggestions_store(&self) -> Ref<gio::ListStore> {
        let suggestions_store = self.imp().suggestions_store.borrow();

        Ref::map(suggestions_store, |suggestions_store| {
            let suggestions_store = suggestions_store.as_deref().unwrap();
            suggestions_store
        })
    }

    pub fn setup_suggestions_popover(&self) {
        let imp = self.imp();

        let suggestions_store = gio::ListStore::new::<crate::suggestion_item::SuggestionItem>();
        imp.suggestions_store
            .replace(Some(Rc::new(suggestions_store)));

        imp.suggestions_model
            .set_model(Some(&self.suggestions_store().clone()));
        imp.suggestions_model.set_autoselect(false);
        imp.suggestions_model.connect_selected_item_notify(clone!(
            #[weak]
            imp,
            move |suggestions_model| {
                if let Some(item) = suggestions_model.selected_item() {
                    let suggestion_item = item.downcast_ref::<SuggestionItem>().unwrap();
                    let encoded_uri = suggestion_item.uri();
                    let decoded_uri = html_escape::decode_html_entities(&encoded_uri);
                    imp.nav_entry
                        .set_text(&uri_for_display(&decoded_uri).unwrap_or(decoded_uri.into()));
                }
            }
        ));

        imp.suggestions_factory
            .connect_setup(clone!(move |_, item| {
                let row = widgets::suggestion_row::SuggestionRow::new();
                let list_item = item.downcast_ref::<gtk::ListItem>().unwrap();
                list_item.set_child(Some(&row));
                list_item
                    .property_expression("item")
                    .chain_property::<crate::suggestion_item::SuggestionItem>("title")
                    .bind(&row, "title-property", gtk::Widget::NONE);
                list_item
                    .property_expression("item")
                    .chain_property::<crate::suggestion_item::SuggestionItem>("uri")
                    .bind(&row, "uri", gtk::Widget::NONE);
                list_item
                    .property_expression("item")
                    .chain_property::<crate::suggestion_item::SuggestionItem>("favicon")
                    .bind(&row, "favicon", gtk::Widget::NONE);
            }));

        imp.suggestions_view.set_model(Some(&imp.suggestions_model));
        imp.suggestions_view
            .set_factory(Some(&imp.suggestions_factory));
        imp.suggestions_view.set_enable_rubberband(false);
        imp.suggestions_view
            .set_hscroll_policy(gtk::ScrollablePolicy::Natural);
        imp.suggestions_view
            .set_vscroll_policy(gtk::ScrollablePolicy::Natural);

        imp.suggestions_scrolled_window
            .set_child(Some(&imp.suggestions_view));
        imp.suggestions_scrolled_window
            .set_orientation(gtk::Orientation::Horizontal);
        imp.suggestions_scrolled_window.set_maximum_size(1000);
        imp.suggestions_scrolled_window
            .set_tightening_threshold(1000);

        imp.suggestions_popover
            .set_child(Some(&imp.suggestions_scrolled_window));
        imp.suggestions_popover.set_parent(&imp.nav_entry);
        imp.suggestions_popover.add_css_class("menu");
        imp.suggestions_popover.add_css_class("suggestions");
        imp.suggestions_popover.set_has_arrow(false);
        imp.suggestions_popover.set_autohide(false);
        imp.suggestions_popover.set_can_focus(false);
    }
}
