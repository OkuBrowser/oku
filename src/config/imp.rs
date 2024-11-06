use super::enums::*;
use super::*;
use glib::{ParamSpecEnum, ParamSpecInt};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    pub(crate) colour_scheme: RefCell<ColourScheme>,
    pub(crate) colour_per_domain: RefCell<bool>,
    pub(crate) palette: RefCell<Palette>,
    pub(crate) width: RefCell<i32>,
    pub(crate) height: RefCell<i32>,
    pub(crate) is_maximised: RefCell<bool>,
    pub(crate) is_fullscreen: RefCell<bool>,
}

impl Config {
    pub fn new() -> Self {
        match std::fs::read_to_string(CONFIG_DIR.to_path_buf())
            .ok()
            .map(|x| toml::from_str(&x).ok())
            .flatten()
        {
            Some(config) => config,
            None => Self::default(),
        }
    }

    pub fn save(&self) {
        match toml::to_string_pretty(&self) {
            Ok(config_file_string) => {
                match std::fs::write(CONFIG_DIR.to_path_buf(), config_file_string) {
                    Ok(_) => (),
                    Err(e) => error!("{}", e),
                }
            }
            Err(e) => error!("{}", e),
        }
    }

    pub fn colour_scheme(&self) -> ColourScheme {
        self.colour_scheme.borrow().to_owned()
    }

    pub fn set_colour_scheme(&self, colour_scheme: ColourScheme) -> ColourScheme {
        self.colour_scheme.replace(colour_scheme)
    }

    pub fn palette(&self) -> Palette {
        self.palette.borrow().to_owned()
    }

    pub fn set_palette(&self, palette: Palette) -> Palette {
        self.palette.replace(palette)
    }

    pub fn colour_per_domain(&self) -> bool {
        self.colour_per_domain.borrow().to_owned()
    }

    pub fn set_colour_per_domain(&self, colour_per_domain: bool) -> bool {
        self.colour_per_domain.replace(colour_per_domain)
    }

    pub fn width(&self) -> i32 {
        self.width.borrow().to_owned()
    }

    pub fn set_width(&self, width: i32) -> i32 {
        self.width.replace(width)
    }

    pub fn height(&self) -> i32 {
        self.height.borrow().to_owned()
    }

    pub fn set_height(&self, height: i32) -> i32 {
        self.height.replace(height)
    }

    pub fn is_maximised(&self) -> bool {
        self.is_maximised.borrow().to_owned()
    }

    pub fn set_is_maximised(&self, is_maximised: bool) -> bool {
        self.is_maximised.replace(is_maximised)
    }

    pub fn is_fullscreen(&self) -> bool {
        self.is_fullscreen.borrow().to_owned()
    }

    pub fn set_is_fullscreen(&self, is_fullscreen: bool) -> bool {
        self.is_fullscreen.replace(is_fullscreen)
    }
}

#[glib::object_subclass]
impl ObjectSubclass for Config {
    const NAME: &'static str = "OkuConfig";
    type Type = super::Config;
}
impl ObjectImpl for Config {
    fn properties() -> &'static [ParamSpec] {
        static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
            vec![
                ParamSpecEnum::builder::<ColourScheme>("colour-scheme")
                    .readwrite()
                    .build(),
                ParamSpecBoolean::builder("colour-per-domain")
                    .readwrite()
                    .build(),
                ParamSpecEnum::builder::<Palette>("palette")
                    .readwrite()
                    .build(),
                ParamSpecInt::builder("width").readwrite().build(),
                ParamSpecInt::builder("height").readwrite().build(),
                ParamSpecBoolean::builder("is-maximised")
                    .readwrite()
                    .build(),
                ParamSpecBoolean::builder("is-fullscreen")
                    .readwrite()
                    .build(),
            ]
        });
        PROPERTIES.as_ref()
    }

    fn set_property(&self, _id: usize, value: &Value, pspec: &ParamSpec) {
        match pspec.name() {
            "colour-scheme" => {
                let colour_scheme = value.get::<ColourScheme>().unwrap();
                self.set_colour_scheme(colour_scheme);
            }
            "colour-per-domain" => {
                let colour_per_domain = value.get::<bool>().unwrap();
                self.set_colour_per_domain(colour_per_domain);
            }
            "palette" => {
                let palette = value.get::<Palette>().unwrap();
                self.set_palette(palette);
            }
            "width" => {
                let width = value.get::<i32>().unwrap();
                self.set_width(width);
            }
            "height" => {
                let height = value.get::<i32>().unwrap();
                self.set_height(height);
            }
            "is-maximised" => {
                let is_maximised = value.get::<bool>().unwrap();
                self.set_is_maximised(is_maximised);
            }
            "is-fullscreen" => {
                let is_fullscreen = value.get::<bool>().unwrap();
                self.set_is_fullscreen(is_fullscreen);
            }
            _ => unimplemented!(),
        }
        self.save();
    }

    fn property(&self, _id: usize, pspec: &ParamSpec) -> Value {
        match pspec.name() {
            "colour-scheme" => self.colour_scheme().to_value(),
            "colour-per-domain" => self.colour_per_domain().to_value(),
            "palette" => self.palette().to_value(),
            "width" => self.width().to_value(),
            "height" => self.height().to_value(),
            "is-maximised" => self.is_maximised().to_value(),
            "is-fullscreen" => self.is_fullscreen().to_value(),
            _ => unimplemented!(),
        }
    }
}
