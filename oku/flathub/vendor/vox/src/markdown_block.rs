use comrak::markdown_to_html_with_plugins;
use comrak::options::ListStyleType;
use comrak::options::Plugins;
use comrak::plugins::syntect::SyntectAdapter;
use liquid_core::error::ResultLiquidReplaceExt;
use liquid_core::parser;
use liquid_core::runtime;
use liquid_core::Language;
use liquid_core::Renderable;
use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::Template;
use liquid_core::{BlockReflection, ParseBlock, TagBlock, TagTokenIter};
use std::io::BufWriter;
use std::io::Write;

/// Render Markdown as HTML
///
/// # Arguments
///
/// * `text_to_render` - The Markdown text to render into HTML
pub fn render_markdown(text_to_render: &str) -> String {
    let mut options = comrak::Options::default();
    options.extension.alerts = true;
    options.extension.autolink = false;
    options.extension.block_directive = true;
    options.extension.cjk_friendly_emphasis = true;
    options.extension.description_lists = true;
    options.extension.footnotes = true;
    options.extension.front_matter_delimiter = None;
    options.extension.greentext = true;
    options.extension.header_id_prefix = Some(String::new());
    options.extension.header_id_prefix_in_href = true;
    options.extension.highlight = true;
    options.extension.inline_footnotes = true;
    options.extension.insert = true;
    options.extension.math_dollars = true;
    options.extension.math_code = true;
    options.extension.multiline_block_quotes = true;
    options.extension.shortcodes = true;
    options.extension.spoiler = true;
    options.extension.strikethrough = true;
    options.extension.subscript = true;
    options.extension.subtext = true;
    options.extension.superscript = true;
    options.extension.table = true;
    options.extension.tagfilter = false;
    options.extension.tasklist = true;
    options.extension.underline = true;
    options.extension.description_lists = true;
    options.extension.wikilinks_title_after_pipe = true;
    options.extension.wikilinks_title_before_pipe = false;

    options.parse.ignore_setext = false;
    options.parse.smart = true;
    options.parse.default_info_string = None;
    options.parse.relaxed_tasklist_matching = true;
    options.parse.relaxed_autolinks = true;
    options.parse.tasklist_in_table = true;

    options.render.hardbreaks = false;
    options.render.github_pre_lang = true;
    options.render.full_info_string = true;
    options.render.width = 80;
    options.render.r#unsafe = true;
    options.render.escape = false;
    options.render.list_style = ListStyleType::Dash;
    options.render.sourcepos = false;
    options.render.escaped_char_spans = false;
    options.render.ignore_empty_links = true;
    options.render.gfm_quirks = false;
    options.render.prefer_fenced = false;
    options.render.figure_with_caption = false;
    options.render.tasklist_classes = true;
    let mut plugins = Plugins::default();
    let syntax_highlighting_adapter = SyntectAdapter::new(None);
    plugins.render.codefence_syntax_highlighter = Some(&syntax_highlighting_adapter);
    markdown_to_html_with_plugins(&text_to_render, &options, &plugins)
}

#[derive(Copy, Clone, Debug, Default)]
/// A Liquid template block containing Markdown.
/// The block begins with `{% markdown %}` and ends with `{% endmarkdown %}`.
pub struct MarkdownBlock;

impl MarkdownBlock {
    /// Provides a new instance of the Markdown tag parser.
    pub fn new() -> Self {
        Self
    }
}

impl BlockReflection for MarkdownBlock {
    fn start_tag(&self) -> &str {
        "markdown"
    }

    fn end_tag(&self) -> &str {
        "endmarkdown"
    }

    fn description(&self) -> &str {
        ""
    }
}

impl ParseBlock for MarkdownBlock {
    fn parse(
        &self,
        mut arguments: TagTokenIter<'_>,
        mut tokens: TagBlock<'_, '_>,
        options: &Language,
    ) -> Result<Box<dyn Renderable + 'static>, liquid::Error> {
        arguments.expect_nothing()?;

        let raw_content = tokens.escape_liquid(false)?.to_string();
        let content = parser::parse(&raw_content, options)
            .map(runtime::Template::new)
            .unwrap();

        tokens.assert_empty();
        Ok(Box::new(Markdown { content }))
    }

    fn reflection(&self) -> &dyn BlockReflection {
        self
    }
}

#[derive(Debug)]
struct Markdown {
    content: Template,
}

impl Renderable for Markdown {
    fn render_to(
        &self,
        writer: &mut dyn Write,
        runtime: &dyn Runtime,
    ) -> Result<(), liquid::Error> {
        let mut buf = BufWriter::new(Vec::new());
        self.content.render_to(&mut buf, runtime)?;
        let bytes = buf.into_inner().unwrap_or_default();
        let liquid_rendered = String::from_utf8(bytes).unwrap_or_default();
        let rendered = render_markdown(&liquid_rendered);
        write!(writer, "{}", rendered).replace("Failed to render")?;
        Ok(())
    }
}
