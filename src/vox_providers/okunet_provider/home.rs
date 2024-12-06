use super::core::OkuNetProvider;
use crate::NODE;

impl OkuNetProvider {
    pub async fn view_home(&self) -> miette::Result<String> {
        let node = NODE
            .get()
            .ok_or(miette::miette!("No running Oku node … "))?;

        tokio::spawn(node.refresh_users());

        // Posts
        let me = node.user().await?;
        let my_posts = node.posts_from_user(&me).await.unwrap_or_default();
        let mut posts = oku_fs::database::core::DATABASE
            .get_posts()
            .unwrap_or_default();
        posts.extend(my_posts.into_iter());
        for post in posts.iter() {
            self.create_post_page(&post.user(), post, Some("posts".into()))
                .await?;
        }

        self.render_and_get("output/home")
    }
}
