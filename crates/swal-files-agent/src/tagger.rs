use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::fs;

/// Represents the visual color assigned to a tag.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TagColor {
    Blue,
    Green,
    Yellow,
    Red,
    Purple,
    Orange,
    Gray,
    Custom(String),
}

/// Semantic category for file classification.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TagCategory {
    Document,
    Code,
    Image,
    Audio,
    Video,
    Archive,
    Config,
    System,
    Media,
    Other(String),
}

/// Represents a semantic tag that can be attached to a file.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub color: TagColor,
    pub category: TagCategory,
}

impl Tag {
    pub fn new(id: impl Into<String>, name: impl Into<String>, color: TagColor, category: TagCategory) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            color,
            category,
        }
    }
}

/// Rule used by `AutoTagger` to automatically tag files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagRule {
    pub tag: Tag,
    pub extensions: Vec<String>,
    pub name_patterns: Vec<String>,
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
    pub keywords: Vec<String>,
}

impl TagRule {
    pub fn new(tag: Tag) -> Self {
        Self {
            tag,
            extensions: Vec::new(),
            name_patterns: Vec::new(),
            min_size: None,
            max_size: None,
            keywords: Vec::new(),
        }
    }

    pub fn with_extensions(mut self, exts: &[&str]) -> Self {
        self.extensions = exts.iter().map(|s| s.to_lowercase()).collect();
        self
    }

    pub fn with_name_patterns(mut self, patterns: &[&str]) -> Self {
        self.name_patterns = patterns.iter().map(|s| s.to_lowercase()).collect();
        self
    }

    pub fn with_size_range(mut self, min: Option<u64>, max: Option<u64>) -> Self {
        self.min_size = min;
        self.max_size = max;
        self
    }

    pub fn with_keywords(mut self, keywords: &[&str]) -> Self {
        self.keywords = keywords.iter().map(|s| s.to_lowercase()).collect();
        self
    }

    pub fn matches(&self, path: &Path, file_size: Option<u64>, content_snippet: Option<&str>) -> bool {
        // Extension check
        if !self.extensions.is_empty() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if self.extensions.iter().any(|e| e.eq_ignore_ascii_case(ext)) {
                    return true;
                }
            }
        }

        // Name pattern check
        if let Some(file_name) = path.file_name().and_then(|f| f.to_str()) {
            let name_lower = file_name.to_lowercase();
            for pattern in &self.name_patterns {
                if name_lower.contains(pattern) {
                    return true;
                }
            }
        }

        // Size check
        if let Some(size) = file_size {
            if let Some(min) = self.min_size {
                if size < min {
                    return false;
                }
            }
            if let Some(max) = self.max_size {
                if size > max {
                    return false;
                }
            }
            if self.min_size.is_some() || self.max_size.is_some() {
                return true;
            }
        }

        // Content keywords check
        if let Some(snippet) = content_snippet {
            let snippet_lower = snippet.to_lowercase();
            for kw in &self.keywords {
                if snippet_lower.contains(kw) {
                    return true;
                }
            }
        }

        false
    }
}

/// Represents a file with its assigned tags and confidence score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaggedFile {
    pub path: PathBuf,
    pub tags: Vec<Tag>,
    pub score: f32,
    pub file_size: u64,
}

/// Errors produced during tagging operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaggerError {
    FileNotFound(String),
    IoError(String),
    ParseError(String),
}

impl std::fmt::Display for TaggerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaggerError::FileNotFound(path) => write!(f, "File not found: {}", path),
            TaggerError::IoError(msg) => write!(f, "IO error: {}", msg),
            TaggerError::ParseError(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

impl std::error::Error for TaggerError {}

/// Autonomous semantic auto-tagging engine.
#[derive(Debug, Clone)]
pub struct AutoTagger {
    rules: Vec<TagRule>,
}

impl Default for AutoTagger {
    fn default() -> Self {
        Self::new()
    }
}

impl AutoTagger {
    pub fn new() -> Self {
        let mut tagger = Self { rules: Vec::new() };
        tagger.load_default_rules();
        tagger
    }

    pub fn with_rules(rules: Vec<TagRule>) -> Self {
        Self { rules }
    }

    pub fn add_rule(&mut self, rule: TagRule) {
        self.rules.push(rule);
    }

    fn load_default_rules(&mut self) {
        // Rust / Source Code
        self.add_rule(
            TagRule::new(Tag::new("code-rs", "Rust Code", TagColor::Orange, TagCategory::Code))
                .with_extensions(&["rs"]),
        );

        // General Code
        self.add_rule(
            TagRule::new(Tag::new("code-source", "Source Code", TagColor::Blue, TagCategory::Code))
                .with_extensions(&["py", "js", "ts", "jsx", "tsx", "c", "cpp", "h", "hpp", "go", "java", "kt", "rb", "php", "sh", "bash"]),
        );

        // Documents
        self.add_rule(
            TagRule::new(Tag::new("doc-markdown", "Markdown", TagColor::Purple, TagCategory::Document))
                .with_extensions(&["md", "markdown"]),
        );

        self.add_rule(
            TagRule::new(Tag::new("doc-pdf", "PDF Document", TagColor::Red, TagCategory::Document))
                .with_extensions(&["pdf"]),
        );

        self.add_rule(
            TagRule::new(Tag::new("doc-office", "Office Document", TagColor::Blue, TagCategory::Document))
                .with_extensions(&["docx", "doc", "xlsx", "xls", "pptx", "ppt", "odt"]),
        );

        // Images
        self.add_rule(
            TagRule::new(Tag::new("media-image", "Image", TagColor::Green, TagCategory::Image))
                .with_extensions(&["png", "jpg", "jpeg", "gif", "webp", "svg", "bmp", "ico", "tiff"]),
        );

        // Audio
        self.add_rule(
            TagRule::new(Tag::new("media-audio", "Audio", TagColor::Yellow, TagCategory::Audio))
                .with_extensions(&["mp3", "wav", "flac", "aac", "ogg", "m4a"]),
        );

        // Video
        self.add_rule(
            TagRule::new(Tag::new("media-video", "Video", TagColor::Red, TagCategory::Video))
                .with_extensions(&["mp4", "mkv", "avi", "mov", "webm", "flv"]),
        );

        // Archives
        self.add_rule(
            TagRule::new(Tag::new("archive", "Archive", TagColor::Gray, TagCategory::Archive))
                .with_extensions(&["zip", "tar", "gz", "bz2", "xz", "7z", "rar", "tgz"]),
        );

        // Configuration
        self.add_rule(
            TagRule::new(Tag::new("config", "Configuration", TagColor::Yellow, TagCategory::Config))
                .with_extensions(&["json", "toml", "yaml", "yml", "xml", "ini", "env"]),
        );

        // Hidden / System files
        self.add_rule(
            TagRule::new(Tag::new("system-dot", "Dotfile", TagColor::Gray, TagCategory::System))
                .with_name_patterns(&[".gitignore", ".env", ".bashrc", ".zshrc"]),
        );

        // Large Files (> 100 MB)
        self.add_rule(
            TagRule::new(Tag::new("large-file", "Large File", TagColor::Red, TagCategory::Other("Storage".into())))
                .with_size_range(Some(100 * 1024 * 1024), None),
        );
    }

    /// Tag a single file asynchronously using Tokio non-blocking file metadata inspection.
    pub async fn tag_file_async(&self, path: &Path) -> Result<TaggedFile, TaggerError> {
        let metadata = fs::metadata(path)
            .await
            .map_err(|e| TaggerError::IoError(format!("Failed to read metadata for {}: {}", path.display(), e)))?;

        let file_size = metadata.len();
        let mut matched_tags = Vec::new();
        let mut unique_tag_ids = HashSet::new();

        // Optionally read a small snippet for text/markdown files
        let snippet = if file_size < 64 * 1024 {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if matches!(ext.to_lowercase().as_str(), "txt" | "md" | "json" | "toml" | "rs" | "py" | "js") {
                    fs::read_to_string(path).await.ok().map(|s| s.chars().take(1024).collect::<String>())
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        for rule in &self.rules {
            if rule.matches(path, Some(file_size), snippet.as_deref()) {
                if unique_tag_ids.insert(rule.tag.id.clone()) {
                    matched_tags.push(rule.tag.clone());
                }
            }
        }

        let score = if matched_tags.is_empty() { 0.0 } else { 1.0 };

        Ok(TaggedFile {
            path: path.to_path_buf(),
            tags: matched_tags,
            score,
            file_size,
        })
    }

    /// Tag multiple paths concurrently using Tokio async tasks.
    pub async fn tag_paths_batch_async(&self, paths: &[PathBuf]) -> Vec<Result<TaggedFile, TaggerError>> {
        let self_arc = Arc::new(self.clone());
        let mut tasks = Vec::with_capacity(paths.len());

        for path in paths {
            let path_clone = path.clone();
            let tagger_clone = Arc::clone(&self_arc);
            tasks.push(tokio::spawn(async move {
                tagger_clone.tag_file_async(&path_clone).await
            }));
        }

        let mut results = Vec::with_capacity(tasks.len());
        for task in tasks {
            match task.await {
                Ok(res) => results.push(res),
                Err(e) => results.push(Err(TaggerError::IoError(format!("Task join error: {}", e)))),
            }
        }

        results
    }
}

/// Field to sort query results by.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortField {
    Name,
    Size,
    DateModified,
    Relevance,
}

/// Direction of sorting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortDirection {
    Ascending,
    Descending,
}

/// Filter criteria parsed from a natural language string.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedQuery {
    pub raw_query: String,
    pub text_search: Option<String>,
    pub extensions: Vec<String>,
    pub tags: Vec<String>,
    pub categories: Vec<TagCategory>,
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
    pub modified_within: Option<Duration>,
    pub exact_phrases: Vec<String>,
    pub excluded_terms: Vec<String>,
    pub sort_by: Option<SortField>,
    pub sort_direction: SortDirection,
}

impl ParsedQuery {
    pub fn matches(&self, metadata: &FileMetadata) -> bool {
        // Check extensions filter
        if !self.extensions.is_empty() {
            let ext = metadata.path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if !self.extensions.iter().any(|e| e.eq_ignore_ascii_case(ext)) {
                return false;
            }
        }

        // Check size filter
        if let Some(min_size) = self.min_size {
            if metadata.size < min_size {
                return false;
            }
        }

        if let Some(max_size) = self.max_size {
            if metadata.size > max_size {
                return false;
            }
        }

        // Check modification date filter
        if let Some(within) = self.modified_within {
            if let Some(mod_time) = metadata.modified {
                if let Ok(elapsed) = SystemTime::now().duration_since(mod_time) {
                    if elapsed > within {
                        return false;
                    }
                }
            }
        }

        // Check tags filter
        if !self.tags.is_empty() {
            let file_tag_names: HashSet<String> = metadata
                .tags
                .iter()
                .map(|t| t.name.to_lowercase())
                .chain(metadata.tags.iter().map(|t| t.id.to_lowercase()))
                .collect();

            if !self.tags.iter().any(|req_tag| file_tag_names.contains(&req_tag.to_lowercase())) {
                return false;
            }
        }

        // Check category filter
        if !self.categories.is_empty() {
            if !metadata.tags.iter().any(|t| self.categories.contains(&t.category)) {
                return false;
            }
        }

        // Check excluded terms
        let file_name = metadata
            .path
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("")
            .to_lowercase();

        for excluded in &self.excluded_terms {
            if file_name.contains(&excluded.to_lowercase()) {
                return false;
            }
        }

        // Check exact phrases
        for phrase in &self.exact_phrases {
            if !file_name.contains(&phrase.to_lowercase()) {
                if let Some(snippet) = &metadata.content_snippet {
                    if !snippet.to_lowercase().contains(&phrase.to_lowercase()) {
                        return false;
                    }
                } else {
                    return false;
                }
            }
        }

        // Check text search
        if let Some(text) = &self.text_search {
            let text_lower = text.to_lowercase();
            let matches_name = file_name.contains(&text_lower);
            let matches_content = metadata
                .content_snippet
                .as_ref()
                .map(|s| s.to_lowercase().contains(&text_lower))
                .unwrap_or(false);

            if !matches_name && !matches_content {
                return false;
            }
        }

        true
    }
}

/// Metadata structure used for evaluating parsed NL queries.
#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub path: PathBuf,
    pub size: u64,
    pub modified: Option<SystemTime>,
    pub tags: Vec<Tag>,
    pub content_snippet: Option<String>,
}

/// Parser for natural language queries (e.g., "pdf larger than 5MB modified this week").
pub struct NlQueryParser;

impl NlQueryParser {
    pub fn parse(query_str: &str) -> ParsedQuery {
        let mut extensions = Vec::new();
        let mut tags = Vec::new();
        let mut categories = Vec::new();
        let mut min_size = None;
        let mut max_size = None;
        let mut modified_within = None;
        let mut exact_phrases = Vec::new();
        let mut excluded_terms = Vec::new();
        let mut sort_by = None;
        let mut sort_direction = SortDirection::Ascending;
        let mut text_tokens = Vec::new();

        let raw = query_str.trim();

        // Extract exact phrases wrapped in double quotes
        let mut remaining = String::new();
        let mut in_quote = false;
        let mut current_phrase = String::new();

        for ch in raw.chars() {
            if ch == '"' {
                if in_quote {
                    if !current_phrase.is_empty() {
                        exact_phrases.push(current_phrase.clone());
                        current_phrase.clear();
                    }
                    in_quote = false;
                } else {
                    in_quote = true;
                }
            } else if in_quote {
                current_phrase.push(ch);
            } else {
                remaining.push(ch);
            }
        }

        let tokens: Vec<&str> = remaining.split_whitespace().collect();

        let mut i = 0;
        while i < tokens.len() {
            let token = tokens[i];
            let token_lower = token.to_lowercase();

            // Extension syntax: ext:pdf or type:pdf or .rs
            if let Some(ext) = token_lower.strip_prefix("ext:") {
                extensions.push(ext.trim_start_matches('.').to_string());
            } else if let Some(type_str) = token_lower.strip_prefix("type:") {
                match type_str {
                    "image" | "img" => categories.push(TagCategory::Image),
                    "code" => categories.push(TagCategory::Code),
                    "doc" | "document" => categories.push(TagCategory::Document),
                    "audio" | "music" => categories.push(TagCategory::Audio),
                    "video" => categories.push(TagCategory::Video),
                    "archive" | "zip" => categories.push(TagCategory::Archive),
                    "config" => categories.push(TagCategory::Config),
                    ext => extensions.push(ext.to_string()),
                }
            } else if let Some(tag_str) = token_lower.strip_prefix("tag:") {
                tags.push(tag_str.to_string());
            } else if token_lower.starts_with('#') && token_lower.len() > 1 {
                tags.push(token_lower[1..].to_string());
            } else if let Some(ex) = token_lower.strip_prefix('-') {
                if !ex.is_empty() {
                    excluded_terms.push(ex.to_string());
                }
            } else if let Some(size_str) = token_lower.strip_prefix("size>") {
                if let Some(bytes) = Self::parse_size(size_str) {
                    min_size = Some(bytes);
                }
            } else if let Some(size_str) = token_lower.strip_prefix("size<") {
                if let Some(bytes) = Self::parse_size(size_str) {
                    max_size = Some(bytes);
                }
            } else if token_lower == "larger" && i + 2 < tokens.len() && tokens[i + 1].to_lowercase() == "than" {
                if let Some(bytes) = Self::parse_size(tokens[i + 2]) {
                    min_size = Some(bytes);
                    i += 2;
                } else {
                    text_tokens.push(token);
                }
            } else if token_lower == "smaller" && i + 2 < tokens.len() && tokens[i + 1].to_lowercase() == "than" {
                if let Some(bytes) = Self::parse_size(tokens[i + 2]) {
                    max_size = Some(bytes);
                    i += 2;
                } else {
                    text_tokens.push(token);
                }
            } else if token_lower == "modified:" || token_lower == "created:" {
                if i + 1 < tokens.len() {
                    let when = tokens[i + 1].to_lowercase();
                    modified_within = Self::parse_timeframe(&when);
                    i += 1;
                }
            } else if token_lower == "today" {
                modified_within = Some(Duration::from_secs(24 * 3600));
            } else if token_lower == "yesterday" || token_lower == "this-week" || token_lower == "recent" {
                modified_within = Some(Duration::from_secs(7 * 24 * 3600));
            } else if let Some(sort_str) = token_lower.strip_prefix("sort:") {
                match sort_str {
                    "name" => sort_by = Some(SortField::Name),
                    "size" => sort_by = Some(SortField::Size),
                    "date" | "modified" => sort_by = Some(SortField::DateModified),
                    "relevance" => sort_by = Some(SortField::Relevance),
                    _ => {}
                }
            } else if let Some(order_str) = token_lower.strip_prefix("order:") {
                match order_str {
                    "asc" | "ascending" => sort_direction = SortDirection::Ascending,
                    "desc" | "descending" => sort_direction = SortDirection::Descending,
                    _ => {}
                }
            } else {
                text_tokens.push(token);
            }

            i += 1;
        }

        let text_search = if text_tokens.is_empty() {
            None
        } else {
            Some(text_tokens.join(" "))
        };

        ParsedQuery {
            raw_query: raw.to_string(),
            text_search,
            extensions,
            tags,
            categories,
            min_size,
            max_size,
            modified_within,
            exact_phrases,
            excluded_terms,
            sort_by,
            sort_direction,
        }
    }

    fn parse_size(s: &str) -> Option<u64> {
        let s = s.to_lowercase();
        if let Some(num) = s.strip_suffix("gb") {
            num.parse::<f64>().ok().map(|n| (n * 1024.0 * 1024.0 * 1024.0) as u64)
        } else if let Some(num) = s.strip_suffix("mb") {
            num.parse::<f64>().ok().map(|n| (n * 1024.0 * 1024.0) as u64)
        } else if let Some(num) = s.strip_suffix("kb") {
            num.parse::<f64>().ok().map(|n| (n * 1024.0) as u64)
        } else if let Some(num) = s.strip_suffix('b') {
            num.parse::<u64>().ok()
        } else {
            s.parse::<u64>().ok()
        }
    }

    fn parse_timeframe(s: &str) -> Option<Duration> {
        match s {
            "today" => Some(Duration::from_secs(24 * 3600)),
            "week" | "7days" => Some(Duration::from_secs(7 * 24 * 3600)),
            "month" | "30days" => Some(Duration::from_secs(30 * 24 * 3600)),
            "year" | "365days" => Some(Duration::from_secs(365 * 24 * 3600)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_auto_tagger_rust_file() {
        let tagger = AutoTagger::new();
        let temp_file = NamedTempFile::with_suffix(".rs").unwrap();
        let tagged = tagger.tag_file_async(temp_file.path()).await.unwrap();

        assert!(!tagged.tags.is_empty());
        assert!(tagged.tags.iter().any(|t| t.id == "code-rs"));
    }

    #[tokio::test]
    async fn test_auto_tagger_batch_paths() {
        let tagger = AutoTagger::new();
        let temp_file1 = NamedTempFile::with_suffix(".rs").unwrap();
        let temp_file2 = NamedTempFile::with_suffix(".png").unwrap();

        let paths = vec![temp_file1.path().to_path_buf(), temp_file2.path().to_path_buf()];
        let results = tagger.tag_paths_batch_async(&paths).await;

        assert_eq!(results.len(), 2);
        let tag1 = results[0].as_ref().unwrap();
        let tag2 = results[1].as_ref().unwrap();

        assert!(tag1.tags.iter().any(|t| t.category == TagCategory::Code));
        assert!(tag2.tags.iter().any(|t| t.category == TagCategory::Image));
    }

    #[test]
    fn test_nl_query_parser_basic() {
        let query = NlQueryParser::parse("ext:pdf larger than 5MB #work -draft \"annual report\"");

        assert_eq!(query.extensions, vec!["pdf"]);
        assert_eq!(query.min_size, Some(5 * 1024 * 1024));
        assert_eq!(query.tags, vec!["work"]);
        assert_eq!(query.excluded_terms, vec!["draft"]);
        assert_eq!(query.exact_phrases, vec!["annual report"]);
    }

    #[test]
    fn test_nl_query_parser_types_and_sort() {
        let query = NlQueryParser::parse("type:image size<1MB sort:size order:desc");

        assert_eq!(query.categories, vec![TagCategory::Image]);
        assert_eq!(query.max_size, Some(1024 * 1024));
        assert_eq!(query.sort_by, Some(SortField::Size));
        assert_eq!(query.sort_direction, SortDirection::Descending);
    }

    #[test]
    fn test_parsed_query_matching() {
        let query = NlQueryParser::parse("ext:rs #code -temp");

        let rust_tag = Tag::new("code-rs", "code", TagColor::Orange, TagCategory::Code);

        let match_meta = FileMetadata {
            path: PathBuf::from("main.rs"),
            size: 500,
            modified: Some(SystemTime::now()),
            tags: vec![rust_tag.clone()],
            content_snippet: Some("fn main() {}".into()),
        };

        assert!(query.matches(&match_meta));

        let non_match_meta = FileMetadata {
            path: PathBuf::from("main_temp.rs"),
            size: 500,
            modified: Some(SystemTime::now()),
            tags: vec![rust_tag],
            content_snippet: None,
        };

        assert!(!query.matches(&non_match_meta));
    }
}
