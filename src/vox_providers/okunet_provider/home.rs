use super::core::OkuNetProvider;
use crate::NODE;
use rayon::prelude::*;
use rayon::slice::ParallelSliceMut;
use std::cmp::Reverse;

impl OkuNetProvider {
    pub async fn view_home(&self) -> miette::Result<String> {
        let node = NODE
            .get()
            .ok_or(miette::miette!("No running Oku node … "))?;

        tokio::spawn(node.refresh_users());

        // Posts
        let mut posts = Vec::from_par_iter(node.all_posts().await);
        posts.par_sort_unstable_by_key(|x| Reverse(x.entry.timestamp()));
        for post in posts.iter() {
            self.create_post_page(&post.user(), post, Some("posts".into()))
                .await?;
        }

        self.render_and_get("output/home")
    }
}
