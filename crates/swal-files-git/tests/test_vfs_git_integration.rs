#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use swal_files_git::status::{read_working_tree_status, GitStatusKind, GitWorkingTreeStatus};
use tempfile::TempDir;
use tokio::fs;
use tokio::process::Command;

/// Scanned file entry representing virtual filesystem scanner output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedFileEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
}

/// Fixture for setting up a temporary filesystem directory with Git capabilities
/// to test VFS Scanner and Git Engine integration.
pub struct VfsGitIntegrationFixture {
    temp_dir: TempDir,
}

impl VfsGitIntegrationFixture {
    /// Creates a new test fixture with a fresh git repo initialized.
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let path = temp_dir.path();

        let status = Command::new("git")
            .arg("init")
            .current_dir(path)
            .status()
            .await?;
        if !status.success() {
            return Err("Failed to execute git init".into());
        }

        Command::new("git")
            .args(["config", "user.name", "Integration Test"])
            .current_dir(path)
            .status()
            .await?;

        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(path)
            .status()
            .await?;

        Ok(Self { temp_dir })
    }

    /// Returns the root path of the temporary directory.
    pub fn path(&self) -> &Path {
        self.temp_dir.path()
    }

    /// Writes content to a relative file path inside the fixture repo.
    pub async fn create_file(&self, rel_path: impl AsRef<Path>, content: &str) -> std::io::Result<PathBuf> {
        let full_path = self.path().join(rel_path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).await?;
        }
        fs::write(&full_path, content).await?;
        Ok(full_path)
    }

    /// Stage a file path in git index.
    pub async fn git_add(&self, rel_path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
        let status = Command::new("git")
            .arg("add")
            .arg(rel_path.as_ref())
            .current_dir(self.path())
            .status()
            .await?;
        if !status.success() {
            return Err("git add failed".into());
        }
        Ok(())
    }

    /// Create a commit with the specified message.
    pub async fn git_commit(&self, message: &str) -> Result<(), Box<dyn std::error::Error>> {
        let status = Command::new("git")
            .args(["commit", "-m", message])
            .current_dir(self.path())
            .status()
            .await?;
        if !status.success() {
            return Err("git commit failed".into());
        }
        Ok(())
    }

    /// Recursively scans the VFS workspace tree and collects file entries.
    pub async fn scan_vfs_tree(&self) -> Result<Vec<ScannedFileEntry>, Box<dyn std::error::Error>> {
        let mut entries = Vec::new();
        let mut stack = vec![self.path().to_path_buf()];

        while let Some(dir) = stack.pop() {
            let mut read_dir = fs::read_dir(&dir).await?;
            while let Some(entry) = read_dir.next_entry().await? {
                let entry_path = entry.path();
                let file_name = entry.file_name().to_string_lossy().to_string();

                // Skip .git directory in VFS scanner
                if file_name == ".git" {
                    continue;
                }

                let metadata = entry.metadata().await?;
                let is_dir = metadata.is_dir();

                entries.push(ScannedFileEntry {
                    name: file_name,
                    path: entry_path.clone(),
                    is_dir,
                    size: metadata.len(),
                });

                if is_dir {
                    stack.push(entry_path);
                }
            }
        }

        Ok(entries)
    }

    /// Scans the workspace and reads git working tree status.
    pub async fn scan_and_get_git_status(
        &self,
    ) -> Result<(Vec<ScannedFileEntry>, GitWorkingTreeStatus), Box<dyn std::error::Error>> {
        let entries = self.scan_vfs_tree().await?;
        let git_status = read_working_tree_status(self.path()).await?;
        Ok((entries, git_status))
    }
}

#[tokio::test]
async fn test_vfs_git_fixture_creation() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = VfsGitIntegrationFixture::new().await?;
    let status = read_working_tree_status(fixture.path()).await?;
    assert!(status.is_clean());
    Ok(())
}

#[tokio::test]
async fn test_scanner_with_git_status() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = VfsGitIntegrationFixture::new().await?;

    // Create and commit a clean file
    fixture.create_file("clean_file.txt", "Initial content").await?;
    fixture.git_add("clean_file.txt").await?;
    fixture.git_commit("Initial commit").await?;

    // Create untracked file
    fixture.create_file("untracked_file.txt", "Untracked data").await?;

    // Create and stage a file
    fixture.create_file("staged_file.txt", "Staged data").await?;
    fixture.git_add("staged_file.txt").await?;

    let (entries, git_status) = fixture.scan_and_get_git_status().await?;

    assert_eq!(entries.len(), 3);
    assert!(!git_status.is_clean());

    let untracked_status = git_status.get_status_for_path("untracked_file.txt");
    assert!(untracked_status.is_some());
    assert_eq!(untracked_status.unwrap().worktree_status, GitStatusKind::Untracked);

    let staged_status = git_status.get_status_for_path("staged_file.txt");
    assert!(staged_status.is_some());
    assert_eq!(staged_status.unwrap().index_status, GitStatusKind::Added);

    Ok(())
}

#[tokio::test]
async fn test_scanner_recursive_nested_git_dirty_clean() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = VfsGitIntegrationFixture::new().await?;

    // Setup nested tree:
    // nested/dir/clean.txt (committed)
    // nested/dir/modified.txt (committed then modified -> dirty)
    // nested/dir/untracked.txt (untracked -> dirty)
    fixture.create_file("nested/dir/clean.txt", "Clean content").await?;
    fixture.create_file("nested/dir/modified.txt", "Original content").await?;
    fixture.git_add(".").await?;
    fixture.git_commit("Commit nested baseline").await?;

    // Modify file to make dirty
    fixture.create_file("nested/dir/modified.txt", "Modified content!").await?;
    fixture.create_file("nested/dir/untracked.txt", "New untracked").await?;

    let (entries, git_status) = fixture.scan_and_get_git_status().await?;

    // Ensure VFS scanner found nested entries
    assert!(entries.iter().any(|e| e.name == "clean.txt"));
    assert!(entries.iter().any(|e| e.name == "modified.txt"));
    assert!(entries.iter().any(|e| e.name == "untracked.txt"));

    // Check git statuses across nested directories
    let mod_status = git_status.get_status_for_path("nested/dir/modified.txt");
    assert!(mod_status.is_some());
    assert_eq!(mod_status.unwrap().worktree_status, GitStatusKind::Modified);

    let clean_status = git_status.get_status_for_path("nested/dir/clean.txt");
    assert!(clean_status.is_none() || clean_status.unwrap().worktree_status == GitStatusKind::Unmodified);

    assert_eq!(git_status.untracked().len(), 1);
    assert_eq!(git_status.unstaged().len(), 1);

    Ok(())
}

#[tokio::test]
async fn test_scanner_git_ignored_files() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = VfsGitIntegrationFixture::new().await?;

    fixture.create_file(".gitignore", "*.log\n").await?;
    fixture.create_file("app.log", "some log entry").await?;
    fixture.create_file("main.rs", "fn main() {}").await?;

    let (entries, git_status) = fixture.scan_and_get_git_status().await?;

    assert!(entries.iter().any(|e| e.name == "app.log"));
    let log_status = git_status.get_status_for_path("app.log");
    if let Some(st) = log_status {
        assert_eq!(st.worktree_status, GitStatusKind::Ignored);
    }

    Ok(())
}
