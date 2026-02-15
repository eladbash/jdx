use anyhow::Result;
use directories::ProjectDirs;
use std::fs;
use std::path::PathBuf;

const MAX_HISTORY: usize = 500;

/// Persistent query history and bookmarks.
#[derive(Debug, Clone)]
pub struct History {
    /// List of past queries (most recent last)
    pub queries: Vec<String>,
    /// Named bookmarks: (label, query)
    pub bookmarks: Vec<(String, String)>,
    /// File path for persistence
    path: Option<PathBuf>,
}

impl History {
    /// Create a new history, loading from disk if available.
    pub fn load() -> Self {
        let path = data_dir().map(|d| d.join("history"));
        let mut history = History {
            queries: Vec::new(),
            bookmarks: Vec::new(),
            path: path.clone(),
        };

        if let Some(ref p) = path {
            if p.exists() {
                if let Ok(content) = fs::read_to_string(p) {
                    for line in content.lines() {
                        if let Some(query) = line.strip_prefix("Q:") {
                            history.queries.push(query.to_string());
                        } else if let Some(rest) = line.strip_prefix("B:") {
                            if let Some((label, query)) = rest.split_once('=') {
                                history
                                    .bookmarks
                                    .push((label.to_string(), query.to_string()));
                            }
                        }
                    }
                }
            }
        }

        history
    }

    /// Add a query to history.
    pub fn add_query(&mut self, query: &str) {
        let query = query.to_string();
        // Remove duplicates
        self.queries.retain(|q| q != &query);
        self.queries.push(query);
        // Cap at max
        if self.queries.len() > MAX_HISTORY {
            self.queries.drain(0..self.queries.len() - MAX_HISTORY);
        }
    }

    /// Search history for queries matching a pattern.
    pub fn search(&self, pattern: &str) -> Vec<&str> {
        self.queries
            .iter()
            .rev()
            .filter(|q| q.contains(pattern))
            .map(|q| q.as_str())
            .collect()
    }

    /// Add a bookmark.
    pub fn add_bookmark(&mut self, label: &str, query: &str) {
        // Remove existing bookmark with same label
        self.bookmarks.retain(|(l, _)| l != label);
        self.bookmarks.push((label.to_string(), query.to_string()));
    }

    /// Get all bookmarks.
    pub fn get_bookmarks(&self) -> &[(String, String)] {
        &self.bookmarks
    }

    /// Save history to disk.
    pub fn save(&self) -> Result<()> {
        if let Some(ref path) = self.path {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut content = String::new();
            for query in &self.queries {
                content.push_str(&format!("Q:{query}\n"));
            }
            for (label, query) in &self.bookmarks {
                content.push_str(&format!("B:{label}={query}\n"));
            }
            fs::write(path, content)?;
        }
        Ok(())
    }
}

fn data_dir() -> Option<PathBuf> {
    ProjectDirs::from("", "", "jdx").map(|dirs| dirs.data_dir().to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_search() {
        let mut history = History {
            queries: Vec::new(),
            bookmarks: Vec::new(),
            path: None,
        };
        history.add_query(".users[0].name");
        history.add_query(".users[1].email");
        history.add_query(".items.count");

        let results = history.search("users");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_dedup() {
        let mut history = History {
            queries: Vec::new(),
            bookmarks: Vec::new(),
            path: None,
        };
        history.add_query(".foo");
        history.add_query(".bar");
        history.add_query(".foo");
        assert_eq!(history.queries.len(), 2);
        assert_eq!(history.queries.last().unwrap(), ".foo");
    }

    #[test]
    fn test_max_history() {
        let mut history = History {
            queries: Vec::new(),
            bookmarks: Vec::new(),
            path: None,
        };
        for i in 0..600 {
            history.add_query(&format!(".query{i}"));
        }
        assert!(history.queries.len() <= MAX_HISTORY);
    }

    #[test]
    fn test_bookmarks() {
        let mut history = History {
            queries: Vec::new(),
            bookmarks: Vec::new(),
            path: None,
        };
        history.add_bookmark("users", ".users");
        history.add_bookmark("items", ".items[*]");
        assert_eq!(history.get_bookmarks().len(), 2);

        // Overwrite
        history.add_bookmark("users", ".users[0]");
        assert_eq!(history.get_bookmarks().len(), 2);
        assert_eq!(history.get_bookmarks()[1].1, ".users[0]");
    }
}
