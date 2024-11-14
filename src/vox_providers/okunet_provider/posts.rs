use super::core::OkuNetProvider;
use crate::NODE;
use miette::IntoDiagnostic;
use oku_fs::{
    database::{OkuIdentity, OkuPost, OkuUser},
    fs::entry_key_to_path,
    iroh::docs::AuthorId,
};
use std::{collections::HashSet, path::PathBuf, str::FromStr};
use vox::provider::VoxProvider;

impl OkuNetProvider {
    pub async fn get_post_permalink(&self, post: &OkuPost) -> miette::Result<String> {
        let node = NODE
            .get()
            .ok_or(miette::miette!("No running Oku node … "))?;
        let author = match node
            .default_author()
            .await
            .map_err(|e| miette::miette!("{}", e))?
            == post.entry.author()
        {
            true => "me".to_string(),
            false => post.entry.author().to_string(),
        };
        let key_path = entry_key_to_path(post.entry.key())?;
        let relative_key_path = key_path.strip_prefix("/").into_diagnostic()?;
        let key_path_str = relative_key_path.to_string_lossy();
        let post_url = key_path_str.strip_suffix(".toml").unwrap_or(&key_path_str);
        let path: PathBuf = [&author, post_url].iter().collect();
        Ok(path.to_string_lossy().to_string())
    }
    pub fn get_post_path(&self, post: &OkuPost, tag: Option<String>) -> String {
        let post_id = {
            let post_id_bytes = [
                post.entry.author().as_bytes().to_vec(),
                post.entry.key().to_vec(),
            ];
            bs58::encode(post_id_bytes.concat()).into_string()
        };
        match tag {
            Some(tag) => format!("{}/{}.vox", tag, post_id),
            None => format!("{}/{}.vox", post.entry.author(), post_id),
        }
    }
    pub async fn get_post_frontmatter(
        &self,
        user: &OkuUser,
        post: &OkuPost,
    ) -> miette::Result<toml::Table> {
        let node = NODE
            .get()
            .ok_or(miette::miette!("No running Oku node … "))?;
        let page_permalink = self.get_post_permalink(post).await?;
        let post_date = toml::value::Datetime::from_str(
            &chrono::DateTime::from_timestamp_micros(
                post.entry.timestamp().try_into().unwrap_or(0),
            )
            .map(|x| x.to_rfc3339())
            .unwrap_or_default(),
        )
        .into_diagnostic()?;
        let author_identity = if let Some(identity) = user.identity.clone() {
            identity
        } else {
            OkuIdentity {
                name: user.author_id.to_string(),
                following: HashSet::new(),
                blocked: HashSet::new(),
            }
        };
        let mut table = toml::Table::new();
        table.insert("layout".into(), "post".into());
        table.insert("permalink".into(), page_permalink.into());
        table.insert("date".into(), post_date.into());
        table.insert("note_url".into(), post.note.url.to_string().into());
        table.insert("title".into(), post.note.title.clone().into());
        table.insert(
            "tags".into(),
            post.note
                .tags
                .clone()
                .into_iter()
                .collect::<Vec<_>>()
                .into(),
        );
        table.insert("author_id".into(), user.author_id.to_string().into());
        table.insert("by_me".into(), node.is_me(&user.author_id).await.into());
        table.insert(
            "author".into(),
            toml::Table::try_from(author_identity)
                .into_diagnostic()?
                .into(),
        );
        Ok(table)
    }
    pub async fn create_post_page(
        &self,
        user: &OkuUser,
        post: &OkuPost,
        tag: Option<String>,
    ) -> miette::Result<()> {
        let page_path = self.get_post_path(post, tag);
        let table = self.get_post_frontmatter(user, post).await?;
        let page_contents = format!(
            "---
{}
---
{{% markdown %}}
{{% raw %}}
{}
{{% endraw %}}
{{% endmarkdown %}}",
            table, post.note.body
        );
        self.0.write_file(page_path, page_contents)?;
        Ok(())
    }

    pub async fn view_post(&self, author_id: AuthorId, path: PathBuf) -> miette::Result<String> {
        let node = NODE
            .get()
            .ok_or(miette::miette!("No running Oku node … "))?;
        let post_path = format!(
            "{}.toml",
            path.to_string_lossy()
                .strip_suffix(".html")
                .unwrap_or(&path.to_string_lossy())
        );
        let user = node.get_or_fetch_user(author_id).await?;
        let post = node.get_or_fetch_post(author_id, post_path.into()).await?;
        self.create_post_page(&user, &post, None).await?;
        self.render_and_get(format!("output/{}", self.get_post_permalink(&post).await?))
    }

    pub async fn view_self_post(&self, path: PathBuf) -> miette::Result<String> {
        let node = NODE
            .get()
            .ok_or(miette::miette!("No running Oku node … "))?;
        let post_path = format!(
            "{}.toml",
            path.to_string_lossy()
                .strip_suffix(".html")
                .unwrap_or(&path.to_string_lossy())
        );
        let post = node.post(post_path.into()).await?;
        let me = node.user().await?;
        self.create_post_page(&me, &post, None).await?;
        self.render_and_get(format!("output/{}", self.get_post_permalink(&post).await?))
    }
}
