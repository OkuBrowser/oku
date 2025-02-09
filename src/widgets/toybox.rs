use crate::database::policy::PolicyDecision;
use crate::database::policy::PolicySetting;
use crate::database::policy::PolicySettingRecord;
use crate::scheme_handlers::oku_path::OkuPath;
use crate::NODE;
use glib::clone;
use glib::closure;
use glib::property::PropertySet;
use glib::subclass::object::ObjectImpl;
use glib::subclass::types::ObjectSubclass;
use glib::subclass::types::ObjectSubclassIsExt;
use glib::value::ToValue;
use glib::ParamSpec;
use glib::ParamSpecObject;
use glib::ParamSpecString;
use glib::Value;
use gtk::glib;
use gtk::prelude::BoxExt;
use gtk::prelude::ButtonExt;
use gtk::prelude::WidgetExt;
use gtk::subclass::prelude::*;
use libadwaita::prelude::PreferencesRowExt;
use libadwaita::prelude::*;
use libadwaita::subclass::dialog::AdwDialogImpl;
use log::error;
use std::cell::RefCell;
use std::sync::LazyLock;
use webkit2gtk::functions::uri_for_display;
use webkit2gtk::prelude::WebViewExt;

pub mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct Toybox {
        // General
        pub(crate) uri: RefCell<String>,
        pub(crate) content: gtk::Box,
        pub(crate) header: libadwaita::HeaderBar,
        pub(crate) view_switcher: libadwaita::ViewSwitcher,
        pub(crate) view_stack: libadwaita::ViewStack,
        // Toolbox page
        pub(crate) toolbox_scrolled_window: gtk::ScrolledWindow,
        pub(crate) toolbox_content: gtk::Box,
        // OkuNet
        pub(crate) okunet_box: gtk::Box,
        pub(crate) okunet_refresh_button: gtk::Button,
        pub(crate) okunet_refresh_button_content: libadwaita::ButtonContent,
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

    impl Toybox {}

    #[glib::object_subclass]
    impl ObjectSubclass for Toybox {
        const NAME: &'static str = "OkuToybox";
        type Type = super::Toybox;
        type ParentType = libadwaita::Dialog;
    }

    impl ObjectImpl for Toybox {
        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: LazyLock<Vec<ParamSpec>> = LazyLock::new(|| {
                vec![
                    ParamSpecObject::builder::<PolicySetting>("policy-setting")
                        .readwrite()
                        .build(),
                    ParamSpecString::builder("uri").readwrite().build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "uri" => {
                    let uri = value.get::<String>().unwrap_or_default();
                    self.uri.set(
                        html_escape::encode_text(&uri_for_display(&uri).unwrap_or(uri.into()))
                            .to_string(),
                    );
                }
                "policy-setting" => {
                    if let Ok(policy_setting) = value.get::<PolicySetting>() {
                        self.policy_setting.update(policy_setting);
                    }
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &ParamSpec) -> Value {
            match pspec.name() {
                "uri" => self.uri.borrow().to_value(),
                "policy-setting" => self.policy_setting.to_owned().to_value(),
                _ => unimplemented!(),
            }
        }
    }
    impl WidgetImpl for Toybox {}
    impl AdwDialogImpl for Toybox {}
}

glib::wrapper! {
    pub struct Toybox(ObjectSubclass<imp::Toybox>)
    @extends libadwaita::Dialog, gtk::Widget;
}

unsafe impl Send for Toybox {}
unsafe impl Sync for Toybox {}

impl Toybox {
    pub fn new(window: Option<&super::window::Window>) -> Self {
        let this: Self = glib::Object::builder::<Self>().build();
        let imp = this.imp();

        this.property_expression("uri")
            .chain_closure::<PolicySetting>(closure!(|_: Option<glib::Object>, x: String| {
                PolicySetting::from(PolicySettingRecord::from_uri(x))
            }))
            .bind(&this, "policy-setting", gtk::Widget::NONE);
        this.property_expression("uri")
            .chain_closure::<bool>(closure!(|_: Option<glib::Object>, x: String| {
                let uri = &x.replacen("oku:", "", 1);
                matches!(
                    OkuPath::parse(uri),
                    Ok(OkuPath::Home) | Ok(OkuPath::User(_, _))
                )
            }))
            .bind(&imp.okunet_refresh_button, "visible", gtk::Widget::NONE);
        if let Some(window) = window {
            window
                .imp()
                .okunet_fetch_overlay_box
                .property_expression("opacity")
                .chain_closure::<bool>(closure!(|_: Option<glib::Object>, x: f64| { x == 0.0 }))
                .bind(&imp.okunet_refresh_button, "sensitive", gtk::Widget::NONE);
        };

        this.setup_content();
        this.setup_toolbox();

        let uri = window
            .and_then(|x| x.get_view().uri().map(|y| y.to_string()))
            .unwrap_or("about:blank".into());
        this.set_property("uri", uri);

        this.set_visible(true);
        this.present(window);

        this
    }

    pub fn setup_content(&self) {
        let imp = self.imp();

        imp.view_stack.add_titled_with_icon(
            &imp.toolbox_scrolled_window,
            Some("toolbox"),
            "Toolbox",
            "wrench-wide-symbolic",
        );

        imp.view_switcher.set_stack(Some(&imp.view_stack));
        imp.view_switcher
            .set_policy(libadwaita::ViewSwitcherPolicy::Wide);

        imp.header.set_title_widget(Some(&imp.view_switcher));
        imp.header
            .set_centering_policy(libadwaita::CenteringPolicy::Strict);

        imp.content.append(&imp.header);
        imp.content.append(&imp.view_stack);
        imp.content.set_orientation(gtk::Orientation::Vertical);

        self.set_child(Some(&imp.content));
        self.set_follows_content_size(true);
        self.set_presentation_mode(libadwaita::DialogPresentationMode::Auto);
    }

    pub fn setup_toolbox(&self) {
        let imp = self.imp();

        self.setup_okunet_box();
        self.setup_policy_box();
        imp.toolbox_content.append(&imp.okunet_box);
        imp.toolbox_content.append(&imp.policy_box);
        imp.toolbox_content
            .set_orientation(gtk::Orientation::Vertical);
        imp.toolbox_content.set_margin_start(8);
        imp.toolbox_content.set_margin_top(8);
        imp.toolbox_content.set_margin_bottom(8);
        imp.toolbox_content.set_margin_end(8);
        imp.toolbox_scrolled_window
            .set_child(Some(&imp.toolbox_content));
        imp.toolbox_scrolled_window
            .set_propagate_natural_width(true);
        imp.toolbox_scrolled_window
            .set_propagate_natural_height(true);
        imp.toolbox_scrolled_window.set_max_content_width(300);
        imp.toolbox_scrolled_window.set_max_content_height(400);
    }

    pub fn setup_okunet_box(&self) {
        let imp = self.imp();
        imp.okunet_refresh_button_content
            .set_label("Fetch from OkuNet");
        imp.okunet_refresh_button_content
            .set_icon_name("update-symbolic");
        imp.okunet_refresh_button
            .set_child(Some(&imp.okunet_refresh_button_content));
        imp.okunet_refresh_button
            .add_css_class("destructive-action");
        imp.okunet_refresh_button.add_css_class("card");
        imp.okunet_refresh_button.set_hexpand(true);
        imp.okunet_refresh_button.set_vexpand(true);
        imp.okunet_refresh_button.connect_clicked(clone!(
            #[weak(rename_to = this)]
            self,
            move |_| {
                tokio::spawn(async move { this.refresh_okunet().await });
            }
        ));
        imp.okunet_box.append(&imp.okunet_refresh_button);
    }

    pub async fn refresh_okunet(&self) {
        self.imp().okunet_refresh_button.set_sensitive(false);
        if let Some(node) = NODE.get() {
            let uri = &*self.imp().uri.borrow().replacen("oku:", "", 1);
            let parsed_uri = OkuPath::parse(uri);
            match parsed_uri {
                Ok(OkuPath::Home) => {
                    if let Err(e) = node.fetch_users().await {
                        error!("{}", e);
                    }
                }
                Ok(OkuPath::User(author_id, None)) => {
                    if let Err(e) = node.fetch_user(&author_id).await {
                        error!("{}", e);
                    }
                }
                Ok(OkuPath::User(author_id, Some(path))) => {
                    let post_path = format!(
                        "{}.toml",
                        path.to_string_lossy()
                            .strip_suffix(".html")
                            .unwrap_or(&path.to_string_lossy())
                    );
                    if let Err(e) = node.fetch_post(&author_id, &post_path.into()).await {
                        error!("{}", e);
                    }
                }
                _ => (),
            }
        }
        self.imp().okunet_refresh_button.set_sensitive(true);
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
