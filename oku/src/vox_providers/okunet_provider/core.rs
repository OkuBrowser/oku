use std::{collections::HashMap, path::PathBuf, sync::LazyLock};
use vox::{provider::VoxProvider, ram_provider::RamProvider};

pub static OKUNET_VOX_FILES: LazyLock<HashMap<PathBuf, String>> = LazyLock::new(|| {
    HashMap::from([
        (
            "global.toml".into(),
            include_str!("../../okunet_pages/global.toml").into(),
        ),
        (
            "layouts/default.vox".into(),
            include_str!("../../okunet_pages/layouts/default.vox").into(),
        ),
        (
            "layouts/post.vox".into(),
            include_str!("../../okunet_pages/layouts/post.vox").into(),
        ),
        (
            "snippets/profile.voxs".into(),
            include_str!("../../okunet_pages/snippets/profile.voxs").into(),
        ),
        (
            "snippets/logo.svg".into(),
            include_str!("../../browser_pages/snippets/logo.svg").into(),
        ),
        (
            "snippets/head.html".into(),
            include_str!("../../okunet_pages/snippets/head.html").into(),
        ),
        (
            "snippets/normalise.css".into(),
            include_str!("../../browser_pages/snippets/normalise.css").into(),
        ),
        (
            "snippets/post.voxs".into(),
            include_str!("../../okunet_pages/snippets/post.voxs").into(),
        ),
        (
            "snippets/posts.voxs".into(),
            include_str!("../../okunet_pages/snippets/posts.voxs").into(),
        ),
        (
            "snippets/style.css".into(),
            include_str!("../../okunet_pages/snippets/style.css").into(),
        ),
        (
            "snippets/tag.voxs".into(),
            include_str!("../../okunet_pages/snippets/tag.voxs").into(),
        ),
        (
            "snippets/tags.voxs".into(),
            include_str!("../../okunet_pages/snippets/tags.voxs").into(),
        ),
        (
            "snippets/search.voxs".into(),
            include_str!("../../okunet_pages/snippets/search.voxs").into(),
        ),
        (
            "snippets/follow_button.html".into(),
            include_str!("../../okunet_pages/snippets/follow_button.html").into(),
        ),
        (
            "snippets/block_button.html".into(),
            include_str!("../../okunet_pages/snippets/block_button.html").into(),
        ),
        (
            "snippets/delete_button.html".into(),
            include_str!("../../okunet_pages/snippets/delete_button.html").into(),
        ),
        (
            "snippets/user_header.html".into(),
            include_str!("../../okunet_pages/snippets/user_header.html").into(),
        ),
        (
            "snippets/tab_pages.html".into(),
            include_str!("../../okunet_pages/snippets/tab_pages.html").into(),
        ),
        (
            "snippets/masthead.html".into(),
            include_str!("../../okunet_pages/snippets/masthead.html").into(),
        ),
        (
            "snippets/user-trash-symbolic.svg".into(),
            include_str!("../../../data/hicolor/scalable/actions/user-trash-symbolic.svg").into(),
        ),
        (
            "tags.vox".into(),
            include_str!("../../okunet_pages/tags.vox").into(),
        ),
        (
            "home.vox".into(),
            include_str!("../../okunet_pages/home.vox").into(),
        ),
    ])
});

#[derive(Debug, Clone)]
pub struct OkuNetProvider(pub RamProvider);

impl Default for OkuNetProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl OkuNetProvider {
    pub fn new() -> Self {
        Self(RamProvider::new(Some(OKUNET_VOX_FILES.clone())))
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
