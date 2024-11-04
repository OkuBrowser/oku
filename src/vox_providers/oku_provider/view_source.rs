use super::core::OkuProvider;
use uuid::Uuid;
use vox::provider::VoxProvider;

impl OkuProvider {
    pub fn view_source(&self, html: String, uri: String) -> miette::Result<String> {
        let file_id = Uuid::now_v7();
        let file_path = format!("{}.vox", file_id);
        let mut table = toml::Table::new();
        table.insert("layout".into(), "view_source".into());
        table.insert("permalink".into(), format!("{}.html", file_id).into());
        table.insert("title".into(), uri.into());
        self.0
            .write_file(file_path.clone(), format!("---\n{}\n---\n{}", table, html))?;
        self.render_and_get(format!("output/{}.html", file_id))
    }
}
