use super::core::OkuNetProvider;
use oku_fs::database::core::OkuDatabase;
use vox::provider::VoxProvider;

impl OkuNetProvider {
    pub fn get_search_frontmatter(&self, query: &str) -> miette::Result<toml::Table> {
        let mut table = toml::Table::new();
        table.insert("layout".into(), "default".into());
        table.insert("depends".into(), vec!["search"].into());
        table.insert("permalink".into(), "search".into());
        table.insert("title".into(), query.into());
        Ok(table)
    }

    pub fn create_search_page(&self, query: &str) -> miette::Result<()> {
        let table = self.get_search_frontmatter(query)?;
        let page_contents = format!(
            "---
{}
---
{{% if search[0] %}}
{{% include search.voxs posts = search %}}
{{% else %}}
{{% include search.voxs posts = \"\" %}}
{{% endif %}}
",
            table
        );
        self.0.write_file("search.vox", page_contents)?;
        Ok(())
    }
    pub async fn search(&self, query: String) -> miette::Result<String> {
        let search_results = OkuDatabase::search_posts(query.clone(), None)?;
        for post in search_results.iter() {
            self.create_post_page(&post.user(), post, None).await?;
        }
        self.create_search_page(&query)?;
        self.render_and_get("output/search")
    }
}
