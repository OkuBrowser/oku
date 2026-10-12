use std::collections::HashSet;

use super::core::OkuNetProvider;
use crate::NODE;
use oku_core::database::posts::core::OkuPost;
use rayon::iter::FromParallelIterator;
use vox::provider::VoxProvider;

impl OkuNetProvider {
    pub async fn get_tag_frontmatter(
        &self,
        tag: String,
        posts: Vec<OkuPost>,
    ) -> miette::Result<toml::Table> {
        let mut tag_post_frontmatter: Vec<toml::Table> = Vec::new();
        for post in &posts {
            if let Ok(post_frontmatter) = self.get_post_frontmatter(&post.user(), post).await {
                tag_post_frontmatter.push(post_frontmatter);
            }
        }
        let mut table = toml::Table::new();
        table.insert("layout".into(), "default".into());
        table.insert("depends".into(), vec![tag.clone()].into());
        table.insert("permalink".into(), format!("tag/{tag}").into());
        table.insert("title".into(), tag.into());
        table.insert("posts".into(), tag_post_frontmatter.into());
        Ok(table)
    }
    pub async fn create_tag_page(&self, tag: String) -> miette::Result<()> {
        let node = NODE
            .get()
            .ok_or(miette::miette!("No running Oku node … "))?;
        let tag_posts = node.posts_with_tags(
            &Vec::from_par_iter(node.all_posts().await),
            &HashSet::from_par_iter(vec![tag.clone()]),
        );
        for post in &tag_posts {
            self.create_post_page(&post.user(), post, Some(tag.clone()))
                .await?;
        }
        let page_path = format!("/tag/{tag}.vox");
        let table = self.get_tag_frontmatter(tag.clone(), tag_posts).await?;
        let page_contents = format!(
            "---
{table}
---
{{% if {tag}[0] %}}
{{% include tag.voxs posts = {tag} %}}
{{% else %}}
{{% include tag.voxs posts = \"\" %}}
{{% endif %}}
"
        );
        self.0.write_file(page_path, page_contents)?;
        Ok(())
    }

    pub async fn view_tag(&self, tag: String) -> miette::Result<String> {
        self.create_tag_page(tag.clone()).await?;
        self.render_and_get(format!("output/tag/{tag}"))
    }

    pub async fn view_tags(&self) -> miette::Result<String> {
        let node = NODE
            .get()
            .ok_or(miette::miette!("No running Oku node … "))?;
        let tags = node.all_tags(&node.all_posts().await);
        for tag in tags {
            self.create_tag_page(tag.clone()).await?;
        }
        self.render_and_get("output/tags")
    }
}
