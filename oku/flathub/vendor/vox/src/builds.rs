use crate::page::Page;
use ahash::AHashMap;
use chrono::Locale;
use daggy::{
    petgraph::{algo::toposort, Direction},
    stable_dag::StableDag,
    NodeIndex, Walker,
};
use liquid::{to_object, Object, Parser};
use liquid_core::to_value;
use miette::IntoDiagnostic;
use path_clean::PathClean;
use std::path::PathBuf;
use tracing::{debug, trace};

/// Information held in memory while performing a build.
#[derive(Clone, Default)]
pub struct Build {
    /// A Liquid template parser.
    pub template_parser: Parser,
    /// The Liquid contexts necessary to render templates in pages.
    pub contexts: Object,
    /// The locale information of the build, primarily used to render dates and times.
    pub locale: Locale,
    /// A directed acyclic graph (DAG) populated with pages and their children.
    pub dag: StableDag<Page, EdgeType>,
}

/// The type of edge in the DAG.
#[derive(PartialEq, Clone, Debug)]
pub enum EdgeType {
    /// An edge between a layout and its parent page.
    Layout,
    /// An edge between a page and its parent collection page.
    Collection,
}

impl Build {
    /// Get all descendants of a page in a DAG.
    ///
    /// # Arguments
    ///
    /// * `dag` - The DAG to search.
    ///
    /// * `root_index` - The index of the page in the DAG.
    ///
    /// # Returns
    ///
    /// A list of indices of all descendants of the page.
    pub fn get_descendants(
        dag: &StableDag<Page, EdgeType>,
        root_index: &NodeIndex,
    ) -> Vec<NodeIndex> {
        let mut descendants = Vec::new();
        let children = dag.children(*root_index).iter(dag).collect::<Vec<_>>();
        for child in children {
            descendants.push(child.1);
            let child_descendants = Build::get_descendants(dag, &child.1);
            descendants.extend(child_descendants);
        }
        descendants
    }

    /// Get all ancestors of a page in a DAG.
    ///
    /// # Arguments
    ///
    /// * `dag` - The DAG to search.
    ///
    /// * `root_index` - The index of the page in the DAG.
    ///
    /// # Returns
    ///
    /// A list of indices of all ancestors of the page.
    pub fn get_ancestors(dag: &StableDag<Page, EdgeType>, root_index: NodeIndex) -> Vec<NodeIndex> {
        let mut ancestors = Vec::new();
        let parents = dag.parents(root_index).iter(dag).collect::<Vec<_>>();
        for parent in parents {
            ancestors.push(parent.1);
            let parent_ancestors = Build::get_ancestors(dag, parent.1);
            ancestors.extend(parent_ancestors);
        }
        ancestors
    }

    /// Get all ancestors of a page in a DAG that are not layout pages.
    ///
    /// # Arguments
    ///
    /// * `dag` - The DAG to search.
    ///
    /// * `root_index` - The index of the page in the DAG.
    ///
    /// # Returns
    ///
    /// A list of indices of all ancestors of the page that are not layout pages.
    pub fn get_non_layout_ancestors(
        dag: &StableDag<Page, EdgeType>,
        root_index: NodeIndex,
    ) -> miette::Result<Vec<NodeIndex>> {
        let mut ancestors = Vec::new();
        let parents = dag.parents(root_index).iter(dag).collect::<Vec<_>>();
        for parent in parents {
            let parent_page = &dag.graph()[parent.1];
            if !parent_page.is_layout() {
                ancestors.push(parent.1);
            }
            let parent_ancestors = Build::get_non_layout_ancestors(dag, parent.1)?;
            ancestors.extend(parent_ancestors);
        }
        Ok(ancestors)
    }

    /// Insert the contexts of all ancestors of a layout page.
    /// Intended to be used when rendering a layout page.
    ///
    /// # Arguments
    ///
    /// * `root_index` - The index of the layout page in the DAG.
    ///
    /// * `root_contexts` - The contexts of the layout page.
    pub fn insert_layout_ancestor_contexts(
        &self,
        root_index: NodeIndex,
        root_contexts: &mut Object,
    ) -> miette::Result<()> {
        let mut layout_ancestor_contexts = Vec::new();
        let ancestors = Build::get_ancestors(&self.dag, root_index);
        for ancestor in ancestors {
            let ancestor_page = &self.dag.graph()[ancestor];
            let ancestor_object =
                liquid_core::Value::Object(to_object(&ancestor_page).into_diagnostic()?);
            if ancestor_page.is_layout() {
                let ancestor_object =
                    liquid_core::Value::Object(to_object(&ancestor_page).into_diagnostic()?);
                layout_ancestor_contexts.push(ancestor_object);
            } else {
                layout_ancestor_contexts.push(ancestor_object.clone());
                root_contexts.insert("page".into(), ancestor_object);
                break;
            }
        }
        root_contexts.insert(
            "layouts".into(),
            liquid_core::Value::Array(layout_ancestor_contexts),
        );
        Ok(())
    }

    /// Render all pages in the DAG.
    ///
    /// # Returns
    ///
    /// A list of all nodes that were rendered.
    pub fn render_all(&mut self) -> miette::Result<Vec<NodeIndex>> {
        trace!("Rendering all pages … ");
        let mut rendered_indices = Vec::new();
        let indices = toposort(&self.dag.graph(), None).unwrap_or_default();
        for index in indices {
            self.render_page(index, false, &mut rendered_indices)?;
        }
        Ok(rendered_indices)
    }

    /// Render a page.
    ///
    /// # Arguments
    ///
    /// * `root_index` - The index of the page in the DAG.
    ///
    /// * `recursive` - Whether to render descendants of the page.
    ///
    /// * `rendered_indices` - A list of all nodes that have already been rendered.
    ///
    /// # Returns
    ///
    /// A list of all nodes that were rendered.
    pub fn render_page(
        &mut self,
        root_index: NodeIndex,
        recursive: bool,
        rendered_indices: &mut Vec<NodeIndex>,
    ) -> miette::Result<()> {
        let root_page = self.dag.graph()[root_index].to_owned();
        let root_path: PathBuf = PathBuf::from(root_page.to_path_string()).clean();
        debug!("Rendering page: {:?}", root_path);
        let mut root_contexts = self.contexts.clone();
        if root_path.starts_with(PathBuf::from("layouts/")) {
            debug!("Page is a layout page … ");
            let layout_object =
                liquid_core::Value::Object(to_object(&root_page).into_diagnostic()?);
            root_contexts.insert("layout".into(), layout_object.clone());
            self.insert_layout_ancestor_contexts(root_index, &mut root_contexts)?;
        } else {
            debug!("Page is not a layout page … ");
            let page_object = liquid_core::Value::Object(to_object(&root_page).into_diagnostic()?);
            root_contexts.insert("page".into(), page_object.clone());
        }
        let mut collection_pages: AHashMap<String, Vec<NodeIndex>> = AHashMap::new();
        // Find all parent pages of the root page.
        let parents = self
            .dag
            .parents(root_index)
            .iter(&self.dag)
            .collect::<Vec<_>>();
        for parent in parents {
            let parent_page = &self.dag.graph()[parent.1];
            let edge = self.dag.edge_weight(parent.0).unwrap();
            match edge {
                EdgeType::Layout => {}
                // If the parent page is in a collection this page depends on, make note of it.
                EdgeType::Collection => {
                    let parent_path = parent_page.to_path_string();
                    let collection_names = parent_page.get_collections()?.unwrap();
                    debug!(
                        "Parent page ({:?}) is in collections: {:?}",
                        parent_path, collection_names
                    );
                    for collection_name in collection_names {
                        if collection_pages.contains_key(&collection_name) {
                            collection_pages
                                .get_mut(&collection_name)
                                .unwrap()
                                .push(parent.1);
                        } else {
                            collection_pages.insert(collection_name.clone(), vec![parent.1]);
                        }
                    }
                }
            }
        }
        // Add the collection pages to the root page's contexts.
        trace!("Adding any collections to page's contexts … ");
        for (collection_name, collection) in collection_pages.iter_mut() {
            let collection_pages: Vec<liquid::Object> = collection
                .iter()
                .map(|page_index| {
                    to_object(&self.dag.node_weight_mut(*page_index).unwrap().to_owned())
                        .into_diagnostic()
                        .unwrap()
                })
                .collect();
            let collection_object = to_value(&collection_pages).into_diagnostic()?;
            root_contexts.insert(collection_name.clone().into(), collection_object.clone());
        }
        let root_page = self.dag.node_weight_mut(root_index).unwrap();
        if root_page.render(&root_contexts, &self.template_parser)? {
            rendered_indices.push(root_index);
        }

        if recursive {
            let children = self
                .dag
                .children(root_index)
                .iter(&self.dag)
                .collect::<Vec<_>>();
            for child in children {
                self.render_page(child.1, recursive, rendered_indices)?;
            }
        }

        Ok(())
    }

    /// Find the indices of all pages in the DAG that have no parent pages.
    ///
    /// # Returns
    ///
    /// A list of indices.
    pub fn find_root_indices(&self) -> Vec<NodeIndex> {
        self.dag.graph().externals(Direction::Incoming).collect()
    }
}
