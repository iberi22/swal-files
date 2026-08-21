//! Async Directory Scanner & Sorter for SWAL Files (`swal-files-core`).
//!
//! Provides non-blocking directory scanning, metadata extraction,
//! filtering (e.g., hidden files), and multi-criteria sorting algorithms.

#![forbid(unsafe_code)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};

/// Represents the type of a filesystem entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FileType {
    Directory,
    File,
    Symlink,
    Other,
}

impl FileType {
    /// Returns true if this entry is a directory.
    pub fn is_dir(&self) -> bool {
        matches!(self, FileType::Directory)
    }

    /// Returns true if this entry is a regular file.
    pub fn is_file(&self) -> bool {
        matches!(self, FileType::File)
    }

    /// Returns true if this entry is a symbolic link.
    pub fn is_symlink(&self) -> bool {
        matches!(self, FileType::Symlink)
    }
}

/// Detailed metadata representation for a scanned file system entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub file_type: FileType,
    pub size: u64,
    pub modified: Option<DateTime<Utc>>,
    pub created: Option<DateTime<Utc>>,
    pub is_hidden: bool,
    pub extension: Option<String>,
    pub readonly: bool,
}

impl FileEntry {
    /// Returns true if the entry is a directory.
    pub fn is_dir(&self) -> bool {
        self.file_type.is_dir()
    }
}

/// Supported criteria for sorting directory entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SortBy {
    #[default]
    Name,
    Size,
    Modified,
    Created,
    Extension,
    FileType,
}

/// Ordering direction for sorting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SortOrder {
    #[default]
    Ascending,
    Descending,
}

/// Configuration options for scanning and sorting directories.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanOptions {
    /// Include hidden files (names starting with '.').
    pub show_hidden: bool,
    /// Scan subdirectories recursively.
    pub recursive: bool,
    /// Maximum depth for recursive scanning (None = unlimited).
    pub max_depth: Option<usize>,
    /// Field to sort entries by.
    pub sort_by: SortBy,
    /// Sorting direction.
    pub sort_order: SortOrder,
    /// Keep directories at the top regardless of sort field.
    pub directories_first: bool,
    /// Follow symlinks when querying metadata or scanning directories.
    pub follow_symlinks: bool,
    /// Case-sensitive string comparison for name sorting.
    pub case_sensitive_sort: bool,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            show_hidden: false,
            recursive: false,
            max_depth: None,
            sort_by: SortBy::Name,
            sort_order: SortOrder::Ascending,
            directories_first: true,
            follow_symlinks: false,
            case_sensitive_sort: false,
        }
    }
}

impl ScanOptions {
    /// Builder method to toggle `show_hidden`.
    pub fn with_hidden(mut self, show_hidden: bool) -> Self {
        self.show_hidden = show_hidden;
        self
    }

    /// Builder method to toggle `recursive` mode.
    pub fn with_recursive(mut self, recursive: bool) -> Self {
        self.recursive = recursive;
        self
    }

    /// Builder method to set `max_depth`.
    pub fn with_max_depth(mut self, max_depth: Option<usize>) -> Self {
        self.max_depth = max_depth;
        self
    }

    /// Builder method to set sorting field.
    pub fn with_sort_by(mut self, sort_by: SortBy) -> Self {
        self.sort_by = sort_by;
        self
    }

    /// Builder method to set sort order direction.
    pub fn with_sort_order(mut self, sort_order: SortOrder) -> Self {
        self.sort_order = sort_order;
        self
    }

    /// Builder method to set `directories_first`.
    pub fn with_directories_first(mut self, directories_first: bool) -> Self {
        self.directories_first = directories_first;
        self
    }
}

/// High-performance asynchronous directory scanner and sorter.
#[derive(Debug, Clone, Default)]
pub struct DirectoryScanner {
    options: ScanOptions,
}

impl DirectoryScanner {
    /// Creates a new `DirectoryScanner` with specified configuration options.
    pub fn new(options: ScanOptions) -> Self {
        Self { options }
    }

    /// Returns a reference to current scanner options.
    pub fn options(&self) -> &ScanOptions {
        &self.options
    }

    /// Returns a mutable reference to current scanner options.
    pub fn options_mut(&mut self) -> &mut ScanOptions {
        &mut self.options
    }

    /// Asynchronously scans the target directory using configured options.
    pub async fn scan<P: AsRef<Path>>(&self, path: P) -> std::io::Result<Vec<FileEntry>> {
        Self::scan_dir(path, &self.options).await
    }

    /// Static helper method to asynchronously scan a directory with custom `ScanOptions`.
    pub async fn scan_dir<P: AsRef<Path>>(
        path: P,
        options: &ScanOptions,
    ) -> std::io::Result<Vec<FileEntry>> {
        let root = path.as_ref().to_path_buf();
        if !root.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Path does not exist: {}", root.display()),
            ));
        }

        let mut entries = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back((root, 0usize));

        while let Some((dir_path, depth)) = queue.pop_front() {
            let mut read_dir = match tokio::fs::read_dir(&dir_path).await {
                Ok(rd) => rd,
                Err(err) => {
                    // Skip unreadable directories during recursive scan unless it's root
                    if queue.is_empty() && entries.is_empty() && depth == 0 {
                        return Err(err);
                    }
                    continue;
                }
            };

            while let Ok(Some(entry)) = read_dir.next_entry().await {
                let entry_path = entry.path();
                let file_name = entry.file_name().to_string_lossy().to_string();

                let is_hidden = file_name.starts_with('.');
                if is_hidden && !options.show_hidden {
                    continue;
                }

                let metadata_result = if options.follow_symlinks {
                    tokio::fs::metadata(&entry_path).await
                } else {
                    tokio::fs::symlink_metadata(&entry_path).await
                };

                let metadata = match metadata_result {
                    Ok(m) => m,
                    Err(_) => continue,
                };

                let raw_ft = entry.file_type().await.ok();
                let file_type = match raw_ft {
                    Some(ft) if ft.is_dir() => FileType::Directory,
                    Some(ft) if ft.is_file() => FileType::File,
                    Some(ft) if ft.is_symlink() => FileType::Symlink,
                    _ => {
                        if metadata.is_dir() {
                            FileType::Directory
                        } else if metadata.is_file() {
                            FileType::File
                        } else if metadata.is_symlink() {
                            FileType::Symlink
                        } else {
                            FileType::Other
                        }
                    }
                };

                let modified = metadata.modified().ok().map(DateTime::from);
                let created = metadata.created().ok().map(DateTime::from);
                let extension = entry_path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|s| s.to_lowercase());
                let readonly = metadata.permissions().readonly();

                let file_entry = FileEntry {
                    name: file_name,
                    path: entry_path.clone(),
                    file_type,
                    size: metadata.len(),
                    modified,
                    created,
                    is_hidden,
                    extension,
                    readonly,
                };

                // Check if we should descend recursively into subdirectories
                if options.recursive && file_entry.is_dir() {
                    let can_descend = match options.max_depth {
                        Some(max_d) => depth + 1 <= max_d,
                        None => true,
                    };
                    if can_descend {
                        queue.push_back((entry_path, depth + 1));
                    }
                }

                entries.push(file_entry);
            }
        }

        Self::sort_entries(&mut entries, options);
        Ok(entries)
    }

    /// Sorts a slice of `FileEntry` in-place based on `ScanOptions`.
    pub fn sort_entries(entries: &mut [FileEntry], options: &ScanOptions) {
        entries.sort_by(|a, b| {
            if options.directories_first {
                let a_is_dir = a.is_dir();
                let b_is_dir = b.is_dir();
                if a_is_dir && !b_is_dir {
                    return std::cmp::Ordering::Less;
                } else if !a_is_dir && b_is_dir {
                    return std::cmp::Ordering::Greater;
                }
            }

            let primary_cmp = match options.sort_by {
                SortBy::Name => {
                    if options.case_sensitive_sort {
                        a.name.cmp(&b.name)
                    } else {
                        a.name.to_lowercase().cmp(&b.name.to_lowercase())
                    }
                }
                SortBy::Size => a.size.cmp(&b.size),
                SortBy::Modified => a.modified.cmp(&b.modified),
                SortBy::Created => a.created.cmp(&b.created),
                SortBy::Extension => {
                    let ext_a = a.extension.as_deref().unwrap_or("");
                    let ext_b = b.extension.as_deref().unwrap_or("");
                    let cmp = ext_a.cmp(ext_b);
                    if cmp == std::cmp::Ordering::Equal {
                        a.name.to_lowercase().cmp(&b.name.to_lowercase())
                    } else {
                        cmp
                    }
                }
                SortBy::FileType => {
                    let ft_val = |ft: &FileType| match ft {
                        FileType::Directory => 0,
                        FileType::Symlink => 1,
                        FileType::File => 2,
                        FileType::Other => 3,
                    };
                    let cmp = ft_val(&a.file_type).cmp(&ft_val(&b.file_type));
                    if cmp == std::cmp::Ordering::Equal {
                        a.name.to_lowercase().cmp(&b.name.to_lowercase())
                    } else {
                        cmp
                    }
                }
            };

            let cmp = match options.sort_order {
                SortOrder::Ascending => primary_cmp,
                SortOrder::Descending => primary_cmp.reverse(),
            };

            if cmp == std::cmp::Ordering::Equal {
                a.name.cmp(&b.name)
            } else {
                cmp
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use tokio::fs::{create_dir, write};

    #[tokio::test]
    async fn test_scan_dir_basic() {
        let dir = tempdir().unwrap();
        let dir_path = dir.path();

        write(dir_path.join("file_b.txt"), "hello").await.unwrap();
        write(dir_path.join("file_a.txt"), "world!").await.unwrap();
        create_dir(dir_path.join("subdir")).await.unwrap();

        let scanner = DirectoryScanner::default();
        let entries = scanner.scan(dir_path).await.unwrap();

        // Default: directories first, sorted by name ascending, hidden excluded
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].name, "subdir");
        assert!(entries[0].is_dir());

        assert_eq!(entries[1].name, "file_a.txt");
        assert_eq!(entries[2].name, "file_b.txt");
    }

    #[tokio::test]
    async fn test_hidden_file_filtering() {
        let dir = tempdir().unwrap();
        let dir_path = dir.path();

        write(dir_path.join("visible.txt"), "content").await.unwrap();
        write(dir_path.join(".hidden.txt"), "secret").await.unwrap();

        // Without hidden files
        let scanner_no_hidden = DirectoryScanner::new(ScanOptions::default().with_hidden(false));
        let entries = scanner_no_hidden.scan(dir_path).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "visible.txt");

        // With hidden files
        let scanner_hidden = DirectoryScanner::new(ScanOptions::default().with_hidden(true));
        let entries = scanner_hidden.scan(dir_path).await.unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[tokio::test]
    async fn test_recursive_scanning() {
        let dir = tempdir().unwrap();
        let dir_path = dir.path();

        let sub = dir_path.join("sub");
        create_dir(&sub).await.unwrap();
        write(sub.join("nested.txt"), "data").await.unwrap();
        write(dir_path.join("root.txt"), "root").await.unwrap();

        // Non-recursive
        let scanner_flat = DirectoryScanner::new(ScanOptions::default().with_recursive(false));
        let entries = scanner_flat.scan(dir_path).await.unwrap();
        assert_eq!(entries.len(), 2); // sub, root.txt

        // Recursive
        let scanner_rec = DirectoryScanner::new(ScanOptions::default().with_recursive(true));
        let entries = scanner_rec.scan(dir_path).await.unwrap();
        assert_eq!(entries.len(), 3); // sub, root.txt, nested.txt
        assert!(entries.iter().any(|e| e.name == "nested.txt"));
    }

    #[tokio::test]
    async fn test_sorting_options() {
        let dir = tempdir().unwrap();
        let dir_path = dir.path();

        write(dir_path.join("small.txt"), "a").await.unwrap();
        write(dir_path.join("large.txt"), "a long paragraph").await.unwrap();

        // Sort by size ascending
        let opts_size = ScanOptions::default()
            .with_sort_by(SortBy::Size)
            .with_sort_order(SortOrder::Ascending)
            .with_directories_first(false);
        let scanner = DirectoryScanner::new(opts_size);
        let entries = scanner.scan(dir_path).await.unwrap();

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "small.txt");
        assert_eq!(entries[1].name, "large.txt");

        // Sort by size descending
        let opts_size_desc = ScanOptions::default()
            .with_sort_by(SortBy::Size)
            .with_sort_order(SortOrder::Descending)
            .with_directories_first(false);
        let scanner_desc = DirectoryScanner::new(opts_size_desc);
        let entries_desc = scanner_desc.scan(dir_path).await.unwrap();

        assert_eq!(entries_desc[0].name, "large.txt");
        assert_eq!(entries_desc[1].name, "small.txt");
    }

    #[tokio::test]
    async fn test_non_existent_path() {
        let scanner = DirectoryScanner::default();
        let result = scanner.scan("/non/existent/path/for/swal/test").await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::NotFound);
    }

    #[tokio::test]
    async fn test_empty_directory() {
        let dir = tempdir().unwrap();
        let scanner = DirectoryScanner::default();
        let entries = scanner.scan(dir.path()).await.unwrap();
        assert!(entries.is_empty());
    }

    #[tokio::test]
    async fn test_file_type_helpers() {
        let dir_ft = FileType::Directory;
        let file_ft = FileType::File;
        let sym_ft = FileType::Symlink;

        assert!(dir_ft.is_dir());
        assert!(!dir_ft.is_file());

        assert!(file_ft.is_file());
        assert!(!file_ft.is_dir());

        assert!(sym_ft.is_symlink());
    }
}
