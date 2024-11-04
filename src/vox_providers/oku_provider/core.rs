use std::{collections::HashMap, path::PathBuf, sync::LazyLock};
use vox::{provider::VoxProvider, ram_provider::RamProvider};

pub static OKU_VOX_FILES: LazyLock<HashMap<PathBuf, String>> = LazyLock::new(|| {
    HashMap::from([
        (
            "global.toml".into(),
            include_str!("../../browser_pages/global.toml").into(),
        ),
        (
            "home.vox".into(),
            include_str!("../../browser_pages/home.vox").into(),
        ),
        (
            "layouts/view_source.vox".into(),
            include_str!("../../browser_pages/layouts/view_source.vox").into(),
        ),
        (
            "layouts/default.vox".into(),
            include_str!("../../browser_pages/layouts/default.vox").into(),
        ),
        (
            "layouts/home.vox".into(),
            include_str!("../../browser_pages/layouts/home.vox").into(),
        ),
        (
            "snippets/head.html".into(),
            include_str!("../../browser_pages/snippets/head.html").into(),
        ),
        (
            "snippets/highlight.min.js".into(),
            include_str!("../../browser_pages/snippets/highlight.min.js").into(),
        ),
        (
            "snippets/highlightjs-line-numbers.min.js".into(),
            include_str!("../../browser_pages/snippets/highlightjs-line-numbers.min.js").into(),
        ),
        (
            "snippets/hljs.default.min.css".into(),
            include_str!("../../browser_pages/snippets/hljs.default.min.css").into(),
        ),
        (
            "snippets/logo.svg".into(),
            include_str!("../../browser_pages/snippets/logo.svg").into(),
        ),
        (
            "snippets/normalise.css".into(),
            include_str!("../../browser_pages/snippets/normalise.css").into(),
        ),
        (
            "snippets/style.css".into(),
            include_str!("../../browser_pages/snippets/style.css").into(),
        ),
    ])
});

#[derive(Debug, Clone)]
pub struct OkuProvider(pub RamProvider);

impl OkuProvider {
    pub fn new() -> Self {
        Self(RamProvider::new(Some(OKU_VOX_FILES.clone())))
    }

    pub fn render_and_get(&self, path: impl AsRef<std::path::Path>) -> miette::Result<String> {
        let parser = self.0.create_liquid_parser()?;
        let global = self.0.get_global_context()?;
        let (dag, _pages, _layouts) = self.0.generate_dag()?;
        let (_updated_pages, _updated_dag) = self.0.generate_site(
            parser.clone(),
            global.0.clone(),
            global.1,
            dag,
            false,
            false,
        )?;
        self.0.read_to_string(path)
    }
}
