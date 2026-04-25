use iroh_docs::AuthorId;
use jiff::{
    fmt::friendly::{Designator, Spacing, SpanPrinter},
    Timestamp,
};
use oku_core::{
    database::{posts::core::OkuPost, users::OkuUser},
    fs::OkuFs,
};

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
