use std::fmt;

/// mi mathvariant attribute
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Variant {
    Normal,
    Italic,
    Bold,
    BoldItalic,
    DoubleStruck,
    BoldFraktur,
    Script,
    BoldScript,
    Fraktur,
    SansSerif,
    BoldSansSerif,
    SansSerifItalic,
    SansSerifBoldItalic,
    Monospace,
}

impl fmt::Display for Variant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Variant::Normal              => write!(f, "normal"),
            Variant::Italic              => write!(f, "italic"),
            Variant::Bold                => write!(f, "bold"),
            Variant::BoldItalic          => write!(f, "bold-italic"),
            Variant::DoubleStruck        => write!(f, "double-struck"),
            Variant::BoldFraktur         => write!(f, "bold-fraktur"),
            Variant::Script              => write!(f, "script"),
            Variant::BoldScript          => write!(f, "bold-script"),
            Variant::Fraktur             => write!(f, "fraktur"),
            Variant::SansSerif           => write!(f, "sans-serif"),
            Variant::BoldSansSerif       => write!(f, "bold-sans-serif"),
            Variant::SansSerifItalic     => write!(f, "sans-serif-italic"),
            Variant::SansSerifBoldItalic => write!(f, "sans-serif-bold-italic"),
            Variant::Monospace           => write!(f, "monospace"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Accent {
    True,
    False,
}

impl fmt::Display for Accent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Accent::True => write!(f, "true"),
            Accent::False => write!(f, "false"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineThickness {
    Thin,
    Medium,
    Thick,
    Length(u8),
}
impl fmt::Display for LineThickness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LineThickness::Thin      => write!(f, r#" linethickness="thin""#),
            LineThickness::Medium    => write!(f, r#""#),
            LineThickness::Thick     => write!(f, r#" linethickness="medium""#),
            LineThickness::Length(l) => write!(f, r#" linethickness="{}""#, l),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnAlign {
    Center,
    Left,
    Right,
}

impl fmt::Display for ColumnAlign {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ColumnAlign::Center => write!(f, r#""#),
            ColumnAlign::Left => write!(f, r#" columnalign=left"#),
            ColumnAlign::Right => write!(f, r#" columnalign=right"#),
        }
    }
}
