use super::BrowserDatabase;
use glib::object::ObjectExt;
use glib::property::PropertySet;
use glib::subclass::object::ObjectImpl;
use glib::subclass::types::ObjectSubclass;
use glib::subclass::types::ObjectSubclassIsExt;
use glib::value::ToValue;
use glib::ParamSpec;
use glib::ParamSpecBuilderExt;
use glib::ParamSpecEnum;
use glib::ParamSpecString;
use glib::Value;
use miette::IntoDiagnostic;
use native_db::*;
use native_model::{native_model, Model};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::sync::LazyLock;
use webkit2gtk::functions::uri_for_display;
use webkit2gtk::prelude::PermissionRequestExt;

#[derive(Serialize, Deserialize, Default, PartialEq, Debug, Clone)]
#[native_model(id = 3, version = 1)]
#[native_db]
pub struct PolicySettingRecord {
    #[primary_key]
    pub uri: String,
    pub clipboard_policy: PolicyDecision,
    pub device_info_policy: PolicyDecision,
    pub geolocation_policy: PolicyDecision,
    pub cdm_policy: PolicyDecision,
    pub notification_policy: PolicyDecision,
    pub pointer_lock_policy: PolicyDecision,
    pub user_media_policy: PolicyDecision,
    pub data_access_policy: PolicyDecision,
}

impl PolicySettingRecord {
    pub fn from_uri(uri: String) -> Self {
        crate::DATABASE
            .get_policy_setting(uri.clone())
            .ok()
            .flatten()
            .unwrap_or(Self {
                uri,
                ..Self::default()
            })
    }

    pub fn save(&self) {
        let _ = crate::DATABASE.upsert_policy_setting(self.to_owned());
    }

    pub fn handle(
        &self,
        window: &crate::widgets::window::Window,
        permission_request: &webkit2gtk::PermissionRequest,
    ) {
        if permission_request.is::<webkit2gtk::ClipboardPermissionRequest>() {
            match self.clipboard_policy {
                PolicyDecision::Ask => window.ask_permission(permission_request),
                PolicyDecision::Allow => permission_request.allow(),
                PolicyDecision::Deny => permission_request.deny(),
            }
        } else if permission_request.is::<webkit2gtk::DeviceInfoPermissionRequest>() {
            match self.device_info_policy {
                PolicyDecision::Ask => window.ask_permission(permission_request),
                PolicyDecision::Allow => permission_request.allow(),
                PolicyDecision::Deny => permission_request.deny(),
            }
        } else if permission_request.is::<webkit2gtk::GeolocationPermissionRequest>() {
            match self.geolocation_policy {
                PolicyDecision::Ask => window.ask_permission(permission_request),
                PolicyDecision::Allow => permission_request.allow(),
                PolicyDecision::Deny => permission_request.deny(),
            }
        } else if permission_request.is::<webkit2gtk::MediaKeySystemPermissionRequest>() {
            match self.cdm_policy {
                PolicyDecision::Ask => window.ask_permission(permission_request),
                PolicyDecision::Allow => permission_request.allow(),
                PolicyDecision::Deny => permission_request.deny(),
            }
        } else if permission_request.is::<webkit2gtk::NotificationPermissionRequest>() {
            match self.notification_policy {
                PolicyDecision::Ask => window.ask_permission(permission_request),
                PolicyDecision::Allow => permission_request.allow(),
                PolicyDecision::Deny => permission_request.deny(),
            }
        } else if permission_request.is::<webkit2gtk::PointerLockPermissionRequest>() {
            match self.pointer_lock_policy {
                PolicyDecision::Ask => window.ask_permission(permission_request),
                PolicyDecision::Allow => permission_request.allow(),
                PolicyDecision::Deny => permission_request.deny(),
            }
        } else if permission_request.is::<webkit2gtk::UserMediaPermissionRequest>() {
            match self.user_media_policy {
                PolicyDecision::Ask => window.ask_permission(permission_request),
                PolicyDecision::Allow => permission_request.allow(),
                PolicyDecision::Deny => permission_request.deny(),
            }
        } else if permission_request.is::<webkit2gtk::WebsiteDataAccessPermissionRequest>() {
            match self.data_access_policy {
                PolicyDecision::Ask => window.ask_permission(permission_request),
                PolicyDecision::Allow => permission_request.allow(),
                PolicyDecision::Deny => permission_request.deny(),
            }
        };
    }
}

impl From<&imp::PolicySetting> for PolicySettingRecord {
    fn from(value: &imp::PolicySetting) -> Self {
        Self {
            uri: value.uri.borrow().to_owned(),
            clipboard_policy: value.clipboard_policy.borrow().to_owned(),
            device_info_policy: value.device_info_policy.borrow().to_owned(),
            geolocation_policy: value.geolocation_policy.borrow().to_owned(),
            cdm_policy: value.cdm_policy.borrow().to_owned(),
            notification_policy: value.notification_policy.borrow().to_owned(),
            pointer_lock_policy: value.pointer_lock_policy.borrow().to_owned(),
            user_media_policy: value.user_media_policy.borrow().to_owned(),
            data_access_policy: value.data_access_policy.borrow().to_owned(),
        }
    }
}

impl From<&PolicySetting> for PolicySettingRecord {
    fn from(value: &PolicySetting) -> Self {
        let imp = value.imp();
        Self::from(imp)
    }
}

pub mod imp {
    use super::*;
    #[derive(Serialize, Deserialize, Default, PartialEq, Debug)]
    pub struct PolicySetting {
        pub uri: RefCell<String>,
        pub clipboard_policy: RefCell<PolicyDecision>,
        pub device_info_policy: RefCell<PolicyDecision>,
        pub geolocation_policy: RefCell<PolicyDecision>,
        pub cdm_policy: RefCell<PolicyDecision>,
        pub notification_policy: RefCell<PolicyDecision>,
        pub pointer_lock_policy: RefCell<PolicyDecision>,
        pub user_media_policy: RefCell<PolicyDecision>,
        pub data_access_policy: RefCell<PolicyDecision>,
    }

    impl From<PolicySettingRecord> for PolicySetting {
        fn from(value: PolicySettingRecord) -> Self {
            Self {
                uri: RefCell::new(value.uri),
                clipboard_policy: RefCell::new(value.clipboard_policy),
                device_info_policy: RefCell::new(value.device_info_policy),
                geolocation_policy: RefCell::new(value.geolocation_policy),
                cdm_policy: RefCell::new(value.cdm_policy),
                notification_policy: RefCell::new(value.notification_policy),
                pointer_lock_policy: RefCell::new(value.pointer_lock_policy),
                user_media_policy: RefCell::new(value.user_media_policy),
                data_access_policy: RefCell::new(value.data_access_policy),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PolicySetting {
        const NAME: &'static str = "OkuPolicySetting";
        type Type = super::PolicySetting;
    }
    impl ObjectImpl for PolicySetting {
        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: LazyLock<Vec<ParamSpec>> = LazyLock::new(|| {
                vec![
                    ParamSpecString::builder("uri").readwrite().build(),
                    ParamSpecEnum::builder::<PolicyDecision>("clipboard-policy")
                        .readwrite()
                        .build(),
                    ParamSpecEnum::builder::<PolicyDecision>("device-info-policy")
                        .readwrite()
                        .build(),
                    ParamSpecEnum::builder::<PolicyDecision>("geolocation-policy")
                        .readwrite()
                        .build(),
                    ParamSpecEnum::builder::<PolicyDecision>("cdm-policy")
                        .readwrite()
                        .build(),
                    ParamSpecEnum::builder::<PolicyDecision>("notification-policy")
                        .readwrite()
                        .build(),
                    ParamSpecEnum::builder::<PolicyDecision>("pointer-lock-policy")
                        .readwrite()
                        .build(),
                    ParamSpecEnum::builder::<PolicyDecision>("user-media-policy")
                        .readwrite()
                        .build(),
                    ParamSpecEnum::builder::<PolicyDecision>("data-access-policy")
                        .readwrite()
                        .build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "uri" => {
                    let uri = value.get::<String>().unwrap();
                    self.uri.set(
                        html_escape::encode_text(&uri_for_display(&uri).unwrap_or(uri.into()))
                            .to_string(),
                    );
                }
                "clipboard-policy" => {
                    let clipboard_policy = value.get::<PolicyDecision>().unwrap();
                    self.clipboard_policy.replace(clipboard_policy);
                }
                "device-info-policy" => {
                    let device_info_policy = value.get::<PolicyDecision>().unwrap();
                    self.device_info_policy.replace(device_info_policy);
                }
                "geolocation-policy" => {
                    let geolocation_policy = value.get::<PolicyDecision>().unwrap();
                    self.geolocation_policy.replace(geolocation_policy);
                }
                "cdm-policy" => {
                    let cdm_policy = value.get::<PolicyDecision>().unwrap();
                    self.cdm_policy.replace(cdm_policy);
                }
                "notification-policy" => {
                    let notification_policy = value.get::<PolicyDecision>().unwrap();
                    self.notification_policy.replace(notification_policy);
                }
                "pointer-lock-policy" => {
                    let pointer_lock_policy = value.get::<PolicyDecision>().unwrap();
                    self.pointer_lock_policy.replace(pointer_lock_policy);
                }
                "user-media-policy" => {
                    let user_media_policy = value.get::<PolicyDecision>().unwrap();
                    self.user_media_policy.replace(user_media_policy);
                }
                "data-access-policy" => {
                    let data_access_policy = value.get::<PolicyDecision>().unwrap();
                    self.data_access_policy.replace(data_access_policy);
                }
                _ => unimplemented!(),
            }
            PolicySettingRecord::from(self).save();
        }

        fn property(&self, _id: usize, pspec: &ParamSpec) -> Value {
            match pspec.name() {
                "uri" => self.uri.borrow().to_owned().to_value(),
                "clipboard-policy" => self.clipboard_policy.borrow().to_owned().to_value(),
                "device-info-policy" => self.device_info_policy.borrow().to_owned().to_value(),
                "geolocation-policy" => self.geolocation_policy.borrow().to_owned().to_value(),
                "cdm-policy" => self.cdm_policy.borrow().to_owned().to_value(),
                "notification-policy" => self.notification_policy.borrow().to_owned().to_value(),
                "pointer-lock-policy" => self.pointer_lock_policy.borrow().to_owned().to_value(),
                "user-media-policy" => self.user_media_policy.borrow().to_owned().to_value(),
                "data-access-policy" => self.data_access_policy.borrow().to_owned().to_value(),
                _ => unimplemented!(),
            }
        }
    }
}

glib::wrapper! {
    pub struct PolicySetting(ObjectSubclass<imp::PolicySetting>);
}

impl From<PolicySettingRecord> for PolicySetting {
    fn from(value: PolicySettingRecord) -> Self {
        let this = Self::default();
        this.update_from_record(value);
        this
    }
}

impl Default for PolicySetting {
    fn default() -> Self {
        let default_record = PolicySettingRecord::default();
        let object = glib::Object::builder::<Self>()
            .property("uri", &default_record.uri)
            .property("clipboard-policy", default_record.clipboard_policy)
            .property("device-info-policy", default_record.device_info_policy)
            .property("geolocation-policy", default_record.geolocation_policy)
            .property("cdm-policy", default_record.cdm_policy)
            .property("notification-policy", default_record.notification_policy)
            .property("pointer-lock-policy", default_record.pointer_lock_policy)
            .property("user-media-policy", default_record.user_media_policy)
            .property("data-access-policy", default_record.data_access_policy)
            .build();
        object
    }
}

impl PolicySetting {
    pub fn update_from_record(&self, record: PolicySettingRecord) {
        self.set_properties(&[
            ("uri", &record.uri),
            ("clipboard-policy", &record.clipboard_policy),
            ("device-info-policy", &record.device_info_policy),
            ("geolocation-policy", &record.geolocation_policy),
            ("cdm-policy", &record.cdm_policy),
            ("notification-policy", &record.notification_policy),
            ("pointer-lock-policy", &record.pointer_lock_policy),
            ("user-media-policy", &record.user_media_policy),
            ("data-access-policy", &record.data_access_policy),
        ]);
    }

    pub fn update(&self, other: PolicySetting) {
        let other_imp = other.imp();
        self.set_properties(&[
            ("uri", &other_imp.uri.borrow().to_owned()),
            (
                "clipboard-policy",
                &other_imp.clipboard_policy.borrow().to_owned(),
            ),
            (
                "device-info-policy",
                &other_imp.device_info_policy.borrow().to_owned(),
            ),
            (
                "geolocation-policy",
                &other_imp.geolocation_policy.borrow().to_owned(),
            ),
            ("cdm-policy", &other_imp.cdm_policy.borrow().to_owned()),
            (
                "notification-policy",
                &other_imp.notification_policy.borrow().to_owned(),
            ),
            (
                "pointer-lock-policy",
                &other_imp.pointer_lock_policy.borrow().to_owned(),
            ),
            (
                "user-media-policy",
                &other_imp.user_media_policy.borrow().to_owned(),
            ),
            (
                "data-access-policy",
                &other_imp.data_access_policy.borrow().to_owned(),
            ),
        ]);
    }
}

#[derive(
    Default,
    Debug,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    glib::Enum,
)]
#[enum_type(name = "OkuPolicyDecision")]
#[non_exhaustive]
#[repr(i32)]
pub enum PolicyDecision {
    #[default]
    Ask,
    Allow,
    Deny,
}

impl PolicyDecision {
    pub fn selected(&self) -> u32 {
        match self {
            Self::Ask => 0_u32,
            Self::Allow => 1_u32,
            Self::Deny => 2_u32,
        }
    }
}

impl From<&str> for PolicyDecision {
    fn from(value: &str) -> Self {
        match value {
            "Ask" => Self::Ask,
            "Allow" => Self::Allow,
            "Deny" => Self::Deny,
            _ => Self::default(),
        }
    }
}

impl From<u32> for PolicyDecision {
    fn from(value: u32) -> Self {
        match value {
            0 => Self::Ask,
            1 => Self::Allow,
            2 => Self::Deny,
            _ => Self::default(),
        }
    }
}

impl BrowserDatabase {
    pub fn upsert_policy_setting(
        &self,
        policy_setting: PolicySettingRecord,
    ) -> miette::Result<Option<PolicySettingRecord>> {
        let rw: transaction::RwTransaction<'_> =
            self.database.rw_transaction().into_diagnostic()?;
        let old_value: Option<PolicySettingRecord> =
            rw.upsert(policy_setting.clone()).into_diagnostic()?;
        rw.commit().into_diagnostic()?;

        Ok(old_value)
    }

    pub fn upsert_policy_settings(
        &self,
        policy_settings: Vec<PolicySettingRecord>,
    ) -> miette::Result<Vec<Option<PolicySettingRecord>>> {
        let rw = self.database.rw_transaction().into_diagnostic()?;
        let old_policy_settings: Vec<_> = policy_settings
            .clone()
            .into_iter()
            .filter_map(|policy_setting| rw.upsert(policy_setting).ok())
            .collect();
        rw.commit().into_diagnostic()?;

        Ok(old_policy_settings)
    }

    pub fn delete_policy_setting(
        &self,
        policy_setting: PolicySettingRecord,
    ) -> miette::Result<PolicySettingRecord> {
        let rw = self.database.rw_transaction().into_diagnostic()?;
        let removed_policy_setting = rw.remove(policy_setting).into_diagnostic()?;
        rw.commit().into_diagnostic()?;

        Ok(removed_policy_setting)
    }

    pub fn delete_policy_settings(
        &self,
        policy_settings: Vec<PolicySettingRecord>,
    ) -> miette::Result<Vec<PolicySettingRecord>> {
        let rw = self.database.rw_transaction().into_diagnostic()?;
        let removed_policy_settings: Vec<_> = policy_settings
            .into_iter()
            .filter_map(|policy_setting| rw.remove(policy_setting).ok())
            .collect();
        rw.commit().into_diagnostic()?;

        Ok(removed_policy_settings)
    }

    pub fn get_policy_settings(&self) -> miette::Result<Vec<PolicySettingRecord>> {
        let r = self.database.r_transaction().into_diagnostic()?;
        r.scan()
            .primary()
            .into_diagnostic()?
            .all()
            .into_diagnostic()?
            .collect::<Result<Vec<_>, _>>()
            .into_diagnostic()
    }

    pub fn get_policy_setting(&self, uri: String) -> miette::Result<Option<PolicySettingRecord>> {
        let r = self.database.r_transaction().into_diagnostic()?;
        if uri.trim().is_empty() {
            return Err(miette::miette!("Empty URI leads to panic … "));
        }
        r.get()
            .primary(
                webkit2gtk::SecurityOrigin::for_uri(&uri)
                    .to_str()
                    .to_string(),
            )
            .into_diagnostic()
    }
}
