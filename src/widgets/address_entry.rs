use crate::database::policy::PolicyDecision;
use crate::database::policy::PolicySetting;
use crate::database::policy::PolicySettingRecord;
use glib::clone;
use glib::closure;
use glib::subclass::object::ObjectImpl;
use glib::subclass::types::ObjectSubclass;
use glib::ParamSpec;
use glib::ParamSpecObject;
use glib::SignalHandlerId;
use glib::Value;
use gtk::glib;
use gtk::prelude::WidgetExt;
use gtk::subclass::prelude::*;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::sync::LazyLock;

pub mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct AddressEntry {
        pub(crate) clicked_handler_id: RefCell<Option<SignalHandlerId>>,
        pub(crate) toolbox: gtk::Popover,
        // Site policies
        pub(crate) policy_setting: PolicySetting,
        pub(crate) policy_box: gtk::Box,
        pub(crate) policy_group: libadwaita::PreferencesGroup,
        // Clipboard
        pub(crate) clipboard_policy_list: gtk::StringList,
        pub(crate) clipboard_policy_selection: gtk::SingleSelection,
        pub(crate) clipboard_policy_row: libadwaita::ComboRow,
        // Device info
        pub(crate) device_info_policy_list: gtk::StringList,
        pub(crate) device_info_policy_selection: gtk::SingleSelection,
        pub(crate) device_info_policy_row: libadwaita::ComboRow,
        // Geolocation
        pub(crate) geolocation_policy_list: gtk::StringList,
        pub(crate) geolocation_policy_selection: gtk::SingleSelection,
        pub(crate) geolocation_policy_row: libadwaita::ComboRow,
        // CDM
        pub(crate) cdm_policy_list: gtk::StringList,
        pub(crate) cdm_policy_selection: gtk::SingleSelection,
        pub(crate) cdm_policy_row: libadwaita::ComboRow,
        // Notification
        pub(crate) notification_policy_list: gtk::StringList,
        pub(crate) notification_policy_selection: gtk::SingleSelection,
        pub(crate) notification_policy_row: libadwaita::ComboRow,
        // Pointer lock
        pub(crate) pointer_lock_policy_list: gtk::StringList,
        pub(crate) pointer_lock_policy_selection: gtk::SingleSelection,
        pub(crate) pointer_lock_policy_row: libadwaita::ComboRow,
        // User media
        pub(crate) user_media_policy_list: gtk::StringList,
        pub(crate) user_media_policy_selection: gtk::SingleSelection,
        pub(crate) user_media_policy_row: libadwaita::ComboRow,
        // Data access
        pub(crate) data_access_policy_list: gtk::StringList,
        pub(crate) data_access_policy_selection: gtk::SingleSelection,
        pub(crate) data_access_policy_row: libadwaita::ComboRow,
    }

    impl AddressEntry {}

    #[glib::object_subclass]
    impl ObjectSubclass for AddressEntry {
        const NAME: &'static str = "OkuAddressEntry";
        type Type = super::AddressEntry;
        type ParentType = gtk::Entry;
    }

    impl ObjectImpl for AddressEntry {
        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: LazyLock<Vec<ParamSpec>> = LazyLock::new(|| {
                vec![ParamSpecObject::builder::<PolicySetting>("policy-setting")
                    .readwrite()
                    .build()]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "policy-setting" => {
                    let policy_setting = value.get::<PolicySetting>().unwrap();
                    self.policy_setting.set_properties_from_value(&[
                        ("uri", policy_setting.property_value("uri")),
                        (
                            "clipboard-policy",
                            policy_setting.property_value("clipboard-policy"),
                        ),
                        (
                            "device-info-policy",
                            policy_setting.property_value("device-info-policy"),
                        ),
                        (
                            "geolocation-policy",
                            policy_setting.property_value("geolocation-policy"),
                        ),
                        ("cdm-policy", policy_setting.property_value("cdm-policy")),
                        (
                            "notification-policy",
                            policy_setting.property_value("notification-policy"),
                        ),
                        (
                            "pointer-lock-policy",
                            policy_setting.property_value("pointer-lock-policy"),
                        ),
                        (
                            "user-media-policy",
                            policy_setting.property_value("user-media-policy"),
                        ),
                        (
                            "data-access-policy",
                            policy_setting.property_value("data-access-policy"),
                        ),
                    ]);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &ParamSpec) -> Value {
            match pspec.name() {
                "policy-setting" => self.policy_setting.to_owned().to_value(),
                _ => unimplemented!(),
            }
        }
    }
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

unsafe impl Send for AddressEntry {}
unsafe impl Sync for AddressEntry {}

impl AddressEntry {
    pub fn new() -> Self {
        let this: Self = glib::Object::builder::<Self>().build();

        this.set_can_focus(true);
        this.set_focusable(true);
        this.set_focus_on_click(true);
        this.set_editable(true);
        this.set_hexpand(true);
        this.set_placeholder_text(Some("Enter an address … "));
        this.set_enable_undo(true);
        this.set_input_purpose(gtk::InputPurpose::Url);
        this.set_halign(gtk::Align::Fill);

        this.set_primary_icon_name(Some("shapes-symbolic"));
        this.set_secondary_icon_name(Some("entry-clear-symbolic"));
        this.set_secondary_icon_sensitive(true);

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
        this.setup_click_handler();
        this.set_visible(true);

        this
    }

    pub fn update_policy_setting_from_uri(&self, uri: String) {
        self.set_primary_icon_sensitive(false);
        let this = self.clone();
        tokio::task::spawn_blocking(move || {
            this.set_property(
                "policy-setting",
                PolicySetting::from(PolicySettingRecord::from_uri(uri)),
            );
            this.set_primary_icon_sensitive(true);
        });
    }

    pub fn disconnect_click_handler(&self) {
        if let Some(id) = self.imp().clicked_handler_id.take() {
            self.disconnect(id)
        }
    }

    pub fn set_click_handler(&self, handler_id: SignalHandlerId) {
        self.disconnect_click_handler();
        self.imp().clicked_handler_id.replace(Some(handler_id));
    }

    pub fn setup_click_handler(&self) {
        self.setup_toolbox();
        let click_handler = self.connect_icon_release(clone!(move |this, icon_position| {
            match icon_position {
                gtk::EntryIconPosition::Primary => {
                    this.imp().toolbox.popup();
                }
                gtk::EntryIconPosition::Secondary => {
                    this.buffer().delete_text(0, None);
                }
                _ => unreachable!(),
            }
        }));
        self.set_click_handler(click_handler);
    }

    pub fn primary_icon(&self) -> Option<gtk::Image> {
        let primary_icon_name = self.primary_icon_name();
        let mut icon_widget = self.first_child()?;
        let mut icon_name = icon_widget
            .clone()
            .downcast::<gtk::Image>()
            .ok()
            .and_then(|x| x.icon_name());
        if primary_icon_name != icon_name {
            while let Some(sibling) = icon_widget.next_sibling() {
                icon_widget = sibling;
                icon_name = icon_widget
                    .clone()
                    .downcast::<gtk::Image>()
                    .ok()
                    .and_then(|x| x.icon_name());
                if primary_icon_name == icon_name {
                    break;
                }
            }
        }
        icon_widget.downcast().ok()
    }

    pub fn setup_toolbox(&self) {
        let imp = self.imp();

        self.setup_policy_box();
        imp.toolbox.set_child(Some(&imp.policy_box));
        if let Some(primary_icon) = self.primary_icon().as_ref() {
            imp.toolbox.set_parent(primary_icon);
        } else {
            imp.toolbox.set_parent(self);
        }
        imp.toolbox.set_autohide(true);
    }

    pub fn setup_policy_box(&self) {
        let imp = self.imp();

        imp.clipboard_policy_list.append("Ask");
        imp.clipboard_policy_list.append("Allow");
        imp.clipboard_policy_list.append("Deny");
        imp.device_info_policy_list.append("Ask");
        imp.device_info_policy_list.append("Allow");
        imp.device_info_policy_list.append("Deny");
        imp.geolocation_policy_list.append("Ask");
        imp.geolocation_policy_list.append("Allow");
        imp.geolocation_policy_list.append("Deny");
        imp.cdm_policy_list.append("Ask");
        imp.cdm_policy_list.append("Allow");
        imp.cdm_policy_list.append("Deny");
        imp.notification_policy_list.append("Ask");
        imp.notification_policy_list.append("Allow");
        imp.notification_policy_list.append("Deny");
        imp.pointer_lock_policy_list.append("Ask");
        imp.pointer_lock_policy_list.append("Allow");
        imp.pointer_lock_policy_list.append("Deny");
        imp.user_media_policy_list.append("Ask");
        imp.user_media_policy_list.append("Allow");
        imp.user_media_policy_list.append("Deny");
        imp.data_access_policy_list.append("Ask");
        imp.data_access_policy_list.append("Allow");
        imp.data_access_policy_list.append("Deny");

        imp.clipboard_policy_selection
            .set_model(Some(&imp.clipboard_policy_list));
        imp.device_info_policy_selection
            .set_model(Some(&imp.device_info_policy_list));
        imp.geolocation_policy_selection
            .set_model(Some(&imp.geolocation_policy_list));
        imp.cdm_policy_selection
            .set_model(Some(&imp.cdm_policy_list));
        imp.notification_policy_selection
            .set_model(Some(&imp.notification_policy_list));
        imp.pointer_lock_policy_selection
            .set_model(Some(&imp.pointer_lock_policy_list));
        imp.user_media_policy_selection
            .set_model(Some(&imp.user_media_policy_list));
        imp.data_access_policy_selection
            .set_model(Some(&imp.data_access_policy_list));

        imp.clipboard_policy_row
            .set_model(imp.clipboard_policy_selection.model().as_ref());
        imp.device_info_policy_row
            .set_model(imp.device_info_policy_selection.model().as_ref());
        imp.geolocation_policy_row
            .set_model(imp.geolocation_policy_selection.model().as_ref());
        imp.cdm_policy_row
            .set_model(imp.cdm_policy_selection.model().as_ref());
        imp.notification_policy_row
            .set_model(imp.notification_policy_selection.model().as_ref());
        imp.pointer_lock_policy_row
            .set_model(imp.pointer_lock_policy_selection.model().as_ref());
        imp.user_media_policy_row
            .set_model(imp.user_media_policy_selection.model().as_ref());
        imp.data_access_policy_row
            .set_model(imp.data_access_policy_selection.model().as_ref());

        imp.clipboard_policy_row.set_title("Access clipboard");
        imp.device_info_policy_row
            .set_title("Access audio & video device information");
        imp.geolocation_policy_row.set_title("Access location");
        imp.cdm_policy_row
            .set_title("Access content decryption modules");
        imp.notification_policy_row.set_title("Show notifications");
        imp.pointer_lock_policy_row.set_title("Lock pointer");
        imp.user_media_policy_row
            .set_title("Access audio & video devices");
        imp.data_access_policy_row
            .set_title("Cookie access by third-party");

        imp.policy_setting
            .bind_property("clipboard-policy", &imp.clipboard_policy_row, "selected")
            .transform_to(move |_, x: PolicyDecision| Some(x.selected()))
            .transform_from(|_, x: u32| Some(PolicyDecision::from(x)))
            .bidirectional()
            .build();
        imp.policy_setting
            .bind_property(
                "device-info-policy",
                &imp.device_info_policy_row,
                "selected",
            )
            .transform_to(move |_, x: PolicyDecision| Some(x.selected()))
            .transform_from(|_, x: u32| Some(PolicyDecision::from(x)))
            .bidirectional()
            .build();
        imp.policy_setting
            .bind_property(
                "geolocation-policy",
                &imp.geolocation_policy_row,
                "selected",
            )
            .transform_to(move |_, x: PolicyDecision| Some(x.selected()))
            .transform_from(|_, x: u32| Some(PolicyDecision::from(x)))
            .bidirectional()
            .build();
        imp.policy_setting
            .bind_property("cdm-policy", &imp.cdm_policy_row, "selected")
            .transform_to(move |_, x: PolicyDecision| Some(x.selected()))
            .transform_from(|_, x: u32| Some(PolicyDecision::from(x)))
            .bidirectional()
            .build();
        imp.policy_setting
            .bind_property(
                "notification-policy",
                &imp.notification_policy_row,
                "selected",
            )
            .transform_to(move |_, x: PolicyDecision| Some(x.selected()))
            .transform_from(|_, x: u32| Some(PolicyDecision::from(x)))
            .bidirectional()
            .build();
        imp.policy_setting
            .bind_property(
                "pointer-lock-policy",
                &imp.pointer_lock_policy_row,
                "selected",
            )
            .transform_to(move |_, x: PolicyDecision| Some(x.selected()))
            .transform_from(|_, x: u32| Some(PolicyDecision::from(x)))
            .bidirectional()
            .build();
        imp.policy_setting
            .bind_property("user-media-policy", &imp.user_media_policy_row, "selected")
            .transform_to(move |_, x: PolicyDecision| Some(x.selected()))
            .transform_from(|_, x: u32| Some(PolicyDecision::from(x)))
            .bidirectional()
            .build();
        imp.policy_setting
            .bind_property(
                "data-access-policy",
                &imp.data_access_policy_row,
                "selected",
            )
            .transform_to(move |_, x: PolicyDecision| Some(x.selected()))
            .transform_from(|_, x: u32| Some(PolicyDecision::from(x)))
            .bidirectional()
            .build();

        imp.policy_group.set_title("Permissions");
        imp.policy_group.set_separate_rows(false);
        imp.policy_group.add(&imp.clipboard_policy_row);
        imp.policy_group.add(&imp.device_info_policy_row);
        imp.policy_group.add(&imp.geolocation_policy_row);
        imp.policy_group.add(&imp.cdm_policy_row);
        imp.policy_group.add(&imp.notification_policy_row);
        imp.policy_group.add(&imp.pointer_lock_policy_row);
        imp.policy_group.add(&imp.user_media_policy_row);
        imp.policy_group.add(&imp.data_access_policy_row);
        imp.policy_box.append(&imp.policy_group);
    }
}
