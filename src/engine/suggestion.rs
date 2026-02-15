use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

/// A single autocomplete candidate with score and match positions.
#[derive(Debug, Clone)]
pub struct Candidate {
    /// The full text of the candidate (key name or index)
    pub text: String,
    /// Match score (higher is better). 0 means no match.
    pub score: i64,
    /// Indices into `text` where the match occurs (for highlight rendering)
    pub match_indices: Vec<usize>,
}

/// Suggestion engine using fuzzy matching.
pub struct Suggester {
    matcher: SkimMatcherV2,
}

impl Default for Suggester {
    fn default() -> Self {
        Self::new()
    }
}

impl Suggester {
    pub fn new() -> Self {
        Self {
            matcher: SkimMatcherV2::default(),
        }
    }

    /// Get ranked candidates from a list of available keys matching the given input.
    ///
    /// If `input` is empty, returns all keys with equal score.
    /// Otherwise, performs fuzzy matching and returns sorted results (best first).
    pub fn get_candidates(&self, keys: &[String], input: &str) -> Vec<Candidate> {
        if input.is_empty() {
            return keys
                .iter()
                .map(|k| Candidate {
                    text: k.clone(),
                    score: 0,
                    match_indices: vec![],
                })
                .collect();
        }

        let mut candidates: Vec<Candidate> = keys
            .iter()
            .filter_map(|key| {
                self.matcher
                    .fuzzy_indices(key, input)
                    .map(|(score, indices)| Candidate {
                        text: key.clone(),
                        score,
                        match_indices: indices,
                    })
            })
            .collect();

        // Sort by score descending (best match first)
        candidates.sort_by(|a, b| b.score.cmp(&a.score));
        candidates
    }

    /// Get the best completion suggestion (the remaining text to auto-fill).
    ///
    /// Returns `Some((completion, full_suggestion))` if there's a match,
    /// where `completion` is the text to append and `full_suggestion` is the full key.
    pub fn get_completion(&self, keys: &[String], input: &str) -> Option<(String, String)> {
        if input.is_empty() {
            return None;
        }

        // First try exact prefix match (most intuitive)
        let prefix_matches: Vec<&String> = keys.iter().filter(|k| k.starts_with(input)).collect();

        if prefix_matches.len() == 1 {
            let suggestion = prefix_matches[0];
            let completion = suggestion[input.len()..].to_string();
            return Some((completion, suggestion.clone()));
        }

        // If multiple prefix matches, find longest common prefix
        if prefix_matches.len() > 1 {
            let lcp = longest_common_prefix(&prefix_matches);
            if lcp.len() > input.len() {
                let completion = lcp[input.len()..].to_string();
                return Some((completion, lcp));
            }
        }

        // Fall back to fuzzy: use the best match
        let candidates = self.get_candidates(keys, input);
        if let Some(best) = candidates.first() {
            if best.text.starts_with(input) {
                let completion = best.text[input.len()..].to_string();
                return Some((completion, best.text.clone()));
            }
        }

        None
    }
}

/// Find the longest common prefix among a set of strings.
fn longest_common_prefix(strings: &[&String]) -> String {
    if strings.is_empty() {
        return String::new();
    }
    if strings.len() == 1 {
        return strings[0].clone();
    }

    let first = strings[0].as_bytes();
    let mut len = first.len();

    for s in &strings[1..] {
        len = len.min(s.len());
        for (i, byte) in s.as_bytes().iter().enumerate().take(len) {
            if *byte != first[i] {
                len = i;
                break;
            }
        }
    }

    strings[0][..len].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(strs: &[&str]) -> Vec<String> {
        strs.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn test_empty_input_returns_all() {
        let suggester = Suggester::new();
        let k = keys(&["name", "age", "email"]);
        let candidates = suggester.get_candidates(&k, "");
        assert_eq!(candidates.len(), 3);
    }

    #[test]
    fn test_exact_prefix_match() {
        let suggester = Suggester::new();
        let k = keys(&["name", "namespace", "age"]);
        let candidates = suggester.get_candidates(&k, "nam");
        assert!(candidates.len() >= 2);
        // Both "name" and "namespace" should match
        let texts: Vec<&str> = candidates.iter().map(|c| c.text.as_str()).collect();
        assert!(texts.contains(&"name"));
        assert!(texts.contains(&"namespace"));
    }

    #[test]
    fn test_fuzzy_match() {
        let suggester = Suggester::new();
        let k = keys(&["firstName", "lastName", "email"]);
        let candidates = suggester.get_candidates(&k, "fn");
        // "firstName" should match fuzzy "fn"
        assert!(
            candidates.iter().any(|c| c.text == "firstName"),
            "expected firstName to match 'fn'"
        );
    }

    #[test]
    fn test_no_match() {
        let suggester = Suggester::new();
        let k = keys(&["name", "age"]);
        let candidates = suggester.get_candidates(&k, "zzz");
        assert!(candidates.is_empty());
    }

    #[test]
    fn test_ranking_exact_beats_fuzzy() {
        let suggester = Suggester::new();
        let k = keys(&["age", "average", "aggregate"]);
        let candidates = suggester.get_candidates(&k, "age");
        assert!(!candidates.is_empty());
        // "age" (exact match) should be ranked first
        assert_eq!(candidates[0].text, "age");
    }

    #[test]
    fn test_completion_single_prefix() {
        let suggester = Suggester::new();
        let k = keys(&["username", "password"]);
        let result = suggester.get_completion(&k, "user");
        assert_eq!(result, Some(("name".into(), "username".into())));
    }

    #[test]
    fn test_completion_multiple_prefix_common() {
        let suggester = Suggester::new();
        let k = keys(&["name", "namespace"]);
        let result = suggester.get_completion(&k, "na");
        assert_eq!(result, Some(("me".into(), "name".into())));
    }

    #[test]
    fn test_completion_no_match() {
        let suggester = Suggester::new();
        let k = keys(&["name", "age"]);
        let result = suggester.get_completion(&k, "zzz");
        assert_eq!(result, None);
    }

    #[test]
    fn test_match_indices_present() {
        let suggester = Suggester::new();
        let k = keys(&["username"]);
        let candidates = suggester.get_candidates(&k, "user");
        assert!(!candidates.is_empty());
        assert!(!candidates[0].match_indices.is_empty());
    }

    #[test]
    fn test_longest_common_prefix() {
        let a = "namespace".to_string();
        let b = "name".to_string();
        let c = "named".to_string();
        let s = vec![&a, &b, &c];
        assert_eq!(longest_common_prefix(&s), "name");
    }

    #[test]
    fn test_longest_common_prefix_identical() {
        let a = "test".to_string();
        let b = "test".to_string();
        let s = vec![&a, &b];
        assert_eq!(longest_common_prefix(&s), "test");
    }

    #[test]
    fn test_longest_common_prefix_empty() {
        let result = longest_common_prefix(&[]);
        assert_eq!(result, "");
    }
}
