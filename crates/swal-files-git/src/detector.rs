#![deny(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;

/// Current HEAD state of a Git repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HeadState {
    /// Attached to a local branch (e.g., "main").
    Branch(String),
    /// Detached HEAD state pointing directly to a commit hash.
    Detached(String),
    /// Unborn branch state (repository initialized without commits).
    Unborn(String),
    /// Unknown state if HEAD cannot be resolved.
    Unknown,
}

/// Operational state of a Git repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RepoState {
    Normal,
    Merging,
    Rebasing,
    CherryPicking,
    Reverting,
    Bisecting,
    Bare,
}

/// Indicates whether a branch is local or remote tracking.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BranchType {
    Local,
    Remote,
}

/// Details about a Git branch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BranchInfo {
    /// Display name (e.g., "main" or "origin/main").
    pub name: String,
    /// Canonical Git ref path (e.g., "refs/heads/main" or "refs/remotes/origin/main").
    pub ref_name: String,
    /// Commit hash (SHA-1) associated with this branch, if available.
    pub commit_hash: Option<String>,
    /// Whether this branch is currently checked out (HEAD).
    pub is_head: bool,
    /// Local or Remote branch.
    pub branch_type: BranchType,
    /// Upstream tracking branch name, if configured (e.g., "origin/main").
    pub upstream: Option<String>,
}

/// Metadata and state of a detected Git repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitRepoInfo {
    /// Absolute or normalized path to the working tree root.
    pub root_path: PathBuf,
    /// Absolute or normalized path to the `.git` directory.
    pub git_dir: PathBuf,
    /// Current HEAD state.
    pub head_state: HeadState,
    /// Repository operational state.
    pub repo_state: RepoState,
    /// Whether this repository is a bare repository.
    pub is_bare: bool,
    /// Whether this repository is a worktree or submodule pointing to an external gitdir.
    pub is_worktree: bool,
}

/// High-performance Git repository detector and metadata extractor.
#[derive(Debug, Clone, Default)]
pub struct GitDetector;

impl GitDetector {
    /// Creates a new `GitDetector` instance.
    pub fn new() -> Self {
        Self
    }

    /// Asynchronously detects a Git repository starting from `path` and walking upwards.
    pub async fn detect<P: AsRef<Path>>(&self, path: P) -> Option<GitRepoInfo> {
        let (root_path, dot_git_path, is_bare) = self.find_repo_root_async(path.as_ref()).await?;
        let is_worktree = dot_git_path.is_file();
        let git_dir = BranchResolver::resolve_git_dir_async(&dot_git_path).await;

        let head_state = BranchResolver::resolve_head_async(&git_dir).await;
        let repo_state = if is_bare {
            RepoState::Bare
        } else {
            BranchResolver::detect_repo_state_async(&git_dir).await
        };

        Some(GitRepoInfo {
            root_path,
            git_dir,
            head_state,
            repo_state,
            is_bare,
            is_worktree,
        })
    }

    /// Synchronously detects a Git repository starting from `path` and walking upwards.
    pub fn detect_sync<P: AsRef<Path>>(&self, path: P) -> Option<GitRepoInfo> {
        let (root_path, dot_git_path, is_bare) = self.find_repo_root_sync(path.as_ref())?;
        let is_worktree = dot_git_path.is_file();
        let git_dir = BranchResolver::resolve_git_dir_sync(&dot_git_path);

        let head_state = BranchResolver::resolve_head_sync(&git_dir);
        let repo_state = if is_bare {
            RepoState::Bare
        } else {
            BranchResolver::detect_repo_state_sync(&git_dir)
        };

        Some(GitRepoInfo {
            root_path,
            git_dir,
            head_state,
            repo_state,
            is_bare,
            is_worktree,
        })
    }

    /// Checks if the given path is inside or at the root of a Git repository.
    pub async fn is_git_repository<P: AsRef<Path>>(&self, path: P) -> bool {
        self.find_repo_root_async(path.as_ref()).await.is_some()
    }

    /// Asynchronously finds the repository root directory and `.git` path walking upwards from `path`.
    pub async fn find_repo_root_async(&self, path: &Path) -> Option<(PathBuf, PathBuf, bool)> {
        let target = if path.is_relative() {
            std::env::current_dir().ok()?.join(path)
        } else {
            path.to_path_buf()
        };

        let mut curr = target.as_path();
        loop {
            let dot_git = curr.join(".git");
            if fs::metadata(&dot_git).await.is_ok() {
                return Some((curr.to_path_buf(), dot_git, false));
            }

            if is_bare_directory_sync(curr) {
                return Some((curr.to_path_buf(), curr.to_path_buf(), true));
            }

            match curr.parent() {
                Some(parent) => curr = parent,
                None => break,
            }
        }
        None
    }

    /// Synchronously finds the repository root directory and `.git` path walking upwards from `path`.
    pub fn find_repo_root_sync(&self, path: &Path) -> Option<(PathBuf, PathBuf, bool)> {
        let target = if path.is_relative() {
            std::env::current_dir().ok()?.join(path)
        } else {
            path.to_path_buf()
        };

        let mut curr = target.as_path();
        loop {
            let dot_git = curr.join(".git");
            if dot_git.exists() {
                return Some((curr.to_path_buf(), dot_git, false));
            }

            if is_bare_directory_sync(curr) {
                return Some((curr.to_path_buf(), curr.to_path_buf(), true));
            }

            match curr.parent() {
                Some(parent) => curr = parent,
                None => break,
            }
        }
        None
    }
}

/// Helper struct for resolving Git branches, HEAD states, refs, and configuration.
#[derive(Debug, Clone, Default)]
pub struct BranchResolver;

impl BranchResolver {
    /// Asynchronously resolves `.git` path if it is a gitdir pointer file (submodule or worktree).
    pub async fn resolve_git_dir_async(dot_git: &Path) -> PathBuf {
        if dot_git.is_dir() {
            return dot_git.to_path_buf();
        }
        if dot_git.is_file() {
            if let Ok(content) = fs::read_to_string(dot_git).await {
                if let Some(gitdir_path) = parse_gitdir_file(&content) {
                    let p = Path::new(&gitdir_path);
                    if p.is_absolute() {
                        return p.to_path_buf();
                    } else if let Some(parent) = dot_git.parent() {
                        return parent.join(p);
                    }
                }
            }
        }
        dot_git.to_path_buf()
    }

    /// Synchronously resolves `.git` path if it is a gitdir pointer file.
    pub fn resolve_git_dir_sync(dot_git: &Path) -> PathBuf {
        if dot_git.is_dir() {
            return dot_git.to_path_buf();
        }
        if dot_git.is_file() {
            if let Ok(content) = std::fs::read_to_string(dot_git) {
                if let Some(gitdir_path) = parse_gitdir_file(&content) {
                    let p = Path::new(&gitdir_path);
                    if p.is_absolute() {
                        return p.to_path_buf();
                    } else if let Some(parent) = dot_git.parent() {
                        return parent.join(p);
                    }
                }
            }
        }
        dot_git.to_path_buf()
    }

    /// Asynchronously resolves current HEAD state from git_dir.
    pub async fn resolve_head_async(git_dir: &Path) -> HeadState {
        let head_path = git_dir.join("HEAD");
        let content = match fs::read_to_string(&head_path).await {
            Ok(c) => c,
            Err(_) => return HeadState::Unknown,
        };
        Self::parse_head_content_async(git_dir, &content).await
    }

    /// Synchronously resolves current HEAD state from git_dir.
    pub fn resolve_head_sync(git_dir: &Path) -> HeadState {
        let head_path = git_dir.join("HEAD");
        let content = match std::fs::read_to_string(&head_path) {
            Ok(c) => c,
            Err(_) => return HeadState::Unknown,
        };
        Self::parse_head_content_sync(git_dir, &content)
    }

    async fn parse_head_content_async(git_dir: &Path, content: &str) -> HeadState {
        let trimmed = content.trim();
        if let Some(ref_path) = trimmed.strip_prefix("ref: ") {
            let ref_path = ref_path.trim();
            if let Some(branch_name) = ref_path.strip_prefix("refs/heads/") {
                let commit = Self::resolve_ref_async(git_dir, ref_path).await;
                if commit.is_some() {
                    HeadState::Branch(branch_name.to_string())
                } else {
                    HeadState::Unborn(branch_name.to_string())
                }
            } else {
                let commit = Self::resolve_ref_async(git_dir, ref_path).await;
                if let Some(sha) = commit {
                    HeadState::Detached(sha)
                } else {
                    HeadState::Unknown
                }
            }
        } else if is_valid_sha1(trimmed) {
            HeadState::Detached(trimmed.to_string())
        } else {
            HeadState::Unknown
        }
    }

    fn parse_head_content_sync(git_dir: &Path, content: &str) -> HeadState {
        let trimmed = content.trim();
        if let Some(ref_path) = trimmed.strip_prefix("ref: ") {
            let ref_path = ref_path.trim();
            if let Some(branch_name) = ref_path.strip_prefix("refs/heads/") {
                let commit = Self::resolve_ref_sync(git_dir, ref_path);
                if commit.is_some() {
                    HeadState::Branch(branch_name.to_string())
                } else {
                    HeadState::Unborn(branch_name.to_string())
                }
            } else {
                let commit = Self::resolve_ref_sync(git_dir, ref_path);
                if let Some(sha) = commit {
                    HeadState::Detached(sha)
                } else {
                    HeadState::Unknown
                }
            }
        } else if is_valid_sha1(trimmed) {
            HeadState::Detached(trimmed.to_string())
        } else {
            HeadState::Unknown
        }
    }

    /// Asynchronously resolves a Git reference (e.g. "refs/heads/main") to a commit SHA hash.
    pub async fn resolve_ref_async(git_dir: &Path, ref_name: &str) -> Option<String> {
        let ref_file = git_dir.join(ref_name);
        if let Ok(content) = fs::read_to_string(&ref_file).await {
            let trimmed = content.trim();
            if is_valid_sha1(trimmed) {
                return Some(trimmed.to_string());
            }
        }

        Self::resolve_packed_ref_async(git_dir, ref_name).await
    }

    /// Synchronously resolves a Git reference to a commit SHA hash.
    pub fn resolve_ref_sync(git_dir: &Path, ref_name: &str) -> Option<String> {
        let ref_file = git_dir.join(ref_name);
        if let Ok(content) = std::fs::read_to_string(&ref_file) {
            let trimmed = content.trim();
            if is_valid_sha1(trimmed) {
                return Some(trimmed.to_string());
            }
        }

        Self::resolve_packed_ref_sync(git_dir, ref_name)
    }

    async fn resolve_packed_ref_async(git_dir: &Path, ref_name: &str) -> Option<String> {
        let packed_file = git_dir.join("packed-refs");
        if let Ok(content) = fs::read_to_string(&packed_file).await {
            return parse_packed_refs(&content, ref_name);
        }
        None
    }

    fn resolve_packed_ref_sync(git_dir: &Path, ref_name: &str) -> Option<String> {
        let packed_file = git_dir.join("packed-refs");
        if let Ok(content) = std::fs::read_to_string(&packed_file) {
            return parse_packed_refs(&content, ref_name);
        }
        None
    }

    /// Asynchronously lists all local and remote branches in the repository.
    pub async fn list_branches_async(git_dir: &Path) -> Vec<BranchInfo> {
        let current_head = Self::resolve_head_async(git_dir).await;
        let upstreams = Self::parse_upstreams_async(git_dir).await;
        let packed_map = Self::read_all_packed_refs_async(git_dir).await;

        let mut branches = Vec::new();
        let mut seen_refs = std::collections::HashSet::new();

        // 1. Local branches from refs/heads
        let heads_dir = git_dir.join("refs").join("heads");
        let local_refs = collect_ref_files_sync(&heads_dir, "refs/heads");
        for (ref_name, display_name) in local_refs {
            seen_refs.insert(ref_name.clone());
            let commit = Self::resolve_ref_async(git_dir, &ref_name).await;
            let is_head = match &current_head {
                HeadState::Branch(b) => b == &display_name,
                _ => false,
            };
            let upstream = upstreams.get(&display_name).cloned();

            branches.push(BranchInfo {
                name: display_name,
                ref_name,
                commit_hash: commit,
                is_head,
                branch_type: BranchType::Local,
                upstream,
            });
        }

        // 2. Remote branches from refs/remotes
        let remotes_dir = git_dir.join("refs").join("remotes");
        let remote_refs = collect_ref_files_sync(&remotes_dir, "refs/remotes");
        for (ref_name, display_name) in remote_refs {
            if display_name.ends_with("/HEAD") {
                continue;
            }
            seen_refs.insert(ref_name.clone());
            let commit = Self::resolve_ref_async(git_dir, &ref_name).await;

            branches.push(BranchInfo {
                name: display_name,
                ref_name,
                commit_hash: commit,
                is_head: false,
                branch_type: BranchType::Remote,
                upstream: None,
            });
        }

        // 3. Packed refs not seen yet
        for (ref_name, sha) in packed_map {
            if seen_refs.contains(&ref_name) {
                continue;
            }
            if let Some(display_name) = ref_name.strip_prefix("refs/heads/") {
                seen_refs.insert(ref_name.clone());
                let is_head = match &current_head {
                    HeadState::Branch(b) => b == display_name,
                    _ => false,
                };
                let upstream = upstreams.get(display_name).cloned();

                branches.push(BranchInfo {
                    name: display_name.to_string(),
                    ref_name,
                    commit_hash: Some(sha),
                    is_head,
                    branch_type: BranchType::Local,
                    upstream,
                });
            } else if let Some(display_name) = ref_name.strip_prefix("refs/remotes/") {
                if display_name.ends_with("/HEAD") {
                    continue;
                }
                seen_refs.insert(ref_name.clone());

                branches.push(BranchInfo {
                    name: display_name.to_string(),
                    ref_name,
                    commit_hash: Some(sha),
                    is_head: false,
                    branch_type: BranchType::Remote,
                    upstream: None,
                });
            }
        }

        branches.sort_by(|a, b| a.ref_name.cmp(&b.ref_name));
        branches
    }

    /// Synchronously lists all local and remote branches in the repository.
    pub fn list_branches_sync(git_dir: &Path) -> Vec<BranchInfo> {
        let current_head = Self::resolve_head_sync(git_dir);
        let upstreams = Self::parse_upstreams_sync(git_dir);
        let packed_map = Self::read_all_packed_refs_sync(git_dir);

        let mut branches = Vec::new();
        let mut seen_refs = std::collections::HashSet::new();

        // 1. Local branches from refs/heads
        let heads_dir = git_dir.join("refs").join("heads");
        let local_refs = collect_ref_files_sync(&heads_dir, "refs/heads");
        for (ref_name, display_name) in local_refs {
            seen_refs.insert(ref_name.clone());
            let commit = Self::resolve_ref_sync(git_dir, &ref_name);
            let is_head = match &current_head {
                HeadState::Branch(b) => b == &display_name,
                _ => false,
            };
            let upstream = upstreams.get(&display_name).cloned();

            branches.push(BranchInfo {
                name: display_name,
                ref_name,
                commit_hash: commit,
                is_head,
                branch_type: BranchType::Local,
                upstream,
            });
        }

        // 2. Remote branches from refs/remotes
        let remotes_dir = git_dir.join("refs").join("remotes");
        let remote_refs = collect_ref_files_sync(&remotes_dir, "refs/remotes");
        for (ref_name, display_name) in remote_refs {
            if display_name.ends_with("/HEAD") {
                continue;
            }
            seen_refs.insert(ref_name.clone());
            let commit = Self::resolve_ref_sync(git_dir, &ref_name);

            branches.push(BranchInfo {
                name: display_name,
                ref_name,
                commit_hash: commit,
                is_head: false,
                branch_type: BranchType::Remote,
                upstream: None,
            });
        }

        // 3. Packed refs
        for (ref_name, sha) in packed_map {
            if seen_refs.contains(&ref_name) {
                continue;
            }
            if let Some(display_name) = ref_name.strip_prefix("refs/heads/") {
                seen_refs.insert(ref_name.clone());
                let is_head = match &current_head {
                    HeadState::Branch(b) => b == display_name,
                    _ => false,
                };
                let upstream = upstreams.get(display_name).cloned();

                branches.push(BranchInfo {
                    name: display_name.to_string(),
                    ref_name,
                    commit_hash: Some(sha),
                    is_head,
                    branch_type: BranchType::Local,
                    upstream,
                });
            } else if let Some(display_name) = ref_name.strip_prefix("refs/remotes/") {
                if display_name.ends_with("/HEAD") {
                    continue;
                }
                seen_refs.insert(ref_name.clone());

                branches.push(BranchInfo {
                    name: display_name.to_string(),
                    ref_name,
                    commit_hash: Some(sha),
                    is_head: false,
                    branch_type: BranchType::Remote,
                    upstream: None,
                });
            }
        }

        branches.sort_by(|a, b| a.ref_name.cmp(&b.ref_name));
        branches
    }

    /// Asynchronously detects the repository operational state (Rebasing, Merging, etc.).
    pub async fn detect_repo_state_async(git_dir: &Path) -> RepoState {
        if fs::metadata(git_dir.join("rebase-merge")).await.is_ok()
            || fs::metadata(git_dir.join("rebase-apply")).await.is_ok()
        {
            return RepoState::Rebasing;
        }
        if fs::metadata(git_dir.join("MERGE_HEAD")).await.is_ok() {
            return RepoState::Merging;
        }
        if fs::metadata(git_dir.join("CHERRY_PICK_HEAD")).await.is_ok() {
            return RepoState::CherryPicking;
        }
        if fs::metadata(git_dir.join("REVERT_HEAD")).await.is_ok() {
            return RepoState::Reverting;
        }
        if fs::metadata(git_dir.join("BISECT_LOG")).await.is_ok() {
            return RepoState::Bisecting;
        }
        RepoState::Normal
    }

    /// Synchronously detects the repository operational state.
    pub fn detect_repo_state_sync(git_dir: &Path) -> RepoState {
        if git_dir.join("rebase-merge").exists() || git_dir.join("rebase-apply").exists() {
            return RepoState::Rebasing;
        }
        if git_dir.join("MERGE_HEAD").exists() {
            return RepoState::Merging;
        }
        if git_dir.join("CHERRY_PICK_HEAD").exists() {
            return RepoState::CherryPicking;
        }
        if git_dir.join("REVERT_HEAD").exists() {
            return RepoState::Reverting;
        }
        if git_dir.join("BISECT_LOG").exists() {
            return RepoState::Bisecting;
        }
        RepoState::Normal
    }

    async fn parse_upstreams_async(git_dir: &Path) -> HashMap<String, String> {
        let config_path = git_dir.join("config");
        if let Ok(content) = fs::read_to_string(&config_path).await {
            return parse_git_config_upstreams(&content);
        }
        HashMap::new()
    }

    fn parse_upstreams_sync(git_dir: &Path) -> HashMap<String, String> {
        let config_path = git_dir.join("config");
        if let Ok(content) = std::fs::read_to_string(&config_path) {
            return parse_git_config_upstreams(&content);
        }
        HashMap::new()
    }

    async fn read_all_packed_refs_async(git_dir: &Path) -> HashMap<String, String> {
        let packed_file = git_dir.join("packed-refs");
        if let Ok(content) = fs::read_to_string(&packed_file).await {
            return parse_all_packed_refs(&content);
        }
        HashMap::new()
    }

    fn read_all_packed_refs_sync(git_dir: &Path) -> HashMap<String, String> {
        let packed_file = git_dir.join("packed-refs");
        if let Ok(content) = std::fs::read_to_string(&packed_file) {
            return parse_all_packed_refs(&content);
        }
        HashMap::new()
    }
}

fn is_bare_directory_sync(path: &Path) -> bool {
    let head = path.join("HEAD");
    let refs = path.join("refs");
    let objects = path.join("objects");
    let config = path.join("config");

    if head.is_file() && refs.is_dir() && objects.is_dir() && config.is_file() {
        if let Ok(content) = std::fs::read_to_string(&config) {
            if content.contains("bare = true") {
                return true;
            }
        }
    }
    false
}

fn parse_gitdir_file(content: &str) -> Option<String> {
    for line in content.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("gitdir:") {
            return Some(rest.trim().to_string());
        }
    }
    None
}

fn is_valid_sha1(s: &str) -> bool {
    s.len() == 40 && s.chars().all(|c| c.is_ascii_hexdigit())
}

fn parse_packed_refs(content: &str, target_ref: &str) -> Option<String> {
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.starts_with('^') || line.is_empty() {
            continue;
        }
        let mut parts = line.split_whitespace();
        if let (Some(sha), Some(ref_name)) = (parts.next(), parts.next()) {
            if ref_name == target_ref && is_valid_sha1(sha) {
                return Some(sha.to_string());
            }
        }
    }
    None
}

fn parse_all_packed_refs(content: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.starts_with('^') || line.is_empty() {
            continue;
        }
        let mut parts = line.split_whitespace();
        if let (Some(sha), Some(ref_name)) = (parts.next(), parts.next()) {
            if is_valid_sha1(sha) {
                map.insert(ref_name.to_string(), sha.to_string());
            }
        }
    }
    map
}

fn parse_git_config_upstreams(content: &str) -> HashMap<String, String> {
    let mut upstreams = HashMap::new();
    let mut current_branch: Option<String> = None;
    let mut remote: Option<String> = None;
    let mut merge: Option<String> = None;

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            if let (Some(branch), Some(rem), Some(mrg)) =
                (&current_branch, &remote, &merge)
            {
                let upstream = build_upstream_string(rem, mrg);
                upstreams.insert(branch.clone(), upstream);
            }

            current_branch = None;
            remote = None;
            merge = None;

            let section = &line[1..line.len() - 1];
            if section.starts_with("branch ") {
                let name = section["branch ".len()..]
                    .trim()
                    .trim_matches('"');
                current_branch = Some(name.to_string());
            }
        } else if current_branch.is_some() {
            if let Some((key, val)) = line.split_once('=') {
                let key = key.trim();
                let val = val.trim().trim_matches('"');
                if key == "remote" {
                    remote = Some(val.to_string());
                } else if key == "merge" {
                    merge = Some(val.to_string());
                }
            }
        }
    }

    if let (Some(branch), Some(rem), Some(mrg)) =
        (&current_branch, &remote, &merge)
    {
        let upstream = build_upstream_string(rem, mrg);
        upstreams.insert(branch.clone(), upstream);
    }

    upstreams
}

fn build_upstream_string(remote: &str, merge: &str) -> String {
    let branch = merge.strip_prefix("refs/heads/").unwrap_or(merge);
    if remote == "." {
        branch.to_string()
    } else {
        format!("{}/{}", remote, branch)
    }
}

fn collect_ref_files_sync(base_dir: &Path, prefix: &str) -> Vec<(String, String)> {
    let mut results = Vec::new();
    if !base_dir.exists() {
        return results;
    }
    walk_dir_for_refs(base_dir, base_dir, prefix, &mut results);
    results
}

fn walk_dir_for_refs(
    dir: &Path,
    base_dir: &Path,
    prefix: &str,
    results: &mut Vec<(String, String)>,
) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk_dir_for_refs(&path, base_dir, prefix, results);
            } else if path.is_file() {
                if let Ok(rel) = path.strip_prefix(base_dir) {
                    let rel_str = rel.to_string_lossy().replace('\\', "/");
                    let ref_name = format!("{}/{}", prefix, rel_str);
                    results.push((ref_name, rel_str));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_repo() -> (TempDir, PathBuf) {
        let dir = TempDir::new().unwrap();
        let repo_path = dir.path().to_path_buf();
        let git_dir = repo_path.join(".git");
        std::fs::create_dir_all(git_dir.join("refs").join("heads")).unwrap();
        std::fs::create_dir_all(git_dir.join("refs").join("remotes").join("origin")).unwrap();
        std::fs::create_dir_all(git_dir.join("objects")).unwrap();

        std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").unwrap();
        std::fs::write(
            git_dir.join("refs").join("heads").join("main"),
            "1111111111111111111111111111111111111111\n",
        )
        .unwrap();

        (dir, repo_path)
    }

    #[tokio::test]
    async fn test_detect_normal_repo() {
        let (_dir, repo_path) = create_test_repo();
        let detector = GitDetector::new();

        let is_git = detector.is_git_repository(&repo_path).await;
        assert!(is_git);

        let info = detector.detect(&repo_path).await.unwrap();
        assert_eq!(info.root_path, repo_path);
        assert_eq!(info.git_dir, repo_path.join(".git"));
        assert_eq!(info.head_state, HeadState::Branch("main".to_string()));
        assert_eq!(info.repo_state, RepoState::Normal);
        assert!(!info.is_bare);
        assert!(!info.is_worktree);

        // Sync counterpart
        let info_sync = detector.detect_sync(&repo_path).unwrap();
        assert_eq!(info, info_sync);
    }

    #[tokio::test]
    async fn test_detect_nested_dir() {
        let (_dir, repo_path) = create_test_repo();
        let sub_dir = repo_path.join("src").join("utils");
        std::fs::create_dir_all(&sub_dir).unwrap();

        let detector = GitDetector::new();
        let info = detector.detect(&sub_dir).await.unwrap();
        assert_eq!(info.root_path, repo_path);
    }

    #[tokio::test]
    async fn test_detect_detached_head() {
        let (dir, repo_path) = create_test_repo();
        let git_dir = repo_path.join(".git");
        let sha = "2222222222222222222222222222222222222222";
        std::fs::write(git_dir.join("HEAD"), format!("{}\n", sha)).unwrap();

        let detector = GitDetector::new();
        let info = detector.detect(&repo_path).await.unwrap();
        assert_eq!(info.head_state, HeadState::Detached(sha.to_string()));

        let _keep_alive = dir;
    }

    #[tokio::test]
    async fn test_detect_unborn_branch() {
        let (dir, repo_path) = create_test_repo();
        let git_dir = repo_path.join(".git");
        std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/new-feature\n").unwrap();

        let detector = GitDetector::new();
        let info = detector.detect(&repo_path).await.unwrap();
        assert_eq!(info.head_state, HeadState::Unborn("new-feature".to_string()));

        let _keep_alive = dir;
    }

    #[tokio::test]
    async fn test_worktree_pointer_file() {
        let (_dir1, main_repo) = create_test_repo();
        let main_git_dir = main_repo.join(".git");

        let wt_dir = TempDir::new().unwrap();
        let wt_path = wt_dir.path().to_path_buf();
        std::fs::write(
            wt_path.join(".git"),
            format!("gitdir: {}\n", main_git_dir.display()),
        )
        .unwrap();

        let detector = GitDetector::new();
        let info = detector.detect(&wt_path).await.unwrap();
        assert!(info.is_worktree);
        assert_eq!(info.git_dir, main_git_dir);
    }

    #[tokio::test]
    async fn test_list_branches_and_packed_refs() {
        let (_dir, repo_path) = create_test_repo();
        let git_dir = repo_path.join(".git");

        // Create feature branch loose ref
        std::fs::write(
            git_dir.join("refs").join("heads").join("feature-a"),
            "3333333333333333333333333333333333333333\n",
        )
        .unwrap();

        // Create remote branch loose ref
        std::fs::write(
            git_dir.join("refs").join("remotes").join("origin").join("main"),
            "1111111111111111111111111111111111111111\n",
        )
        .unwrap();

        // Write packed-refs with branch not in loose refs
        let packed_content = "\
# pack-refs with: peeled fully-peeled sorted
4444444444444444444444444444444444444444 refs/heads/packed-branch
5555555555555555555555555555555555555555 refs/remotes/origin/packed-remote
";
        std::fs::write(git_dir.join("packed-refs"), packed_content).unwrap();

        // Write config with upstream
        let config_content = "\
[core]
\trepositoryformatversion = 0
[branch \"main\"]
\tremote = origin
\tmerge = refs/heads/main
";
        std::fs::write(git_dir.join("config"), config_content).unwrap();

        let branches = BranchResolver::list_branches_async(&git_dir).await;
        assert_eq!(branches.len(), 5);

        let main_b = branches.iter().find(|b| b.name == "main").unwrap();
        assert!(main_b.is_head);
        assert_eq!(main_b.branch_type, BranchType::Local);
        assert_eq!(main_b.upstream, Some("origin/main".to_string()));

        let feat_b = branches.iter().find(|b| b.name == "feature-a").unwrap();
        assert!(!feat_b.is_head);
        assert_eq!(feat_b.branch_type, BranchType::Local);

        let remote_b = branches.iter().find(|b| b.name == "origin/main").unwrap();
        assert_eq!(remote_b.branch_type, BranchType::Remote);

        let packed_b = branches.iter().find(|b| b.name == "packed-branch").unwrap();
        assert_eq!(
            packed_b.commit_hash,
            Some("4444444444444444444444444444444444444444".to_string())
        );

        let packed_remote_b = branches.iter().find(|b| b.name == "origin/packed-remote").unwrap();
        assert_eq!(packed_remote_b.branch_type, BranchType::Remote);
        assert_eq!(
            packed_remote_b.commit_hash,
            Some("5555555555555555555555555555555555555555".to_string())
        );

        let branches_sync = BranchResolver::list_branches_sync(&git_dir);
        assert_eq!(branches, branches_sync);
    }

    #[tokio::test]
    async fn test_repo_operational_states() {
        let (dir, repo_path) = create_test_repo();
        let git_dir = repo_path.join(".git");

        assert_eq!(
            BranchResolver::detect_repo_state_async(&git_dir).await,
            RepoState::Normal
        );

        // Merging state
        std::fs::write(git_dir.join("MERGE_HEAD"), "1111111111111111111111111111111111111111\n").unwrap();
        assert_eq!(
            BranchResolver::detect_repo_state_async(&git_dir).await,
            RepoState::Merging
        );
        std::fs::remove_file(git_dir.join("MERGE_HEAD")).unwrap();

        // Rebasing state
        std::fs::create_dir_all(git_dir.join("rebase-merge")).unwrap();
        assert_eq!(
            BranchResolver::detect_repo_state_async(&git_dir).await,
            RepoState::Rebasing
        );
        std::fs::remove_dir(git_dir.join("rebase-merge")).unwrap();

        // Cherry-picking state
        std::fs::write(git_dir.join("CHERRY_PICK_HEAD"), "1111111111111111111111111111111111111111\n").unwrap();
        assert_eq!(
            BranchResolver::detect_repo_state_async(&git_dir).await,
            RepoState::CherryPicking
        );

        let _keep = dir;
    }

    #[tokio::test]
    async fn test_bare_repository() {
        let dir = TempDir::new().unwrap();
        let bare_path = dir.path().to_path_buf();
        std::fs::create_dir_all(bare_path.join("refs")).unwrap();
        std::fs::create_dir_all(bare_path.join("objects")).unwrap();
        std::fs::write(bare_path.join("HEAD"), "ref: refs/heads/main\n").unwrap();
        std::fs::write(
            bare_path.join("config"),
            "[core]\n\tbare = true\n\trepositoryformatversion = 0\n",
        )
        .unwrap();

        let detector = GitDetector::new();
        let info = detector.detect(&bare_path).await.unwrap();
        assert!(info.is_bare);
        assert_eq!(info.repo_state, RepoState::Bare);
    }

    #[tokio::test]
    async fn test_non_git_dir() {
        let dir = TempDir::new().unwrap();
        let detector = GitDetector::new();
        let is_git = detector.is_git_repository(dir.path()).await;
        assert!(!is_git);
        assert!(detector.detect(dir.path()).await.is_none());
    }
}
