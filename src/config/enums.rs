use serde::Deserialize;
use serde::Serialize;

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
#[enum_type(name = "OkuColourScheme")]
#[non_exhaustive]
#[repr(i32)]
pub enum ColourScheme {
    #[default]
    Default,
    ForceLight,
    PreferLight,
    PreferDark,
    ForceDark,
}

impl From<&str> for ColourScheme {
    fn from(value: &str) -> Self {
        match value {
            "Automatic" => Self::Default,
            "Force Light" => Self::ForceLight,
            "Prefer Light" => Self::PreferLight,
            "Prefer Dark" => Self::PreferDark,
            "Force Dark" => Self::ForceDark,
            _ => Self::Default,
        }
    }
}

impl From<libadwaita::ColorScheme> for ColourScheme {
    fn from(value: libadwaita::ColorScheme) -> Self {
        match value {
            libadwaita::ColorScheme::Default => Self::Default,
            libadwaita::ColorScheme::ForceLight => Self::ForceLight,
            libadwaita::ColorScheme::PreferLight => Self::PreferLight,
            libadwaita::ColorScheme::PreferDark => Self::PreferDark,
            libadwaita::ColorScheme::ForceDark => Self::ForceDark,
            _ => Self::Default,
        }
    }
}

impl Into<libadwaita::ColorScheme> for ColourScheme {
    fn into(self) -> libadwaita::ColorScheme {
        match self {
            Self::Default => libadwaita::ColorScheme::Default,
            Self::ForceLight => libadwaita::ColorScheme::ForceLight,
            Self::PreferLight => libadwaita::ColorScheme::PreferLight,
            Self::PreferDark => libadwaita::ColorScheme::PreferDark,
            Self::ForceDark => libadwaita::ColorScheme::ForceDark,
        }
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
#[enum_type(name = "OkuPalette")]
#[repr(i32)]
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

impl From<&str> for Palette {
    fn from(value: &str) -> Self {
        match value {
            "None" => Self::None,
            "Blue" => Self::Blue,
            "Green" => Self::Green,
            "Yellow" => Self::Yellow,
            "Orange" => Self::Orange,
            "Red" => Self::Red,
            "Purple" => Self::Purple,
            "Brown" => Self::Brown,
            _ => Self::None,
        }
    }
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
