use crate::builds::EdgeType;
use crate::date::{self, Date};
use crate::markdown_block::MarkdownBlock;
use crate::math_block::MathBlock;
use crate::{builds::Build, page::Page};
use ahash::{AHashMap, AHashSet, HashSet, HashSetExt};
use chrono::{Locale, Utc};
use daggy::petgraph::algo::toposort;
use daggy::petgraph::dot::{Config, Dot};
use daggy::Walker;
use daggy::{stable_dag::StableDag, NodeIndex};
use layout::backends::svg::SVGWriter;
use layout::core::color::Color;
use layout::gv::{DotParser, GraphBuilder};
use layout::std_shapes::shapes::ShapeKind;
use liquid::{object, Object};
use miette::IntoDiagnostic;
use path_clean::PathClean;
use std::path::Path;
use std::path::PathBuf;
use syntect::highlighting::ThemeSet;
use syntect::html::css_for_theme_with_class_style;
use ticky::Stopwatch;
use toml::Table;
use tracing::{debug, info, trace, warn};

/// The Vox crate version number.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// An implementation of the Vox build process.
pub trait VoxProvider {
    /// Read a file's contents as a string.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the file.
    ///
    /// # Returns
    ///
    /// The file's contents as a string.
    fn read_to_string(&self, path: impl AsRef<std::path::Path>) -> miette::Result<String>;

    /// Write data to a file.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the file.
    ///
    /// * `contents` - The bytes to be written.
    fn write_file(
        &self,
        path: impl AsRef<std::path::Path> + Clone,
        contents: impl AsRef<[u8]>,
    ) -> miette::Result<()>;

    /// Remove a file.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the file.
    fn remove_file(&self, path: impl AsRef<Path>) -> miette::Result<()>;

    /// List all Vox pages.
    ///
    /// # Returns
    ///
    /// A list of paths to Vox pages.
    fn list_vox_files(&self) -> miette::Result<Vec<PathBuf>>;

    /// List all Vox snippets.
    ///
    /// # Returns
    ///
    /// A list of paths to Vox snippets.
    fn list_snippets(&self) -> miette::Result<Vec<PathBuf>>;

    /// Create a Liquid parser.
    ///
    /// # Returns
    ///
    /// A Liquid parser.
    fn create_liquid_parser(&self) -> miette::Result<liquid::Parser> {
        liquid::ParserBuilder::with_stdlib()
            .tag(liquid_lib::jekyll::IncludeTag)
            .filter(liquid_lib::jekyll::ArrayToSentenceString)
            .filter(liquid_lib::jekyll::Pop)
            .filter(liquid_lib::jekyll::Push)
            .filter(liquid_lib::jekyll::Shift)
            .filter(liquid_lib::jekyll::Slugify)
            .filter(liquid_lib::jekyll::Unshift)
            .filter(liquid_lib::jekyll::Sort)
            .filter(liquid_lib::shopify::Pluralize)
            .filter(liquid_lib::extra::DateInTz)
            .block(MathBlock)
            .block(MarkdownBlock)
            .partials(liquid::partials::EagerCompiler::new(self.partial_source()))
            .build()
            .into_diagnostic()
    }

    /// Given a path and locale, get a page.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the page.
    ///
    /// * `locale` - The locale for date formatting.
    ///
    /// # Returns
    ///
    /// A Vox page.
    fn path_to_page(&self, path: &PathBuf, locale: &Locale) -> miette::Result<Page> {
        Page::new(&self.read_to_string(&path)?, path, locale)
    }

    /// Get the global Liquid context.
    ///
    /// # Returns
    ///
    /// The global Liquid context and detected locale.
    fn get_global_context(&self) -> miette::Result<(Object, Locale)> {
        let global_context = match self.read_to_string("global.toml") {
            Ok(global_file) => global_file.parse::<Table>().into_diagnostic()?,
            Err(_) => format!("locale = '{}'", date::default_locale_string())
                .parse::<Table>()
                .into_diagnostic()?,
        };
        let locale: String = global_context
            .get("locale")
            .unwrap_or(&toml::Value::String(date::default_locale_string()))
            .as_str()
            .unwrap_or(&date::default_locale_string())
            .to_string();
        let locale = date::locale_string_to_locale(&locale);
        let current_date = Date::chrono_to_date(&Utc::now(), &locale);
        Ok((
            object!({
                "global": global_context,
                "meta": {
                    "builder": "Vox",
                    "version": VERSION,
                    "date": current_date,
                }
            }),
            locale,
        ))
    }

    /// Upsert a page into a DAG.
    ///
    /// # Arguments
    ///
    /// * `entry` - The path to the page.
    ///
    /// * `layout_index` - The DAG index of the page's layout, if it has one.
    ///
    /// * `dag` - The DAG to upsert the page into.
    ///
    /// * `pages` - Mapping of paths to DAG indices.
    ///
    /// * `layouts` - Mapping of paths to a set of DAG indices.
    ///
    /// * `collection_dependents` - Mapping of collection names to a set of dependent pages.
    ///
    /// * `collection_members` - Mapping of collection names to a set of pages in said collection.
    ///
    /// * `locale` - The locale for date formatting.
    #[allow(clippy::too_many_arguments)]
    fn insert_or_update_page(
        &self,
        entry: &PathBuf,
        layout_index: &Option<NodeIndex>,
        dag: &mut StableDag<Page, EdgeType>,
        pages: &mut AHashMap<PathBuf, NodeIndex>,
        layouts: &mut AHashMap<PathBuf, HashSet<NodeIndex>>,
        collection_dependents: &mut AHashMap<String, HashSet<NodeIndex>>,
        collection_members: &mut AHashMap<String, HashSet<NodeIndex>>,
        locale: &Locale,
    ) -> miette::Result<()> {
        let entry = entry.clean();
        let (page, index) = if !Page::is_layout_path(&entry) {
            debug!("Inserting or updating page: {:?} … ", entry);
            let page = self.path_to_page(&entry, &locale)?;
            // If the page already exists in the DAG, update it. Otherwise, insert it.
            let index = if pages.contains_key(&entry) {
                debug!("Updating page: {:?} … ", entry);
                let index = pages[&entry];
                let node = dag.node_weight_mut(index).unwrap();
                *node = page.clone();
                index
            } else {
                debug!("Inserting page: {:?} … ", entry);
                let index = dag.add_node(page.clone());
                pages.insert(entry, index);
                index
            };
            (page, index)
        } else {
            debug!("Inserting layout: {:?} … ", entry);
            let index = layout_index.unwrap();
            let page = dag.graph()[layout_index.unwrap()].clone();
            (page, index)
        };

        // A page's parents are pages in the collections it depends on. Its layout is a child.
        let layout = page.layout;
        let collections = page.collections;
        let depends = page.depends;
        debug!("Layout used: {:?} … ", layout);
        debug!("Collections used: {:?} … ", depends);
        if let Some(layout) = layout {
            // Layouts are inserted multiple times, once for each page that uses them.
            let layout_path = PathBuf::from(format!("layouts/{}.vox", layout)).clean();
            let children = dag.children(index).iter(dag).collect::<Vec<_>>();
            // If this page is being updated, the old layout should be replaced with the current one in the DAG.
            let old_layout = children
                .iter()
                .find(|child| *dag.edge_weight(child.0).unwrap() == EdgeType::Layout);
            if let Some(old_layout) = old_layout {
                trace!("Removing old layout … ");
                dag.remove_node(old_layout.1);
            }
            debug!("Inserting layout: {:?} … ", layout_path);
            let layout_page = self.path_to_page(&layout_path, &locale)?;
            let layout_index = dag.add_child(index, EdgeType::Layout, layout_page);
            if let Some(layouts) = layouts.get_mut(&layout_path) {
                layouts.insert(layout_index.1);
            } else {
                let mut new_set = HashSet::new();
                new_set.insert(layout_index.1);
                layouts.insert(layout_path, new_set);
            }
        }
        if let Some(collections) = collections {
            for collection in collections {
                if let Some(collection_members) = collection_members.get_mut(&collection) {
                    collection_members.insert(index);
                } else {
                    let mut new_set = HashSet::new();
                    new_set.insert(index);
                    collection_members.insert(collection, new_set);
                }
            }
        }
        if let Some(depends) = depends {
            for collection in depends {
                if let Some(collection_dependents) = collection_dependents.get_mut(&collection) {
                    collection_dependents.insert(index);
                } else {
                    let mut new_set = HashSet::new();
                    new_set.insert(index);
                    collection_dependents.insert(collection, new_set);
                }
            }
        }

        Ok(())
    }

    /// Obtain the URL of a layout page.
    ///
    /// # Arguments
    ///
    /// * `layout_node_index` - The index of a layout page in a DAG.
    ///
    /// * `dag` - The DAG containing the layout page.
    ///
    /// # Returns
    ///
    /// The layout page's URL.
    fn get_layout_url(
        &self,
        layout_node_index: &NodeIndex,
        dag: &StableDag<Page, EdgeType>,
    ) -> Option<String> {
        let layout_node = &dag.graph()[*layout_node_index];
        if !layout_node.url.is_empty() {
            return Some(layout_node.url.to_string());
        }

        let parents = dag
            .parents(*layout_node_index)
            .iter(dag)
            .collect::<Vec<_>>();
        let mut result = String::new();
        for parent in parents {
            if *dag.edge_weight(parent.0).unwrap() != EdgeType::Layout {
                continue;
            }
            result = self.get_layout_url(&parent.1, dag)?;
        }
        if result.is_empty() {
            None
        } else {
            Some(result)
        }
    }

    /// Obtain the output path of a page.
    ///
    /// # Arguments
    ///
    /// * `page` - The page.
    ///
    /// * `page_index` - The index of the page in the build's DAG.
    ///
    /// * `build` - The build.
    ///
    /// # Returns
    ///
    /// The page's output path.
    fn get_output_path(
        &self,
        page: &Page,
        page_index: &NodeIndex,
        build: &Build,
    ) -> Option<String> {
        // If a page has no URL, it may be a layout.
        // Layouts contain rendered content but must be written using their parent's URL.

        if page.url.is_empty() {
            let layout_url = self.get_layout_url(page_index, &build.dag);
            layout_url.map(|layout_url| format!("output/{}", layout_url))
        } else if !page.url.is_empty() {
            Some(format!("output/{}", page.url))
        } else {
            None
        }
    }

    /// Generate stylesheets for syntax highlighting.
    fn generate_syntax_stylesheets(&self) -> miette::Result<()> {
        let css_path = PathBuf::from("output/css/");
        let dark_css_path = css_path.join("dark-code.css");
        let light_css_path = css_path.join("light-code.css");
        let code_css_path = css_path.join("code.css");

        let ts = ThemeSet::load_defaults();
        let dark_theme = &ts.themes["base16-ocean.dark"];
        let css_dark =
            css_for_theme_with_class_style(dark_theme, syntect::html::ClassStyle::Spaced)
                .into_diagnostic()?;
        self.write_file(dark_css_path, css_dark)?;

        let light_theme = &ts.themes["base16-ocean.light"];
        let css_light =
            css_for_theme_with_class_style(light_theme, syntect::html::ClassStyle::Spaced)
                .into_diagnostic()?;
        self.write_file(light_css_path, css_light)?;

        let css = r#"@import url("light-code.css") (prefers-color-scheme: light);@import url("dark-code.css") (prefers-color-scheme: dark);"#;
        self.write_file(code_css_path, css)?;
        Ok(())
    }

    /// Output a visualisation of a build's DAG.
    ///
    /// # Arguments
    ///
    /// * `build` - A Vox build.
    fn visualise_dag(&self, build: &Build) -> miette::Result<()> {
        let dag_graph = build.dag.graph();
        let dag_graphviz = Dot::with_attr_getters(
            dag_graph,
            &[Config::NodeNoLabel, Config::EdgeNoLabel],
            &|_graph, edge| format!("label = \"{:?}\"", edge.weight()),
            &|_graph, node| {
                let path = PathBuf::from(node.1.to_path_string()).clean();
                let label = path.to_string_lossy().to_string();
                format!("label = \"{}\"", label)
            },
        );
        let mut parser = DotParser::new(&format!("{:?}", dag_graphviz));
        let tree = parser.process();
        if let Ok(tree) = tree {
            let mut gb = GraphBuilder::new();
            gb.visit_graph(&tree);
            let mut vg = gb.get();
            let mut svg = SVGWriter::new();
            for node_handle in vg.iter_nodes() {
                let node = vg.element_mut(node_handle);
                let old_shape = node.shape.clone();
                if let ShapeKind::Circle(label) = old_shape {
                    if Page::is_layout_path(&label) {
                        node.look.fill_color = Some(Color::fast("#FFDFBA"));
                    } else {
                        match Page::get_collections_from_path(&label)? {
                            Some(_) => {
                                node.look.fill_color = Some(Color::fast("#DAFFBA"));
                            }
                            None => {
                                node.look.fill_color = Some(Color::fast("#BADAFF"));
                            }
                        }
                    }
                    node.shape = ShapeKind::Box(label);
                }
            }
            vg.do_it(false, false, false, &mut svg);
            let content = svg.finalize();
            self.write_file("output/dag.svg", content)?;
        } else {
            warn!("Unable to visualise the DAG.")
        }
        Ok(())
    }

    /// Perform an initial build of a Vox site.
    ///
    /// # Arguments
    ///
    /// * `template_parser` - A Liquid parser.
    ///
    /// * `contexts` - The Liquid contexts to render with.
    ///
    /// * `locale` - The locale for date formatting.
    ///
    /// * `dag` - The DAG representing the structure of the site.
    ///
    /// * `visualise_dag` - Whether or not to output a visualisation of the DAG.
    ///
    /// * `generate_syntax_css` - Whether or not to output a stylesheet for syntax highlighting.
    ///
    /// # Returns
    ///
    /// A list of rendered pages and the DAG of the finished Vox build.
    fn generate_site(
        &self,
        template_parser: &liquid::Parser,
        contexts: &liquid::Object,
        locale: &Locale,
        dag: &StableDag<Page, EdgeType>,
        visualise_dag: &bool,
        generate_syntax_css: &bool,
    ) -> miette::Result<(Vec<NodeIndex>, StableDag<Page, EdgeType>)> {
        let mut timer = Stopwatch::start_new();
        let mut build = Build {
            template_parser: template_parser.clone(),
            contexts: contexts.clone(),
            locale: locale.clone(),
            dag: dag.clone(),
        };
        let updated_pages = build.render_all()?;
        if *visualise_dag {
            self.visualise_dag(&build)?;
        }
        info!("{} pages were rendered … ", updated_pages.len());
        for updated_page_index in updated_pages.iter() {
            let updated_page = &build.dag.graph()[*updated_page_index];
            // If a page has no URL, it may be a layout.
            // Layouts contain rendered content but must be written using their parent's URL.
            let output_path = self.get_output_path(updated_page, updated_page_index, &build);
            match output_path {
                None => {
                    warn!("Page has no URL: {:#?} … ", updated_page.to_path_string());
                    continue;
                }
                Some(output_path) => {
                    info!(
                        "Writing `{}` to `{}` … ",
                        updated_page.to_path_string(),
                        output_path
                    );
                    self.write_file(output_path, &updated_page.rendered)?;
                }
            }
        }
        if *generate_syntax_css {
            self.generate_syntax_stylesheets()?;
        }
        timer.stop();
        info!(
            "Generated {} pages in {:.2} seconds … ",
            updated_pages.len(),
            timer.elapsed_s()
        );
        Ok((updated_pages, build.dag))
    }

    /// Perform the rendering pipeline after changes have been detected.
    ///
    /// # Arguments
    ///
    /// * `global_or_snippets_changed` - Whether the global context or any snippets have changed.
    ///
    /// * `parser` - A Liquid parser.
    ///
    /// * `visualise_dag` - Whether or not to output a visualisation of the DAG.
    ///
    /// * `generate_syntax_css` - Whether or not to output a stylesheet for syntax highlighting.
    ///
    /// * `old_dag` - The former DAG.
    ///
    /// * `old_pages` - Former mapping of paths to DAG indices.
    ///
    /// * `old_layouts` - Former mapping of paths to a set of DAG indices.
    ///
    /// # Returns
    ///
    /// The DAG of the new finished Vox build, a new mapping of paths to DAG indices, and a new mapping of paths to a set of DAG indices.
    #[allow(clippy::type_complexity)]
    fn incremental_regeneration(
        &self,
        global_or_snippets_changed: &bool,
        parser: &liquid::Parser,
        visualise_dag: &bool,
        generate_syntax_css: &bool,
        old_dag: &StableDag<Page, crate::builds::EdgeType>,
        old_pages: &AHashMap<PathBuf, NodeIndex>,
        old_layouts: &AHashMap<PathBuf, HashSet<NodeIndex>>,
    ) -> miette::Result<(
        StableDag<Page, crate::builds::EdgeType>,
        AHashMap<PathBuf, NodeIndex<u32>>,
        AHashMap<PathBuf, HashSet<NodeIndex>>,
    )> {
        let (mut new_dag, new_pages, new_layouts) = self.generate_dag()?;
        let (added_or_modified, removed, removed_output_paths) = self.get_dag_difference(
            &old_dag,
            &old_pages,
            &old_layouts,
            &new_dag,
            &new_pages,
            &new_layouts,
        )?;
        let pages_to_render = self.pages_to_render(
            &old_dag,
            &new_dag,
            &new_pages,
            &new_layouts,
            &global_or_snippets_changed,
            &added_or_modified,
            &removed,
        )?;
        self.merge_dags(
            &pages_to_render,
            old_dag,
            &mut new_dag,
            old_pages,
            &new_pages,
        )?;
        Ok((
            self.output_regenerated(
                visualise_dag,
                generate_syntax_css,
                parser,
                &removed_output_paths,
                &new_dag,
                &pages_to_render,
            )?,
            new_pages,
            new_layouts,
        ))
    }

    /// Fifth stage of the rendering pipeline.
    ///
    /// Render & output the appropriate pages.
    ///
    /// # Arguments
    ///
    /// * `visualise_dag` - Whether or not to output a visualisation of the DAG.
    ///
    /// * `generate_syntax_css` - Whether or not to output a stylesheet for syntax highlighting.
    ///
    /// * `parser` - A Liquid parser.
    ///
    /// * `removed_output_paths` - A set of paths pointing to removed output files.
    ///
    /// * `new_dag` - The current DAG to use in rendering.
    ///
    /// * `pages_to_render` - A set of pages needing to be rendered.
    ///
    /// # Returns
    ///
    /// The DAG of the new finished Vox build.
    fn output_regenerated(
        &self,
        visualise_dag: &bool,
        generate_syntax_css: &bool,
        parser: &liquid::Parser,
        removed_output_paths: &AHashSet<PathBuf>,
        new_dag: &StableDag<Page, crate::builds::EdgeType>,
        pages_to_render: &AHashSet<NodeIndex>,
    ) -> miette::Result<StableDag<Page, crate::builds::EdgeType>> {
        let global = self.get_global_context()?;
        info!("Rebuilding … ");
        let mut timer = Stopwatch::start_new();
        let mut build = Build {
            template_parser: parser.clone(),
            contexts: global.0,
            locale: global.1,
            dag: new_dag.clone(),
        };
        if *visualise_dag {
            self.visualise_dag(&build)?;
        }

        // Delete the output of removed pages.
        for removed_output_path in removed_output_paths {
            debug!("Removing {:?} … ", removed_output_path);
            self.remove_file(removed_output_path)?;
        }

        let mut rendered_pages = Vec::new();
        let render_order = toposort(&build.dag.graph(), None).unwrap_or_default();
        for page in render_order
            .iter()
            .filter(|page| pages_to_render.contains(page))
        {
            build.render_page(*page, false, &mut rendered_pages)?;
        }

        for updated_page_index in rendered_pages.iter() {
            let updated_page = &build.dag.graph()[*updated_page_index];
            let output_path = self.get_output_path(updated_page, updated_page_index, &build);
            match output_path {
                None => {
                    warn!("Page has no URL: {:#?} … ", updated_page.to_path_string());
                    continue;
                }
                Some(output_path) => {
                    info!(
                        "Writing `{}` to `{}` … ",
                        updated_page.to_path_string(),
                        output_path
                    );
                    self.write_file(output_path, &updated_page.rendered)?;
                }
            }
        }
        if *generate_syntax_css {
            self.generate_syntax_stylesheets()?;
        }
        timer.stop();
        info!(
            "Generated {} pages in {:.2} seconds … ",
            rendered_pages.len(),
            timer.elapsed_s()
        );
        Ok(build.dag)
    }

    /// Fourth stage of the rendering pipeline.
    ///
    /// Merge the DAGs.
    ///     - In the new DAG, replace all pages not needing rendering with their rendered counterparts from the old DAG.
    ///
    /// # Arguments
    ///
    /// * `pages_to_render` - A set of pages needing to be rendered.
    ///
    /// * `old_dag` - The former DAG.
    ///
    /// * `new_dag` - The current DAG that the former DAG is to be merged into.
    ///
    /// * `old_pages` - Former mapping of paths to DAG indices.
    ///
    /// * `new_pages` - New mapping of paths to DAG indices.
    fn merge_dags(
        &self,
        pages_to_render: &AHashSet<NodeIndex>,
        old_dag: &StableDag<Page, crate::builds::EdgeType>,
        new_dag: &mut StableDag<Page, crate::builds::EdgeType>,
        old_pages: &AHashMap<PathBuf, NodeIndex>,
        new_pages: &AHashMap<PathBuf, NodeIndex>,
    ) -> miette::Result<()> {
        for (page_path, page_index) in new_pages {
            if !pages_to_render.contains(page_index) {
                // Pages may be added, so it is necessary to check if the page already exists in the old DAG.
                if let Some(old_page) = old_dag.node_weight(old_pages[page_path]) {
                    let new_page = new_dag.node_weight_mut(*page_index).unwrap();
                    new_page.url.clone_from(&old_page.url);
                    new_page.rendered.clone_from(&old_page.rendered);
                }
            }
        }
        Ok(())
    }

    /// Third stage of the rendering pipeline.
    ///
    /// Compute which pages need to be rendered, noting their node IDs.
    ///     - All pages that were modified need to be re-rendered.
    ///         - Their descendants in the new DAG also need to be rendered.
    ///     - All pages that were added need to be rendered.
    ///         - Their descendants in the new DAG also need to be rendered.
    ///     - All pages that were removed need their descendants in the new DAG rendered.
    ///         - Their old output also needs to be deleted.
    ///
    /// # Arguments
    ///
    /// * `old_dag` - The former DAG.
    ///
    /// * `new_dag` - The new DAG.
    ///
    /// * `new_pages` - New mapping of paths to DAG indices.
    ///
    /// * `new_layouts` - New mapping of paths to a set of DAG indices.
    ///
    /// * `global_or_snippets_changed` - Whether the global context or any snippets have changed.
    ///
    /// * `added_or_modified` - A set of pages that were added or modified.
    ///
    /// * `removed` - A set of pages that were removed.
    ///
    /// # Returns
    ///
    /// A set of pages needing to be rendered.
    fn pages_to_render(
        &self,
        old_dag: &StableDag<Page, crate::builds::EdgeType>,
        new_dag: &StableDag<Page, crate::builds::EdgeType>,
        new_pages: &AHashMap<PathBuf, NodeIndex>,
        new_layouts: &AHashMap<PathBuf, HashSet<NodeIndex>>,
        global_or_snippets_changed: &bool,
        added_or_modified: &AHashSet<NodeIndex>,
        removed: &AHashSet<NodeIndex>,
    ) -> miette::Result<AHashSet<NodeIndex>> {
        let mut pages_to_render = added_or_modified.clone();
        // If the global context or snippets have changed, all pages need to be re-rendered.
        if *global_or_snippets_changed {
            pages_to_render.extend(new_pages.values());
            pages_to_render.extend(new_layouts.values().flatten());
        }
        for page_index in added_or_modified.iter() {
            let descendants = Build::get_descendants(new_dag, page_index);
            for descendant in descendants {
                pages_to_render.insert(descendant);
            }
        }
        for page_index in removed.iter() {
            let descendants = Build::get_descendants(old_dag, page_index);
            for descendant_page_index in descendants
                .iter()
                .filter_map(|descendant| {
                    old_dag.node_weight(*descendant).map(|x| x.to_path_string())
                })
                .map(PathBuf::from)
                .filter_map(|x| new_pages.get(&x))
            {
                pages_to_render.insert(*descendant_page_index);
            }
        }
        // Only the root pages need to be passed to the rendering code, as it will recursively render their descendants.
        for page_index in removed.iter() {
            let children = old_dag
                .children(*page_index)
                .iter(old_dag)
                .collect::<Vec<_>>();
            for child_page_index in children
                .iter()
                .filter_map(|child| old_dag.node_weight(child.1).map(|x| x.to_path_string()))
                .map(PathBuf::from)
                .filter_map(|x| new_pages.get(&x))
            {
                pages_to_render.insert(*child_page_index);
            }
        }
        Ok(pages_to_render)
    }

    /// Second stage of the rendering pipeline.
    ///
    /// Obtain the difference between the old and new DAGs; ie, calculate the set of added or modified nodes.
    ///     - A node is modified if it has the same label, but its page is different (not comparing `url` or `rendered`).
    ///         - If a node's page is the same (excluding `url` or `rendered`), it is unchanged.
    ///     - A node is added if its label appears in the new DAG, but not the old one.
    ///     - A node is removed if its label appears in the old DAG, but not the new one.
    ///
    /// # Arguments
    ///
    /// * `old_dag` - The former DAG.
    ///
    /// * `old_pages` - Former mapping of paths to DAG indices.
    ///
    /// * `old_layouts` - Former mapping of paths to a set of DAG indices.
    ///
    /// * `new_dag` - The new DAG.
    ///
    /// * `new_pages` - New mapping of paths to DAG indices.
    ///
    /// * `new_layouts` - New mapping of paths to a set of DAG indices.
    ///
    /// # Returns
    ///
    /// A set of pages that were added or modified, a set of pages that were removed, and a set of paths pointing to removed output files.
    fn get_dag_difference(
        &self,
        old_dag: &StableDag<Page, crate::builds::EdgeType>,
        old_pages: &AHashMap<PathBuf, NodeIndex>,
        old_layouts: &AHashMap<PathBuf, HashSet<NodeIndex>>,
        new_dag: &StableDag<Page, crate::builds::EdgeType>,
        new_pages: &AHashMap<PathBuf, NodeIndex>,
        new_layouts: &AHashMap<PathBuf, HashSet<NodeIndex>>,
    ) -> miette::Result<(AHashSet<NodeIndex>, AHashSet<NodeIndex>, AHashSet<PathBuf>)> {
        let mut old_dag_pages = AHashMap::new();
        for (page_path, page) in old_pages.iter().filter_map(|(page_path, page_index)| {
            old_dag.node_weight(*page_index).map(|x| (page_path, x))
        }) {
            old_dag_pages.insert(page_path, page);
        }
        let mut new_dag_pages = AHashMap::new();
        for (page_path, page) in new_pages.iter().filter_map(|(page_path, page_index)| {
            new_dag.node_weight(*page_index).map(|x| (page_path, x))
        }) {
            new_dag_pages.insert(page_path, page);
        }
        let mut added_or_modified = AHashSet::new();
        let mut removed = AHashSet::new();
        let mut removed_output_paths = AHashSet::new();
        for (page_path, new_page) in new_dag_pages.iter() {
            match old_dag_pages.get(page_path) {
                // If the page has been modified, its index is noted.
                Some(old_page) => {
                    if !new_page.is_equivalent(old_page) {
                        added_or_modified.insert(new_pages[*page_path]);
                    }
                }
                // If the page is new, its index is noted.
                None => {
                    added_or_modified.insert(new_pages[*page_path]);
                }
            }
        }
        // The ancestors of modified or added layouts are themselves modified or added.
        for (layout_path, new_layout_indices) in new_layouts {
            let new_layout = new_layout_indices
                .iter()
                .last()
                .and_then(|x| new_dag.node_weight(*x));
            match old_layouts.get(layout_path) {
                Some(old_layout_indices) => {
                    let old_layout = old_layout_indices
                        .iter()
                        .last()
                        .and_then(|x| old_dag.node_weight(*x));
                    // Layout has been modified.
                    if !matches!((new_layout, old_layout), (Some(new_layout), Some(old_layout)) if new_layout.is_equivalent(old_layout))
                    {
                        for new_layout_index in new_layout_indices {
                            let ancestors =
                                Build::get_non_layout_ancestors(new_dag, *new_layout_index)?;
                            for ancestor in ancestors {
                                added_or_modified.insert(ancestor);
                            }
                        }
                    }
                }
                None => {
                    // Layout is new.
                    for new_layout_index in new_layout_indices {
                        let ancestors =
                            Build::get_non_layout_ancestors(new_dag, *new_layout_index)?;
                        for ancestor in ancestors {
                            added_or_modified.insert(ancestor);
                        }
                    }
                }
            }
        }
        // The ancestors of removed layouts are modified.
        for (layout_path, old_layout_indices) in old_layouts {
            if new_layouts.get(layout_path).is_none() {
                for old_layout_index in old_layout_indices {
                    let ancestors = Build::get_non_layout_ancestors(old_dag, *old_layout_index)?;
                    let ancestor_paths = ancestors
                        .iter()
                        .map(|ancestor| {
                            PathBuf::from(old_dag.node_weight(*ancestor).unwrap().to_path_string())
                        })
                        .collect::<Vec<_>>();
                    for ancestor_path in ancestor_paths {
                        if let Some(ancestor_index) = new_pages.get(&ancestor_path) {
                            added_or_modified.insert(*ancestor_index);
                        }
                    }
                }
            }
        }
        for (page_path, _old_page) in old_dag_pages.iter() {
            if new_dag_pages.get(*page_path).is_none() {
                // If the page has been removed, its index is noted.
                removed.insert(old_pages[*page_path]);
                if let Some(old_page) = old_dag_pages.get(page_path) {
                    let output_path = if old_page.url.is_empty() {
                        let layout_url = self.get_layout_url(&old_pages[*page_path], old_dag);
                        layout_url.map(|layout_url| format!("output/{}", layout_url))
                    } else if !old_page.url.is_empty() {
                        Some(format!("output/{}", old_page.url))
                    } else {
                        None
                    };
                    match output_path {
                        None => {
                            warn!("Page has no URL: {:#?} … ", old_page.to_path_string());
                            continue;
                        }
                        Some(output_path) => {
                            removed_output_paths.insert(PathBuf::from(output_path));
                        }
                    }
                }
            }
        }
        Ok((added_or_modified, removed, removed_output_paths))
    }

    /// First stage of the rendering pipeline.
    ///
    /// Constructing a DAG.
    ///
    /// # Returns
    ///
    /// The new DAG, a mapping of paths to DAG indices, and a mapping of paths to a set of DAG indices.
    #[allow(clippy::type_complexity)]
    fn generate_dag(
        &self,
    ) -> miette::Result<(
        StableDag<Page, crate::builds::EdgeType>,
        AHashMap<PathBuf, NodeIndex>,
        AHashMap<PathBuf, HashSet<NodeIndex>>,
    )> {
        let global = self.get_global_context()?;
        let mut dag = StableDag::new();
        let mut pages: AHashMap<PathBuf, NodeIndex> = AHashMap::new();
        let mut layouts: AHashMap<PathBuf, HashSet<NodeIndex>> = AHashMap::new();
        let mut collection_dependents: AHashMap<String, HashSet<NodeIndex>> = AHashMap::new();
        let mut collection_members: AHashMap<String, HashSet<NodeIndex>> = AHashMap::new();

        // DAG construction.
        debug!("Constructing DAG … ");
        // In the event that a layout has collection parents, we do not want it duplicated, so we avoid inserting it at first.
        for entry in self
            .list_vox_files()?
            .into_iter()
            .filter(|x| !Page::is_layout_path(x))
        {
            self.insert_or_update_page(
                &entry,
                &None,
                &mut dag,
                &mut pages,
                &mut layouts,
                &mut collection_dependents,
                &mut collection_members,
                &global.1,
            )?;
        }
        // We update the layouts with their parents and children once all other pages have been inserted.
        for (layout_path, layout_indices) in layouts.clone() {
            for layout_index in layout_indices {
                self.insert_or_update_page(
                    &layout_path,
                    &Some(layout_index),
                    &mut dag,
                    &mut pages,
                    &mut layouts,
                    &mut collection_dependents,
                    &mut collection_members,
                    &global.1,
                )?;
            }
        }
        // We construct edges between collection members and dependents.
        for (collection, members) in collection_members {
            if let Some(dependents) = collection_dependents.get(&collection) {
                for member in members {
                    for dependent in dependents {
                        dag.add_edge(member, *dependent, EdgeType::Collection)
                            .into_diagnostic()?;
                    }
                }
            }
        }
        Ok((dag, pages, layouts))
    }

    /// Obtain a source of Liquid partials.
    ///
    /// # Returns
    ///
    /// A source of Liquid partials.
    fn partial_source(&self) -> liquid::partials::InMemorySource {
        let snippet_paths = self.list_snippets().unwrap_or_default();
        let snippets: Vec<_> = snippet_paths
            .clone()
            .into_iter()
            .filter_map(|x| self.read_to_string(x).ok())
            .collect();
        let snippet_names: Vec<_> = snippet_paths
            .clone()
            .into_iter()
            .filter_map(|x| x.file_name().map(|y| y.to_os_string()))
            .map(|x| x.to_string_lossy().to_string())
            .collect();
        let mut partial_source = liquid::partials::InMemorySource::new();
        for (name, snippet) in snippet_names.iter().zip(snippets.iter()) {
            partial_source.add(name, snippet);
        }
        partial_source
    }
}
