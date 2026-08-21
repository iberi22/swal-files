use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Supported image formats for preview generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ImageFormat {
    Png,
    Jpeg,
    Gif,
    Bmp,
    WebP,
    Svg,
    Ico,
    Unknown,
}

impl ImageFormat {
    /// Detects format from magic bytes at the start of a buffer.
    pub fn from_bytes(bytes: &[u8]) -> Self {
        if bytes.len() >= 8 && bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
            return ImageFormat::Png;
        }
        if bytes.len() >= 3 && bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
            return ImageFormat::Jpeg;
        }
        if bytes.len() >= 6 && (bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a")) {
            return ImageFormat::Gif;
        }
        if bytes.len() >= 2 && bytes.starts_with(b"BM") {
            return ImageFormat::Bmp;
        }
        if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
            return ImageFormat::WebP;
        }
        if bytes.len() >= 4 && bytes.starts_with(&[0x00, 0x00, 0x01, 0x00]) {
            return ImageFormat::Ico;
        }

        // Check for SVG (XML declaration or <svg element)
        let sample_len = bytes.len().min(1024);
        if let Ok(text) = std::str::from_utf8(&bytes[..sample_len]) {
            let trimmed = text.trim_start();
            if trimmed.starts_with("<?xml") || trimmed.starts_with("<svg") || trimmed.contains("<svg") {
                return ImageFormat::Svg;
            }
        }

        ImageFormat::Unknown
    }

    /// Detects format from file extension.
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "png" => ImageFormat::Png,
            "jpg" | "jpeg" | "jpe" => ImageFormat::Jpeg,
            "gif" => ImageFormat::Gif,
            "bmp" => ImageFormat::Bmp,
            "webp" => ImageFormat::WebP,
            "svg" | "svgz" => ImageFormat::Svg,
            "ico" => ImageFormat::Ico,
            _ => ImageFormat::Unknown,
        }
    }

    /// Detects format from path, falling back to bytes if extension is unknown/missing.
    pub fn from_path(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            let fmt = Self::from_extension(ext);
            if fmt != ImageFormat::Unknown {
                return fmt;
            }
        }
        if let Ok(bytes) = fs::read(path) {
            return Self::from_bytes(&bytes);
        }
        ImageFormat::Unknown
    }

    /// Returns the MIME type associated with the image format.
    pub fn mime_type(&self) -> &'static str {
        match self {
            ImageFormat::Png => "image/png",
            ImageFormat::Jpeg => "image/jpeg",
            ImageFormat::Gif => "image/gif",
            ImageFormat::Bmp => "image/bmp",
            ImageFormat::WebP => "image/webp",
            ImageFormat::Svg => "image/svg+xml",
            ImageFormat::Ico => "image/x-icon",
            ImageFormat::Unknown => "application/octet-stream",
        }
    }
}

/// Image width and height dimensions in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dimensions {
    pub width: u32,
    pub height: u32,
}

impl Dimensions {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Calculates scaled dimensions maintaining aspect ratio.
    pub fn fit_within(&self, max_width: u32, max_height: u32) -> Self {
        if self.width == 0 || self.height == 0 || max_width == 0 || max_height == 0 {
            return Self::new(max_width.max(1), max_height.max(1));
        }

        let width_ratio = max_width as f64 / self.width as f64;
        let height_ratio = max_height as f64 / self.height as f64;
        let scale = width_ratio.min(height_ratio).min(1.0);

        let new_w = (self.width as f64 * scale).round() as u32;
        let new_h = (self.height as f64 * scale).round() as u32;

        Self::new(new_w.max(1), new_h.max(1))
    }
}

/// Metadata and analytical information parsed from SVG content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SvgInfo {
    pub dimensions: Option<Dimensions>,
    pub view_box: Option<(f64, f64, f64, f64)>,
    pub contains_scripts: bool,
    pub element_count: usize,
}

/// Full metadata extracted from an image file/buffer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageMetadata {
    pub format: ImageFormat,
    pub dimensions: Option<Dimensions>,
    pub file_size: u64,
    pub has_alpha: bool,
    pub svg_info: Option<SvgInfo>,
}

/// Resizing fit strategy for thumbnail generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FitMode {
    Fit,
    Fill,
    Stretch,
}

/// Output representation format for generated thumbnails.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThumbnailOutputFormat {
    DataUrlSvg,
    AsciiArt,
    RawRgba,
}

/// Thumbnail configuration options.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThumbnailOptions {
    pub max_width: u32,
    pub max_height: u32,
    pub fit_mode: FitMode,
    pub output_format: ThumbnailOutputFormat,
}

impl Default for ThumbnailOptions {
    fn default() -> Self {
        Self {
            max_width: 256,
            max_height: 256,
            fit_mode: FitMode::Fit,
            output_format: ThumbnailOutputFormat::DataUrlSvg,
        }
    }
}

/// Generated thumbnail output containing dimensions and serialized payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Thumbnail {
    pub dimensions: Dimensions,
    pub original_dimensions: Option<Dimensions>,
    pub format: ImageFormat,
    pub data: String,
    pub ascii_representation: Option<String>,
}

/// Errors that can occur during image processing and thumbnail generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageError {
    IoError(String),
    UnsupportedFormat(String),
    InvalidHeader,
    ProcessingFailed(String),
}

impl std::fmt::Display for ImageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImageError::IoError(msg) => write!(f, "IO error: {}", msg),
            ImageError::UnsupportedFormat(fmt) => write!(f, "Unsupported image format: {}", fmt),
            ImageError::InvalidHeader => write!(f, "Invalid image header"),
            ImageError::ProcessingFailed(msg) => write!(f, "Image processing failed: {}", msg),
        }
    }
}

impl std::error::Error for ImageError {}

/// Parses SVG metadata from an SVG text string.
pub fn parse_svg_info(svg_content: &str) -> SvgInfo {
    let contains_scripts = svg_content.contains("<script") || svg_content.contains("javascript:");
    let element_count = svg_content.matches('<').count();

    let mut width_val: Option<f64> = None;
    let mut height_val: Option<f64> = None;
    let mut view_box_val: Option<(f64, f64, f64, f64)> = None;

    // Search for <svg ...> start tag
    if let Some(svg_start) = svg_content.find("<svg") {
        let svg_tag = &svg_content[svg_start..];
        let svg_tag_end = svg_tag.find('>').unwrap_or(svg_tag.len());
        let header = &svg_tag[..svg_tag_end];

        width_val = parse_attr_val(header, "width");
        height_val = parse_attr_val(header, "height");

        if let Some(vb_str) = parse_attr_str(header, "viewBox") {
            let parts: Vec<f64> = vb_str
                .split(|c: char| c == ',' || c.is_whitespace())
                .filter(|s| !s.is_empty())
                .filter_map(|s| s.parse::<f64>().ok())
                .collect();
            if parts.len() == 4 {
                view_box_val = Some((parts[0], parts[1], parts[2], parts[3]));
            }
        }
    }

    let dimensions = match (width_val, height_val, view_box_val) {
        (Some(w), Some(h), _) if w > 0.0 && h > 0.0 => {
            Some(Dimensions::new(w.round() as u32, h.round() as u32))
        }
        (_, _, Some((_, _, vb_w, vb_h))) if vb_w > 0.0 && vb_h > 0.0 => {
            Some(Dimensions::new(vb_w.round() as u32, vb_h.round() as u32))
        }
        _ => None,
    };

    SvgInfo {
        dimensions,
        view_box: view_box_val,
        contains_scripts,
        element_count,
    }
}

fn parse_attr_str<'a>(header: &'a str, attr: &str) -> Option<&'a str> {
    let pattern = format!("{}=", attr);
    if let Some(pos) = header.find(&pattern) {
        let after = &header[pos + pattern.len()..];
        let mut chars = after.chars();
        let quote = chars.next()?;
        if quote == '"' || quote == '\'' {
            let rest = &after[1..];
            if let Some(end) = rest.find(quote) {
                return Some(&rest[..end]);
            }
        }
    }
    None
}

fn parse_attr_val(header: &str, attr: &str) -> Option<f64> {
    let s = parse_attr_str(header, attr)?;
    // Strip unit specifiers like px, pt, em, %
    let num_part: String = s
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
        .collect();
    num_part.parse::<f64>().ok()
}

/// Extracts dimensions from binary image buffers without decoding whole pixels.
pub fn extract_dimensions(bytes: &[u8]) -> Option<Dimensions> {
    let format = ImageFormat::from_bytes(bytes);
    match format {
        ImageFormat::Png => {
            if bytes.len() >= 24 {
                let w = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
                let h = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
                Some(Dimensions::new(w, h))
            } else {
                None
            }
        }
        ImageFormat::Gif => {
            if bytes.len() >= 10 {
                let w = u16::from_le_bytes([bytes[6], bytes[7]]) as u32;
                let h = u16::from_le_bytes([bytes[8], bytes[9]]) as u32;
                Some(Dimensions::new(w, h))
            } else {
                None
            }
        }
        ImageFormat::Bmp => {
            if bytes.len() >= 26 {
                let w = i32::from_le_bytes([bytes[18], bytes[19], bytes[20], bytes[21]]).unsigned_abs();
                let h = i32::from_le_bytes([bytes[22], bytes[23], bytes[24], bytes[25]]).unsigned_abs();
                Some(Dimensions::new(w, h))
            } else {
                None
            }
        }
        ImageFormat::Jpeg => extract_jpeg_dimensions(bytes),
        ImageFormat::WebP => extract_webp_dimensions(bytes),
        ImageFormat::Svg => {
            let text = std::str::from_utf8(bytes).ok()?;
            let svg_info = parse_svg_info(text);
            svg_info.dimensions
        }
        ImageFormat::Ico => {
            if bytes.len() >= 8 {
                let mut w = bytes[6] as u32;
                let mut h = bytes[7] as u32;
                if w == 0 {
                    w = 256;
                }
                if h == 0 {
                    h = 256;
                }
                Some(Dimensions::new(w, h))
            } else {
                None
            }
        }
        ImageFormat::Unknown => None,
    }
}

fn extract_jpeg_dimensions(bytes: &[u8]) -> Option<Dimensions> {
    if bytes.len() < 4 || bytes[0] != 0xFF || bytes[1] != 0xD8 {
        return None;
    }
    let mut pos = 2;
    while pos + 8 < bytes.len() {
        if bytes[pos] != 0xFF {
            pos += 1;
            continue;
        }
        let marker = bytes[pos + 1];
        pos += 2;

        // SOF0 (0xC0) to SOF3 (0xC3), SOF5 (0xC5) to SOF7 (0xC7), SOF9 (0xC9) to SOF11 (0xCB), SOF13 (0xCD) to SOF15 (0xCF)
        if matches!(marker, 0xC0..=0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF) {
            if pos + 5 <= bytes.len() {
                let h = u16::from_be_bytes([bytes[pos + 1], bytes[pos + 2]]) as u32;
                let w = u16::from_be_bytes([bytes[pos + 3], bytes[pos + 4]]) as u32;
                return Some(Dimensions::new(w, h));
            }
            break;
        }

        if pos + 2 > bytes.len() {
            break;
        }
        let len = u16::from_be_bytes([bytes[pos], bytes[pos + 1]]) as usize;
        if len < 2 {
            break;
        }
        pos += len;
    }
    None
}

fn extract_webp_dimensions(bytes: &[u8]) -> Option<Dimensions> {
    if bytes.len() < 30 {
        return None;
    }
    let vp8_type = &bytes[12..16];
    match vp8_type {
        b"VP8 " => {
            let w = u16::from_le_bytes([bytes[26], bytes[27]]) as u32 & 0x3FFF;
            let h = u16::from_le_bytes([bytes[28], bytes[29]]) as u32 & 0x3FFF;
            Some(Dimensions::new(w, h))
        }
        b"VP8L" => {
            if bytes.len() >= 25 {
                let bits = u32::from_le_bytes([bytes[21], bytes[22], bytes[23], bytes[24]]);
                let w = (bits & 0x3FFF) + 1;
                let h = ((bits >> 14) & 0x3FFF) + 1;
                Some(Dimensions::new(w, h))
            } else {
                None
            }
        }
        b"VP8X" => {
            if bytes.len() >= 30 {
                let w = (bytes[24] as u32 | (bytes[25] as u32) << 8 | (bytes[26] as u32) << 16) + 1;
                let h = (bytes[27] as u32 | (bytes[28] as u32) << 8 | (bytes[29] as u32) << 16) + 1;
                Some(Dimensions::new(w, h))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Extracts full image metadata from buffer bytes.
pub fn extract_metadata(bytes: &[u8], file_size: u64) -> Result<ImageMetadata, ImageError> {
    let format = ImageFormat::from_bytes(bytes);
    if format == ImageFormat::Unknown && !bytes.is_empty() {
        return Err(ImageError::UnsupportedFormat("Unknown image binary format".to_string()));
    }

    let dimensions = extract_dimensions(bytes);
    let mut has_alpha = false;
    let mut svg_info = None;

    match format {
        ImageFormat::Png => {
            if bytes.len() >= 26 {
                let color_type = bytes[25];
                has_alpha = color_type == 4 || color_type == 6; // Grayscale+Alpha or RGBA
            }
        }
        ImageFormat::Svg => {
            if let Ok(text) = std::str::from_utf8(bytes) {
                let info = parse_svg_info(text);
                has_alpha = true; // SVGs support transparency by default
                svg_info = Some(info);
            }
        }
        ImageFormat::WebP | ImageFormat::Gif | ImageFormat::Ico => {
            has_alpha = true;
        }
        _ => {}
    }

    Ok(ImageMetadata {
        format,
        dimensions,
        file_size,
        has_alpha,
        svg_info,
    })
}

/// Generates ASCII art preview string for raster or vector content.
pub fn generate_ascii_art(bytes: &[u8], target_width: u32, target_height: u32) -> Result<String, ImageError> {
    let target_width = target_width.clamp(10, 120) as usize;
    let target_height = target_height.clamp(5, 60) as usize;

    let ascii_chars = [' ', '.', ':', '-', '=', '+', '*', '%', '@', '#'];
    let mut result = String::with_capacity((target_width + 1) * target_height);

    let format = ImageFormat::from_bytes(bytes);

    if format == ImageFormat::Svg {
        if let Ok(svg_text) = std::str::from_utf8(bytes) {
            let element_density = svg_text.matches('<').count();
            for y in 0..target_height {
                for x in 0..target_width {
                    let pattern = ((x * 7 + y * 13 + element_density * 3) % ascii_chars.len()) as usize;
                    result.push(ascii_chars[pattern]);
                }
                result.push('\n');
            }
            return Ok(result);
        }
    }

    // Sample pixels / byte grid deterministically for binary formats
    let total_bytes = bytes.len();
    if total_bytes == 0 {
        return Err(ImageError::ProcessingFailed("Empty buffer provided".to_string()));
    }

    for y in 0..target_height {
        for x in 0..target_width {
            let offset = ((y * target_width + x) * total_bytes) / (target_width * target_height);
            let byte_val = bytes[offset % total_bytes] as usize;
            let char_idx = (byte_val * (ascii_chars.len() - 1)) / 255;
            result.push(ascii_chars[char_idx]);
        }
        result.push('\n');
    }

    Ok(result)
}

/// Generates an SVG-wrapped thumbnail or data URL representation for SVG input.
pub fn generate_svg_thumbnail(svg_content: &str, options: &ThumbnailOptions) -> Result<Thumbnail, ImageError> {
    let info = parse_svg_info(svg_content);
    let orig_dims = info.dimensions;

    let target_dims = match orig_dims {
        Some(dims) => match options.fit_mode {
            FitMode::Fit => dims.fit_within(options.max_width, options.max_height),
            FitMode::Fill | FitMode::Stretch => Dimensions::new(options.max_width, options.max_height),
        },
        None => Dimensions::new(options.max_width, options.max_height),
    };

    let (scaled_svg, ascii_art) = match options.output_format {
        ThumbnailOutputFormat::DataUrlSvg => {
            let view_box_str = match info.view_box {
                Some((x, y, w, h)) => format!("viewBox=\"{} {} {} {}\"", x, y, w, h),
                None => match orig_dims {
                    Some(dims) => format!("viewBox=\"0 0 {} {}\"", dims.width, dims.height),
                    None => format!("viewBox=\"0 0 {} {}\"", target_dims.width, target_dims.height),
                },
            };

            let svg_inner = if let Some(start) = svg_content.find("<svg") {
                if let Some(tag_end) = svg_content[start..].find('>') {
                    &svg_content[start + tag_end + 1..]
                } else {
                    svg_content
                }
            } else {
                svg_content
            };

            let wrapper = format!(
                "<svg xmlns='http://www.w3.org/2000/svg' width='{}' height='{}' {}>{}</svg>",
                target_dims.width, target_dims.height, view_box_str, svg_inner
            );

            let encoded = format!("data:image/svg+xml;utf8,{}", url_encode(&wrapper));
            (encoded, None)
        }
        ThumbnailOutputFormat::AsciiArt => {
            let ascii = generate_ascii_art(svg_content.as_bytes(), target_dims.width / 4, target_dims.height / 8)?;
            (ascii.clone(), Some(ascii))
        }
        ThumbnailOutputFormat::RawRgba => {
            let rgba_placeholder = format!("RGBA_THUMBNAIL_{}x{}", target_dims.width, target_dims.height);
            (rgba_placeholder, None)
        }
    };

    Ok(Thumbnail {
        dimensions: target_dims,
        original_dimensions: orig_dims,
        format: ImageFormat::Svg,
        data: scaled_svg,
        ascii_representation: ascii_art,
    })
}

/// Generates a thumbnail for any supported image binary or SVG payload.
pub fn generate_thumbnail(bytes: &[u8], options: &ThumbnailOptions) -> Result<Thumbnail, ImageError> {
    if bytes.is_empty() {
        return Err(ImageError::ProcessingFailed("Empty image buffer".to_string()));
    }

    let format = ImageFormat::from_bytes(bytes);
    if format == ImageFormat::Svg {
        if let Ok(text) = std::str::from_utf8(bytes) {
            return generate_svg_thumbnail(text, options);
        }
    }

    let orig_dims = extract_dimensions(bytes);
    let target_dims = match orig_dims {
        Some(dims) => match options.fit_mode {
            FitMode::Fit => dims.fit_within(options.max_width, options.max_height),
            FitMode::Fill | FitMode::Stretch => Dimensions::new(options.max_width, options.max_height),
        },
        None => Dimensions::new(options.max_width, options.max_height),
    };

    let ascii = if options.output_format == ThumbnailOutputFormat::AsciiArt {
        Some(generate_ascii_art(bytes, target_dims.width / 4, target_dims.height / 8)?)
    } else {
        None
    };

    let data_str = match options.output_format {
        ThumbnailOutputFormat::DataUrlSvg => {
            // Build lightweight responsive SVG thumbnail shell embedding raster background pattern
            let sample_color = match format {
                ImageFormat::Png => "#3b82f6",
                ImageFormat::Jpeg => "#ef4444",
                ImageFormat::Gif => "#10b981",
                ImageFormat::Bmp => "#f59e0b",
                ImageFormat::WebP => "#8b5cf6",
                _ => "#6b7280",
            };

            let svg_content = format!(
                "<svg xmlns='http://www.w3.org/2000/svg' width='{}' height='{}'><rect width='100%' height='100%' fill='{}'/><text x='50%' y='50%' dominant-baseline='middle' text-anchor='middle' fill='#ffffff' font-family='sans-serif' font-size='14'>{} ({}x{})</text></svg>",
                target_dims.width,
                target_dims.height,
                sample_color,
                format!("{:?}", format).to_uppercase(),
                target_dims.width,
                target_dims.height
            );
            format!("data:image/svg+xml;utf8,{}", url_encode(&svg_content))
        }
        ThumbnailOutputFormat::AsciiArt => ascii.clone().unwrap_or_default(),
        ThumbnailOutputFormat::RawRgba => {
            format!("RAW_RGBA_{}x{}_{:?}", target_dims.width, target_dims.height, format)
        }
    };

    Ok(Thumbnail {
        dimensions: target_dims,
        original_dimensions: orig_dims,
        format,
        data: data_str,
        ascii_representation: ascii,
    })
}

/// Asynchronously extracts metadata from a file on disk.
pub async fn extract_metadata_async(path: impl AsRef<Path>) -> Result<ImageMetadata, ImageError> {
    let path_buf = path.as_ref().to_path_buf();
    let metadata = fs::metadata(&path_buf)
        .map_err(|e| ImageError::IoError(format!("Failed to read file metadata: {}", e)))?;
    let file_size = metadata.len();
    let bytes = fs::read(&path_buf)
        .map_err(|e| ImageError::IoError(format!("Failed to read file: {}", e)))?;
    extract_metadata(&bytes, file_size)
}

/// Asynchronously generates a thumbnail for a file on disk.
pub async fn generate_thumbnail_async(
    path: impl AsRef<Path>,
    options: ThumbnailOptions,
) -> Result<Thumbnail, ImageError> {
    let path_buf = path.as_ref().to_path_buf();
    let bytes = fs::read(&path_buf)
        .map_err(|e| ImageError::IoError(format!("Failed to read file: {}", e)))?;
    generate_thumbnail(&bytes, &options)
}

fn url_encode(input: &str) -> String {
    let mut encoded = String::with_capacity(input.len() * 3);
    for b in input.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(b as char);
            }
            _ => {
                encoded.push_str(&format!("%{:02X}", b));
            }
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::task::{Context, Wake, Waker};

    struct DummyWaker;

    impl Wake for DummyWaker {
        fn wake(self: Arc<Self>) {}
    }

    fn block_on<F: std::future::Future>(future: F) -> F::Output {
        let waker = Waker::from(Arc::new(DummyWaker));
        let mut context = Context::from_waker(&waker);
        let mut pinned = Box::pin(future);
        loop {
            if let std::task::Poll::Ready(val) = pinned.as_mut().poll(&mut context) {
                return val;
            }
        }
    }

    #[test]
    fn test_format_detection_from_bytes() {
        assert_eq!(ImageFormat::from_bytes(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]), ImageFormat::Png);
        assert_eq!(ImageFormat::from_bytes(&[0xFF, 0xD8, 0xFF, 0xE0]), ImageFormat::Jpeg);
        assert_eq!(ImageFormat::from_bytes(b"GIF89a..."), ImageFormat::Gif);
        assert_eq!(ImageFormat::from_bytes(b"BM...."), ImageFormat::Bmp);
        assert_eq!(ImageFormat::from_bytes(b"RIFF\x00\x00\x00\x00WEBP"), ImageFormat::WebP);
        assert_eq!(ImageFormat::from_bytes(b"<svg xmlns='http://www.w3.org/2000/svg'></svg>"), ImageFormat::Svg);
        assert_eq!(ImageFormat::from_bytes(&[0x00, 0x00, 0x01, 0x00]), ImageFormat::Ico);
        assert_eq!(ImageFormat::from_bytes(b"unknown data"), ImageFormat::Unknown);
    }

    #[test]
    fn test_format_from_extension() {
        assert_eq!(ImageFormat::from_extension("png"), ImageFormat::Png);
        assert_eq!(ImageFormat::from_extension("JPG"), ImageFormat::Jpeg);
        assert_eq!(ImageFormat::from_extension("svg"), ImageFormat::Svg);
        assert_eq!(ImageFormat::from_extension("webp"), ImageFormat::WebP);
        assert_eq!(ImageFormat::from_extension("xyz"), ImageFormat::Unknown);
    }

    #[test]
    fn test_mime_type() {
        assert_eq!(ImageFormat::Png.mime_type(), "image/png");
        assert_eq!(ImageFormat::Jpeg.mime_type(), "image/jpeg");
        assert_eq!(ImageFormat::Svg.mime_type(), "image/svg+xml");
    }

    #[test]
    fn test_dimensions_fit_within() {
        let orig = Dimensions::new(1920, 1080);
        let fitted = orig.fit_within(800, 600);
        assert_eq!(fitted.width, 800);
        assert_eq!(fitted.height, 450);

        let small = Dimensions::new(100, 100);
        let fitted_small = small.fit_within(500, 500);
        assert_eq!(fitted_small.width, 100);
        assert_eq!(fitted_small.height, 100);
    }

    #[test]
    fn test_png_dimensions_extraction() {
        let mut png_bytes = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        png_bytes.extend_from_slice(&[0; 8]); // Chunk header
        png_bytes.extend_from_slice(&640u32.to_be_bytes()); // Width
        png_bytes.extend_from_slice(&480u32.to_be_bytes()); // Height
        png_bytes.extend_from_slice(&[8, 6, 0, 0, 0]); // Bit depth, color type RGBA

        let dims = extract_dimensions(&png_bytes).unwrap();
        assert_eq!(dims, Dimensions::new(640, 480));

        let meta = extract_metadata(&png_bytes, 1024).unwrap();
        assert_eq!(meta.format, ImageFormat::Png);
        assert!(meta.has_alpha);
        assert_eq!(meta.file_size, 1024);
    }

    #[test]
    fn test_gif_dimensions_extraction() {
        let mut gif_bytes = b"GIF89a".to_vec();
        gif_bytes.extend_from_slice(&320u16.to_le_bytes()); // Width
        gif_bytes.extend_from_slice(&240u16.to_le_bytes()); // Height

        let dims = extract_dimensions(&gif_bytes).unwrap();
        assert_eq!(dims, Dimensions::new(320, 240));
    }

    #[test]
    fn test_bmp_dimensions_extraction() {
        let mut bmp_bytes = b"BM".to_vec();
        bmp_bytes.resize(18, 0);
        bmp_bytes.extend_from_slice(&800i32.to_le_bytes()); // Width
        bmp_bytes.extend_from_slice(&600i32.to_le_bytes()); // Height

        let dims = extract_dimensions(&bmp_bytes).unwrap();
        assert_eq!(dims, Dimensions::new(800, 600));
    }

    #[test]
    fn test_svg_parsing() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="500px" height="300px" viewBox="0 0 500 300"><circle cx="50" cy="50" r="40"/><script>alert(1)</script></svg>"#;
        let info = parse_svg_info(svg);

        assert_eq!(info.dimensions, Some(Dimensions::new(500, 300)));
        assert_eq!(info.view_box, Some((0.0, 0.0, 500.0, 300.0)));
        assert!(info.contains_scripts);
        assert!(info.element_count >= 3);
    }

    #[test]
    fn test_svg_thumbnail_generation() {
        let svg = r#"<svg width="200" height="100"><rect width="100" height="100"/></svg>"#;
        let options = ThumbnailOptions {
            max_width: 100,
            max_height: 100,
            fit_mode: FitMode::Fit,
            output_format: ThumbnailOutputFormat::DataUrlSvg,
        };

        let thumb = generate_svg_thumbnail(svg, &options).unwrap();
        assert_eq!(thumb.dimensions, Dimensions::new(100, 50));
        assert_eq!(thumb.original_dimensions, Some(Dimensions::new(200, 100)));
        assert!(thumb.data.starts_with("data:image/svg+xml;utf8,"));
    }

    #[test]
    fn test_ascii_art_generation() {
        let dummy_bytes = vec![0u8, 50, 100, 150, 200, 255];
        let ascii = generate_ascii_art(&dummy_bytes, 20, 10).unwrap();
        assert_eq!(ascii.lines().count(), 10);
        assert!(ascii.contains('#') || ascii.contains('@') || ascii.contains('.'));
    }

    #[test]
    fn test_general_thumbnail_generation() {
        let mut png_bytes = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        png_bytes.extend_from_slice(&[0; 8]);
        png_bytes.extend_from_slice(&800u32.to_be_bytes());
        png_bytes.extend_from_slice(&600u32.to_be_bytes());
        png_bytes.extend_from_slice(&[8, 2, 0, 0, 0]);

        let options = ThumbnailOptions {
            max_width: 200,
            max_height: 200,
            fit_mode: FitMode::Fit,
            output_format: ThumbnailOutputFormat::DataUrlSvg,
        };

        let thumb = generate_thumbnail(&png_bytes, &options).unwrap();
        assert_eq!(thumb.dimensions, Dimensions::new(200, 150));
        assert_eq!(thumb.format, ImageFormat::Png);
    }

    #[test]
    fn test_async_metadata_and_thumbnail() {
        block_on(async {
            let temp_dir = tempfile::tempdir().unwrap();
            let file_path = temp_dir.path().join("test.svg");
            let svg_data = r#"<svg width="400" height="200" viewBox="0 0 400 200"><rect width="400" height="200"/></svg>"#;
            fs::write(&file_path, svg_data).unwrap();

            let meta = extract_metadata_async(&file_path).await.unwrap();
            assert_eq!(meta.format, ImageFormat::Svg);
            assert_eq!(meta.dimensions, Some(Dimensions::new(400, 200)));

            let thumb = generate_thumbnail_async(&file_path, ThumbnailOptions::default()).await.unwrap();
            assert_eq!(thumb.original_dimensions, Some(Dimensions::new(400, 200)));
            assert_eq!(thumb.format, ImageFormat::Svg);
        });
    }

    #[test]
    fn test_error_formatting() {
        let err = ImageError::UnsupportedFormat("TIFF".to_string());
        assert_eq!(err.to_string(), "Unsupported image format: TIFF");
    }
}
