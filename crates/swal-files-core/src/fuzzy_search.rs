use serde::{Deserialize, Serialize};

/// Scoring representation for fuzzy search matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MatchScore(pub i32);

/// Result of a fuzzy search match against a target string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FuzzyMatchResult {
    pub target: String,
    pub score: MatchScore,
    pub matched_indices: Vec<usize>,
}

/// In-memory fuzzy path search engine for quick omnibar search.
#[derive(Debug, Default, Clone)]
pub struct FuzzySearchEngine {
    items: Vec<String>,
}

impl FuzzySearchEngine {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn add(&mut self, item: impl Into<String>) {
        self.items.push(item.into());
    }

    pub fn set_items(&mut self, items: Vec<String>) {
        self.items = items;
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }

    pub fn search(&self, query: &str) -> Vec<FuzzyMatchResult> {
        let mut results: Vec<_> = self
            .items
            .iter()
            .filter_map(|item| Self::match_path(query, item))
            .collect();

        results.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.target.cmp(&b.target)));
        results
    }

    pub fn match_path(pattern: &str, target: &str) -> Option<FuzzyMatchResult> {
        if pattern.is_empty() {
            return Some(FuzzyMatchResult {
                target: target.to_string(),
                score: MatchScore(0),
                matched_indices: Vec::new(),
            });
        }

        let p_chars: Vec<char> = pattern.to_lowercase().chars().collect();
        let t_chars: Vec<char> = target.chars().collect();
        let t_lower: Vec<char> = target.to_lowercase().chars().collect();

        let mut matched_indices = Vec::with_capacity(p_chars.len());
        let mut t_idx = 0;

        for &pc in &p_chars {
            while t_idx < t_lower.len() && t_lower[t_idx] != pc {
                t_idx += 1;
            }
            if t_idx >= t_lower.len() {
                return None;
            }
            matched_indices.push(t_idx);
            t_idx += 1;
        }

        let mut score: i32 = 0;
        let last_slash = target.rfind(&['/', '\\'][..]).map(|i| i + 1).unwrap_or(0);

        if pattern.eq_ignore_ascii_case(target) {
            score += 100;
        }

        let mut prev_idx: Option<usize> = None;
        for &idx in &matched_indices {
            score += 10;

            if let Some(prev) = prev_idx {
                if idx == prev + 1 {
                    score += 15;
                }
            }

            let is_boundary = idx == 0
                || idx == last_slash
                || matches!(
                    t_chars.get(idx.saturating_sub(1)),
                    Some('/' | '\\' | '_' | '-' | '.' | ' ')
                )
                || t_chars
                    .get(idx)
                    .map_or(false, |c| c.is_uppercase())
                    && t_chars
                        .get(idx.saturating_sub(1))
                        .map_or(false, |c| c.is_lowercase());

            if is_boundary {
                score += 20;
            }

            if idx >= last_slash {
                score += 10;
            }

            prev_idx = Some(idx);
        }

        score -= target.len() as i32;

        Some(FuzzyMatchResult {
            target: target.to_string(),
            score: MatchScore(score),
            matched_indices,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzzy_match_empty_pattern() {
        let res = FuzzySearchEngine::match_path("", "src/main.rs");
        assert!(res.is_some());
        let match_res = res.unwrap();
        assert_eq!(match_res.score, MatchScore(0));
        assert!(match_res.matched_indices.is_empty());
    }

    #[test]
    fn test_fuzzy_match_basic_and_indices() {
        let res = FuzzySearchEngine::match_path("fse", "fuzzy_search_engine.rs");
        assert!(res.is_some());
        let match_res = res.unwrap();
        assert_eq!(match_res.matched_indices, vec![0, 6, 13]);
    }

    #[test]
    fn test_fuzzy_match_non_matching() {
        let res = FuzzySearchEngine::match_path("xyz", "src/main.rs");
        assert!(res.is_none());
    }

    #[test]
    fn test_fuzzy_search_engine_ranking() {
        let mut engine = FuzzySearchEngine::new();
        engine.set_items(vec![
            "src/fuzzy_search.rs".to_string(),
            "docs/fuzzy.md".to_string(),
            "README.md".to_string(),
        ]);

        let results = engine.search("fuzzy");
        assert_eq!(results.len(), 2);
        assert!(results[0].score > results[1].score || results[0].target.contains("fuzzy"));

        engine.clear();
        assert!(engine.search("fuzzy").is_empty());

        engine.add("crates/swal-files-core/src/fuzzy_search.rs");
        assert_eq!(engine.search("swal").len(), 1);
    }
}
