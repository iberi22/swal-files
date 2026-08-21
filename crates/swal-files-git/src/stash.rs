use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};
use tokio::process::Command;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StashEntry {
    pub index: usize,
    pub name: String,
    pub branch: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StashApplyResult {
    pub success: bool,
    pub message: String,
    pub has_conflicts: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GitStashError {
    RepositoryNotFound(PathBuf),
    ExecutionFailed(String),
    StashNotFound(usize),
    Io(String),
}

impl fmt::Display for GitStashError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RepositoryNotFound(p) => write!(f, "Repo not found: {}", p.display()),
            Self::ExecutionFailed(msg) => write!(f, "Git failed: {}", msg),
            Self::StashNotFound(idx) => write!(f, "Stash @{{{}}} not found", idx),
            Self::Io(err) => write!(f, "I/O error: {}", err),
        }
    }
}

impl std::error::Error for GitStashError {}

#[derive(Debug, Clone, Default)]
pub struct GitStashManager;

impl GitStashManager {
    pub fn new() -> Self { Self }

    pub async fn list(&self, repo: impl AsRef<Path>) -> Result<Vec<StashEntry>, GitStashError> {
        let path = repo.as_ref();
        if !path.exists() { return Err(GitStashError::RepositoryNotFound(path.to_path_buf())); }
        let out = Command::new("git").current_dir(path).args(["stash", "list"]).output().await.map_err(|e| GitStashError::Io(e.to_string()))?;
        if !out.status.success() { return Err(GitStashError::ExecutionFailed(String::from_utf8_lossy(&out.stderr).into())); }
        Ok(parse_stash_list(&String::from_utf8_lossy(&out.stdout)))
    }

    pub async fn push(&self, repo: impl AsRef<Path>, msg: Option<&str>, untracked: bool) -> Result<bool, GitStashError> {
        let path = repo.as_ref();
        if !path.exists() { return Err(GitStashError::RepositoryNotFound(path.to_path_buf())); }
        let mut cmd = Command::new("git");
        cmd.current_dir(path).args(["stash", "push"]);
        if untracked { cmd.arg("--include-untracked"); }
        if let Some(m) = msg { cmd.args(["-m", m]); }
        let out = cmd.output().await.map_err(|e| GitStashError::Io(e.to_string()))?;
        if !out.status.success() { return Err(GitStashError::ExecutionFailed(String::from_utf8_lossy(&out.stderr).into())); }
        Ok(!String::from_utf8_lossy(&out.stdout).contains("No local changes to save"))
    }

    pub async fn pop(&self, repo: impl AsRef<Path>, idx: usize) -> Result<StashApplyResult, GitStashError> {
        self.apply_or_pop(repo, idx, true).await
    }

    pub async fn apply(&self, repo: impl AsRef<Path>, idx: usize) -> Result<StashApplyResult, GitStashError> {
        self.apply_or_pop(repo, idx, false).await
    }

    pub async fn drop(&self, repo: impl AsRef<Path>, idx: usize) -> Result<(), GitStashError> {
        let path = repo.as_ref();
        if !path.exists() { return Err(GitStashError::RepositoryNotFound(path.to_path_buf())); }
        let sref = format!("stash@{{{}}}", idx);
        let out = Command::new("git").current_dir(path).args(["stash", "drop", &sref]).output().await.map_err(|e| GitStashError::Io(e.to_string()))?;
        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr);
            if err.contains("is not a valid reference") || err.contains("unknown revision") {
                return Err(GitStashError::StashNotFound(idx));
            }
            return Err(GitStashError::ExecutionFailed(err.into()));
        }
        Ok(())
    }

    pub async fn discard_file_changes(&self, repo: impl AsRef<Path>, file: impl AsRef<Path>) -> Result<(), GitStashError> {
        let path = repo.as_ref();
        if !path.exists() { return Err(GitStashError::RepositoryNotFound(path.to_path_buf())); }
        let fstr = file.as_ref().to_str().unwrap_or_default();
        let _ = Command::new("git").current_dir(path).args(["restore", "--staged", "--worktree", "--", fstr]).output().await;
        let out = Command::new("git").current_dir(path).args(["clean", "-fd", "--", fstr]).output().await.map_err(|e| GitStashError::Io(e.to_string()))?;
        if !out.status.success() { return Err(GitStashError::ExecutionFailed(String::from_utf8_lossy(&out.stderr).into())); }
        Ok(())
    }

    async fn apply_or_pop(&self, repo: impl AsRef<Path>, idx: usize, is_pop: bool) -> Result<StashApplyResult, GitStashError> {
        let path = repo.as_ref();
        if !path.exists() { return Err(GitStashError::RepositoryNotFound(path.to_path_buf())); }
        let act = if is_pop { "pop" } else { "apply" };
        let sref = format!("stash@{{{}}}", idx);
        let out = Command::new("git").current_dir(path).args(["stash", act, &sref]).output().await.map_err(|e| GitStashError::Io(e.to_string()))?;
        let err = String::from_utf8_lossy(&out.stderr);
        let combined = format!("{}\n{}", String::from_utf8_lossy(&out.stdout), err);
        if combined.contains("is not a valid reference") || err.contains("unknown revision") {
            return Err(GitStashError::StashNotFound(idx));
        }
        let conflicts = combined.contains("CONFLICT") || combined.contains("needs merge");
        Ok(StashApplyResult { success: out.status.success() && !conflicts, message: combined.trim().into(), has_conflicts: conflicts })
    }
}

pub fn parse_stash_list(output: &str) -> Vec<StashEntry> {
    output.lines().filter_map(|l| {
        let (ref_part, rest) = l.trim().split_once(": ")?;
        let idx_str = ref_part.strip_prefix("stash@{")?.strip_suffix('}')?;
        let index = idx_str.parse().ok()?;
        let (branch, message) = parse_branch_msg(rest);
        Some(StashEntry { index, name: format!("stash@{{{}}}", index), branch, message })
    }).collect()
}

fn parse_branch_msg(rest: &str) -> (String, String) {
    if let Some(r) = rest.strip_prefix("WIP on ") {
        r.split_once(": ").map_or((r.into(), rest.into()), |(b, m)| (b.into(), m.into()))
    } else if let Some(r) = rest.strip_prefix("On ") {
        r.split_once(": ").map_or((r.into(), rest.into()), |(b, m)| (b.into(), m.into()))
    } else {
        ("unknown".into(), rest.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use tokio::fs;

    #[test]
    fn test_parse_stash_list() {
        let sample = "stash@{0}: On main: custom msg\nstash@{1}: WIP on feat: 1234abc init";
        let list = parse_stash_list(sample);
        assert_eq!(list.len(), 2);
        assert_eq!(list[0], StashEntry { index: 0, name: "stash@{0}".into(), branch: "main".into(), message: "custom msg".into() });
        assert_eq!(list[1], StashEntry { index: 1, name: "stash@{1}".into(), branch: "feat".into(), message: "1234abc init".into() });
    }

    async fn setup_repo() -> tempfile::TempDir {
        let dir = tempdir().unwrap();
        let p = dir.path();
        for args in [vec!["init"], vec!["config", "user.name", "T"], vec!["config", "user.email", "t@t.com"]] {
            Command::new("git").current_dir(p).args(args).output().await.unwrap();
        }
        fs::write(p.join("f.txt"), "init").await.unwrap();
        Command::new("git").current_dir(p).args(["add", "."]).output().await.unwrap();
        Command::new("git").current_dir(p).args(["commit", "-m", "init"]).output().await.unwrap();
        dir
    }

    #[tokio::test]
    async fn test_stash_operations() {
        let repo = setup_repo().await;
        let path = repo.path();
        let mgr = GitStashManager::new();
        let f = path.join("f.txt");

        fs::write(&f, "mod").await.unwrap();
        assert!(mgr.push(path, Some("msg"), false).await.unwrap());
        let list = mgr.list(path).await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].message, "msg");

        assert!(mgr.apply(path, 0).await.unwrap().success);
        assert_eq!(fs::read_to_string(&f).await.unwrap(), "mod");
        mgr.drop(path, 0).await.unwrap();
        assert!(mgr.list(path).await.unwrap().is_empty());

        fs::write(&f, "mod2").await.unwrap();
        mgr.push(path, None, false).await.unwrap();
        assert!(mgr.pop(path, 0).await.unwrap().success);

        mgr.discard_file_changes(path, "f.txt").await.unwrap();
        assert_eq!(fs::read_to_string(&f).await.unwrap(), "init");
    }
}
