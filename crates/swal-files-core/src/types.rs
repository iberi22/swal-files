use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fs::Metadata;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Represents the high-level category of a filesystem node.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FileType {
    File,
    Directory,
    Symlink { target: Option<PathBuf> },
    Special,
}

impl FileType {
    pub fn is_file(&self) -> bool {
        matches!(self, FileType::File)
    }

    pub fn is_dir(&self) -> bool {
        matches!(self, FileType::Directory)
    }

    pub fn is_symlink(&self) -> bool {
        matches!(self, FileType::Symlink { .. })
    }

    pub fn is_special(&self) -> bool {
        matches!(self, FileType::Special)
    }
}

/// Categorizes a file by format or role for icons and QuickLook preview dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FileKind {
    Image,
    Audio,
    Video,
    Document,
    Code,
    Archive,
    Directory,
    Symlink,
    Unknown,
}

impl FileKind {
    /// Determines the `FileKind` from a file path and file type.
    pub fn from_path_and_type(path: &Path, file_type: &FileType) -> Self {
        if file_type.is_dir() {
            return FileKind::Directory;
        }
        if file_type.is_symlink() {
            return FileKind::Symlink;
        }

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        match ext.as_str() {
            // Images
            "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "bmp" | "ico" | "tiff" | "avif" => {
                FileKind::Image
            }
            // Audio
            "mp3" | "flac" | "wav" | "aac" | "ogg" | "m4a" | "opus" => FileKind::Audio,
            // Video
            "mp4" | "mkv" | "webm" | "avi" | "mov" | "flv" | "wmv" => FileKind::Video,
            // Documents
            "pdf" | "md" | "markdown" | "txt" | "doc" | "docx" | "odt" | "epub" | "rtf" => {
                FileKind::Document
            }
            // Source Code / Config
            "rs" | "js" | "ts" | "jsx" | "tsx" | "py" | "c" | "cpp" | "h" | "hpp" | "go" | "java"
            | "json" | "toml" | "yaml" | "yml" | "html" | "css" | "sh" | "bash" | "zsh" | "sql"
            | "xml" => FileKind::Code,
            // Archives
            "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" | "zst" => FileKind::Archive,
            _ => FileKind::Unknown,
        }
    }
}

/// File permissions metadata across OS platforms.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilePermissions {
    pub readonly: bool,
    pub mode: Option<u32>,
}

impl FilePermissions {
    pub fn new(readonly: bool, mode: Option<u32>) -> Self {
        Self { readonly, mode }
    }

    /// Formats unix mode as a POSIX permissions string (e.g., `rwxr-xr-x` or `rw-r--r--`).
    pub fn to_mode_string(&self) -> String {
        if let Some(mode) = self.mode {
            let user = format!(
                "{}{}{}",
                if mode & 0o400 != 0 { 'r' } else { '-' },
                if mode & 0o200 != 0 { 'w' } else { '-' },
                if mode & 0o100 != 0 { 'x' } else { '-' }
            );
            let group = format!(
                "{}{}{}",
                if mode & 0o040 != 0 { 'r' } else { '-' },
                if mode & 0o020 != 0 { 'w' } else { '-' },
                if mode & 0o010 != 0 { 'x' } else { '-' }
            );
            let other = format!(
                "{}{}{}",
                if mode & 0o004 != 0 { 'r' } else { '-' },
                if mode & 0o002 != 0 { 'w' } else { '-' },
                if mode & 0o001 != 0 { 'x' } else { '-' }
            );
            format!("{}{}{}", user, group, other)
        } else if self.readonly {
            "r--r--r--".to_string()
        } else {
            "rw-r--r--".to_string()
        }
    }
}

/// Complete detailed metadata for a file entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileMetadata {
    pub size: u64,
    pub created: Option<DateTime<Utc>>,
    pub modified: Option<DateTime<Utc>>,
    pub accessed: Option<DateTime<Utc>>,
    pub permissions: FilePermissions,
    pub is_hidden: bool,
}

impl FileMetadata {
    /// Formats size into a human-readable string (e.g. "500 B", "12.4 KB", "3.1 MB", "1.5 GB").
    pub fn formatted_size(&self) -> String {
        format_bytes(self.size)
    }

    /// Creates `FileMetadata` from standard library `Metadata` and target `Path`.
    pub fn from_std_metadata(std_meta: &Metadata, path: &Path) -> Self {
        let size = std_meta.len();
        let created = std_meta.created().ok().map(system_time_to_datetime);
        let modified = std_meta.modified().ok().map(system_time_to_datetime);
        let accessed = std_meta.accessed().ok().map(system_time_to_datetime);

        #[cfg(unix)]
        let mode = {
            use std::os::unix::fs::PermissionsExt;
            Some(std_meta.permissions().mode())
        };
        #[cfg(not(unix))]
        let mode = None;

        let permissions = FilePermissions::new(std_meta.permissions().readonly(), mode);
        let is_hidden = is_path_hidden(path);

        Self {
            size,
            created,
            modified,
            accessed,
            permissions,
            is_hidden,
        }
    }
}

/// Represents a file entry in the virtual filesystem and file views.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: PathBuf,
    pub name: String,
    pub file_type: FileType,
    pub file_kind: FileKind,
    pub metadata: FileMetadata,
    pub git_status: Option<String>,
    pub ai_tags: Vec<String>,
}

impl FileEntry {
    /// Constructs a `FileEntry` synchronously from a path.
    pub fn from_path(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let symlink_meta = std::fs::symlink_metadata(&path)?;
        let file_type = resolve_file_type(&path, &symlink_meta);
        let std_meta = std::fs::metadata(&path).unwrap_or_else(|_| symlink_meta.clone());
        let metadata = FileMetadata::from_std_metadata(&std_meta, &path);
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();
        let file_kind = FileKind::from_path_and_type(&path, &file_type);

        Ok(Self {
            path,
            name,
            file_type,
            file_kind,
            metadata,
            git_status: None,
            ai_tags: Vec::new(),
        })
    }

    /// Constructs a `FileEntry` asynchronously using Tokio filesystem primitives.
    pub async fn from_path_async(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let symlink_meta = tokio::fs::symlink_metadata(&path).await?;
        let file_type = resolve_file_type(&path, &symlink_meta);
        let std_meta = match tokio::fs::metadata(&path).await {
            Ok(m) => m,
            Err(_) => symlink_meta.clone(),
        };
        let metadata = FileMetadata::from_std_metadata(&std_meta, &path);
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();
        let file_kind = FileKind::from_path_and_type(&path, &file_type);

        Ok(Self {
            path,
            name,
            file_type,
            file_kind,
            metadata,
            git_status: None,
            ai_tags: Vec::new(),
        })
    }

    /// Constructs a `FileEntry` directly from a path and `Metadata`.
    pub fn from_std(path: PathBuf, std_meta: Metadata) -> Self {
        let file_type = resolve_file_type(&path, &std_meta);
        let metadata = FileMetadata::from_std_metadata(&std_meta, &path);
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();
        let file_kind = FileKind::from_path_and_type(&path, &file_type);

        Self {
            path,
            name,
            file_type,
            file_kind,
            metadata,
            git_status: None,
            ai_tags: Vec::new(),
        }
    }

    pub fn extension(&self) -> Option<&str> {
        self.path.extension().and_then(|e| e.to_str())
    }

    pub fn is_hidden(&self) -> bool {
        self.metadata.is_hidden
    }

    pub fn is_dir(&self) -> bool {
        self.file_type.is_dir()
    }

    pub fn is_file(&self) -> bool {
        self.file_type.is_file()
    }

    pub fn is_symlink(&self) -> bool {
        self.file_type.is_symlink()
    }
}

/// Criteria by which file entries can be sorted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileSortKey {
    Name,
    Size,
    Modified,
    Extension,
    Kind,
}

/// Direction of sorting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortDirection {
    Ascending,
    Descending,
}

/// Criteria container combining key and direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SortCriteria {
    pub key: FileSortKey,
    pub direction: SortDirection,
}

impl Default for SortCriteria {
    fn default() -> Self {
        Self {
            key: FileSortKey::Name,
            direction: SortDirection::Ascending,
        }
    }
}

/// Helper function to sort a slice of `FileEntry` items in-place.
pub fn sort_entries(entries: &mut [FileEntry], criteria: SortCriteria, dirs_first: bool) {
    entries.sort_by(|a, b| {
        if dirs_first {
            match (a.is_dir(), b.is_dir()) {
                (true, false) => return Ordering::Less,
                (false, true) => return Ordering::Greater,
                _ => {}
            }
        }

        let cmp = match criteria.key {
            FileSortKey::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            FileSortKey::Size => a.metadata.size.cmp(&b.metadata.size),
            FileSortKey::Modified => a.metadata.modified.cmp(&b.metadata.modified),
            FileSortKey::Extension => a
                .extension()
                .unwrap_or("")
                .to_lowercase()
                .cmp(&b.extension().unwrap_or("").to_lowercase()),
            FileSortKey::Kind => format!("{:?}", a.file_kind).cmp(&format!("{:?}", b.file_kind)),
        };

        match criteria.direction {
            SortDirection::Ascending => cmp,
            SortDirection::Descending => cmp.reverse(),
        }
    });
}

// Internal helper functions
fn resolve_file_type(path: &Path, std_meta: &Metadata) -> FileType {
    let ft = std_meta.file_type();
    if ft.is_symlink() {
        let target = std::fs::read_link(path).ok();
        FileType::Symlink { target }
    } else if ft.is_dir() {
        FileType::Directory
    } else if ft.is_file() {
        FileType::File
    } else {
        FileType::Special
    }
}

fn system_time_to_datetime(st: SystemTime) -> DateTime<Utc> {
    DateTime::<Utc>::from(st)
}

fn is_path_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.starts_with('.'))
        .unwrap_or(false)
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.1} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_file_type_helpers() {
        let file = FileType::File;
        let dir = FileType::Directory;
        let symlink = FileType::Symlink { target: None };
        let special = FileType::Special;

        assert!(file.is_file());
        assert!(!file.is_dir());

        assert!(dir.is_dir());
        assert!(!dir.is_file());

        assert!(symlink.is_symlink());
        assert!(!symlink.is_file());

        assert!(special.is_special());
        assert!(!special.is_dir());
    }

    #[test]
    fn test_file_kind_classification() {
        let path_img = Path::new("photo.PNG");
        let path_code = Path::new("main.rs");
        let path_doc = Path::new("notes.md");
        let path_archive = Path::new("archive.tar.gz");
        let path_audio = Path::new("song.flac");
        let path_video = Path::new("movie.mp4");
        let path_unknown = Path::new("data.bin");

        let file_type = FileType::File;
        assert_eq!(FileKind::from_path_and_type(path_img, &file_type), FileKind::Image);
        assert_eq!(FileKind::from_path_and_type(path_code, &file_type), FileKind::Code);
        assert_eq!(FileKind::from_path_and_type(path_doc, &file_type), FileKind::Document);
        assert_eq!(FileKind::from_path_and_type(path_archive, &file_type), FileKind::Archive);
        assert_eq!(FileKind::from_path_and_type(path_audio, &file_type), FileKind::Audio);
        assert_eq!(FileKind::from_path_and_type(path_video, &file_type), FileKind::Video);
        assert_eq!(FileKind::from_path_and_type(path_unknown, &file_type), FileKind::Unknown);

        let dir_type = FileType::Directory;
        assert_eq!(FileKind::from_path_and_type(path_img, &dir_type), FileKind::Directory);
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1048576), "1.0 MB");
        assert_eq!(format_bytes(1073741824), "1.0 GB");
    }

    #[test]
    fn test_file_permissions_mode_string() {
        let perm_rw = FilePermissions::new(false, Some(0o644));
        assert_eq!(perm_rw.to_mode_string(), "rw-r--r--");

        let perm_rwx = FilePermissions::new(false, Some(0o755));
        assert_eq!(perm_rwx.to_mode_string(), "rwxr-xr-x");

        let perm_readonly_no_mode = FilePermissions::new(true, None);
        assert_eq!(perm_readonly_no_mode.to_mode_string(), "r--r--r--");
    }

    #[test]
    fn test_file_entry_sync_creation() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_file.txt");
        {
            let mut f = std::fs::File::create(&file_path).unwrap();
            writeln!(f, "Hello SWAL Files!").unwrap();
        }

        let entry = FileEntry::from_path(&file_path).unwrap();
        assert_eq!(entry.name, "test_file.txt");
        assert!(entry.is_file());
        assert_eq!(entry.file_kind, FileKind::Document);
        assert!(!entry.is_hidden());
        assert!(entry.metadata.size > 0);
        assert_eq!(entry.extension(), Some("txt"));
    }

    #[tokio::test]
    async fn test_file_entry_async_creation() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join(".hidden_config.json");
        {
            let mut f = std::fs::File::create(&file_path).unwrap();
            writeln!(f, "{{\"key\": \"val\"}}").unwrap();
        }

        let entry = FileEntry::from_path_async(&file_path).await.unwrap();
        assert_eq!(entry.name, ".hidden_config.json");
        assert!(entry.is_file());
        assert_eq!(entry.file_kind, FileKind::Code);
        assert!(entry.is_hidden());
        assert_eq!(entry.extension(), Some("json"));
    }

    #[test]
    fn test_file_entry_serde() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("sample.rs");
        {
            let mut f = std::fs::File::create(&file_path).unwrap();
            writeln!(f, "fn main() {{}}").unwrap();
        }

        let entry = FileEntry::from_path(&file_path).unwrap();
        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: FileEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(entry.name, deserialized.name);
        assert_eq!(entry.file_kind, deserialized.file_kind);
        assert_eq!(entry.metadata.size, deserialized.metadata.size);
    }

    #[test]
    fn test_sort_entries() {
        let dir = tempdir().unwrap();
        let file_a = dir.path().join("b_file.txt");
        let file_b = dir.path().join("a_file.rs");
        let sub_dir = dir.path().join("z_dir");

        std::fs::File::create(&file_a).unwrap();
        std::fs::File::create(&file_b).unwrap();
        std::fs::create_dir(&sub_dir).unwrap();

        let mut entries = vec![
            FileEntry::from_path(&file_a).unwrap(),
            FileEntry::from_path(&file_b).unwrap(),
            FileEntry::from_path(&sub_dir).unwrap(),
        ];

        // Sort with dirs_first = true, Name Ascending
        sort_entries(
            &mut entries,
            SortCriteria {
                key: FileSortKey::Name,
                direction: SortDirection::Ascending,
            },
            true,
        );

        assert_eq!(entries[0].name, "z_dir");
        assert_eq!(entries[1].name, "a_file.rs");
        assert_eq!(entries[2].name, "b_file.txt");

        // Sort with dirs_first = false, Name Ascending
        sort_entries(
            &mut entries,
            SortCriteria {
                key: FileSortKey::Name,
                direction: SortDirection::Ascending,
            },
            false,
        );

        assert_eq!(entries[0].name, "a_file.rs");
        assert_eq!(entries[1].name, "b_file.txt");
        assert_eq!(entries[2].name, "z_dir");
    }
}
