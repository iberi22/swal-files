use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents the type of line in a unified diff format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineType {
    Context,
    Addition,
    Deletion,
    Header,
}

/// A single line in a diff hunk, including old and new line numbers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffLine {
    pub line_type: LineType,
    pub content: String,
    pub old_line_number: Option<usize>,
    pub new_line_number: Option<usize>,
}

/// Metadata header for a diff hunk (`@@ -old_start,old_lines +new_start,new_lines @@ heading`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HunkHeader {
    pub old_start: usize,
    pub old_lines: usize,
    pub new_start: usize,
    pub new_lines: usize,
    pub heading: Option<String>,
}

/// A hunk within a file diff containing a header and a sequence of diff lines.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffHunk {
    pub header: HunkHeader,
    pub lines: Vec<DiffLine>,
}

/// Unified diff representation for a single file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileDiff {
    pub old_path: Option<String>,
    pub new_path: Option<String>,
    pub is_binary: bool,
    pub is_new_file: bool,
    pub is_deleted_file: bool,
    pub is_rename: bool,
    pub hunks: Vec<DiffHunk>,
    pub additions: usize,
    pub deletions: usize,
}

impl FileDiff {
    /// Returns the total number of line modifications (additions + deletions).
    pub fn total_changes(&self) -> usize {
        self.additions + self.deletions
    }

    /// Converts this unified file diff into a side-by-side diff format.
    pub fn to_side_by_side(&self) -> SideBySideDiff {
        let side_hunks = self.hunks.iter().map(convert_hunk_to_side_by_side).collect();

        SideBySideDiff {
            old_path: self.old_path.clone(),
            new_path: self.new_path.clone(),
            is_binary: self.is_binary,
            is_new_file: self.is_new_file,
            is_deleted_file: self.is_deleted_file,
            is_rename: self.is_rename,
            hunks: side_hunks,
            additions: self.additions,
            deletions: self.deletions,
        }
    }
}

/// Line change classification for side-by-side diff rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SideBySideLineType {
    Equal,
    Addition,
    Deletion,
    Modification,
}

/// A single side-by-side diff row containing left (old) and right (new) diff lines.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SideBySideLine {
    pub left: Option<DiffLine>,
    pub right: Option<DiffLine>,
    pub line_type: SideBySideLineType,
}

/// A side-by-side diff hunk holding side-by-side rows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SideBySideHunk {
    pub header: HunkHeader,
    pub lines: Vec<SideBySideLine>,
}

/// Side-by-side diff representation for a single file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SideBySideDiff {
    pub old_path: Option<String>,
    pub new_path: Option<String>,
    pub is_binary: bool,
    pub is_new_file: bool,
    pub is_deleted_file: bool,
    pub is_rename: bool,
    pub hunks: Vec<SideBySideHunk>,
    pub additions: usize,
    pub deletions: usize,
}

impl SideBySideDiff {
    /// Returns total additions and deletions in this diff.
    pub fn total_changes(&self) -> usize {
        self.additions + self.deletions
    }
}

/// Custom error type for Git diff parsing errors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiffError {
    ParseError(String),
    AsyncJoinError(String),
}

impl fmt::Display for DiffError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiffError::ParseError(msg) => write!(f, "Parse error: {msg}"),
            DiffError::AsyncJoinError(msg) => write!(f, "Async task join error: {msg}"),
        }
    }
}

impl std::error::Error for DiffError {}

/// High-performance Git diff parser supporting unified and side-by-side outputs.
pub struct GitDiffParser;

impl GitDiffParser {
    /// Parses a raw unified git diff string into a vector of [`FileDiff`] structs.
    pub fn parse(raw_diff: &str) -> Vec<FileDiff> {
        let lines: Vec<&str> = raw_diff.lines().collect();
        let mut file_diffs = Vec::new();

        let mut i = 0;
        while i < lines.len() {
            let line = lines[i];

            if line.starts_with("diff --git ") {
                let (file_diff, next_idx) = parse_file_diff(&lines, i);
                file_diffs.push(file_diff);
                i = next_idx;
            } else {
                i += 1;
            }
        }

        file_diffs
    }

    /// Parses a raw unified git diff string directly into side-by-side file diffs.
    pub fn parse_side_by_side(raw_diff: &str) -> Vec<SideBySideDiff> {
        Self::parse(raw_diff)
            .into_iter()
            .map(|fd| fd.to_side_by_side())
            .collect()
    }
}

/// Non-blocking async helper to parse unified git diffs using Tokio blocking threadpool.
pub async fn parse_git_diff_async(raw_diff: impl Into<String>) -> Result<Vec<FileDiff>, DiffError> {
    let raw = raw_diff.into();
    tokio::task::spawn_blocking(move || GitDiffParser::parse(&raw))
        .await
        .map_err(|e| DiffError::AsyncJoinError(e.to_string()))
}

/// Non-blocking async helper to parse side-by-side git diffs using Tokio blocking threadpool.
pub async fn parse_side_by_side_async(raw_diff: impl Into<String>) -> Result<Vec<SideBySideDiff>, DiffError> {
    let raw = raw_diff.into();
    tokio::task::spawn_blocking(move || GitDiffParser::parse_side_by_side(&raw))
        .await
        .map_err(|e| DiffError::AsyncJoinError(e.to_string()))
}

fn parse_file_diff(lines: &[&str], start_idx: usize) -> (FileDiff, usize) {
    let mut old_path = None;
    let mut new_path = None;
    let mut is_binary = false;
    let mut is_new_file = false;
    let mut is_deleted_file = false;
    let mut is_rename = false;
    let mut hunks = Vec::new();
    let mut additions = 0;
    let mut deletions = 0;

    // Parse header line: diff --git a/foo b/bar
    let header_line = lines[start_idx];
    if let Some(paths) = header_line.strip_prefix("diff --git ") {
        let parts = split_git_paths(paths);
        if let Some((old_p, new_p)) = parts {
            old_path = Some(old_p);
            new_path = Some(new_p);
        }
    }

    let mut i = start_idx + 1;
    while i < lines.len() {
        let line = lines[i];

        if line.starts_with("diff --git ") {
            break;
        }

        if line.starts_with("new file mode ") {
            is_new_file = true;
        } else if line.starts_with("deleted file mode ") {
            is_deleted_file = true;
        } else if line.starts_with("rename from ") {
            is_rename = true;
            old_path = Some(line.trim_start_matches("rename from ").trim().to_string());
        } else if line.starts_with("rename to ") {
            is_rename = true;
            new_path = Some(line.trim_start_matches("rename to ").trim().to_string());
        } else if line.starts_with("Binary files ") && line.ends_with(" differ") {
            is_binary = true;
        } else if line.starts_with("--- ") {
            let path_str = line.trim_start_matches("--- ").trim();
            old_path = if path_str == "/dev/null" {
                None
            } else {
                Some(strip_prefix_path(path_str))
            };
        } else if line.starts_with("+++ ") {
            let path_str = line.trim_start_matches("+++ ").trim();
            new_path = if path_str == "/dev/null" {
                None
            } else {
                Some(strip_prefix_path(path_str))
            };
        } else if line.starts_with("@@ ") {
            let (hunk, next_i) = parse_hunk(lines, i, &mut additions, &mut deletions);
            hunks.push(hunk);
            i = next_i;
            continue;
        }

        i += 1;
    }

    if is_new_file {
        old_path = None;
    }
    if is_deleted_file {
        new_path = None;
    }

    (
        FileDiff {
            old_path,
            new_path,
            is_binary,
            is_new_file,
            is_deleted_file,
            is_rename,
            hunks,
            additions,
            deletions,
        },
        i,
    )
}

fn parse_hunk(
    lines: &[&str],
    start_idx: usize,
    total_additions: &mut usize,
    total_deletions: &mut usize,
) -> (DiffHunk, usize) {
    let header_line = lines[start_idx];
    let header = parse_hunk_header(header_line);

    let mut curr_old = header.old_start;
    let mut curr_new = header.new_start;
    let mut hunk_lines = Vec::new();

    let mut i = start_idx + 1;
    while i < lines.len() {
        let line = lines[i];

        if line.starts_with("diff --git ") || line.starts_with("@@ ") {
            break;
        }

        if line.starts_with('+') {
            hunk_lines.push(DiffLine {
                line_type: LineType::Addition,
                content: line[1..].to_string(),
                old_line_number: None,
                new_line_number: Some(curr_new),
            });
            curr_new += 1;
            *total_additions += 1;
        } else if line.starts_with('-') {
            hunk_lines.push(DiffLine {
                line_type: LineType::Deletion,
                content: line[1..].to_string(),
                old_line_number: Some(curr_old),
                new_line_number: None,
            });
            curr_old += 1;
            *total_deletions += 1;
        } else if line.starts_with('\\') {
            hunk_lines.push(DiffLine {
                line_type: LineType::Header,
                content: line.to_string(),
                old_line_number: None,
                new_line_number: None,
            });
        } else if line.starts_with(' ') || line.is_empty() {
            let content = if line.starts_with(' ') {
                &line[1..]
            } else {
                line
            };
            hunk_lines.push(DiffLine {
                line_type: LineType::Context,
                content: content.to_string(),
                old_line_number: Some(curr_old),
                new_line_number: Some(curr_new),
            });
            curr_old += 1;
            curr_new += 1;
        } else {
            // Unrecognized line inside hunk, break out
            break;
        }

        i += 1;
    }

    (
        DiffHunk {
            header,
            lines: hunk_lines,
        },
        i,
    )
}

fn parse_hunk_header(line: &str) -> HunkHeader {
    // Format: @@ -old_start,old_lines +new_start,new_lines @@ heading
    let mut header = HunkHeader {
        old_start: 0,
        old_lines: 0,
        new_start: 0,
        new_lines: 0,
        heading: None,
    };

    let rest = line.trim_start_matches("@@ ");
    if let Some((range_part, heading_part)) = rest.split_once(" @@") {
        let heading = heading_part.trim();
        if !heading.is_empty() {
            header.heading = Some(heading.to_string());
        }

        let parts: Vec<&str> = range_part.split_whitespace().collect();
        if parts.len() >= 2 {
            let (old_start, old_lines) = parse_range_spec(parts[0], '-');
            let (new_start, new_lines) = parse_range_spec(parts[1], '+');

            header.old_start = old_start;
            header.old_lines = old_lines;
            header.new_start = new_start;
            header.new_lines = new_lines;
        }
    }

    header
}

fn parse_range_spec(spec: &str, prefix: char) -> (usize, usize) {
    let clean = spec.trim_start_matches(prefix);
    if let Some((start_str, count_str)) = clean.split_once(',') {
        let start = start_str.parse().unwrap_or(0);
        let count = count_str.parse().unwrap_or(0);
        (start, count)
    } else {
        let start = clean.parse().unwrap_or(0);
        let count = if start > 0 { 1 } else { 0 };
        (start, count)
    }
}

fn split_git_paths(raw: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = raw.split_whitespace().collect();
    if parts.len() >= 2 {
        let old_p = strip_prefix_path(parts[0]);
        let new_p = strip_prefix_path(parts[1]);
        Some((old_p, new_p))
    } else {
        None
    }
}

fn strip_prefix_path(path: &str) -> String {
    let path = path.trim_matches('"');
    if let Some(stripped) = path.strip_prefix("a/") {
        stripped.to_string()
    } else if let Some(stripped) = path.strip_prefix("b/") {
        stripped.to_string()
    } else {
        path.to_string()
    }
}

fn convert_hunk_to_side_by_side(hunk: &DiffHunk) -> SideBySideHunk {
    let mut side_lines = Vec::new();
    let mut del_block: Vec<DiffLine> = Vec::new();
    let mut add_block: Vec<DiffLine> = Vec::new();

    let flush_blocks = |del_block: &mut Vec<DiffLine>,
                        add_block: &mut Vec<DiffLine>,
                        side_lines: &mut Vec<SideBySideLine>| {
        let max_len = del_block.len().max(add_block.len());
        for i in 0..max_len {
            let left = del_block.get(i).cloned();
            let right = add_block.get(i).cloned();

            let line_type = match (&left, &right) {
                (Some(_), Some(_)) => SideBySideLineType::Modification,
                (Some(_), None) => SideBySideLineType::Deletion,
                (None, Some(_)) => SideBySideLineType::Addition,
                (None, None) => SideBySideLineType::Equal,
            };

            side_lines.push(SideBySideLine {
                left,
                right,
                line_type,
            });
        }
        del_block.clear();
        add_block.clear();
    };

    for line in &hunk.lines {
        match line.line_type {
            LineType::Deletion => {
                del_block.push(line.clone());
            }
            LineType::Addition => {
                add_block.push(line.clone());
            }
            LineType::Context | LineType::Header => {
                flush_blocks(&mut del_block, &mut add_block, &mut side_lines);
                side_lines.push(SideBySideLine {
                    left: Some(line.clone()),
                    right: Some(line.clone()),
                    line_type: SideBySideLineType::Equal,
                });
            }
        }
    }

    flush_blocks(&mut del_block, &mut add_block, &mut side_lines);

    SideBySideHunk {
        header: hunk.header.clone(),
        lines: side_lines,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_diff() {
        let diffs = GitDiffParser::parse("");
        assert!(diffs.is_empty());
    }

    #[test]
    fn test_single_file_unified_diff() {
        let sample = r#"diff --git a/src/main.rs b/src/main.rs
index 1234567..89abcde 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,5 +1,6 @@ fn main()
 fn main() {
-    println!("Hello World");
+    println!("Hello SWAL");
+    println!("New line");
 }
"#;

        let diffs = GitDiffParser::parse(sample);
        assert_eq!(diffs.len(), 1);

        let fd = &diffs[0];
        assert_eq!(fd.old_path.as_deref(), Some("src/main.rs"));
        assert_eq!(fd.new_path.as_deref(), Some("src/main.rs"));
        assert!(!fd.is_binary);
        assert!(!fd.is_new_file);
        assert!(!fd.is_deleted_file);
        assert_eq!(fd.additions, 2);
        assert_eq!(fd.deletions, 1);
        assert_eq!(fd.total_changes(), 3);

        assert_eq!(fd.hunks.len(), 1);
        let hunk = &fd.hunks[0];
        assert_eq!(hunk.header.old_start, 1);
        assert_eq!(hunk.header.old_lines, 5);
        assert_eq!(hunk.header.new_start, 1);
        assert_eq!(hunk.header.new_lines, 6);
        assert_eq!(hunk.header.heading.as_deref(), Some("fn main()"));

        assert_eq!(hunk.lines.len(), 5);
        assert_eq!(hunk.lines[0].line_type, LineType::Context);
        assert_eq!(hunk.lines[0].old_line_number, Some(1));
        assert_eq!(hunk.lines[0].new_line_number, Some(1));

        assert_eq!(hunk.lines[1].line_type, LineType::Deletion);
        assert_eq!(hunk.lines[1].content, "    println!(\"Hello World\");");
        assert_eq!(hunk.lines[1].old_line_number, Some(2));
        assert_eq!(hunk.lines[1].new_line_number, None);

        assert_eq!(hunk.lines[2].line_type, LineType::Addition);
        assert_eq!(hunk.lines[2].content, "    println!(\"Hello SWAL\");");
        assert_eq!(hunk.lines[2].old_line_number, None);
        assert_eq!(hunk.lines[2].new_line_number, Some(2));

        assert_eq!(hunk.lines[3].line_type, LineType::Addition);
        assert_eq!(hunk.lines[3].content, "    println!(\"New line\");");
        assert_eq!(hunk.lines[3].old_line_number, None);
        assert_eq!(hunk.lines[3].new_line_number, Some(3));

        assert_eq!(hunk.lines[4].line_type, LineType::Context);
        assert_eq!(hunk.lines[4].content, "}");
        assert_eq!(hunk.lines[4].old_line_number, Some(3));
        assert_eq!(hunk.lines[4].new_line_number, Some(4));
    }

    #[test]
    fn test_multiple_files_diff() {
        let sample = r#"diff --git a/file1.txt b/file1.txt
--- a/file1.txt
+++ b/file1.txt
@@ -1 +1 @@
-old
+new
diff --git a/file2.txt b/file2.txt
--- a/file2.txt
+++ b/file2.txt
@@ -1 +1 @@
-a
+b
"#;

        let diffs = GitDiffParser::parse(sample);
        assert_eq!(diffs.len(), 2);
        assert_eq!(diffs[0].old_path.as_deref(), Some("file1.txt"));
        assert_eq!(diffs[1].old_path.as_deref(), Some("file2.txt"));
    }

    #[test]
    fn test_new_and_deleted_file() {
        let sample_new = r#"diff --git a/created.txt b/created.txt
new file mode 100644
--- /dev/null
+++ b/created.txt
@@ -0,0 +1,2 @@
+Line 1
+Line 2
"#;

        let diffs = GitDiffParser::parse(sample_new);
        assert_eq!(diffs.len(), 1);
        let fd = &diffs[0];
        assert!(fd.is_new_file);
        assert_eq!(fd.old_path, None);
        assert_eq!(fd.new_path.as_deref(), Some("created.txt"));

        let sample_del = r#"diff --git a/deleted.txt b/deleted.txt
deleted file mode 100644
--- a/deleted.txt
+++ /dev/null
@@ -1,2 +0,0 @@
-Line 1
-Line 2
"#;

        let diffs_del = GitDiffParser::parse(sample_del);
        assert_eq!(diffs_del.len(), 1);
        let fd_del = &diffs_del[0];
        assert!(fd_del.is_deleted_file);
        assert_eq!(fd_del.old_path.as_deref(), Some("deleted.txt"));
        assert_eq!(fd_del.new_path, None);
    }

    #[test]
    fn test_binary_diff() {
        let sample = r#"diff --git a/image.png b/image.png
index 1234..5678 100644
Binary files a/image.png and b/image.png differ
"#;

        let diffs = GitDiffParser::parse(sample);
        assert_eq!(diffs.len(), 1);
        let fd = &diffs[0];
        assert!(fd.is_binary);
        assert_eq!(fd.old_path.as_deref(), Some("image.png"));
        assert_eq!(fd.new_path.as_deref(), Some("image.png"));
    }

    #[test]
    fn test_rename_file() {
        let sample = r#"diff --git a/old_name.txt b/new_name.txt
similarity index 100%
rename from old_name.txt
rename to new_name.txt
"#;

        let diffs = GitDiffParser::parse(sample);
        assert_eq!(diffs.len(), 1);
        let fd = &diffs[0];
        assert!(fd.is_rename);
        assert_eq!(fd.old_path.as_deref(), Some("old_name.txt"));
        assert_eq!(fd.new_path.as_deref(), Some("new_name.txt"));
    }

    #[test]
    fn test_side_by_side_conversion() {
        let sample = r#"diff --git a/test.rs b/test.rs
--- a/test.rs
+++ b/test.rs
@@ -1,3 +1,3 @@
 context
-old_1
-old_2
+new_1
+new_2
+new_3
"#;

        let sbs = GitDiffParser::parse_side_by_side(sample);
        assert_eq!(sbs.len(), 1);
        let sbs_file = &sbs[0];
        assert_eq!(sbs_file.total_changes(), 5);

        let hunk = &sbs_file.hunks[0];
        assert_eq!(hunk.lines.len(), 4);

        assert_eq!(hunk.lines[0].line_type, SideBySideLineType::Equal);
        assert_eq!(
            hunk.lines[0].left.as_ref().unwrap().content,
            "context"
        );

        assert_eq!(hunk.lines[1].line_type, SideBySideLineType::Modification);
        assert_eq!(
            hunk.lines[1].left.as_ref().unwrap().content,
            "old_1"
        );
        assert_eq!(
            hunk.lines[1].right.as_ref().unwrap().content,
            "new_1"
        );

        assert_eq!(hunk.lines[2].line_type, SideBySideLineType::Modification);
        assert_eq!(
            hunk.lines[2].left.as_ref().unwrap().content,
            "old_2"
        );
        assert_eq!(
            hunk.lines[2].right.as_ref().unwrap().content,
            "new_2"
        );

        assert_eq!(hunk.lines[3].line_type, SideBySideLineType::Addition);
        assert!(hunk.lines[3].left.is_none());
        assert_eq!(
            hunk.lines[3].right.as_ref().unwrap().content,
            "new_3"
        );
    }

    #[tokio::test]
    async fn test_async_parsing() {
        let sample = r#"diff --git a/foo.txt b/foo.txt
--- a/foo.txt
+++ b/foo.txt
@@ -1 +1 @@
-hello
+world
"#;

        let diffs = parse_git_diff_async(sample).await.unwrap();
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].additions, 1);
        assert_eq!(diffs[0].deletions, 1);

        let sbs = parse_side_by_side_async(sample).await.unwrap();
        assert_eq!(sbs.len(), 1);
        assert_eq!(sbs[0].hunks[0].lines[0].line_type, SideBySideLineType::Modification);
    }

    #[test]
    fn test_diff_error_display() {
        let err1 = DiffError::ParseError("invalid hunk".to_string());
        assert_eq!(format!("{err1}"), "Parse error: invalid hunk");

        let err2 = DiffError::AsyncJoinError("cancelled".to_string());
        assert_eq!(format!("{err2}"), "Async task join error: cancelled");
    }

    #[test]
    fn test_no_newline_at_eof() {
        let sample = r#"diff --git a/a.txt b/a.txt
--- a/a.txt
+++ b/a.txt
@@ -1 +1 @@
-foo
\ No newline at end of file
+bar
\ No newline at end of file
"#;

        let diffs = GitDiffParser::parse(sample);
        assert_eq!(diffs.len(), 1);
        let lines = &diffs[0].hunks[0].lines;
        assert_eq!(lines.len(), 4);
        assert_eq!(lines[1].line_type, LineType::Header);
        assert_eq!(lines[1].content, "\\ No newline at end of file");
    }
}
