use serde::{Deserialize, Serialize};
use std::fmt;

/// Errors that can occur during Markdown parsing and preview generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarkdownError {
    /// Indicates that the provided input was somehow invalid, though our current
    /// parser is very permissive and won't usually fail on pure string input.
    ProcessingFailed(String),
    /// Indicates an unsupported feature or structure was encountered, if we ever
    /// decide to strict-parse.
    UnsupportedStructure(String),
    /// General catch-all error for Markdown processing.
    Unknown(String),
}

impl fmt::Display for MarkdownError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MarkdownError::ProcessingFailed(msg) => write!(f, "Markdown processing failed: {}", msg),
            MarkdownError::UnsupportedStructure(msg) => write!(f, "Unsupported structure: {}", msg),
            MarkdownError::Unknown(msg) => write!(f, "Unknown Markdown error: {}", msg),
        }
    }
}

impl std::error::Error for MarkdownError {}

/// A summary of a Markdown document for QuickLook preview purposes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MarkdownPreview {
    /// The title of the document, typically the text of the first heading found.
    pub title: Option<String>,
    /// The total number of headings (`#` through `######`) found in the document.
    pub heading_count: usize,
    /// The total number of fenced code blocks (using ``` or ~~~) found.
    pub code_block_count: usize,
    /// The plain-text body of the document, truncated to a specified maximum number of characters.
    pub plain_text: String,
}

/// Parses a Markdown string and generates a `MarkdownPreview` summary.
///
/// This function performs a single-pass line scan over the input to extract basic
/// metrics like the title, heading count, and code block count. It also attempts
/// to extract a simplified plain-text version of the body, which is truncated at
/// a character boundary to `max_chars`.
///
/// # Arguments
///
/// * `input` - The Markdown string to parse.
/// * `max_chars` - The maximum number of characters to include in the `plain_text` output.
///
/// # Returns
///
/// A `Result` containing the `MarkdownPreview` or a `MarkdownError`.
pub fn parse_markdown_preview(input: &str, max_chars: usize) -> Result<MarkdownPreview, MarkdownError> {
    let mut title = None;
    let mut heading_count = 0;
    let mut code_block_count = 0;
    let mut plain_text = String::new();

    let mut in_code_block = false;
    let mut current_fence_char = ' ';
    let mut current_fence_len = 0;

    for line in input.lines() {
        let trimmed = line.trim_start();

        // Handle code blocks
        if in_code_block {
            // Check if the block ends
            let mut is_end = false;
            if trimmed.starts_with(current_fence_char) {
                let fence_len = trimmed.chars().take_while(|&c| c == current_fence_char).count();
                if fence_len >= current_fence_len {
                    is_end = true;
                }
            }

            if is_end {
                in_code_block = false;
                code_block_count += 1;
            } else {
                // Accumulate code block text in plain text
                if !plain_text.is_empty() {
                    plain_text.push('\n');
                }
                plain_text.push_str(line);
            }
            continue;
        }

        // Check if a code block starts
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_code_block = true;
            current_fence_char = trimmed.chars().next().unwrap();
            current_fence_len = trimmed.chars().take_while(|&c| c == current_fence_char).count();
            continue;
        }

        // Check for headings
        let mut is_heading = false;
        let mut heading_text = "";

        if trimmed.starts_with('#') {
            let hashes = trimmed.chars().take_while(|&c| c == '#').count();
            if hashes <= 6 {
                let after_hashes = &trimmed[hashes..];
                if after_hashes.is_empty() || after_hashes.starts_with(' ') {
                    is_heading = true;
                    heading_text = after_hashes.trim();
                    heading_count += 1;
                    if title.is_none() && !heading_text.is_empty() {
                        title = Some(heading_text.to_string());
                    }
                }
            }
        }

        // Append to plain text
        if !plain_text.is_empty() {
            plain_text.push('\n');
        }

        if is_heading {
            plain_text.push_str(heading_text);
        } else {
            // Check for blockquotes, lists, etc. and simplify them loosely
            // For simplicity, we just strip leading > for blockquotes
            let mut text_to_add = line;
            if let Some(stripped) = trimmed.strip_prefix("> ") {
                text_to_add = stripped.trim_start();
            }
            plain_text.push_str(text_to_add);
        }
    }

    // If we end while still in a code block, count it
    if in_code_block {
        code_block_count += 1;
    }

    // Truncate plain_text respecting character boundaries
    let truncated_plain_text: String = plain_text.chars().take(max_chars).collect();

    Ok(MarkdownPreview {
        title,
        heading_count,
        code_block_count,
        plain_text: truncated_plain_text,
    })
}
