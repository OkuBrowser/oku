use miette::{Diagnostic, NamedSource};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
#[error("Frontmatter not found ({0}).\n{1}", src.name(), src.inner())]
#[diagnostic(
    code(page::frontmatter_not_found),
    url(docsrs),
    help("Please ensure that all pages have frontmatter before attempting builds.")
)]
/// Frontmatter not found.
pub struct FrontmatterNotFound {
    #[source_code]
    /// The page missing its frontmatter.
    pub src: NamedSource<String>,
}

#[derive(Error, Debug, Diagnostic)]
#[error("Page date not valid ({0}).\n{1}", src.name(), src.inner())]
#[diagnostic(
    code(page::date_not_valid),
    url(docsrs),
    help("Please ensure that all pages have their date specified correctly before attempting builds.")
)]
/// Page date not valid.
pub struct DateNotValid {
    #[source_code]
    /// The page with an invalid date.
    pub src: NamedSource<String>,
}

#[derive(Error, Debug, Diagnostic)]
#[error("Invalid list of dependent collections ({0}).\n{1}", src.name(), src.inner())]
#[diagnostic(
    code(page::invalid_collections_property),
    url(docsrs),
    help(
        "Please ensure that your `depends` property is a list of collections this page depends on."
    )
)]
/// Invalid list of dependent collections.
pub struct InvalidDependsProperty {
    #[source_code]
    /// The page with the invalid `depends` property.
    pub src: NamedSource<String>,
}
