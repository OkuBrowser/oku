use glib::clone;
use glib::closure;
use glib::object::ObjectExt;
use glib::property::PropertySet;
use glib::subclass::object::ObjectImpl;
use glib::subclass::types::ObjectSubclass;
use glib::subclass::types::ObjectSubclassExt;
use glib::subclass::types::ObjectSubclassIsExt;
use glib::value::ToValue;
use glib::Object;
use glib::ParamSpec;
use glib::ParamSpecBoolean;
use glib::ParamSpecBuilderExt;
use glib::ParamSpecDouble;
use glib::ParamSpecObject;
use glib::ParamSpecString;
use glib::Value;
use gtk::prelude::BoxExt;
use gtk::prelude::ButtonExt;
use gtk::prelude::GObjectPropertyExpressionExt;
use gtk::prelude::WidgetExt;
use gtk::subclass::prelude::*;
use libadwaita::prelude::ActionRowExt;
use libadwaita::prelude::AnimationExt;
use libadwaita::subclass::prelude::*;
use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::LazyLock;
use webkit2gtk::prelude::WebViewExt;
use webkit2gtk::Download;
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
        pub(crate) row: libadwaita::ActionRow,
        pub(crate) progress_overlay: gtk::Overlay,
        pub(crate) progress: gtk::ProgressBar,
        pub(crate) progress_animation: RefCell<Option<libadwaita::SpringAnimation>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DownloadRow {
        const NAME: &'static str = "OkuDownloadRow";
        type Type = super::DownloadRow;
        type ParentType = gtk::Box;
    }

    impl ObjectImpl for DownloadRow {
        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: LazyLock<Vec<ParamSpec>> = LazyLock::new(|| {
                vec![
                    ParamSpecObject::builder::<Download>("download")
                        .readwrite()
                        .build(),
                    ParamSpecString::builder("destination").readwrite().build(),
                    ParamSpecString::builder("uri").readwrite().build(),
                    ParamSpecDouble::builder("estimated-progress")
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
                    if let Ok(download) = value.get::<Download>() {
                        self.download.set(Some(download.clone()));
                        self.obj().setup(download);
                    }
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
                    self.obj().set_progress_animated(estimated_progress);
                }
                "error" => {
                    if let Ok(error) = value.get::<String>() {
                        self.error.set(Some(error));
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
    impl BoxImpl for DownloadRow {}
}

glib::wrapper! {
    pub struct DownloadRow(ObjectSubclass<imp::DownloadRow>)
    @extends gtk::Box, libadwaita::ActionRow, libadwaita::PreferencesRow, gtk::ListBoxRow, gtk::Widget,
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
        let this = Self::default();

        this.set_property("download", download);

        this
    }

    pub fn setup(&self, download: Download) {
        self.set_property("is-finished", false);

        let imp = self.imp();

        imp.retry_button.set_visible(false);
        imp.progress.set_show_text(true);
        imp.progress.set_valign(gtk::Align::End);

        download
            .property_expression("destination")
            .bind(self, "destination", gtk::Widget::NONE);
        download.property_expression("estimated-progress").bind(
            self,
            "estimated-progress",
            gtk::Widget::NONE,
        );
        download
            .property_expression("response")
            .chain_property::<URIResponse>("uri")
            .bind(self, "uri", gtk::Widget::NONE);
        self.property_expression("destination")
            .chain_closure::<String>(closure!(|_: Option<Object>, x: String| {
                PathBuf::from(&x)
                    .file_name()
                    .map(|x| x.to_string_lossy().to_string())
                    .unwrap_or(x)
            }))
            .bind(&imp.row, "title", gtk::Widget::NONE);
        self.property_expression("uri")
            .bind(&imp.row, "subtitle", gtk::Widget::NONE);
        self.property_expression("is-finished").bind(
            &imp.open_button,
            "visible",
            gtk::Widget::NONE,
        );
        self.property_expression("is-finished").bind(
            &imp.open_parent_button,
            "visible",
            gtk::Widget::NONE,
        );
        self.property_expression("is-finished")
            .chain_closure::<bool>(closure!(|_: Option<Object>, x: bool| { !x }))
            .bind(&imp.cancel_button, "visible", gtk::Widget::NONE);
        self.property_expression("is-finished")
            .chain_closure::<bool>(closure!(|_: Option<Object>, x: bool| { !x }))
            .bind(&imp.spinner, "visible", gtk::Widget::NONE);
        self.property_expression("error")
            .bind(self, "tooltip-text", gtk::Widget::NONE);
        self.property_expression("estimated-progress").bind(
            &imp.progress,
            "fraction",
            gtk::Widget::NONE,
        );

        download.connect_failed(clone!(
            #[weak(rename_to = this)]
            self,
            move |_, error| {
                this.set_property("error", error.to_string());
            }
        ));
        download.connect_finished(clone!(
            #[weak(rename_to = this)]
            self,
            #[weak]
            imp,
            move |_| {
                this.set_property("is-finished", true);
                match this.error() {
                    Some(error) => {
                        imp.progress.set_text(Some(&error));
                        imp.progress.add_css_class("error");
                        imp.open_button.set_visible(false);
                        imp.open_parent_button.set_visible(false);
                        imp.retry_button.set_visible(true);
                    }
                    None => {
                        imp.progress.set_visible(false);
                    }
                }
            }
        ));

        imp.open_button.connect_clicked(clone!(
            #[weak(rename_to = this)]
            self,
            move |_| {
                let _ = open::that_detached(PathBuf::from(this.destination()));
            }
        ));
        imp.open_parent_button.connect_clicked(clone!(
            #[weak(rename_to = this)]
            self,
            move |_| {
                let destination = this.destination();
                gio::spawn_blocking(|| showfile::show_path_in_file_manager(destination));
            }
        ));
        imp.retry_button.connect_clicked(clone!(
            #[weak(rename_to = this)]
            self,
            move |_| {
                if let Some(web_view) = this.download().web_view() {
                    web_view.download_uri(&this.uri());
                }
            }
        ));
        imp.cancel_button.connect_clicked(clone!(
            #[weak(rename_to = this)]
            self,
            move |_| {
                this.download().cancel();
            }
        ));

        imp.open_button.set_icon_name("external-link-symbolic");
        imp.open_button.add_css_class("circular");
        imp.open_button.set_vexpand(false);
        imp.open_button.set_hexpand(false);

        imp.open_parent_button.set_icon_name("folder-open-symbolic");
        imp.open_parent_button.add_css_class("circular");
        imp.open_parent_button.add_css_class("linked");
        imp.open_parent_button.set_vexpand(false);
        imp.open_parent_button.set_hexpand(false);

        imp.retry_button
            .set_icon_name("arrow-circular-top-right-symbolic");
        imp.retry_button.add_css_class("circular");
        imp.retry_button.add_css_class("linked");
        imp.retry_button.set_vexpand(false);
        imp.retry_button.set_hexpand(false);

        imp.cancel_button.set_icon_name("window-close-symbolic");
        imp.cancel_button.add_css_class("circular");
        imp.cancel_button.add_css_class("linked");
        imp.cancel_button.set_vexpand(false);
        imp.cancel_button.set_hexpand(false);

        imp.prefix_box.append(&imp.spinner);
        imp.prefix_box.append(&imp.open_button);
        imp.prefix_box.set_homogeneous(false);
        imp.prefix_box.set_valign(gtk::Align::Center);
        imp.prefix_box.set_halign(gtk::Align::End);
        imp.prefix_box.add_css_class("linked");

        imp.suffix_box.append(&imp.open_parent_button);
        imp.suffix_box.append(&imp.retry_button);
        imp.suffix_box.append(&imp.cancel_button);
        imp.suffix_box.set_homogeneous(false);
        imp.suffix_box.set_valign(gtk::Align::Center);
        imp.suffix_box.set_halign(gtk::Align::End);
        imp.suffix_box.add_css_class("linked");

        imp.row.add_prefix(&imp.prefix_box);
        imp.row.add_suffix(&imp.suffix_box);
        imp.row.set_margin_bottom(24);
        imp.row.set_title_lines(1);
        imp.row.set_subtitle_lines(1);
        imp.row.add_css_class("caption");

        imp.progress_overlay.set_child(Some(&imp.row));
        imp.progress_overlay.add_overlay(&imp.progress);
        imp.progress_overlay.set_margin_bottom(4);

        self.set_margin_bottom(4);
        self.set_vexpand(true);
        self.set_hexpand(true);
        self.set_homogeneous(true);
        self.add_css_class("card");
        self.append(&imp.progress_overlay);
    }

    /// Adapted from Geopard (https://github.com/ranfdev/Geopard)
    fn set_progress_animated(&self, progress: f64) {
        let imp = self.imp();

        if let Some(animation) = imp.progress_animation.borrow().as_ref() {
            animation.pause()
        }
        if progress == 0.0 {
            imp.progress.set_fraction(0.0);
            return;
        }
        let animation = libadwaita::SpringAnimation::new(
            &imp.progress,
            imp.progress.fraction(),
            progress,
            libadwaita::SpringParams::new(1.0, 1.0, 100.0),
            libadwaita::CallbackAnimationTarget::new(clone!(
                #[weak]
                imp,
                move |v| {
                    imp.progress.set_fraction(v);
                }
            )),
        );
        animation.play();
        imp.progress_animation.replace(Some(animation));
    }
}
