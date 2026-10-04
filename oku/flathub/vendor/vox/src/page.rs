use crate::{
    date::Date,
    error::{DateNotValid, FrontmatterNotFound, InvalidDependsProperty},
};
use chrono::Locale;
use core::fmt;
use liquid::{Object, Parser};
use miette::IntoDiagnostic;
use miette::NamedSource;
use path_clean::PathClean;
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsString,
    path::{Component, PathBuf},
};
use toml::Table;

#[derive(PartialEq, Clone, Default, Debug, Serialize, Deserialize)]
/// Internal representation of a page.
pub struct Page {
    /// A page's contextual data, represented as TOML at the head of the file.
    pub data: Table,
    /// A page's contents following the frontmatter.
    pub content: String,
    /// Data representing the output path of a page.
    /// This is defined in a page's frontmatter.
    pub permalink: String,
    /// A page's date-time metadata, formatted per the RFC 3339 standard.
    /// This is defined in a page's frontmatter.
    pub date: Option<Date>,
    /// The collections a page belongs to.
    /// This is defined by a page's path, with one collection per path component, and collections including each successive path component as well.
    /// Example: `books/fantasy/page.vox` is in `books`, `fantasy`, and `books_fantasy`.
    pub collections: Option<Vec<String>>,
    /// The collections a page depends on.
    /// This is defined in a page's frontmatter.
    pub depends: Option<Vec<String>>,
    /// The layout a page uses.
    /// This is defined in a page's frontmatter.
    pub layout: Option<String>,
    /// Path to the page, not including the page itself.
    pub directory: String,
    /// The page's base filename.
    pub name: String,
    /// Whether or not a page is a layout.
    pub is_layout: bool,
    /// The output path of a file; a processed `permalink` value.
    pub url: String,
    /// The rendered content of a page.
    pub rendered: String,
}

impl fmt::Display for Page {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_path_string())
    }
}

impl Page {
    /// Determine if a page is a layout based on its path.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the page.
    ///
    /// # Returns
    ///
    /// Whether or not the page is a layout.
    pub fn is_layout_path(path: impl Into<PathBuf>) -> bool {
        path.into().clean().starts_with("layouts/")
    }

    /// Get the names of the collections a page belongs to based on its path.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the page.
    ///
    /// # Returns
    ///
    /// The names of the collections a page belongs to, or `None` if the page does not belong to a collection.
    pub fn get_collections_from_path(
        path: impl Into<PathBuf>,
    ) -> miette::Result<Option<Vec<String>>> {
        let path_components: Vec<_> = path
            .into()
            .clean()
            .components()
            .filter_map(|x| match x {
                Component::CurDir => None,
                Component::RootDir => None,
                Component::ParentDir => None,
                Component::Prefix(_) => None,
                Component::Normal(x) => Some(PathBuf::from(x)),
            })
            .filter(|x| x.extension().is_none())
            .collect();
        if let Some(first_path_component) = path_components.first() {
            if first_path_component.extension().is_some()
                || *first_path_component == PathBuf::from("layouts")
            {
                return Ok(None);
            }
        }
        let mut results = Vec::new();
        for path_component in path_components {
            let path_to_this_component: Vec<_> = path_component
                .components()
                .filter_map(|x| match x {
                    Component::CurDir => None,
                    Component::RootDir => None,
                    Component::ParentDir => None,
                    Component::Prefix(_) => None,
                    Component::Normal(x) => Some(x.to_string_lossy().to_string()),
                })
                .collect();
            results.push(path_to_this_component.join("_"));
            results.push(path_component.to_string_lossy().to_string())
        }
        results.dedup();
        Ok(Some(results))
    }

    /// Determine if two pages are equivalent despite their rendered content.
    ///
    /// # Arguments
    ///
    /// * `lhs` - The first page to compare.
    ///
    /// * `rhs` - The second page to compare.
    ///
    /// # Returns
    ///
    /// Whether or not the two pages are equivalent.
    pub fn are_equivalent(lhs: &Page, rhs: &Page) -> bool {
        lhs.data == rhs.data
            && lhs.content == rhs.content
            && lhs.permalink == rhs.permalink
            && lhs.date == rhs.date
            && lhs.collections == rhs.collections
            && lhs.depends == rhs.depends
            && lhs.layout == rhs.layout
            && lhs.directory == rhs.directory
            && lhs.name == rhs.name
            && lhs.is_layout == rhs.is_layout
    }

    /// Determine if a page is equivalent to another page aside from rendered content.
    ///
    /// # Arguments
    ///
    /// * `other` - The page to compare to.
    ///
    /// # Returns
    ///
    /// Whether or not the two pages are equivalent.
    pub fn is_equivalent(&self, other: &Page) -> bool {
        Self::are_equivalent(self, other)
    }

    /// Determine if a page is a layout.
    ///
    /// # Returns
    ///
    /// Whether or not the page is a layout.
    pub fn is_layout(&self) -> bool {
        Page::is_layout_path(&self.to_path_string())
    }

    /// Get the names of the collections a page belongs to.
    ///
    /// # Returns
    ///
    /// The names of the collections a page belongs to, or `None` if the page does not belong to a collection.
    pub fn get_collections(&self) -> miette::Result<Option<Vec<String>>> {
        Page::get_collections_from_path(self.to_path_string())
    }

    /// Renders a page's content and URL.
    ///
    /// # Arguments
    ///
    /// * `contexts` - The Liquid contexts to render with.
    ///
    /// * `parser` - The Liquid parser to render with.
    ///
    /// # Returns
    ///
    /// Whether or not the page changed when rendering.
    pub fn render(&mut self, contexts: &Object, parser: &Parser) -> miette::Result<bool> {
        let permalink_changed = self.render_url(contexts, parser)?;
        let rendered_content = parser
            .parse(&self.content)
            .into_diagnostic()?
            .render(contexts)
            .into_diagnostic()?;
        if !permalink_changed && rendered_content == self.rendered {
            return Ok(false);
        }
        self.rendered = rendered_content;
        Ok(true)
    }

    /// Render a page's URL from its permalink value.
    ///
    /// # Arguments
    ///
    /// * `contexts` - The Liquid contexts to render with.
    ///
    /// * `parser` - The Liquid parser to render with.
    ///
    /// # Returns
    ///
    /// Whether or not the page's URL changed when rendering.
    pub fn render_url(&mut self, contexts: &Object, parser: &Parser) -> miette::Result<bool> {
        let expanded_permalink = match self.permalink.as_str() {
            "date" => {
                "{{ page.collections.last }}/{{ page.date.year }}/{{ page.date.month }}/{{ page.date.day }}/{{ page.data.title }}.html".to_owned()
            }
            "pretty" => {
                "{{ page.collections.last }}/{{ page.date.year }}/{{ page.date.month }}/{{ page.date.day }}/{{ page.data.title }}/index.html".to_owned()
            }
            "ordinal" => {
                "{{ page.collections.last }}/{{ page.date.year }}/{{ page.date.y_day }}/{{ page.data.title }}.html"
                    .to_owned()
            }
            "weekdate" => {
                "{{ page.collections.last }}/{{ page.date.year }}/W{{ page.date.week }}/{{ page.date.short_day }}/{{ page.data.title }}.html".to_owned()
            }
            "none" => {
                "{{ page.collections.last }}/{{ page.data.title }}.html".to_owned()
            }
            _ => {
                self.permalink.to_owned()
            }
        };
        let rendered_permalink = parser
            .parse(&expanded_permalink)
            .into_diagnostic()?
            .render(contexts)
            .into_diagnostic()?;
        if rendered_permalink == self.url {
            return Ok(false);
        }
        self.url = rendered_permalink;
        Ok(true)
    }

    /// Separate a page's contents into the frontmatter and body.
    ///
    /// # Arguments
    ///
    /// * `contents` - The contents of the page.
    ///
    /// * `path` - The path to the page.
    ///
    /// # Returns
    ///
    /// A tuple where the first element is the frontmatter, and where the second element is the body.
    pub fn get_frontmatter_and_body<'a>(
        contents: &'a str,
        path: &'a PathBuf,
    ) -> miette::Result<(&'a str, &'a str)> {
        let start_of_frontmatter = contents
            .find("---\n")
            .ok_or(FrontmatterNotFound {
                src: NamedSource::new(path.to_string_lossy(), contents.to_string()),
            })
            .into_diagnostic()?;
        let end_of_frontmatter = start_of_frontmatter
            + 4
            + (&contents[start_of_frontmatter + 4..])
                .find("---\n")
                .ok_or(FrontmatterNotFound {
                    src: NamedSource::new(path.to_string_lossy(), contents.to_string()),
                })
                .into_diagnostic()?;
        let frontmatter = &contents[start_of_frontmatter + 4..end_of_frontmatter];
        let body = &contents[end_of_frontmatter + 4..];
        Ok((frontmatter, body))
    }

    /// Create a representation of a page in memory.
    ///
    /// # Arguments
    ///
    /// * `contents` - The contents of the page.
    ///
    /// * `path` - The path to the page.
    ///
    /// * `locale` - The locale used to render dates and times.
    ///
    /// # Returns
    ///
    /// An instance of a page.
    pub fn new(contents: &str, path: impl Into<PathBuf>, locale: &Locale) -> miette::Result<Page> {
        let path = path.into().clean();
        let (frontmatter, body) = Self::get_frontmatter_and_body(&contents, &path)?;
        let frontmatter_data = frontmatter.parse::<Table>().into_diagnostic()?;
        let date = if let Some(date) = frontmatter_data.get("date") {
            let date_value = date
                .as_datetime()
                .ok_or(DateNotValid {
                    src: NamedSource::new(path.to_string_lossy(), frontmatter.to_string()),
                })
                .into_diagnostic()?;
            Some(Date::value_to_date(date_value, locale))
        } else {
            None
        };
        let layout = frontmatter_data
            .get("layout")
            .map(|p| p.as_str().unwrap().to_string());
        let permalink = frontmatter_data
            .get("permalink")
            .map(|p| p.as_str().unwrap().to_string());
        let depends = match frontmatter_data.get("depends") {
            Some(depends) => Some(
                depends
                    .as_array()
                    .ok_or(InvalidDependsProperty {
                        src: NamedSource::new(path.to_string_lossy(), frontmatter.to_string()),
                    })
                    .into_diagnostic()?
                    .iter()
                    .map(|x| {
                        x.as_str()
                            .ok_or(InvalidDependsProperty {
                                src: NamedSource::new(
                                    path.to_string_lossy(),
                                    frontmatter.to_string(),
                                ),
                            })
                            .unwrap()
                            .to_string()
                    })
                    .collect(),
            ),
            None => None,
        };
        Ok(Page {
            data: frontmatter_data,
            content: body.to_string(),
            permalink: permalink.unwrap_or_default(),
            date,
            layout,
            depends,
            directory: path
                .parent()
                .unwrap_or(&PathBuf::new())
                .to_string_lossy()
                .to_string(),
            name: path
                .file_stem()
                .unwrap_or(&OsString::new())
                .to_string_lossy()
                .to_string(),
            collections: Page::get_collections_from_path(&path)?,
            is_layout: Page::is_layout_path(&path),
            url: String::new(),
            rendered: String::new(),
        })
    }

    /// Return the path to a page.
    ///
    /// # Returns
    ///
    /// The path to a page as a string.
    pub fn to_path_string(&self) -> String {
        format!("{}/{}.vox", self.directory, self.name)
    }
}
