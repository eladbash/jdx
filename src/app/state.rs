use std::collections::HashSet;
use std::sync::mpsc;

use crate::ai::service::AiResponse;

/// Query-mode state: the dot-notation query, cursor, scroll, and autocomplete.
pub struct QueryState {
    /// Current query string
    pub text: String,
    /// Cursor position in the query
    pub cursor: usize,
    /// Vertical scroll offset for JSON view
    pub scroll: u16,
    /// Whether to show key-only mode
    pub key_mode: bool,
    /// Candidate popup visible
    pub show_candidates: bool,
    /// Currently selected candidate index
    pub candidate_idx: usize,
}

impl Default for QueryState {
    fn default() -> Self {
        Self {
            text: ".".into(),
            cursor: 1,
            scroll: 0,
            key_mode: false,
            show_candidates: false,
            candidate_idx: 0,
        }
    }
}

/// Tree-view mode state: expanded nodes, selection, scroll.
#[derive(Default)]
pub struct TreeState {
    /// Expanded paths
    pub expanded: HashSet<String>,
    /// Selected node index
    pub selected: usize,
    /// Scroll offset
    pub scroll: u16,
}

/// AI-mode state: input, response, loading, error.
#[derive(Default)]
pub struct AiState {
    /// AI input text
    pub input: String,
    /// Cursor position in the AI input
    pub cursor: usize,
    /// AI text answer
    pub response: Option<String>,
    /// AI suggested query (optional)
    pub suggested_query: Option<String>,
    /// AI loading state
    pub loading: bool,
    /// AI error
    pub error: Option<String>,
    /// AI panel scroll offset
    pub scroll: u16,
    /// Receiver for async AI results
    pub(crate) result_rx: Option<mpsc::Receiver<Result<AiResponse, String>>>,
}

/// Schema-view state.
#[derive(Default)]
pub struct SchemaState {
    /// Cached schema text
    pub text: Option<String>,
}
