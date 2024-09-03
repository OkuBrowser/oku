use crate::HISTORY_DIR;
use serde::Deserialize;
use serde::Serialize;
use std::cell::RefCell;
use tracing::error;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct HistoryItem {
    pub(crate) uri: String,
    pub(crate) title: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct HistorySession {
    pub(crate) history_items: Vec<HistoryItem>,
}
