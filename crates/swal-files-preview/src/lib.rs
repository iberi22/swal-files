pub mod audio;
pub mod hex;
pub mod image;
pub mod markdown;
pub mod syntax;

pub use audio::{
    AudioError, AudioFormat, AudioMetadata, AudioPreviewer, WaveformSummary,
};
pub use hex::{HexChunkConfig, HexDumpRow, HexError, HexInspector};
pub use image::{
    extract_dimensions, extract_metadata, extract_metadata_async, generate_ascii_art,
    generate_svg_thumbnail, generate_thumbnail, generate_thumbnail_async, parse_svg_info,
    Dimensions, FitMode, ImageError, ImageFormat, ImageMetadata, SvgInfo, Thumbnail,
    ThumbnailOptions, ThumbnailOutputFormat,
};

