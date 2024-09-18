use crate::CONFIG_DIR;
use serde::Deserialize;
use serde::Serialize;
use std::cell::RefCell;
use tracing::error;

#[derive(
    Default, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Clone, Copy, Serialize, Deserialize,
)]
#[non_exhaustive]
pub enum ColourScheme {
    #[default]
    Default,
    ForceLight,
    PreferLight,
    PreferDark,
    ForceDark,
    __Unknown(i32),
}

impl ColourScheme {
    pub fn to_adw_scheme(&self) -> libadwaita::ColorScheme {
        match self {
            Self::Default => libadwaita::ColorScheme::Default,
            Self::ForceLight => libadwaita::ColorScheme::ForceLight,
            Self::PreferLight => libadwaita::ColorScheme::PreferLight,
            Self::PreferDark => libadwaita::ColorScheme::PreferDark,
            Self::ForceDark => libadwaita::ColorScheme::ForceDark,
            Self::__Unknown(i) => libadwaita::ColorScheme::__Unknown(*i),
        }
    }
}

#[derive(
    Default, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Clone, Copy, Serialize, Deserialize,
)]
pub enum Palette {
    #[default]
    None,
    Blue,
    Green,
    Yellow,
    Orange,
    Red,
    Purple,
    Brown,
}

impl Palette {
    pub fn hue(&self) -> u64 {
        match self {
            Self::None => unreachable!(),
            Self::Blue => 213,
            Self::Green => 152,
            Self::Yellow => 42,
            Self::Orange => 21,
            Self::Red => 353,
            Self::Purple => 274,
            Self::Brown => 27,
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Config {
    pub(crate) colour_scheme: RefCell<ColourScheme>,
    pub(crate) colour_per_domain: RefCell<bool>,
    pub(crate) palette: RefCell<Palette>,
}

impl Config {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_or_default() -> Self {
        match std::fs::read_to_string(CONFIG_DIR.to_path_buf()) {
            Ok(config_file_string) => match toml::from_str(&config_file_string) {
                Ok(config) => config,
                Err(e) => {
                    error!("{}", e);
                    Self::default()
                }
            },
            Err(e) => {
                error!("{}", e);
                Self::default()
            }
        }
    }

    pub fn save(&self) {
        match toml::to_string_pretty(&self) {
            Ok(config_file_string) => {
                match std::fs::write(CONFIG_DIR.to_path_buf(), config_file_string) {
                    Ok(_) => {}
                    Err(e) => {
                        error!("{}", e);
                    }
                }
            }
            Err(e) => {
                error!("{}", e);
            }
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
}
