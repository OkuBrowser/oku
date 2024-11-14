use glib::clone;
use glib::closure;
use glib::object::Cast;
use glib::object::CastNone;
use glib::object::ObjectExt;
use glib::property::PropertySet;
use glib::subclass::object::ObjectImpl;
use glib::subclass::types::ObjectSubclass;
use glib::subclass::types::ObjectSubclassExt;
use glib::subclass::types::ObjectSubclassIsExt;
use glib::value::ToValue;
use glib::Object;
use glib::ParamSpec;
use glib::ParamSpecBoxed;
use glib::ParamSpecBuilderExt;
use glib::ParamSpecObject;
use glib::ParamSpecString;
use glib::Value;
use glib::{ParamSpecBoolean, ParamSpecEnum, ParamSpecFloat};
use gtk::prelude::BoxExt;
use gtk::prelude::ButtonExt;
use gtk::prelude::GObjectPropertyExpressionExt;
use gtk::prelude::ListBoxRowExt;
use gtk::prelude::WidgetExt;
use gtk::subclass::prelude::*;
use libadwaita::prelude::ActionRowExt;
use libadwaita::prelude::PreferencesRowExt;
use libadwaita::subclass::prelude::*;
use log::error;
use std::cell::RefCell;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::LazyLock;
use webkit2gtk::functions::uri_for_display;
use webkit2gtk::prelude::WebViewExt;
use webkit2gtk::Download;
use webkit2gtk::DownloadError;
use webkit2gtk::URIResponse;

pub mod imp {
    use super::*;

    #[derive(Default, Debug)]
    pub struct DownloadRow {
        pub(crate) download: RefCell<Option<Download>>,
        pub(crate) destination: RefCell<String>,
        pub(crate) uri: RefCell<String>,
        pub(crate) estimated_progress: RefCell<f64>,
        pub(crate) error: RefCell<Option<String>>,
        pub(crate) is_finished: RefCell<bool>,
        pub(crate) spinner: libadwaita::Spinner,
        pub(crate) open_button: gtk::Button,
        pub(crate) prefix_box: gtk::Box,
        pub(crate) open_parent_button: gtk::Button,
        pub(crate) cancel_button: gtk::Button,
        pub(crate) retry_button: gtk::Button,
        pub(crate) suffix_box: gtk::Box,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DownloadRow {
        const NAME: &'static str = "OkuDownloadRow";
        type Type = super::DownloadRow;
        type ParentType = libadwaita::ActionRow;
    }

    impl ObjectImpl for DownloadRow {
        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: LazyLock<Vec<ParamSpec>> = LazyLock::new(|| {
                vec![
                    ParamSpecObject::builder::<Download>("download")
                        .construct_only()
                        .build(),
                    ParamSpecString::builder("destination").readwrite().build(),
                    ParamSpecString::builder("uri").readwrite().build(),
                    ParamSpecFloat::builder("estimated-progress")
                        .readwrite()
                        .build(),
                    ParamSpecString::builder("error").readwrite().build(),
                    ParamSpecBoolean::builder("is-finished").readwrite().build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "download" => {
                    let download = value.get::<Download>().unwrap();
                    self.download.set(Some(download));
                }
                "destination" => {
                    let destination = value.get::<String>().unwrap();
                    self.destination
                        .set(html_escape::encode_text(&destination).to_string());
                }
                "uri" => {
                    let uri = value.get::<String>().unwrap();
                    self.uri.set(html_escape::encode_text(&uri).to_string());
                }
                "estimated-progress" => {
                    let estimated_progress = value.get::<f64>().unwrap();
                    self.estimated_progress.set(estimated_progress);
                }
                "error" => {
                    if let Ok(error) = value.get::<String>() {
                        self.error.set(Some(error));
                        self.obj().finish_error();
                    }
                }
                "is-finished" => {
                    let is_finished = value.get::<bool>().unwrap();
                    self.is_finished.set(is_finished);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &ParamSpec) -> Value {
            let obj = self.obj();
            match pspec.name() {
                "download" => obj.download().to_value(),
                "destination" => obj.destination().to_string().to_value(),
                "uri" => obj.uri().to_value(),
                "estimated-progress" => obj.estimated_progress().to_value(),
                "error" => obj.error().to_value(),
                "is-finished" => obj.is_finished().to_value(),
                _ => unimplemented!(),
            }
        }
    }
    impl WidgetImpl for DownloadRow {}
    impl ListBoxRowImpl for DownloadRow {}
    impl PreferencesRowImpl for DownloadRow {}
    impl ActionRowImpl for DownloadRow {}
}

glib::wrapper! {
    pub struct DownloadRow(ObjectSubclass<imp::DownloadRow>)
    @extends libadwaita::ActionRow, libadwaita::PreferencesRow, gtk::ListBoxRow, gtk::Widget,
    @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for DownloadRow {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl DownloadRow {
    pub fn download(&self) -> Download {
        self.imp().download.borrow().as_ref().unwrap().clone()
    }
    pub fn destination(&self) -> String {
        self.imp().destination.borrow().to_string()
    }
    pub fn uri(&self) -> String {
        self.imp().uri.borrow().to_string()
    }
    pub fn estimated_progress(&self) -> f64 {
        *self.imp().estimated_progress.borrow()
    }
    pub fn error(&self) -> Option<String> {
        self.imp().error.borrow().clone()
    }
    pub fn is_finished(&self) -> bool {
        *self.imp().is_finished.borrow()
    }
    pub fn new(download: Download) -> Self {
        let this = glib::Object::builder::<Self>()
            .property("download", download.clone())
            .property("is-finished", false)
            .build();
        let imp = this.imp();

        imp.retry_button.set_visible(false);

        download
            .property_expression("destination")
            .bind(&this, "destination", gtk::Widget::NONE);
        download.property_expression("estimated-progress").bind(
            &this,
            "estimated-progress",
            gtk::Widget::NONE,
        );
        download
            .property_expression("response")
            .chain_property::<URIResponse>("uri")
            .bind(&this, "uri", gtk::Widget::NONE);
        this.property_expression("destination")
            .chain_closure::<String>(closure!(|_: Option<Object>, x: String| {
                PathBuf::from(&x)
                    .file_name()
                    .map(|x| x.to_string_lossy().to_string())
                    .unwrap_or(x)
            }))
            .bind(&this, "title", gtk::Widget::NONE);
        this.property_expression("uri")
            .bind(&this, "subtitle", gtk::Widget::NONE);
        this.property_expression("is-finished").bind(
            &imp.open_button,
            "visible",
            gtk::Widget::NONE,
        );
        this.property_expression("is-finished").bind(
            &imp.open_parent_button,
            "visible",
            gtk::Widget::NONE,
        );
        this.property_expression("is-finished")
            .chain_closure::<bool>(closure!(|_: Option<Object>, x: bool| { !x }))
            .bind(&imp.cancel_button, "visible", gtk::Widget::NONE);
        this.property_expression("is-finished")
            .chain_closure::<bool>(closure!(|_: Option<Object>, x: bool| { !x }))
            .bind(&imp.spinner, "visible", gtk::Widget::NONE);
        this.property_expression("error")
            .bind(&this, "tooltip-text", gtk::Widget::NONE);

        download.connect_failed(clone!(
            #[weak]
            this,
            move |_, error| {
                this.set_property("error", error.to_string());
                this.set_property("is-finished", true);
            }
        ));
        download.connect_finished(clone!(
            #[weak]
            this,
            move |_| {
                this.set_property("is-finished", true);
            }
        ));

        imp.open_button.connect_clicked(clone!(
            #[weak]
            this,
            move |_| {
                let _ = open::that_detached(PathBuf::from(this.destination()));
            }
        ));
        imp.open_parent_button.connect_clicked(clone!(
            #[weak]
            this,
            move |_| {
                if let Some(parent) = PathBuf::from(this.destination()).parent() {
                    let _ = open::that_detached(parent);
                }
            }
        ));
        imp.retry_button.connect_clicked(clone!(
            #[weak]
            this,
            move |_| {
                if let Some(web_view) = this.download().web_view() {
                    web_view.download_uri(&this.uri());
                }
            }
        ));
        imp.cancel_button.connect_clicked(clone!(
            #[weak]
            this,
            move |_| {
                this.download().cancel();
            }
        ));

        imp.prefix_box.append(&imp.spinner);
        imp.prefix_box.append(&imp.open_button);
        imp.suffix_box.append(&imp.open_parent_button);
        imp.suffix_box.append(&imp.retry_button);
        imp.suffix_box.append(&imp.cancel_button);

        this.add_prefix(&imp.prefix_box);
        this.add_suffix(&imp.suffix_box);

        this
    }
    pub fn finish_error(&self) {
        let imp = self.imp();
    }
}
