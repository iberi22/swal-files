use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::Path;
use std::sync::Arc;
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle as SyntectFontStyle, Theme, ThemeSet};
use syntect::html::{highlighted_html_for_string, styled_line_to_highlighted_html, IncludeBackground};
use syntect::parsing::{SyntaxReference, SyntaxSet};
use syntect::util::as_24_bit_terminal_escaped;

/// Represents an RGBA color structure for preview rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Converts syntect `Color` to `Color`.
    pub fn from_syntect(c: syntect::highlighting::Color) -> Self {
        Self {
            r: c.r,
            g: c.g,
            b: c.b,
            a: c.a,
        }
    }

    /// Returns hexadecimal representation `#RRGGBBAA`.
    pub fn to_hex_string(&self) -> String {
        format!("#{:02x}{:02x}{:02x}{:02x}", self.r, self.g, self.b, self.a)
    }

    /// Returns RGB hexadecimal representation `#RRGGBB`.
    pub fn to_rgb_hex_string(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

/// Represents font styling properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct FontStyle {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

impl FontStyle {
    pub fn from_syntect(style: SyntectFontStyle) -> Self {
        Self {
            bold: style.contains(SyntectFontStyle::BOLD),
            italic: style.contains(SyntectFontStyle::ITALIC),
            underline: style.contains(SyntectFontStyle::UNDERLINE),
        }
    }
}

/// A styled segment of highlighted source code text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StyleSpan {
    pub text: String,
    pub fg: Color,
    pub bg: Color,
    pub font_style: FontStyle,
}

/// Represents a single highlighted line containing styled text spans.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HighlightedLine {
    pub line_number: usize,
    pub spans: Vec<StyleSpan>,
}

/// Container for formatted preview output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HighlightedOutput {
    pub content: String,
    pub language: String,
    pub line_count: usize,
    pub truncated: bool,
}

/// Options controlling syntax highlighting execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HighlightOptions {
    pub theme: String,
    pub max_lines: Option<usize>,
    pub max_bytes: Option<usize>,
    pub render_line_numbers: bool,
}

impl Default for HighlightOptions {
    fn default() -> Self {
        Self {
            theme: "base16-ocean.dark".to_string(),
            max_lines: Some(2000),
            max_bytes: Some(1_000_000),
            render_line_numbers: false,
        }
    }
}

/// Error type for syntax highlighting operations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyntaxError {
    IoError(String),
    ThemeNotFound(String),
    HighlightError(String),
    FileTooLarge { size: usize, limit: usize },
}

impl fmt::Display for SyntaxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SyntaxError::IoError(msg) => write!(f, "IO Error: {}", msg),
            SyntaxError::ThemeNotFound(theme) => write!(f, "Theme not found: {}", theme),
            SyntaxError::HighlightError(msg) => write!(f, "Highlight Error: {}", msg),
            SyntaxError::FileTooLarge { size, limit } => write!(
                f,
                "File size ({} bytes) exceeds maximum limit ({} bytes)",
                size, limit
            ),
        }
    }
}

impl std::error::Error for SyntaxError {}

/// Main thread-safe Syntax Highlighter engine managing syntax definitions and themes.
#[derive(Clone)]
pub struct SyntaxHighlighter {
    ss: Arc<SyntaxSet>,
    ts: Arc<ThemeSet>,
}

impl Default for SyntaxHighlighter {
    fn default() -> Self {
        Self::new()
    }
}

impl SyntaxHighlighter {
    /// Creates a new `SyntaxHighlighter` with default embedded syntax definitions and themes.
    pub fn new() -> Self {
        let ss = SyntaxSet::load_defaults_newlines();
        let ts = ThemeSet::load_defaults();
        Self {
            ss: Arc::new(ss),
            ts: Arc::new(ts),
        }
    }

    /// Returns a list of available theme names.
    pub fn list_themes(&self) -> Vec<String> {
        self.ts.themes.keys().cloned().collect()
    }

    /// Returns a list of supported syntax names.
    pub fn list_syntaxes(&self) -> Vec<String> {
        self.ss
            .syntaxes()
            .iter()
            .map(|s| s.name.clone())
            .collect()
    }

    /// Finds a syntax reference by extension, path/filename, or content header/first line.
    pub fn find_syntax(
        &self,
        extension_or_path: &str,
        first_line: Option<&str>,
    ) -> &SyntaxReference {
        let path = Path::new(extension_or_path);

        // Try path / filename first
        if let Ok(Some(syntax)) = self.ss.find_syntax_for_file(path) {
            return syntax;
        }

        // Try by extension
        let clean_ext = extension_or_path.trim_start_matches('.');
        if let Some(syntax) = self.ss.find_syntax_by_extension(clean_ext) {
            return syntax;
        }

        // Try by first line / shebang
        if let Some(line) = first_line {
            if let Some(syntax) = self.ss.find_syntax_by_first_line(line) {
                return syntax;
            }
        }

        // Fallback to Plain Text
        self.ss.find_syntax_plain_text()
    }

    /// Detects language name for the given file or extension.
    pub fn detect_language(&self, extension_or_path: &str, first_line: Option<&str>) -> String {
        self.find_syntax(extension_or_path, first_line).name.clone()
    }

    /// Retrieves theme by name or falls back to default dark theme.
    fn get_theme(&self, theme_name: &str) -> Result<&Theme, SyntaxError> {
        self.ts
            .themes
            .get(theme_name)
            .or_else(|| self.ts.themes.get("base16-ocean.dark"))
            .ok_or_else(|| SyntaxError::ThemeNotFound(theme_name.to_string()))
    }

    /// Highlights source code to HTML format with inline style attributes.
    pub fn highlight_to_html(
        &self,
        code: &str,
        extension_or_filename: &str,
        options: &HighlightOptions,
    ) -> Result<HighlightedOutput, SyntaxError> {
        let first_line = code.lines().next();
        let syntax = self.find_syntax(extension_or_filename, first_line);
        let theme = self.get_theme(&options.theme)?;

        let (truncated_code, line_count, truncated) = truncate_code(code, options.max_lines);

        let html = highlighted_html_for_string(&truncated_code, &self.ss, syntax, theme)
            .map_err(|e| SyntaxError::HighlightError(e.to_string()))?;

        Ok(HighlightedOutput {
            content: html,
            language: syntax.name.clone(),
            line_count,
            truncated,
        })
    }

    /// Highlights source code into terminal ANSI color escape sequences.
    pub fn highlight_to_ansi(
        &self,
        code: &str,
        extension_or_filename: &str,
        options: &HighlightOptions,
    ) -> Result<HighlightedOutput, SyntaxError> {
        let first_line = code.lines().next();
        let syntax = self.find_syntax(extension_or_filename, first_line);
        let theme = self.get_theme(&options.theme)?;

        let (truncated_code, line_count, truncated) = truncate_code(code, options.max_lines);

        let mut highlighter = HighlightLines::new(syntax, theme);
        let mut ansi_content = String::with_capacity(truncated_code.len() * 2);

        for (idx, line) in truncated_code.lines().enumerate() {
            let ranges = highlighter
                .highlight_line(line, &self.ss)
                .map_err(|e| SyntaxError::HighlightError(e.to_string()))?;

            if options.render_line_numbers {
                ansi_content.push_str(&format!("{:4} | ", idx + 1));
            }

            let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
            ansi_content.push_str(&escaped);
            ansi_content.push('\n');
        }

        Ok(HighlightedOutput {
            content: ansi_content,
            language: syntax.name.clone(),
            line_count,
            truncated,
        })
    }

    /// Highlights source code line-by-line into structured `HighlightedLine` spans.
    pub fn highlight_to_lines(
        &self,
        code: &str,
        extension_or_filename: &str,
        options: &HighlightOptions,
    ) -> Result<Vec<HighlightedLine>, SyntaxError> {
        let first_line = code.lines().next();
        let syntax = self.find_syntax(extension_or_filename, first_line);
        let theme = self.get_theme(&options.theme)?;

        let (truncated_code, _, _) = truncate_code(code, options.max_lines);
        let mut highlighter = HighlightLines::new(syntax, theme);
        let mut highlighted_lines = Vec::new();

        for (idx, line) in truncated_code.lines().enumerate() {
            let ranges = highlighter
                .highlight_line(line, &self.ss)
                .map_err(|e| SyntaxError::HighlightError(e.to_string()))?;

            let spans = ranges
                .into_iter()
                .map(|(style, text)| StyleSpan {
                    text: text.to_string(),
                    fg: Color::from_syntect(style.foreground),
                    bg: Color::from_syntect(style.background),
                    font_style: FontStyle::from_syntect(style.font_style),
                })
                .collect();

            highlighted_lines.push(HighlightedLine {
                line_number: idx + 1,
                spans,
            });
        }

        Ok(highlighted_lines)
    }

    /// Highlights source code into HTML lines, line-by-line, each wrapped in HTML span elements.
    pub fn highlight_to_html_lines(
        &self,
        code: &str,
        extension_or_filename: &str,
        options: &HighlightOptions,
    ) -> Result<Vec<String>, SyntaxError> {
        let first_line = code.lines().next();
        let syntax = self.find_syntax(extension_or_filename, first_line);
        let theme = self.get_theme(&options.theme)?;

        let (truncated_code, _, _) = truncate_code(code, options.max_lines);
        let mut highlighter = HighlightLines::new(syntax, theme);
        let mut html_lines = Vec::new();

        for line in truncated_code.lines() {
            let ranges = highlighter
                .highlight_line(line, &self.ss)
                .map_err(|e| SyntaxError::HighlightError(e.to_string()))?;

            let html_line = styled_line_to_highlighted_html(&ranges[..], IncludeBackground::No)
                .map_err(|e| SyntaxError::HighlightError(e.to_string()))?;

            html_lines.push(html_line);
        }

        Ok(html_lines)
    }

    /// Non-blocking async file preview reading and highlighting code to HTML.
    pub async fn highlight_file_async<P: AsRef<Path>>(
        &self,
        path: P,
        options: HighlightOptions,
    ) -> Result<HighlightedOutput, SyntaxError> {
        let path_ref = path.as_ref();
        let path_str = path_ref.to_string_lossy().to_string();

        let metadata = tokio::fs::metadata(path_ref)
            .await
            .map_err(|e| SyntaxError::IoError(e.to_string()))?;

        if let Some(limit) = options.max_bytes {
            if metadata.len() as usize > limit {
                return Err(SyntaxError::FileTooLarge {
                    size: metadata.len() as usize,
                    limit,
                });
            }
        }

        let content = tokio::fs::read_to_string(path_ref)
            .await
            .map_err(|e| SyntaxError::IoError(e.to_string()))?;

        let highlighter = self.clone();

        tokio::task::spawn_blocking(move || {
            highlighter.highlight_to_html(&content, &path_str, &options)
        })
        .await
        .map_err(|e| SyntaxError::HighlightError(e.to_string()))?
    }

    /// Non-blocking async file preview highlighting code into structured lines.
    pub async fn highlight_file_to_lines_async<P: AsRef<Path>>(
        &self,
        path: P,
        options: HighlightOptions,
    ) -> Result<Vec<HighlightedLine>, SyntaxError> {
        let path_ref = path.as_ref();
        let path_str = path_ref.to_string_lossy().to_string();

        let metadata = tokio::fs::metadata(path_ref)
            .await
            .map_err(|e| SyntaxError::IoError(e.to_string()))?;

        if let Some(limit) = options.max_bytes {
            if metadata.len() as usize > limit {
                return Err(SyntaxError::FileTooLarge {
                    size: metadata.len() as usize,
                    limit,
                });
            }
        }

        let content = tokio::fs::read_to_string(path_ref)
            .await
            .map_err(|e| SyntaxError::IoError(e.to_string()))?;

        let highlighter = self.clone();

        tokio::task::spawn_blocking(move || {
            highlighter.highlight_to_lines(&content, &path_str, &options)
        })
        .await
        .map_err(|e| SyntaxError::HighlightError(e.to_string()))?
    }
}

/// Helper function to handle code truncation according to line count limits.
fn truncate_code(code: &str, max_lines: Option<usize>) -> (String, usize, bool) {
    let total_lines = code.lines().count();
    if let Some(limit) = max_lines {
        if total_lines > limit {
            let truncated_lines: Vec<&str> = code.lines().take(limit).collect();
            return (truncated_lines.join("\n"), total_lines, true);
        }
    }
    (code.to_string(), total_lines, false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_color_and_font_style() {
        let c = Color::new(255, 128, 0, 255);
        assert_eq!(c.to_hex_string(), "#ff8000ff");
        assert_eq!(c.to_rgb_hex_string(), "#ff8000");

        let syntect_c = syntect::highlighting::Color {
            r: 10,
            g: 20,
            b: 30,
            a: 255,
        };
        let c2 = Color::from_syntect(syntect_c);
        assert_eq!(c2.r, 10);
        assert_eq!(c2.g, 20);
        assert_eq!(c2.b, 30);

        let fs = FontStyle::from_syntect(SyntectFontStyle::BOLD | SyntectFontStyle::ITALIC);
        assert!(fs.bold);
        assert!(fs.italic);
        assert!(!fs.underline);
    }

    #[test]
    fn test_language_detection() {
        let highlighter = SyntaxHighlighter::new();

        assert_eq!(highlighter.detect_language("main.rs", None), "Rust");
        assert_eq!(
            highlighter.detect_language("script.py", None),
            "Python"
        );
        assert_eq!(
            highlighter.detect_language("index.js", None),
            "JavaScript"
        );
        assert_eq!(
            highlighter.detect_language("unknown.xyz_123", Some("#!/bin/bash")),
            "Bourne Again Shell (bash)"
        );
        assert_eq!(
            highlighter.detect_language("unknown.xyz_123", None),
            "Plain Text"
        );
    }

    #[test]
    fn test_highlight_to_html() {
        let highlighter = SyntaxHighlighter::new();
        let options = HighlightOptions::default();

        let code = "fn main() {\n    println!(\"Hello, world!\");\n}";
        let output = highlighter
            .highlight_to_html(code, "main.rs", &options)
            .unwrap();

        assert_eq!(output.language, "Rust");
        assert_eq!(output.line_count, 3);
        assert!(!output.truncated);
        assert!(output.content.contains("<span"));
        assert!(output.content.contains("main"));
    }

    #[test]
    fn test_highlight_to_ansi() {
        let highlighter = SyntaxHighlighter::new();
        let mut options = HighlightOptions::default();
        options.render_line_numbers = true;

        let code = "let x = 42;";
        let output = highlighter
            .highlight_to_ansi(code, "test.rs", &options)
            .unwrap();

        assert_eq!(output.language, "Rust");
        assert!(output.content.contains("1 |"));
        assert!(output.content.contains("x"));
    }

    #[test]
    fn test_highlight_to_lines() {
        let highlighter = SyntaxHighlighter::new();
        let options = HighlightOptions::default();

        let code = "struct Point {\n    x: i32,\n}";
        let lines = highlighter
            .highlight_to_lines(code, "point.rs", &options)
            .unwrap();

        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].line_number, 1);
        assert!(!lines[0].spans.is_empty());
        let joined: String = lines[0].spans.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(joined, "struct Point {");
    }

    #[test]
    fn test_highlight_to_html_lines() {
        let highlighter = SyntaxHighlighter::new();
        let options = HighlightOptions::default();

        let code = "let a = 10;\nlet b = 20;";
        let html_lines = highlighter
            .highlight_to_html_lines(code, "test.rs", &options)
            .unwrap();

        assert_eq!(html_lines.len(), 2);
        assert!(html_lines[0].contains("<span"));
    }

    #[test]
    fn test_truncation() {
        let highlighter = SyntaxHighlighter::new();
        let mut options = HighlightOptions::default();
        options.max_lines = Some(2);

        let code = "line 1\nline 2\nline 3\nline 4";
        let output = highlighter
            .highlight_to_html(code, "file.txt", &options)
            .unwrap();

        assert_eq!(output.line_count, 4);
        assert!(output.truncated);
    }

    #[test]
    fn test_empty_string() {
        let highlighter = SyntaxHighlighter::new();
        let options = HighlightOptions::default();

        let output = highlighter
            .highlight_to_html("", "empty.rs", &options)
            .unwrap();

        assert_eq!(output.line_count, 0);
        assert!(!output.truncated);
    }

    #[test]
    fn test_list_themes_and_syntaxes() {
        let highlighter = SyntaxHighlighter::new();

        let themes = highlighter.list_themes();
        assert!(themes.contains(&"base16-ocean.dark".to_string()));

        let syntaxes = highlighter.list_syntaxes();
        assert!(syntaxes.contains(&"Rust".to_string()));
    }

    #[tokio::test]
    async fn test_highlight_file_async() {
        let highlighter = SyntaxHighlighter::new();
        let options = HighlightOptions::default();

        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "pub fn add(a: i32, b: i32) -> i32 {{ a + b }}").unwrap();
        let file_path = temp_file.path().to_str().unwrap().to_string() + ".rs";

        std::fs::copy(temp_file.path(), &file_path).unwrap();

        let output = highlighter
            .highlight_file_async(&file_path, options.clone())
            .await
            .unwrap();

        assert_eq!(output.language, "Rust");
        assert!(output.content.contains("add"));

        let lines = highlighter
            .highlight_file_to_lines_async(&file_path, options)
            .await
            .unwrap();

        assert_eq!(lines.len(), 1);

        let _ = std::fs::remove_file(file_path);
    }

    #[tokio::test]
    async fn test_file_too_large() {
        let highlighter = SyntaxHighlighter::new();
        let mut options = HighlightOptions::default();
        options.max_bytes = Some(10); // set small limit

        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "This content is longer than 10 bytes!").unwrap();

        let result = highlighter
            .highlight_file_async(temp_file.path(), options)
            .await;

        assert!(matches!(result, Err(SyntaxError::FileTooLarge { .. })));
    }
}
