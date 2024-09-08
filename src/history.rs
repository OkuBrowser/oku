use crate::suggestion_item::SuggestionItem;
use crate::HISTORY_DIR;
use daggy::petgraph::dot::Dot;
use daggy::petgraph::graph::NodeIndex;
use daggy::petgraph::stable_graph::DefaultIx;
use daggy::petgraph::visit::IntoNodeReferences;
use daggy::stable_dag::StableDag;
use glob::glob;
use indicium::simple::Indexable;
use indicium::simple::SearchIndex;
use indicium::simple::SearchIndexBuilder;
use layout::backends::svg::SVGWriter;
use layout::gv::DotParser;
use layout::gv::GraphBuilder;
use layout::std_shapes::shapes::ShapeKind;
use miette::IntoDiagnostic;
use serde::Deserialize;
use serde::Serialize;
use std::cell::RefCell;
use std::cmp::Reverse;
use std::path::PathBuf;
use tracing::error;
use tracing::warn;
use uuid::Uuid;
use webkit2gtk::FaviconDatabase;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Eq, Ord)]
pub struct HistoryItem {
    pub(crate) uri: RefCell<String>,
    pub(crate) original_uri: RefCell<String>,
    pub(crate) title: RefCell<String>,
    pub(crate) timestamp: chrono::DateTime<chrono::Utc>,
}

impl HistoryItem {
    pub fn new(uri: String, original_uri: String, title: String) -> Self {
        Self {
            uri: RefCell::new(uri),
            original_uri: RefCell::new(original_uri),
            title: RefCell::new(title),
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn uri(&self) -> String {
        self.uri.borrow().to_owned()
    }

    pub fn original_uri(&self) -> String {
        self.original_uri.borrow().to_owned()
    }

    pub fn title(&self) -> String {
        self.title.borrow().to_owned()
    }

    pub fn set_uri(&self, uri: String) -> String {
        self.uri.replace(uri)
    }

    pub fn set_original_uri(&self, original_uri: String) -> String {
        self.original_uri.replace(original_uri)
    }

    pub fn set_title(&self, title: String) -> String {
        self.title.replace(title)
    }

    pub fn to_suggestion_item(&self, favicon_database: &FaviconDatabase) -> SuggestionItem {
        SuggestionItem::new(self.title(), self.uri(), &favicon_database)
    }
}

impl Indexable for HistoryItem {
    fn strings(&self) -> Vec<String> {
        vec![self.uri(), self.original_uri(), self.title()]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistorySession {
    pub(crate) graph: RefCell<StableDag<HistoryItem, ()>>,
    pub(crate) id: Uuid,
}

impl HistorySession {
    pub fn new() -> miette::Result<Self> {
        let _ = std::fs::create_dir_all(HISTORY_DIR.to_path_buf());
        let graph = RefCell::new(StableDag::new());
        let id = Uuid::new_v4();
        let session_file_path = HISTORY_DIR
            .to_path_buf()
            .join(format!("{}.oku-session", id));
        let history_session = Self { graph, id };
        let session_bytes = bincode::serialize(&history_session).into_diagnostic()?;
        std::fs::write(session_file_path, session_bytes).into_diagnostic()?;
        Ok(history_session)
    }

    pub fn save(&self) {
        let session_file_path = HISTORY_DIR
            .to_path_buf()
            .join(format!("{}.oku-session", self.id));
        match bincode::serialize(&self) {
            Ok(session_bytes) => match std::fs::write(session_file_path, session_bytes) {
                Ok(_) => (),
                Err(e) => error!("{}", e),
            },
            Err(e) => error!("{}", e),
        }
        // if self.graph.borrow().node_count() > 0 {
        //     self.visualise_dag()
        // }
    }

    pub fn find_or_add_uri(&self, uri: String) -> NodeIndex<DefaultIx> {
        if let Some(node_index) = self.find_uri(uri.clone()) {
            node_index
        } else {
            self.graph
                .borrow_mut()
                .add_node(HistoryItem::new(uri.clone(), uri, String::new()))
        }
    }

    pub fn find_uri(&self, original_uri: String) -> Option<NodeIndex<DefaultIx>> {
        if let Some(node_index) = self
            .graph
            .borrow()
            .node_references()
            .position(|x| x.1.original_uri() == original_uri)
        {
            if let Ok(node_index) = node_index.try_into() {
                Some(NodeIndex::new(node_index))
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn add_navigation(
        &self,
        old_uri: String,
        new_uri: String,
    ) -> Option<(NodeIndex<DefaultIx>, NodeIndex<DefaultIx>)> {
        if old_uri != new_uri {
            let old_uri_index = self.find_or_add_uri(old_uri);
            let new_uri_index = self.find_or_add_uri(new_uri);
            let _ = self
                .graph
                .borrow_mut()
                .update_edge(old_uri_index, new_uri_index, ());
            return Some((old_uri_index, new_uri_index));
        } else {
            None
        }
    }

    pub fn update_uri(
        &self,
        original_uri: String,
        updated_uri: Option<String>,
        updated_title: Option<String>,
    ) {
        if let Some(node_index) = self.find_uri(original_uri) {
            if let Some(node) = self.graph.borrow().node_weight(node_index) {
                if let Some(updated_uri) = updated_uri {
                    node.set_uri(updated_uri);
                }
                if let Some(updated_title) = updated_title {
                    node.set_title(updated_title);
                }
            }
        }
    }

    pub fn visualise_dag(&self) {
        let svg_file_path = HISTORY_DIR.to_path_buf().join(format!("{}.svg", self.id));
        let graph = self.graph.borrow().clone();
        let graphviz = Dot::with_attr_getters(
            &graph,
            &[
                daggy::petgraph::dot::Config::NodeNoLabel,
                daggy::petgraph::dot::Config::EdgeNoLabel,
            ],
            &|_graph, _edge| String::new(),
            &|_graph, node| {
                format!(
                    "label = \"{}\"",
                    if node.1.title().trim().is_empty() {
                        node.1.uri()
                    } else {
                        format!("{} ({})", node.1.title(), node.1.uri())
                    }
                )
            },
        );
        let mut parser = DotParser::new(&format!("{:?}", graphviz));
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
                    node.shape = ShapeKind::Box(label.clone());
                }
            }
            vg.do_it(false, false, false, &mut svg);
            let content = svg.finalize();
            std::fs::write(svg_file_path, content).unwrap();
        } else {
            warn!("Unable to visualise the DAG.")
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryManager {
    pub(crate) history_sessions: Vec<HistorySession>,
    pub(crate) current_session: RefCell<HistorySession>,
    #[serde(skip)]
    pub(crate) search_index: RefCell<SearchIndex<(Uuid, NodeIndex<DefaultIx>)>>,
}

impl HistoryManager {
    pub fn add_navigation(&self, old_uri: String, new_uri: String) {
        let current_session = self.get_current_session();
        match current_session.add_navigation(old_uri, new_uri) {
            Some((old_uri_index, new_uri_index)) => {
                let graph = current_session.graph.borrow();
                let old_history_item = graph.node_weight(old_uri_index);
                let new_history_item = graph.node_weight(new_uri_index);
                if let (Some(old_history_item), Some(new_history_item)) =
                    (old_history_item, new_history_item)
                {
                    let mut search_index = self.search_index.borrow_mut();
                    search_index.insert(&(current_session.id, old_uri_index), old_history_item);
                    search_index.insert(&(current_session.id, new_uri_index), new_history_item);
                }
            }
            None => (),
        }
    }

    pub fn new_search_index() -> RefCell<SearchIndex<(Uuid, NodeIndex<DefaultIx>)>> {
        RefCell::new(
            SearchIndexBuilder::default()
                .max_search_results(5)
                .search_type(indicium::simple::SearchType::Live)
                .build(),
        )
    }

    pub fn load_sessions_or_create() -> miette::Result<HistoryManager> {
        let search_index = Self::new_search_index();
        let current_session = RefCell::new(HistorySession::new().unwrap());
        current_session.borrow().save();
        let mut history_sessions: Vec<HistorySession> = vec![];
        let files: Vec<PathBuf> = glob(&format!(
            "{}/*.oku-session",
            HISTORY_DIR.to_path_buf().to_string_lossy()
        ))
        .unwrap()
        .filter_map(Result::ok)
        .collect();
        if files.len() == 0 {
            Ok(Self {
                history_sessions,
                current_session,
                search_index,
            })
        } else {
            for file in files {
                match std::fs::read(file.clone()) {
                    Ok(file_bytes) => match bincode::deserialize::<HistorySession>(&file_bytes) {
                        Ok(history_session) => {
                            if history_session.graph.borrow().node_count() == 0
                                && history_session.id != current_session.borrow().id
                            {
                                let _ = std::fs::remove_file(file);
                            } else {
                                history_sessions.push(history_session);
                            }
                        }
                        Err(e) => {
                            error!("{}", e)
                        }
                    },
                    Err(e) => {
                        error!("{}", e);
                    }
                }
            }
            if history_sessions.len() == 0 {
                Ok(Self {
                    history_sessions,
                    current_session,
                    search_index,
                })
            } else {
                let mut search_index_mut = search_index.borrow_mut();
                for history_session in history_sessions.iter() {
                    for (node_index, history_item) in
                        history_session.graph.borrow().node_references()
                    {
                        search_index_mut.insert(&(history_session.id, node_index), history_item);
                    }
                }
                drop(search_index_mut);
                Ok(Self {
                    history_sessions,
                    current_session,
                    search_index,
                })
            }
        }
    }

    pub fn get_current_session(&self) -> std::cell::Ref<'_, HistorySession> {
        self.current_session.borrow()
    }

    pub fn get_suggestions(
        &self,
        favicon_database: &FaviconDatabase,
        search: String,
    ) -> Vec<SuggestionItem> {
        if search.trim().is_empty() {
            return vec![];
        }
        let mut history_items = self
            .search_index
            .borrow()
            .search(&search)
            .iter()
            .map(|x| {
                let history_session = if x.0 == self.current_session.borrow().id {
                    &*self.current_session.borrow()
                } else {
                    self.history_sessions.iter().find(|y| y.id == x.0).unwrap()
                };
                let graph = history_session.graph.borrow();
                graph.node_weight(x.1).unwrap().clone()
            })
            .collect::<Vec<HistoryItem>>();
        history_items.sort_unstable_by_key(|x| (x.uri(), Reverse(x.timestamp)));
        history_items.dedup_by_key(|x| x.uri());
        history_items.sort_unstable_by_key(|x| (x.original_uri(), Reverse(x.timestamp)));
        history_items.dedup_by_key(|x| x.original_uri());
        history_items.truncate(5);
        history_items
            .iter()
            .map(|x| x.to_suggestion_item(favicon_database))
            .collect::<Vec<SuggestionItem>>()
    }
}
