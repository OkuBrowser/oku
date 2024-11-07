use super::core::OkuNetProvider;
use crate::NODE;
use oku_fs::{
    database::{OkuPost, OkuUser},
    iroh::docs::AuthorId,
};
use vox::provider::VoxProvider;

impl OkuNetProvider {
    pub async fn get_user_frontmatter(
        &self,
        user: &OkuUser,
        posts: Vec<OkuPost>,
    ) -> miette::Result<toml::Table> {
        let user_name = match &user.identity {
            Some(identity) => identity.name.clone(),
            None => user.author_id.to_string(),
        };
        let node = NODE
            .get()
            .ok_or(miette::miette!("No running Oku node … "))?;
        let mut following: Vec<_> = Vec::new();
        if let Some(identity) = user.identity.clone() {
            for followed_user in identity.following {
                let followed_user_information = node.get_or_fetch_user(followed_user).await?;
                let mut followed_user_table = toml::Table::new();
                followed_user_table.insert(
                    "id".into(),
                    followed_user_information.author_id.to_string().into(),
                );
                match followed_user_information.identity {
                    Some(discovered_identity) => {
                        followed_user_table.insert("name".into(), discovered_identity.name.into());
                        followed_user_table.insert(
                            "following".into(),
                            discovered_identity
                                .following
                                .into_iter()
                                .map(|x| x.to_string())
                                .collect::<Vec<_>>()
                                .into(),
                        );
                    }
                    None => {
                        followed_user_table.insert(
                            "name".into(),
                            followed_user_information.author_id.to_string().into(),
                        );
                        followed_user_table.insert("following".into(), Vec::<String>::new().into());
                    }
                };
                following.push(followed_user_table);
            }
        }
        let mut table = toml::Table::new();
        table.insert("layout".into(), "default".into());
        table.insert(
            "permalink".into(),
            format!("{}.html", user.author_id.to_string()).into(),
        );
        table.insert("title".into(), user_name.into());
        table.insert("author_id".into(), user.author_id.to_string().into());
        table.insert(
            "is_followed".into(),
            node.is_followed(&user.author_id).await.into(),
        );
        table.insert(
            "is_blocked".into(),
            node.is_blocked(&user.author_id).await.into(),
        );
        table.insert("is_me".into(), node.is_me(&user.author_id).await.into());
        if posts.len() > 0 {
            table.insert("depends".into(), vec![user.author_id.to_string()].into());
        } else {
            table.insert("empty".into(), Vec::<String>::new().into());
        }
        table.insert("following".into(), following.into());
        Ok(table)
    }
    pub async fn create_profile_page(
        &self,
        user: &OkuUser,
        posts: Option<Vec<OkuPost>>,
    ) -> miette::Result<()> {
        let user_posts = posts.unwrap_or(
            oku_fs::database::DATABASE
                .get_posts_by_author(user.author_id)
                .unwrap_or_default(),
        );
        for post in user_posts.iter() {
            self.create_post_page(user, post, None).await?;
        }
        let page_path = format!("{}.vox", user.author_id);
        let include_argument = if user_posts.len() > 0 {
            user.author_id.to_string()
        } else {
            "empty".into()
        };
        let table = self.get_user_frontmatter(user, user_posts).await?;
        let page_contents = format!(
            "---
{}
---
{{% include profile.voxs posts = {} %}}
",
            table, include_argument
        );
        self.0.write_file(page_path, page_contents)?;
        Ok(())
    }

    pub async fn view_user(&self, author_id: AuthorId) -> miette::Result<String> {
        let node = NODE
            .get()
            .ok_or(miette::miette!("No running Oku node … "))?;
        let user = node.get_or_fetch_user(author_id).await?;
        let posts = node.posts_from_user(&user).await?;
        self.create_profile_page(&user, Some(posts)).await?;
        self.render_and_get(format!("output/{}.html", user.author_id.to_string()))
    }

    pub async fn view_self(&self) -> miette::Result<String> {
        let node = NODE
            .get()
            .ok_or(miette::miette!("No running Oku node … "))?;
        let author_id = node
            .default_author()
            .await
            .map_err(|e| miette::miette!("{}", e))?;
        let posts = node.posts().await;
        let me = node.user().await?;
        self.create_profile_page(&me, posts).await?;
        self.render_and_get(format!("output/{}.html", author_id.to_string()))
    }
}
