use crate::okunet::items::post_item::PostItem;
use crate::widgets::okunet::net::Net;
use crate::window_util::get_view_stack_page_by_name;
use crate::NODE;
use gtk::glib;
use gtk::subclass::prelude::*;
use libadwaita::prelude::*;
use oku_core::database::posts::core::OkuPost;
use oku_core::fs::watch::OkuNetPostEvent;
use std::collections::HashSet;

pub fn post_compare(o1: &glib::object::Object, o2: &glib::object::Object) -> std::cmp::Ordering {
    let o1 = o1.clone().downcast::<PostItem>().ok();
    let o2 = o2.clone().downcast::<PostItem>().ok();
    match (o1, o2) {
        (Some(o1), Some(o2)) => o2.timestamp().cmp(&o1.timestamp()),
        _ => std::cmp::Ordering::Equal,
    }
}

pub fn is_post(o: &glib::object::Object, post: &OkuPost) -> bool {
    let o = o.clone().downcast::<PostItem>().ok();
    match o {
        None => false,
        Some(o) => {
            o.author_id() == oku_core::fs::util::fmt(post.entry.author())
                && o.url() == post.note.url.to_string()
                && o.timestamp() == post.entry.timestamp()
                && o.body() == post.note.body
                && o.title() == post.note.title
                && o.tags().into_iter().collect::<HashSet<String>>() == post.note.tags
        }
    }
}

impl Net {
    /// Mutate the post list state based on what the Oku node emits.
    /// Returns if anything changed.
    pub fn handle_post_event(&self, event: &OkuNetPostEvent) -> bool {
        let posts_store = self.posts_store();
        let old_store = posts_store.snapshot();
        match event {
            OkuNetPostEvent::Written(post) => {
                posts_store.insert_sorted(&PostItem::new(post), post_compare);
            }
            OkuNetPostEvent::Deleted(post) => {
                posts_store.retain(|x| !is_post(x, post));
            }
            OkuNetPostEvent::Synced(post) => {
                let post_index = posts_store.find_with_equal_func(|o| is_post(o, post));
                let post_item = post_index
                    .and_then(|x| posts_store.item(x).map(|y| y.downcast::<PostItem>().ok()))
                    .flatten();
                match post_item {
                    None => {
                        posts_store.insert_sorted(&PostItem::new(post), post_compare);
                    }
                    Some(post_item) => {
                        post_item.update(post.clone());
                    }
                }
            }
            OkuNetPostEvent::Initialised => (),
        };
        self.imp().posts_initialised.get() && old_store != posts_store.snapshot()
    }

    pub async fn setup_posts(&self) {
        // Prevent interaction with posts while we're setting up
        if let Some(home_page) =
            get_view_stack_page_by_name("home".to_string(), &self.imp().view_stack)
        {
            home_page.child().set_sensitive(false);
        }
        // Add all posts
        if let Some(node) = NODE.get() {
            let posts = node.all_posts().await;
            let ctx = glib::MainContext::default();
            let this = self.clone();
            ctx.invoke(move || {
                let posts_store = this.posts_store();
                for post in posts.iter() {
                    posts_store.insert_sorted(&PostItem::new(post), post_compare);
                }
            });
        }
        // Allow interaction again
        if let Some(home_page) =
            get_view_stack_page_by_name("home".to_string(), &self.imp().view_stack)
        {
            home_page.child().set_sensitive(true);
        }
    }
}
