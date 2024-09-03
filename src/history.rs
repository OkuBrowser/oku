use crate::HISTORY_DIR;
use glob::glob;
use layout::backends::svg::SVGWriter;
use layout::gv::DotParser;
use layout::gv::GraphBuilder;
use layout::std_shapes::shapes::ShapeKind;
use miette::IntoDiagnostic;
use petgraph::dot::Dot;
use petgraph::graph::NodeIndex;
use petgraph::prelude::DiGraphMap;
use petgraph::prelude::StableDiGraph;
use petgraph::stable_graph::DefaultIx;
use serde::Deserialize;
use serde::Serialize;
use std::cell::RefCell;
use std::path::PathBuf;
use tracing::error;
use tracing::warn;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Eq, Hash, Ord)]
pub struct HistoryItem {
    pub(crate) uri: String,
    pub(crate) title: String,
}

impl HistoryItem {
    pub fn new(uri: String, title: String) -> Self {
        Self { uri, title }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistorySession {
    pub(crate) graph: RefCell<StableDiGraph<HistoryItem, ()>>,
    pub(crate) id: Uuid,
}

impl HistorySession {
    pub fn new() -> miette::Result<Self> {
        let _ = std::fs::create_dir_all(HISTORY_DIR.to_path_buf());
        let graph = RefCell::new(StableDiGraph::new());
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
        if self.graph.borrow().node_count() > 0 {
            self.visualise_dag()
        }
    }

    pub fn find_or_add_uri(&self, uri: String) -> NodeIndex<DefaultIx> {
        if let Some(node_index) = self.find_uri(uri.clone()) {
            node_index
        } else {
            self.graph
                .borrow_mut()
                .add_node(HistoryItem::new(uri, String::new()))
        }
    }

    pub fn find_uri(&self, uri: String) -> Option<NodeIndex<DefaultIx>> {
        if let Some(node_index) = self
            .graph
            .borrow()
            .node_weights()
            .position(|x| x.uri == uri)
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

    pub fn add_navigation(&self, old_uri: String, new_uri: String) {
        if old_uri != new_uri {
            let old_uri_index = self.find_or_add_uri(old_uri);
            let new_uri_index = self.find_or_add_uri(new_uri);
            self.graph
                .borrow_mut()
                .add_edge(old_uri_index, new_uri_index, ());
        }
    }

    pub fn visualise_dag(&self) {
        let svg_file_path = HISTORY_DIR.to_path_buf().join(format!("{}.svg", self.id));
        let graph = self.graph.borrow().clone();
        let graphviz = Dot::with_attr_getters(
            &graph,
            &[
                petgraph::dot::Config::NodeNoLabel,
                petgraph::dot::Config::EdgeNoLabel,
            ],
            &|_graph, _edge| String::new(),
            &|_graph, node| format!("label = \"{}\"", node.1.uri),
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
}

impl HistoryManager {
    pub fn load_sessions_or_create() -> miette::Result<HistoryManager> {
        let files: Vec<PathBuf> = glob(&format!(
            "{}/*.oku-session",
            HISTORY_DIR.to_path_buf().to_string_lossy()
        ))
        .unwrap()
        .filter_map(Result::ok)
        .collect();
        if files.len() == 0 {
            let new_session = HistorySession::new().unwrap();
            new_session.save();
            Ok(Self {
                history_sessions: vec![new_session],
            })
        } else {
            let mut history_sessions = Vec::new();
            for file in files {
                history_sessions.push(
                    bincode::deserialize(
                        std::fs::read_to_string(file).unwrap_or_default().as_bytes(),
                    )
                    .into_diagnostic()?,
                );
            }
            if history_sessions.len() == 0 {
                let new_session = HistorySession::new().unwrap();
                new_session.save();
                Ok(Self {
                    history_sessions: vec![new_session],
                })
            } else {
                Ok(Self { history_sessions })
            }
        }
    }

    pub fn get_current_session(&self) -> &HistorySession {
        self.history_sessions.last().unwrap()
    }
}
