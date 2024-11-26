use glib::closure;
use glib::subclass::object::ObjectImpl;
use glib::subclass::types::ObjectSubclass;
use gtk::glib;
use gtk::prelude::WidgetExt;
use gtk::subclass::prelude::*;
use libadwaita::prelude::*;

pub mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct AddressEntry {}

    impl AddressEntry {}

    #[glib::object_subclass]
    impl ObjectSubclass for AddressEntry {
        const NAME: &'static str = "OkuAddressEntry";
        type Type = super::AddressEntry;
        type ParentType = gtk::Entry;
    }

    impl ObjectImpl for AddressEntry {}
    impl WidgetImpl for AddressEntry {}
    impl EntryImpl for AddressEntry {}
}

glib::wrapper! {
    pub struct AddressEntry(ObjectSubclass<imp::AddressEntry>)
    @extends gtk::Entry, gtk::Widget, gtk::Editable;
}

impl Default for AddressEntry {
    fn default() -> Self {
        Self::new()
    }
}

impl AddressEntry {
    pub fn new() -> Self {
        let this: Self = glib::Object::builder::<Self>().build();

        this.set_can_focus(true);
        this.set_focusable(true);
        this.set_focus_on_click(true);
        this.set_editable(true);
        this.set_hexpand(true);
        this.set_placeholder_text(Some("Enter an address … "));
        this.set_input_purpose(gtk::InputPurpose::Url);
        this.set_halign(gtk::Align::Fill);

        this.property_expression("text")
            .chain_closure::<pango::AttrList>(closure!(|_: Option<glib::Object>, x: String| {
                let attributes = pango::AttrList::new();
                if let Some(authority_start) = x.find("://") {
                    let foreground_alpha_dim =
                        pango::AttrInt::new_foreground_alpha(u16::pow(2, 15));
                    let mut foreground_alpha_dark = pango::AttrInt::new_foreground_alpha(u16::MAX);
                    foreground_alpha_dark.set_start_index((authority_start + 3) as u32);
                    if let Some(authority_end) = x[authority_start + 3..].find("/") {
                        foreground_alpha_dark
                            .set_end_index((authority_start + 3 + authority_end) as u32);
                    }
                    attributes.insert(foreground_alpha_dim);
                    attributes.insert(foreground_alpha_dark);
                }
                attributes
            }))
            .bind(&this, "attributes", gtk::Widget::NONE);
        this.set_visible(true);

        this
    }
}
