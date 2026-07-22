use iroh_docs::AuthorId;
use jiff::{
    fmt::friendly::{Designator, Spacing, SpanPrinter},
    Timestamp,
};
use oku_core::{
    database::{posts::core::OkuPost, users::OkuUser},
    fs::OkuFs,
};
use rayon::slice::ParallelSliceMut;
use std::cmp::Reverse;

pub async fn print_profile(node: &OkuFs, profile: &OkuUser) -> miette::Result<()> {
    let display_name = &profile.identity.clone().map(|x| x.name);
    let following = &profile
        .identity
        .clone()
        .map(|x| x.following)
        .unwrap_or_default();
    let blocked = &profile
        .identity
        .clone()
        .map(|x| x.blocked)
        .unwrap_or_default();
    let mut following_names = Vec::new();
    let mut blocked_names = Vec::new();

    for author_id in following {
        following_names.push(name(node, author_id).await);
    }
    for author_id in blocked {
        blocked_names.push(name(node, author_id).await);
    }

    println!(
        "Author ID: {}\nDisplay name: {:?}\nFollowing: {:?}\nBlocked: {:?}\n",
        oku_core::fs::util::fmt(profile.author_id),
        display_name,
        following_names,
        blocked_names
    );

    let mut posts = node.posts_from_user(profile).await?;
    posts.par_sort_unstable_by_key(|x| Reverse(x.entry.timestamp()));
    for post_entry in posts {
        println!("➤ {}", post(&post_entry).await);
    }
    Ok(())
}

pub async fn name(node: &OkuFs, author_id: &AuthorId) -> String {
    let identity_name = node
        .get_or_fetch_user(author_id)
        .await
        .ok()
        .and_then(|user| user.identity.map(|identity| identity.name));
    match identity_name {
        Some(name) => name,
        None => oku_core::fs::util::fmt(author_id),
    }
}

pub fn user_name(user: &OkuUser) -> String {
    match &user.identity {
        Some(identity) => identity.name.to_owned(),
        None => oku_core::fs::util::fmt(user.author_id),
    }
}

pub async fn post(post: &OkuPost) -> String {
    let user = post.user();
    let timestamp_microseconds = post.entry.timestamp();
    let timestamp = Timestamp::from_microsecond(
        timestamp_microseconds
            .try_into()
            .unwrap_or(timestamp_microseconds as i64),
    )
    .unwrap_or(Timestamp::UNIX_EPOCH);
    let timestamp_string = jiff::fmt::rfc2822::DateTimePrinter::new()
        .timestamp_to_string(&timestamp)
        .unwrap_or(format!("{timestamp:.0}"));
    let unrounded_span = timestamp - Timestamp::now();
    let span = unrounded_span
        .round(
            jiff::SpanRound::new()
                .largest(jiff::Unit::Year)
                .smallest(jiff::Unit::Second),
        )
        .unwrap_or(unrounded_span);
    let timestamp_printer = SpanPrinter::new()
        .direction(jiff::fmt::friendly::Direction::Suffix)
        .precision(Some(0))
        .spacing(Spacing::BetweenUnitsAndDesignators)
        .comma_after_designator(true)
        .designator(Designator::Verbose);
    format!(
        "'{}' ({}) by {} (posted on {}, {}):\n{}\nTags: {:?}",
        post.note.title,
        post.note.url,
        user_name(&user),
        timestamp_string,
        timestamp_printer.span_to_string(&span),
        post.note.body,
        post.note.tags
    )
}
