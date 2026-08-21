#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};
use tokio::process::Command;

/// Information about a Git branch (local or remote).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BranchInfo {
    pub name: String,
    pub is_head: bool,
    pub is_remote: bool,
    pub commit_hash: Option<String>,
    pub upstream: Option<String>,
    pub ahead: usize,
    pub behind: usize,
}

/// Options for checking out / switching branches.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CheckoutOptions {
    pub branch_name: String,
    pub force: bool,
    pub create_if_missing: bool,
    pub start_point: Option<String>,
    pub detach: bool,
}

impl CheckoutOptions {
    pub fn new(branch_name: impl Into<String>) -> Self {
        Self { branch_name: branch_name.into(), ..Default::default() }
    }
    pub fn force(mut self, force: bool) -> Self { self.force = force; self }
    pub fn create_if_missing(mut self, create: bool) -> Self { self.create_if_missing = create; self }
    pub fn start_point(mut self, point: impl Into<String>) -> Self { self.start_point = Some(point.into()); self }
    pub fn detach(mut self, detach: bool) -> Self { self.detach = detach; self }
}

/// Options for constructing a new Git branch.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct BranchCreateOptions {
    pub name: String,
    pub start_point: Option<String>,
    pub force: bool,
    pub track: bool,
}

impl BranchCreateOptions {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), ..Default::default() }
    }
    pub fn start_point(mut self, point: impl Into<String>) -> Self { self.start_point = Some(point.into()); self }
    pub fn force(mut self, force: bool) -> Self { self.force = force; self }
    pub fn track(mut self, track: bool) -> Self { self.track = track; self }
}

/// Errors that can occur during branch operations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GitBranchError {
    BranchNotFound(String),
    BranchAlreadyExists(String),
    CannotDeleteCurrentBranch(String),
    CheckoutFailed(String),
    DeleteFailed(String),
    HasUncommittedChanges(String),
    InvalidBranchName(String),
    NotAGitRepository(PathBuf),
    GitCommandFailed { code: Option<i32>, stderr: String },
    Io(String),
}

impl fmt::Display for GitBranchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GitBranchError::BranchNotFound(n) => write!(f, "Branch not found: '{}'", n),
            GitBranchError::BranchAlreadyExists(n) => write!(f, "Branch already exists: '{}'", n),
            GitBranchError::CannotDeleteCurrentBranch(n) => write!(f, "Cannot delete active branch: '{}'", n),
            GitBranchError::CheckoutFailed(msg) => write!(f, "Checkout failed: {}", msg),
            GitBranchError::DeleteFailed(msg) => write!(f, "Delete branch failed: {}", msg),
            GitBranchError::HasUncommittedChanges(msg) => write!(f, "Uncommitted changes: {}", msg),
            GitBranchError::InvalidBranchName(msg) => write!(f, "Invalid branch name: {}", msg),
            GitBranchError::NotAGitRepository(p) => write!(f, "Not a git repo: {}", p.display()),
            GitBranchError::GitCommandFailed { code, stderr } => write!(f, "Git failed ({:?}): {}", code, stderr),
            GitBranchError::Io(msg) => write!(f, "I/O error: {}", msg),
        }
    }
}

impl std::error::Error for GitBranchError {}

/// Async non-blocking Git branch and checkout manager.
#[derive(Debug, Clone, Default)]
pub struct GitBranchManager;

impl GitBranchManager {
    pub fn new() -> Self { Self }

    /// Validates branch name according to git ref specifications.
    pub fn validate_branch_name(&self, name: &str) -> Result<(), GitBranchError> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(GitBranchError::InvalidBranchName("Empty branch name".into()));
        }
        if trimmed.starts_with('-') || trimmed.ends_with('/') || trimmed.ends_with(".lock") {
            return Err(GitBranchError::InvalidBranchName(format!("Invalid format: '{}'", name)));
        }
        if trimmed.contains("..") || trimmed.contains('~') || trimmed.contains('^') || trimmed.contains(':')
            || trimmed.contains('?') || trimmed.contains('*') || trimmed.contains('[') || trimmed.contains("@{")
            || trimmed.contains('\\') || trimmed.chars().any(|c| c.is_whitespace() || c.is_control())
        {
            return Err(GitBranchError::InvalidBranchName(format!("Invalid character: '{}'", name)));
        }
        Ok(())
    }

    async fn check_repo(&self, path: &Path) -> Result<(), GitBranchError> {
        if !path.exists() { Err(GitBranchError::NotAGitRepository(path.to_path_buf())) } else { Ok(()) }
    }

    /// Lists all local and remote branches in the repository.
    pub async fn list_branches(&self, repo_path: impl AsRef<Path>) -> Result<Vec<BranchInfo>, GitBranchError> {
        let path = repo_path.as_ref();
        self.check_repo(path).await?;
        let output = Command::new("git").current_dir(path)
            .args(["for-each-ref", "--format=%(HEAD)|%(refname)|%(objectname)|%(upstream:short)|%(upstream:track)", "refs/heads/", "refs/remotes/"])
            .output().await.map_err(|e| GitBranchError::Io(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            if stderr.to_lowercase().contains("not a git repository") {
                return Err(GitBranchError::NotAGitRepository(path.to_path_buf()));
            }
            return Err(GitBranchError::GitCommandFailed { code: output.status.code(), stderr });
        }

        let mut branches = Vec::new();
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() < 5 { continue; }
            let (head, refname, hash, upstream_str, track_str) = (parts[0].trim(), parts[1].trim(), parts[2].trim(), parts[3].trim(), parts[4].trim());
            let is_head = head == "*";
            let (name, is_remote) = if let Some(s) = refname.strip_prefix("refs/heads/") { (s.to_string(), false) }
            else if let Some(s) = refname.strip_prefix("refs/remotes/") {
                if s.ends_with("/HEAD") { continue; }
                (s.to_string(), true)
            } else { continue; };

            let commit_hash = if hash.is_empty() { None } else { Some(hash.to_string()) };
            let upstream = if upstream_str.is_empty() { None } else { Some(upstream_str.to_string()) };
            let (ahead, behind) = parse_upstream_track(track_str);
            branches.push(BranchInfo { name, is_head, is_remote, commit_hash, upstream, ahead, behind });
        }
        Ok(branches)
    }

    /// Creates a new branch according to `BranchCreateOptions`.
    pub async fn create_branch(&self, repo_path: impl AsRef<Path>, opts: &BranchCreateOptions) -> Result<String, GitBranchError> {
        let path = repo_path.as_ref();
        self.check_repo(path).await?;
        self.validate_branch_name(&opts.name)?;

        let mut cmd = Command::new("git");
        cmd.current_dir(path).arg("branch");
        if opts.force { cmd.arg("-f"); }
        if opts.track { cmd.arg("--track"); }
        cmd.arg(&opts.name);
        if let Some(ref start) = opts.start_point { cmd.arg(start); }

        let output = cmd.output().await.map_err(|e| GitBranchError::Io(e.to_string()))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            if stderr.contains("already exists") {
                return Err(GitBranchError::BranchAlreadyExists(opts.name.clone()));
            }
            return Err(GitBranchError::GitCommandFailed { code: output.status.code(), stderr });
        }
        Ok(opts.name.clone())
    }

    /// Switches / checks out a branch or commit according to `CheckoutOptions`.
    pub async fn checkout(&self, repo_path: impl AsRef<Path>, opts: &CheckoutOptions) -> Result<(), GitBranchError> {
        let path = repo_path.as_ref();
        self.check_repo(path).await?;

        if !opts.detach {
            self.validate_branch_name(&opts.branch_name)?;
        }

        let mut cmd = Command::new("git");
        cmd.current_dir(path).arg("checkout");

        if opts.create_if_missing {
            cmd.arg(if opts.force { "-B" } else { "-b" }).arg(&opts.branch_name);
            if let Some(ref start) = opts.start_point { cmd.arg(start); }
        } else {
            if opts.force { cmd.arg("--force"); }
            if opts.detach { cmd.arg("--detach"); }
            cmd.arg(&opts.branch_name);
        }

        let output = cmd.output().await.map_err(|e| GitBranchError::Io(e.to_string()))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            if stderr.contains("local changes") || stderr.contains("overwritten by checkout") {
                return Err(GitBranchError::HasUncommittedChanges(stderr.trim().to_string()));
            }
            if stderr.contains("did not match any file") || stderr.contains("pathspec") {
                return Err(GitBranchError::BranchNotFound(opts.branch_name.clone()));
            }
            return Err(GitBranchError::CheckoutFailed(stderr.trim().to_string()));
        }
        Ok(())
    }

    /// Safely deletes a local branch. Prevents deleting the currently checked out branch.
    pub async fn delete_branch(&self, repo_path: impl AsRef<Path>, branch_name: &str, force: bool) -> Result<(), GitBranchError> {
        let path = repo_path.as_ref();
        self.check_repo(path).await?;
        self.validate_branch_name(branch_name)?;

        let branches = self.list_branches(path).await?;
        if let Some(active) = branches.iter().find(|b| b.is_head) {
            if active.name == branch_name {
                return Err(GitBranchError::CannotDeleteCurrentBranch(branch_name.to_string()));
            }
        }

        let output = Command::new("git").current_dir(path)
            .args(["branch", if force { "-D" } else { "-d" }, branch_name])
            .output().await.map_err(|e| GitBranchError::Io(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            if stderr.contains("not found") {
                return Err(GitBranchError::BranchNotFound(branch_name.to_string()));
            }
            return Err(GitBranchError::DeleteFailed(stderr.trim().to_string()));
        }
        Ok(())
    }
}

fn parse_upstream_track(track: &str) -> (usize, usize) {
    let (mut ahead, mut behind) = (0, 0);
    for part in track.trim().trim_matches('[').trim_matches(']').split(',') {
        let p = part.trim();
        if let Some(r) = p.strip_prefix("ahead ") { ahead = r.parse().unwrap_or(0); }
        else if let Some(r) = p.strip_prefix("behind ") { behind = r.parse().unwrap_or(0); }
    }
    (ahead, behind)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use tokio::fs;

    async fn setup_git_repo() -> tempfile::TempDir {
        let dir = tempdir().unwrap();
        let p = dir.path();
        Command::new("git").current_dir(p).args(["init", "-b", "main"]).output().await.unwrap();
        Command::new("git").current_dir(p).args(["config", "user.name", "Test"]).output().await.unwrap();
        Command::new("git").current_dir(p).args(["config", "user.email", "test@test.com"]).output().await.unwrap();
        fs::write(p.join("README.md"), "# Test").await.unwrap();
        Command::new("git").current_dir(p).args(["add", "README.md"]).output().await.unwrap();
        Command::new("git").current_dir(p).args(["commit", "-m", "init"]).output().await.unwrap();
        dir
    }

    #[test]
    fn test_branch_name_validation() {
        let mgr = GitBranchManager::new();
        assert!(mgr.validate_branch_name("main").is_ok());
        assert!(mgr.validate_branch_name("feature/login").is_ok());
        assert!(mgr.validate_branch_name("").is_err());
        assert!(mgr.validate_branch_name("-invalid").is_err());
        assert!(mgr.validate_branch_name("feat..name").is_err());
        assert!(mgr.validate_branch_name("test.lock").is_err());
    }

    #[tokio::test]
    async fn test_create_list_checkout_delete() {
        let dir = setup_git_repo().await;
        let p = dir.path();
        let mgr = GitBranchManager::new();

        let branches = mgr.list_branches(p).await.unwrap();
        assert_eq!(branches.len(), 1);
        assert_eq!(branches[0].name, "main");

        mgr.create_branch(p, &BranchCreateOptions::new("dev")).await.unwrap();
        assert_eq!(mgr.list_branches(p).await.unwrap().len(), 2);

        mgr.checkout(p, &CheckoutOptions::new("dev")).await.unwrap();
        let branches = mgr.list_branches(p).await.unwrap();
        assert!(branches.iter().find(|b| b.name == "dev").unwrap().is_head);

        // Cannot delete active dev branch
        assert!(mgr.delete_branch(p, "dev", false).await.is_err());

        mgr.checkout(p, &CheckoutOptions::new("main")).await.unwrap();
        mgr.delete_branch(p, "dev", false).await.unwrap();
        assert_eq!(mgr.list_branches(p).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_checkout_create_validation() {
        let dir = setup_git_repo().await;
        let p = dir.path();
        let mgr = GitBranchManager::new();

        // Testing invalid branch name in checkout create_if_missing
        let err = mgr.checkout(p, &CheckoutOptions::new("invalid..name").create_if_missing(true)).await;
        assert!(matches!(err, Err(GitBranchError::InvalidBranchName(_))));
    }

    #[tokio::test]
    async fn test_errors_and_helpers() {
        let mgr = GitBranchManager::new();
        assert!(mgr.list_branches(Path::new("/nonexistent_12345")).await.is_err());
        assert_eq!(parse_upstream_track("[ahead 3, behind 2]"), (3, 2));
    }
}
