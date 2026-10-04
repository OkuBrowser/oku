use latex2mathml::latex_to_mathml;
use latex2mathml::DisplayStyle;
use liquid_core::error::ResultLiquidReplaceExt;
use liquid_core::Language;
use liquid_core::Renderable;
use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::{BlockReflection, ParseBlock, TagBlock, TagTokenIter};
use std::io::Write;

#[derive(Copy, Clone, Debug, Default)]
/// A Liquid template block containing math.
/// The block begins with `{% math %}` and ends with `{% endmath %}`.
pub struct MathBlock;

impl MathBlock {
    /// Provides a new instance of the math tag parser.
    pub fn new() -> Self {
        Self
    }
}

impl BlockReflection for MathBlock {
    fn start_tag(&self) -> &str {
        "math"
    }

    fn end_tag(&self) -> &str {
        "endmath"
    }

    fn description(&self) -> &str {
        ""
    }
}

impl ParseBlock for MathBlock {
    fn parse(
        &self,
        mut arguments: TagTokenIter<'_>,
        mut tokens: TagBlock<'_, '_>,
        _options: &Language,
    ) -> Result<Box<dyn Renderable + 'static>, liquid::Error> {
        arguments.expect_nothing()?;

        let raw_content = tokens.escape_liquid(false)?.to_string();
        let content = latex_to_mathml(&raw_content, DisplayStyle::Inline).unwrap();

        tokens.assert_empty();
        Ok(Box::new(Math { content }))
    }

    fn reflection(&self) -> &dyn BlockReflection {
        self
    }
}

#[derive(Clone, Debug)]
struct Math {
    content: String,
}

impl Renderable for Math {
    fn render_to(
        &self,
        writer: &mut dyn Write,
        _runtime: &dyn Runtime,
    ) -> Result<(), liquid::Error> {
        write!(writer, "{}", self.content).replace("Failed to render")?;
        Ok(())
    }
}
