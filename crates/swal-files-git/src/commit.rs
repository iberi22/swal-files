use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};
use tokio::process::Command;

/// Author or committer identity signature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Signature {
    pub name: String,
    pub email: String,
    pub timestamp: Option<i64>,
}

impl Signature {
    pub fn new(name: impl Into<String>, email: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            email: email.into(),
            timestamp: None,
        }
    }

    pub fn with_timestamp(mut self, timestamp: i64) -> Self {
        self.timestamp = Some(timestamp);
        self
    }
}

/// Staging operation type for Git stage operator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StageOperation {
    /// Stage a specific file or directory (`git add <path>`)
    StagePath(PathBuf),
    /// Unstage a specific file or directory (`git restore --staged <path>` or `git rm --cached -r <path>`)
    UnstagePath(PathBuf),
    /// Stage all tracked and untracked changes (`git add -A`)
    StageAll,
    /// Unstage all staged changes (`git reset`)
    UnstageAll,
    /// Discard unstaged changes in working directory (`git restore <path>` or `git clean -fd <path>`)
    DiscardChanges(PathBuf),
}

/// Options for configuring a commit operation.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CommitOptions {
    pub message: String,
    pub author: Option<Signature>,
    pub committer: Option<Signature>,
    pub amend: bool,
    pub allow_empty: bool,
    pub sign_gpg: bool,
}

/// Information returned after a successful commit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitResult {
    pub commit_hash: String,
    pub summary: String,
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
    pub branch: Option<String>,
}

/// Errors that can occur during staging or commit operations.
#[derive(Debug)]
pub enum GitCommitError {
    EmptyCommitMessage,
    GitExecutionFailed(String),
    IoError(std::io::Error),
    RepositoryNotFound(PathBuf),
}

impl fmt::Display for GitCommitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GitCommitError::EmptyCommitMessage => write!(f, "Commit message cannot be empty"),
            GitCommitError::GitExecutionFailed(msg) => write!(f, "Git command failed: {}", msg),
            GitCommitError::IoError(err) => write!(f, "IO error: {}", err),
            GitCommitError::RepositoryNotFound(path) => {
                write!(f, "Git repository not found at path: {}", path.display())
            }
        }
    }
}

impl std::error::Error for GitCommitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            GitCommitError::IoError(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for GitCommitError {
    fn from(err: std::io::Error) -> Self {
        GitCommitError::IoError(err)
    }
}

/// Non-blocking Git stage operator.
#[derive(Debug, Clone, Default)]
pub struct GitStageOperator;

impl GitStageOperator {
    pub fn new() -> Self {
        Self
    }

    /// Executes a stage operation asynchronously on a Git repository path.
    pub async fn execute(
        &self,
        repo_path: impl AsRef<Path>,
        op: StageOperation,
    ) -> Result<(), GitCommitError> {
        let repo_path = repo_path.as_ref();
        if !repo_path.exists() {
            return Err(GitCommitError::RepositoryNotFound(repo_path.to_path_buf()));
        }

        match op {
            StageOperation::StagePath(path) => {
                let output = Command::new("git")
                    .current_dir(repo_path)
                    .args(["add", "--", path.to_str().unwrap_or_default()])
                    .output()
                    .await?;

                if !output.status.success() {
                    let err_msg = String::from_utf8_lossy(&output.stderr);
                    return Err(GitCommitError::GitExecutionFailed(err_msg.trim().to_string()));
                }
            }
            StageOperation::UnstagePath(path) => {
                let path_str = path.to_str().unwrap_or_default();
                let output = Command::new("git")
                    .current_dir(repo_path)
                    .args(["restore", "--staged", "--", path_str])
                    .output()
                    .await?;

                if !output.status.success() {
                    let output_reset = Command::new("git")
                        .current_dir(repo_path)
                        .args(["rm", "--cached", "-r", "--quiet", "--", path_str])
                        .output()
                        .await?;

                    if !output_reset.status.success() {
                        let err_msg = String::from_utf8_lossy(&output.stderr);
                        return Err(GitCommitError::GitExecutionFailed(err_msg.trim().to_string()));
                    }
                }
            }
            StageOperation::StageAll => {
                let output = Command::new("git")
                    .current_dir(repo_path)
                    .args(["add", "-A"])
                    .output()
                    .await?;

                if !output.status.success() {
                    let err_msg = String::from_utf8_lossy(&output.stderr);
                    return Err(GitCommitError::GitExecutionFailed(err_msg.trim().to_string()));
                }
            }
            StageOperation::UnstageAll => {
                let output = Command::new("git")
                    .current_dir(repo_path)
                    .args(["reset"])
                    .output()
                    .await?;

                if !output.status.success() {
                    let err_msg = String::from_utf8_lossy(&output.stderr);
                    return Err(GitCommitError::GitExecutionFailed(err_msg.trim().to_string()));
                }
            }
            StageOperation::DiscardChanges(path) => {
                let path_str = path.to_str().unwrap_or_default();
                let output = Command::new("git")
                    .current_dir(repo_path)
                    .args(["restore", "--", path_str])
                    .output()
                    .await?;

                if !output.status.success() {
                    let err_msg = String::from_utf8_lossy(&output.stderr);
                    return Err(GitCommitError::GitExecutionFailed(err_msg.trim().to_string()));
                }

                // If path is untracked directory/file, clean it
                let clean_output = Command::new("git")
                    .current_dir(repo_path)
                    .args(["clean", "-fd", "--", path_str])
                    .output()
                    .await?;

                if !clean_output.status.success() {
                    let err_msg = String::from_utf8_lossy(&clean_output.stderr);
                    return Err(GitCommitError::GitExecutionFailed(err_msg.trim().to_string()));
                }
            }
        }

        Ok(())
    }
}

/// Builder for constructing and executing git commits.
#[derive(Debug, Clone, Default)]
pub struct CommitBuilder {
    options: CommitOptions,
}

impl CommitBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.options.message = message.into();
        self
    }

    pub fn author(mut self, author: Signature) -> Self {
        self.options.author = Some(author);
        self
    }

    pub fn committer(mut self, committer: Signature) -> Self {
        self.options.committer = Some(committer);
        self
    }

    pub fn amend(mut self, amend: bool) -> Self {
        self.options.amend = amend;
        self
    }

    pub fn allow_empty(mut self, allow_empty: bool) -> Self {
        self.options.allow_empty = allow_empty;
        self
    }

    pub fn sign_gpg(mut self, sign_gpg: bool) -> Self {
        self.options.sign_gpg = sign_gpg;
        self
    }

    /// Executes the commit asynchronously against the given repository path.
    pub async fn commit(&self, repo_path: impl AsRef<Path>) -> Result<CommitResult, GitCommitError> {
        let repo_path = repo_path.as_ref();
        if !repo_path.exists() {
            return Err(GitCommitError::RepositoryNotFound(repo_path.to_path_buf()));
        }

        let message = self.options.message.trim();
        if message.is_empty() {
            return Err(GitCommitError::EmptyCommitMessage);
        }

        let mut cmd = Command::new("git");
        cmd.current_dir(repo_path);
        cmd.args(["commit", "-m", message]);

        if self.options.amend {
            cmd.arg("--amend");
        }

        if self.options.allow_empty {
            cmd.arg("--allow-empty");
        }

        if self.options.sign_gpg {
            cmd.arg("-S");
        } else {
            cmd.arg("--no-gpg-sign");
        }

        if let Some(ref author) = self.options.author {
            let author_str = format!("{} <{}>", author.name, author.email);
            cmd.arg("--author").arg(author_str);
            if let Some(ts) = author.timestamp {
                cmd.env("GIT_AUTHOR_DATE", format!("{} +0000", ts));
            }
        }

        if let Some(ref committer) = self.options.committer {
            cmd.env("GIT_COMMITTER_NAME", &committer.name);
            cmd.env("GIT_COMMITTER_EMAIL", &committer.email);
            if let Some(ts) = committer.timestamp {
                cmd.env("GIT_COMMITTER_DATE", format!("{} +0000", ts));
            }
        }

        let output = cmd.output().await?;

        if !output.status.success() {
            let err_msg = String::from_utf8_lossy(&output.stderr);
            return Err(GitCommitError::GitExecutionFailed(err_msg.trim().to_string()));
        }

        // Get created/amended commit hash
        let hash_output = Command::new("git")
            .current_dir(repo_path)
            .args(["rev-parse", "HEAD"])
            .output()
            .await?;

        let commit_hash = if hash_output.status.success() {
            String::from_utf8_lossy(&hash_output.stdout).trim().to_string()
        } else {
            String::new()
        };

        // Get active branch name
        let branch_output = Command::new("git")
            .current_dir(repo_path)
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .output()
            .await?;

        let branch = if branch_output.status.success() {
            let b = String::from_utf8_lossy(&branch_output.stdout).trim().to_string();
            if b.is_empty() || b == "HEAD" {
                None
            } else {
                Some(b)
            }
        } else {
            None
        };

        // Parse stats from stdout / diff
        let stdout = String::from_utf8_lossy(&output.stdout);
        let (files_changed, insertions, deletions) = parse_commit_stats(&stdout);

        let summary = message
            .lines()
            .next()
            .unwrap_or_default()
            .to_string();

        Ok(CommitResult {
            commit_hash,
            summary,
            files_changed,
            insertions,
            deletions,
            branch,
        })
    }
}

/// Helper to parse stats from `git commit` stdout output.
fn parse_commit_stats(stdout: &str) -> (usize, usize, usize) {
    let mut files_changed = 0;
    let mut insertions = 0;
    let mut deletions = 0;

    for line in stdout.lines() {
        if line.contains("file changed") || line.contains("files changed") {
            for part in line.split(',') {
                let part = part.trim();
                if part.contains("file changed") || part.contains("files changed") {
                    if let Some(num_str) = part.split_whitespace().next() {
                        files_changed = num_str.parse().unwrap_or(0);
                    }
                } else if part.contains("insertion") {
                    if let Some(num_str) = part.split_whitespace().next() {
                        insertions = num_str.parse().unwrap_or(0);
                    }
                } else if part.contains("deletion") {
                    if let Some(num_str) = part.split_whitespace().next() {
                        deletions = num_str.parse().unwrap_or(0);
                    }
                }
            }
        }
    }

    (files_changed, insertions, deletions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use tokio::fs;

    async fn setup_git_repo() -> tempfile::TempDir {
        let dir = tempdir().expect("Failed to create temp dir");
        let path = dir.path();

        Command::new("git")
            .current_dir(path)
            .args(["init"])
            .output()
            .await
            .expect("git init failed");

        Command::new("git")
            .current_dir(path)
            .args(["config", "user.name", "Test User"])
            .output()
            .await
            .expect("git config user.name failed");

        Command::new("git")
            .current_dir(path)
            .args(["config", "user.email", "test@example.com"])
            .output()
            .await
            .expect("git config user.email failed");

        dir
    }

    #[tokio::test]
    async fn test_stage_and_unstage_operations() {
        let dir = setup_git_repo().await;
        let path = dir.path();
        let operator = GitStageOperator::new();

        let file1 = path.join("file1.txt");
        let file2 = path.join("file2.txt");

        fs::write(&file1, "hello 1").await.unwrap();
        fs::write(&file2, "hello 2").await.unwrap();

        // Test StagePath
        operator
            .execute(path, StageOperation::StagePath(PathBuf::from("file1.txt")))
            .await
            .unwrap();

        let status = Command::new("git")
            .current_dir(path)
            .args(["status", "--porcelain"])
            .output()
            .await
            .unwrap();
        let status_str = String::from_utf8_lossy(&status.stdout);
        assert!(status_str.contains("A  file1.txt"));
        assert!(status_str.contains("?? file2.txt"));

        // Test StageAll
        operator.execute(path, StageOperation::StageAll).await.unwrap();
        let status = Command::new("git")
            .current_dir(path)
            .args(["status", "--porcelain"])
            .output()
            .await
            .unwrap();
        let status_str = String::from_utf8_lossy(&status.stdout);
        assert!(status_str.contains("A  file1.txt"));
        assert!(status_str.contains("A  file2.txt"));

        // Test UnstagePath
        operator
            .execute(path, StageOperation::UnstagePath(PathBuf::from("file2.txt")))
            .await
            .unwrap();
        let status = Command::new("git")
            .current_dir(path)
            .args(["status", "--porcelain"])
            .output()
            .await
            .unwrap();
        let status_str = String::from_utf8_lossy(&status.stdout);
        assert!(status_str.contains("A  file1.txt"));
        assert!(status_str.contains("?? file2.txt"));

        // Test UnstageAll
        operator.execute(path, StageOperation::UnstageAll).await.unwrap();
        let status = Command::new("git")
            .current_dir(path)
            .args(["status", "--porcelain"])
            .output()
            .await
            .unwrap();
        let status_str = String::from_utf8_lossy(&status.stdout);
        assert!(status_str.contains("?? file1.txt"));
        assert!(status_str.contains("?? file2.txt"));
    }

    #[tokio::test]
    async fn test_discard_changes() {
        let dir = setup_git_repo().await;
        let path = dir.path();
        let operator = GitStageOperator::new();

        let file1 = path.join("file1.txt");
        fs::write(&file1, "initial").await.unwrap();

        operator.execute(path, StageOperation::StageAll).await.unwrap();
        let _ = CommitBuilder::new()
            .message("initial commit")
            .commit(path)
            .await
            .unwrap();

        // Modify file1
        fs::write(&file1, "modified content").await.unwrap();

        // Discard changes
        operator
            .execute(path, StageOperation::DiscardChanges(PathBuf::from("file1.txt")))
            .await
            .unwrap();

        let content = fs::read_to_string(&file1).await.unwrap();
        assert_eq!(content, "initial");
    }

    #[tokio::test]
    async fn test_commit_builder_basic() {
        let dir = setup_git_repo().await;
        let path = dir.path();
        let operator = GitStageOperator::new();

        let file = path.join("test.txt");
        fs::write(&file, "hello world").await.unwrap();

        operator.execute(path, StageOperation::StageAll).await.unwrap();

        let result = CommitBuilder::new()
            .message("feat: add test file")
            .author(Signature::new("Custom Author", "custom@example.com"))
            .commit(path)
            .await
            .unwrap();

        assert_eq!(result.summary, "feat: add test file");
        assert!(!result.commit_hash.is_empty());

        let log = Command::new("git")
            .current_dir(path)
            .args(["log", "-n", "1", "--pretty=format:%an <%ae>"])
            .output()
            .await
            .unwrap();
        let log_str = String::from_utf8_lossy(&log.stdout);
        assert_eq!(log_str, "Custom Author <custom@example.com>");
    }

    #[tokio::test]
    async fn test_commit_builder_allow_empty_and_amend() {
        let dir = setup_git_repo().await;
        let path = dir.path();

        // Empty commit allowed
        let res1 = CommitBuilder::new()
            .message("initial empty commit")
            .allow_empty(true)
            .commit(path)
            .await
            .unwrap();

        assert_eq!(res1.summary, "initial empty commit");

        // Amend commit with allow_empty
        let res2 = CommitBuilder::new()
            .message("amended commit summary")
            .amend(true)
            .allow_empty(true)
            .commit(path)
            .await
            .unwrap();

        assert_eq!(res2.summary, "amended commit summary");

        // Amend commit with actual file staged
        let file = path.join("file.txt");
        fs::write(&file, "content").await.unwrap();
        GitStageOperator::new()
            .execute(path, StageOperation::StageAll)
            .await
            .unwrap();

        let res3 = CommitBuilder::new()
            .message("amended with content")
            .amend(true)
            .commit(path)
            .await
            .unwrap();

        assert_eq!(res3.summary, "amended with content");
    }

    #[tokio::test]
    async fn test_commit_builder_errors() {
        let dir = setup_git_repo().await;
        let path = dir.path();

        // Empty message error
        let err = CommitBuilder::new().message("   ").commit(path).await.unwrap_err();

        matches!(err, GitCommitError::EmptyCommitMessage);

        // Repo not found error
        let non_existent_path = PathBuf::from("/non_existent_directory_swal_12345");
        let err2 = CommitBuilder::new()
            .message("msg")
            .commit(&non_existent_path)
            .await
            .unwrap_err();

        matches!(err2, GitCommitError::RepositoryNotFound(_));
    }
}
