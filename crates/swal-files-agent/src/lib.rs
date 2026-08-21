pub mod client;
pub mod tagger;

pub use tagger::{
    AutoTagger, FileMetadata, NlQueryParser, ParsedQuery, SortDirection, SortField, Tag, TagCategory, TagColor,
    TagRule, TaggedFile, TaggerError,
};
