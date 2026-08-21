use std::fs;
use std::path::PathBuf;
use std::time::Instant;
use swal_files_preview::syntax::{HighlightOptions, SyntaxHighlighter};
use swal_files_preview::{
    extract_dimensions, extract_metadata, extract_metadata_async, generate_thumbnail,
    generate_thumbnail_async, Dimensions, FitMode, ImageFormat, ThumbnailOptions,
    ThumbnailOutputFormat,
};
use tempfile::TempDir;

/// Test fixture managing temporary directories and realistic multi-format mock files.
pub struct PreviewIntegrationFixture {
    pub temp_dir: TempDir,
    pub highlighter: SyntaxHighlighter,
}

impl PreviewIntegrationFixture {
    pub fn new() -> Self {
        Self {
            temp_dir: TempDir::new().expect("Failed to create temp directory"),
            highlighter: SyntaxHighlighter::new(),
        }
    }

    pub fn path(&self, filename: &str) -> PathBuf {
        self.temp_dir.path().join(filename)
    }

    pub fn create_rust_file(&self) -> PathBuf {
        let p = self.path("sample.rs");
        let content = "fn main() {\n    let greeting = \"Hello QuickLook\";\n    println!(\"{}\", greeting);\n}";
        fs::write(&p, content).expect("Write rust file");
        p
    }

    pub fn create_markdown_file(&self) -> PathBuf {
        let p = self.path("README.md");
        let content = "# QuickLook Integration\n\n## Overview\n* Item 1\n* Item 2\n\n```rust\nlet x = 42;\n```\n";
        fs::write(&p, content).expect("Write markdown file");
        p
    }

    pub fn create_svg_file(&self) -> PathBuf {
        let p = self.path("icon.svg");
        let content = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"400\" height=\"200\" viewBox=\"0 0 400 200\"><rect width=\"400\" height=\"200\" fill=\"#ff0000\"/><circle cx=\"200\" cy=\"100\" r=\"50\" fill=\"#00ff00\"/></svg>";
        fs::write(&p, content).expect("Write svg file");
        p
    }

    pub fn create_png_file(&self) -> PathBuf {
        let p = self.path("sample.png");
        let mut png_bytes = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        png_bytes.extend_from_slice(&[0; 8]);
        png_bytes.extend_from_slice(&640u32.to_be_bytes());
        png_bytes.extend_from_slice(&480u32.to_be_bytes());
        png_bytes.extend_from_slice(&[8, 6, 0, 0, 0]);
        fs::write(&p, &png_bytes).expect("Write png file");
        p
    }

    pub fn create_gif_file(&self) -> PathBuf {
        let p = self.path("animation.gif");
        let mut gif_bytes = b"GIF89a".to_vec();
        gif_bytes.extend_from_slice(&320u16.to_le_bytes());
        gif_bytes.extend_from_slice(&240u16.to_le_bytes());
        fs::write(&p, &gif_bytes).expect("Write gif file");
        p
    }
}

#[test]
fn test_multi_format_decoding() {
    let fixture = PreviewIntegrationFixture::new();

    // 1. Syntax highlighting on Rust source file
    let rust_path = fixture.create_rust_file();
    let rust_code = fs::read_to_string(&rust_path).unwrap();
    let opts = HighlightOptions::default();

    let output = fixture.highlighter.highlight_to_html(&rust_code, "sample.rs", &opts).unwrap();
    assert_eq!(output.language, "Rust");
    assert_eq!(output.line_count, 4);
    assert!(output.content.contains("<span"));

    let lines = fixture.highlighter.highlight_to_lines(&rust_code, "sample.rs", &opts).unwrap();
    assert_eq!(lines.len(), 4);
    assert!(!lines[0].spans.is_empty());

    // 2. Markdown structural AST inspection
    let md_path = fixture.create_markdown_file();
    let md_content = fs::read_to_string(&md_path).unwrap();
    assert!(md_content.contains("# QuickLook Integration"));
    assert!(md_content.contains("```rust"));

    let md_highlight = fixture.highlighter.highlight_to_html(&md_content, "README.md", &opts).unwrap();
    assert_eq!(md_highlight.language, "Markdown");

    // 3. Image thumbnail decoding and metadata verification
    let svg_path = fixture.create_svg_file();
    let svg_bytes = fs::read(&svg_path).unwrap();
    let svg_meta = extract_metadata(&svg_bytes, svg_bytes.len() as u64).unwrap();
    assert_eq!(svg_meta.format, ImageFormat::Svg);
    assert_eq!(svg_meta.dimensions, Some(Dimensions::new(400, 200)));

    let thumb_opts = ThumbnailOptions {
        max_width: 200,
        max_height: 200,
        fit_mode: FitMode::Fit,
        output_format: ThumbnailOutputFormat::DataUrlSvg,
    };
    let svg_thumb = generate_thumbnail(&svg_bytes, &thumb_opts).unwrap();
    assert_eq!(svg_thumb.dimensions, Dimensions::new(200, 100));

    // 4. Binary PNG thumbnail decoding
    let png_path = fixture.create_png_file();
    let png_bytes = fs::read(&png_path).unwrap();
    let png_dims = extract_dimensions(&png_bytes).unwrap();
    assert_eq!(png_dims, Dimensions::new(640, 480));

    let png_thumb = generate_thumbnail(&png_bytes, &thumb_opts).unwrap();
    assert_eq!(png_thumb.dimensions, Dimensions::new(200, 150));
    assert_eq!(png_thumb.format, ImageFormat::Png);

    // 5. Binary GIF decoding
    let gif_path = fixture.create_gif_file();
    let gif_bytes = fs::read(&gif_path).unwrap();
    let gif_meta = extract_metadata(&gif_bytes, gif_bytes.len() as u64).unwrap();
    assert_eq!(gif_meta.format, ImageFormat::Gif);
    assert_eq!(gif_meta.dimensions, Some(Dimensions::new(320, 240)));
}

#[tokio::test]
async fn test_async_preview_pipeline() {
    let fixture = PreviewIntegrationFixture::new();
    let rust_path = fixture.create_rust_file();
    let opts = HighlightOptions::default();

    let output = fixture.highlighter.highlight_file_async(&rust_path, opts).await.unwrap();
    assert_eq!(output.language, "Rust");
    assert!(!output.content.is_empty());

    let svg_path = fixture.create_svg_file();
    let svg_meta = extract_metadata_async(&svg_path).await.unwrap();
    assert_eq!(svg_meta.format, ImageFormat::Svg);

    let thumb = generate_thumbnail_async(&svg_path, ThumbnailOptions::default()).await.unwrap();
    assert_eq!(thumb.format, ImageFormat::Svg);
    assert_eq!(thumb.original_dimensions, Some(Dimensions::new(400, 200)));
}

#[test]
fn test_preview_generation_speed() {
    let fixture = PreviewIntegrationFixture::new();
    let rust_path = fixture.create_rust_file();
    let code = fs::read_to_string(&rust_path).unwrap();

    let start = Instant::now();
    for _ in 0..10 {
        let _ = fixture.highlighter.highlight_to_html(&code, "sample.rs", &HighlightOptions::default()).unwrap();
    }
    let elapsed = start.elapsed();
    assert!(elapsed.as_millis() < 500, "Preview generation too slow: {:?}", elapsed);
}

#[test]
fn test_error_resilience() {
    let fixture = PreviewIntegrationFixture::new();
    let opts = ThumbnailOptions::default();

    // Empty bytes
    let empty_res = generate_thumbnail(&[], &opts);
    assert!(empty_res.is_err());

    // File too large
    let big_path = fixture.path("large.txt");
    fs::write(&big_path, vec![b'a'; 1000]).unwrap();
    let mut hl_opts = HighlightOptions::default();
    hl_opts.max_bytes = Some(100);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let err = rt.block_on(fixture.highlighter.highlight_file_async(&big_path, hl_opts));
    assert!(err.is_err());
}
