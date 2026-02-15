/// Application modes — determines which widgets are active and how input is routed.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    /// Default: typing a dot-notation query to filter JSON
    #[default]
    Query,
    /// Tree view: navigating a collapsible tree with arrow keys
    Tree,
    /// AI panel: typing a natural language question
    Ai,
    /// Schema view: inspecting the shape of the data
    Schema,
    /// Help overlay: showing keybinding reference
    Help,
}

impl AppMode {
    /// Human-readable label for the status bar.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Query => "QUERY",
            Self::Tree => "TREE",
            Self::Ai => "AI",
            Self::Schema => "SCHEMA",
            Self::Help => "HELP",
        }
    }
}
