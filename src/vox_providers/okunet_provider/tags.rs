use super::core::OkuNetProvider;
use oku_fs::database::OkuPost;
use vox::provider::VoxProvider;

impl OkuNetProvider {
    pub async fn get_tag_frontmatter(
        &self,
        tag: String,
        posts: Vec<OkuPost>,
    ) -> miette::Result<toml::Table> {
        let mut tag_post_frontmatter: Vec<toml::Table> = Vec::new();
        for post in posts.iter() {
            if let Ok(post_frontmatter) = self.get_post_frontmatter(&post.user(), post).await {
                tag_post_frontmatter.push(post_frontmatter);
            }
        }
        let mut table = toml::Table::new();
        table.insert("layout".into(), "default".into());
        table.insert("depends".into(), vec![tag.clone()].into());
        table.insert("permalink".into(), format!("tag/{}", tag).into());
        table.insert("title".into(), tag.into());
        table.insert("posts".into(), tag_post_frontmatter.into());
        Ok(table)
    }
    pub async fn create_tag_page(
        &self,
        tag: String,
        posts: Option<Vec<OkuPost>>,
    ) -> miette::Result<()> {
        let tag_posts = posts.unwrap_or(
            oku_fs::database::DATABASE
                .get_posts_by_tag(tag.clone())
                .unwrap_or_default(),
        );
        for post in tag_posts.iter() {
            self.create_post_page(&post.user(), post, Some(tag.clone()))
                .await?;
        }
        let page_path = format!("/tag/{}.vox", tag);
        let table = self.get_tag_frontmatter(tag.clone(), tag_posts).await?;
        let page_contents = format!(
            "---
{}
---
{{% include tag.voxs posts = {} %}}
",
            table, tag
        );
        self.0.write_file(page_path, page_contents)?;
        Ok(())
    }

    pub async fn view_tag(&self, tag: String) -> miette::Result<String> {
        self.create_tag_page(tag.clone(), None).await?;
        self.render_and_get(format!("output/tag/{}", tag))
    }

    pub async fn view_tags(&self) -> miette::Result<String> {
        let tags = oku_fs::database::DATABASE.get_tags().unwrap_or_default();
        for tag in tags {
            self.create_tag_page(tag.clone(), None).await?;
        }
        self.render_and_get("output/tags")
    }
}
