use swal_files_preview::markdown::parse_markdown_preview;

#[test]
fn test_empty_input() {
    let result = parse_markdown_preview("", 100).unwrap();
    assert_eq!(result.title, None);
    assert_eq!(result.heading_count, 0);
    assert_eq!(result.code_block_count, 0);
    assert_eq!(result.plain_text, "");
}

#[test]
fn test_headings_only() {
    let input = "# Heading 1\n## Heading 2\n### Heading 3";
    let result = parse_markdown_preview(input, 100).unwrap();
    assert_eq!(result.title, Some("Heading 1".to_string()));
    assert_eq!(result.heading_count, 3);
    assert_eq!(result.code_block_count, 0);
    assert_eq!(result.plain_text, "Heading 1\nHeading 2\nHeading 3");
}

#[test]
fn test_code_fence_counting() {
    let input = "Some text\n```rust\nfn main() {}\n```\nMore text\n~~~\ncode\n~~~";
    let result = parse_markdown_preview(input, 100).unwrap();
    assert_eq!(result.code_block_count, 2);
}

#[test]
fn test_truncation() {
    let input = "This is a long sentence that should be truncated.";
    let result = parse_markdown_preview(input, 10).unwrap();
    assert_eq!(result.plain_text, "This is a ");
}

#[test]
fn test_unicode_safety() {
    let input = "🌟 Hello World 🌟";
    let result = parse_markdown_preview(input, 5).unwrap();
    assert_eq!(result.plain_text, "🌟 Hel");
}

#[test]
fn test_unclosed_code_block() {
    let input = "```rust\nfn main() {}";
    let result = parse_markdown_preview(input, 100).unwrap();
    assert_eq!(result.code_block_count, 1);
}

#[test]
fn test_ignore_too_many_hashes() {
    let input = "####### Not a heading\n# Heading 1";
    let result = parse_markdown_preview(input, 100).unwrap();
    assert_eq!(result.title, Some("Heading 1".to_string()));
    assert_eq!(result.heading_count, 1);
}

#[test]
fn test_heading_without_space() {
    let input = "#Not a heading\n# Heading 1";
    let result = parse_markdown_preview(input, 100).unwrap();
    assert_eq!(result.title, Some("Heading 1".to_string()));
    assert_eq!(result.heading_count, 1);
}
