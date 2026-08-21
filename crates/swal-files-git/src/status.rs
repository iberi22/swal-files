use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};
use tokio::process::Command;

/// Represents the status of a single file in the index or working tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GitStatusKind {
    Unmodified,
    Modified,
    Added,
    Deleted,
    Renamed,
    Copied,
    TypeChanged,
    Untracked,
    Ignored,
    Conflicted,
    Unknown,
}

impl GitStatusKind {
    pub fn from_char(c: char) -> Self {
        match c {
            '.' | ' ' => GitStatusKind::Unmodified,
            'M' => GitStatusKind::Modified,
            'A' => GitStatusKind::Added,
            'D' => GitStatusKind::Deleted,
            'R' => GitStatusKind::Renamed,
            'C' => GitStatusKind::Copied,
            'T' => GitStatusKind::TypeChanged,
            '?' => GitStatusKind::Untracked,
            '!' => GitStatusKind::Ignored,
            'U' => GitStatusKind::Conflicted,
            _ => GitStatusKind::Unknown,
        }
    }
}

impl fmt::Display for GitStatusKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            GitStatusKind::Unmodified => "Unmodified",
            GitStatusKind::Modified => "Modified",
            GitStatusKind::Added => "Added",
            GitStatusKind::Deleted => "Deleted",
            GitStatusKind::Renamed => "Renamed",
            GitStatusKind::Copied => "Copied",
            GitStatusKind::TypeChanged => "TypeChanged",
            GitStatusKind::Untracked => "Untracked",
            GitStatusKind::Ignored => "Ignored",
            GitStatusKind::Conflicted => "Conflicted",
            GitStatusKind::Unknown => "Unknown",
        };
        write!(f, "{}", s)
    }
}

/// Information about the current branch / HEAD state.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BranchStatus {
    pub head: Option<String>,
    pub oid: Option<String>,
    pub upstream: Option<String>,
    pub ahead: usize,
    pub behind: usize,
    pub is_detached: bool,
}

/// Represents the status of a file entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileStatus {
    pub path: PathBuf,
    pub orig_path: Option<PathBuf>,
    pub index_status: GitStatusKind,
    pub worktree_status: GitStatusKind,
}

impl FileStatus {
    pub fn new(path: impl Into<PathBuf>, index_status: GitStatusKind, worktree_status: GitStatusKind) -> Self {
        Self {
            path: path.into(),
            orig_path: None,
            index_status,
            worktree_status,
        }
    }

    pub fn with_orig_path(
        path: impl Into<PathBuf>,
        orig_path: impl Into<PathBuf>,
        index_status: GitStatusKind,
        worktree_status: GitStatusKind,
    ) -> Self {
        Self {
            path: path.into(),
            orig_path: Some(orig_path.into()),
            index_status,
            worktree_status,
        }
    }

    pub fn is_staged(&self) -> bool {
        !self.is_conflicted()
            && self.index_status != GitStatusKind::Unmodified
            && self.index_status != GitStatusKind::Untracked
            && self.index_status != GitStatusKind::Ignored
    }

    pub fn is_unstaged(&self) -> bool {
        !self.is_conflicted()
            && self.worktree_status != GitStatusKind::Unmodified
            && self.worktree_status != GitStatusKind::Untracked
            && self.worktree_status != GitStatusKind::Ignored
    }

    pub fn is_untracked(&self) -> bool {
        self.index_status == GitStatusKind::Untracked || self.worktree_status == GitStatusKind::Untracked
    }

    pub fn is_conflicted(&self) -> bool {
        self.index_status == GitStatusKind::Conflicted || self.worktree_status == GitStatusKind::Conflicted
    }
}

/// Full Git Working Tree status snapshot.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitWorkingTreeStatus {
    pub branch: BranchStatus,
    pub files: Vec<FileStatus>,
}

impl GitWorkingTreeStatus {
    pub fn is_clean(&self) -> bool {
        self.files.is_empty()
            || self.files.iter().all(|f| {
                f.index_status == GitStatusKind::Unmodified
                    && f.worktree_status == GitStatusKind::Unmodified
            })
    }

    pub fn staged(&self) -> Vec<&FileStatus> {
        self.files.iter().filter(|f| f.is_staged()).collect()
    }

    pub fn unstaged(&self) -> Vec<&FileStatus> {
        self.files.iter().filter(|f| f.is_unstaged()).collect()
    }

    pub fn untracked(&self) -> Vec<&FileStatus> {
        self.files.iter().filter(|f| f.is_untracked()).collect()
    }

    pub fn conflicted(&self) -> Vec<&FileStatus> {
        self.files.iter().filter(|f| f.is_conflicted()).collect()
    }

    pub fn get_status_for_path<P: AsRef<Path>>(&self, path: P) -> Option<&FileStatus> {
        let p = path.as_ref();
        self.files.iter().find(|f| f.path == p || f.orig_path.as_deref() == Some(p))
    }
}

/// Errors that can occur during git status operations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GitStatusError {
    Io(String),
    GitCommandFailed { code: Option<i32>, stderr: String },
    NotAGitRepository(PathBuf),
    ParseError(String),
}

impl fmt::Display for GitStatusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GitStatusError::Io(msg) => write!(f, "I/O error: {}", msg),
            GitStatusError::GitCommandFailed { code, stderr } => {
                write!(f, "Git command failed (code {:?}): {}", code, stderr)
            }
            GitStatusError::NotAGitRepository(path) => {
                write!(f, "Directory is not a git repository: {}", path.display())
            }
            GitStatusError::ParseError(msg) => write!(f, "Failed to parse git status: {}", msg),
        }
    }
}

impl std::error::Error for GitStatusError {}

/// Async reader for git status in a target repository path.
pub async fn read_working_tree_status<P: AsRef<Path>>(
    repo_path: P,
) -> Result<GitWorkingTreeStatus, GitStatusError> {
    let path = repo_path.as_ref();
    let output = Command::new("git")
        .arg("status")
        .arg("--porcelain=v2")
        .arg("--branch")
        .current_dir(path)
        .output()
        .await
        .map_err(|e| GitStatusError::Io(e.to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        if stderr.contains("not a git repository") {
            return Err(GitStatusError::NotAGitRepository(path.to_path_buf()));
        }
        return Err(GitStatusError::GitCommandFailed {
            code: output.status.code(),
            stderr,
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_porcelain_v2(&stdout)
}

/// Parses output from `git status --porcelain=v2 --branch`.
pub fn parse_porcelain_v2(output: &str) -> Result<GitWorkingTreeStatus, GitStatusError> {
    let mut status = GitWorkingTreeStatus::default();

    for line in output.lines() {
        if line.is_empty() {
            continue;
        }

        if line.starts_with("# ") {
            parse_header_line(line, &mut status.branch)?;
            continue;
        }

        let prefix = line.chars().next().unwrap_or(' ');
        match prefix {
            '1' => {
                // Ordinary changed entry: 1 <XY> <sub> <mH> <mI> <mW> <hH> <hI> <path>
                let parts: Vec<&str> = line.splitn(9, ' ').collect();
                if parts.len() < 9 {
                    return Err(GitStatusError::ParseError(format!(
                        "Invalid v2 ordinary entry line: {}",
                        line
                    )));
                }
                let xy = parts[1];
                let file_path = parts[8];
                let (index_st, work_st) = parse_xy(xy);
                status.files.push(FileStatus::new(
                    file_path,
                    index_st,
                    work_st,
                ));
            }
            '2' => {
                // Renamed or copied entry: 2 <XY> <sub> <mH> <mI> <mW> <hH> <hI> <Xscore> <path><TAB><origPath>
                // Note: git porcelain v2 separates path and origPath by tab or space
                let parts: Vec<&str> = line.splitn(10, ' ').collect();
                if parts.len() < 10 {
                    return Err(GitStatusError::ParseError(format!(
                        "Invalid v2 rename/copy line: {}",
                        line
                    )));
                }
                let xy = parts[1];
                let paths_part = parts[9];
                let (path_str, orig_path_str) = if let Some((p, orig)) = paths_part.split_once('\t') {
                    (p, orig)
                } else if let Some((p, orig)) = paths_part.split_once(' ') {
                    (p, orig)
                } else {
                    (paths_part, "")
                };

                let (index_st, work_st) = parse_xy(xy);
                if orig_path_str.is_empty() {
                    status.files.push(FileStatus::new(path_str, index_st, work_st));
                } else {
                    status.files.push(FileStatus::with_orig_path(
                        path_str,
                        orig_path_str,
                        index_st,
                        work_st,
                    ));
                }
            }
            'u' => {
                // Unmerged entry: u <XY> <sub> <m1> <m2> <m3> <mW> <h1> 2> <h3> <path>
                let parts: Vec<&str> = line.splitn(11, ' ').collect();
                if parts.len() < 11 {
                    return Err(GitStatusError::ParseError(format!(
                        "Invalid v2 unmerged line: {}",
                        line
                    )));
                }
                let file_path = parts[10];
                status.files.push(FileStatus::new(
                    file_path,
                    GitStatusKind::Conflicted,
                    GitStatusKind::Conflicted,
                ));
            }
            '?' => {
                // Untracked entry: ? <path>
                if let Some(path_str) = line.strip_prefix("? ") {
                    status.files.push(FileStatus::new(
                        path_str,
                        GitStatusKind::Untracked,
                        GitStatusKind::Untracked,
                    ));
                }
            }
            '!' => {
                // Ignored entry: ! <path>
                if let Some(path_str) = line.strip_prefix("! ") {
                    status.files.push(FileStatus::new(
                        path_str,
                        GitStatusKind::Ignored,
                        GitStatusKind::Ignored,
                    ));
                }
            }
            _ => {}
        }
    }

    Ok(status)
}

fn parse_header_line(line: &str, branch: &mut BranchStatus) -> Result<(), GitStatusError> {
    let header = line.trim_start_matches("# ").trim();
    if let Some(rest) = header.strip_prefix("branch.oid ") {
        if rest != "(initial)" {
            branch.oid = Some(rest.to_string());
        }
    } else if let Some(rest) = header.strip_prefix("branch.head ") {
        if rest == "(detached)" {
            branch.is_detached = true;
            branch.head = None;
        } else {
            branch.head = Some(rest.to_string());
        }
    } else if let Some(rest) = header.strip_prefix("branch.upstream ") {
        branch.upstream = Some(rest.to_string());
    } else if let Some(rest) = header.strip_prefix("branch.ab ") {
        // format: +<ahead> -<behind>
        let parts: Vec<&str> = rest.split_whitespace().collect();
        if parts.len() == 2 {
            if let Some(ahead_str) = parts[0].strip_prefix('+') {
                branch.ahead = ahead_str.parse().unwrap_or(0);
            }
            if let Some(behind_str) = parts[1].strip_prefix('-') {
                branch.behind = behind_str.parse().unwrap_or(0);
            }
        }
    }
    Ok(())
}

fn parse_xy(xy: &str) -> (GitStatusKind, GitStatusKind) {
    let mut chars = xy.chars();
    let x = chars.next().map(GitStatusKind::from_char).unwrap_or(GitStatusKind::Unmodified);
    let y = chars.next().map(GitStatusKind::from_char).unwrap_or(GitStatusKind::Unmodified);
    (x, y)
}

/// Parses output from `git status --porcelain` (v1 format).
pub fn parse_porcelain_v1(output: &str) -> Result<GitWorkingTreeStatus, GitStatusError> {
    let mut status = GitWorkingTreeStatus::default();

    for line in output.lines() {
        if line.len() < 3 {
            continue;
        }

        let xy = &line[0..2];
        let rest = line[2..].trim();

        if xy == "??" {
            status.files.push(FileStatus::new(
                rest,
                GitStatusKind::Untracked,
                GitStatusKind::Untracked,
            ));
            continue;
        }

        if xy == "!!" {
            status.files.push(FileStatus::new(
                rest,
                GitStatusKind::Ignored,
                GitStatusKind::Ignored,
            ));
            continue;
        }

        if xy == "DD" || xy == "AU" || xy == "UD" || xy == "UA" || xy == "DU" || xy == "AA" || xy == "UU" {
            status.files.push(FileStatus::new(
                rest,
                GitStatusKind::Conflicted,
                GitStatusKind::Conflicted,
            ));
            continue;
        }

        let (index_st, work_st) = parse_xy(xy);

        if let Some((path_str, orig_str)) = rest.split_once(" -> ") {
            status.files.push(FileStatus::with_orig_path(
                path_str,
                orig_str,
                index_st,
                work_st,
            ));
        } else {
            let path = rest.trim_matches('"');
            status.files.push(FileStatus::new(
                path,
                index_st,
                work_st,
            ));
        }
    }

    Ok(status)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_git_status_kind_display() {
        assert_eq!(GitStatusKind::Modified.to_string(), "Modified");
        assert_eq!(GitStatusKind::Untracked.to_string(), "Untracked");
    }

    #[test]
    fn test_parse_porcelain_v2_header_and_files() {
        let sample = "\
# branch.oid 1234567890abcdef1234567890abcdef12345678
# branch.head main
# branch.upstream origin/main
# branch.ab +2 -1
1 .M N... 100644 100644 100644 e69de29bb2d1d6434b8b29ae775ad8c2e48c5391 e69de29bb2d1d6434b8b29ae775ad8c2e48c5391 src/lib.rs
1 M. N... 100644 100644 100644 e69de29bb2d1d6434b8b29ae775ad8c2e48c5391 e69de29bb2d1d6434b8b29ae775ad8c2e48c5391 src/main.rs
2 R. N... 100644 100644 100644 e69de29bb2d1d6434b8b29ae775ad8c2e48c5391 e69de29bb2d1d6434b8b29ae775ad8c2e48c5391 R100 src/new.rs\tsrc/old.rs
u UU N... 100644 100644 100644 100644 e69de29bb2d1d6434b8b29ae775ad8c2e48c5391 e69de29bb2d1d6434b8b29ae775ad8c2e48c5391 e69de29bb2d1d6434b8b29ae775ad8c2e48c5391 src/conflict.rs
? untracked.txt
! ignored.log
";

        let status = parse_porcelain_v2(sample).expect("Should parse v2 output successfully");

        assert_eq!(status.branch.head.as_deref(), Some("main"));
        assert_eq!(status.branch.oid.as_deref(), Some("1234567890abcdef1234567890abcdef12345678"));
        assert_eq!(status.branch.upstream.as_deref(), Some("origin/main"));
        assert_eq!(status.branch.ahead, 2);
        assert_eq!(status.branch.behind, 1);
        assert!(!status.branch.is_detached);

        assert!(!status.is_clean());
        assert_eq!(status.files.len(), 6);

        assert_eq!(status.staged().len(), 2); // main.rs (M.) and new.rs (R.)
        assert_eq!(status.unstaged().len(), 1); // lib.rs (.M)
        assert_eq!(status.untracked().len(), 1); // untracked.txt
        assert_eq!(status.conflicted().len(), 1); // conflict.rs

        let lib_status = status.get_status_for_path("src/lib.rs").expect("lib.rs status");
        assert_eq!(lib_status.index_status, GitStatusKind::Unmodified);
        assert_eq!(lib_status.worktree_status, GitStatusKind::Modified);

        let rename_status = status.get_status_for_path("src/new.rs").expect("new.rs status");
        assert_eq!(rename_status.orig_path.as_deref(), Some(Path::new("src/old.rs")));
    }

    #[test]
    fn test_parse_porcelain_v1() {
        let sample = " M src/lib.rs\n\
M  src/main.rs\n\
R  src/new.rs -> src/old.rs\n\
UU src/conflict.rs\n\
?? untracked.txt\n\
!! ignored.log\n";

        let status = parse_porcelain_v1(sample).expect("Should parse v1 output successfully");

        assert_eq!(status.files.len(), 6);
        assert_eq!(status.staged().len(), 2);
        assert_eq!(status.unstaged().len(), 1);
        assert_eq!(status.untracked().len(), 1);
        assert_eq!(status.conflicted().len(), 1);
    }

    #[tokio::test]
    async fn test_read_working_tree_status_not_a_repo() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let result = read_working_tree_status(temp_dir.path()).await;
        assert!(matches!(result, Err(GitStatusError::NotAGitRepository(_))));
    }

    #[tokio::test]
    async fn test_read_working_tree_status_in_real_repo() {
        let temp_dir = TempDir::new().expect("create temp dir");

        // Init git repo
        let init_status = Command::new("git")
            .arg("init")
            .current_dir(temp_dir.path())
            .status()
            .await;

        if let Ok(st) = init_status {
            if st.success() {
                // Config user in temp git repo
                let _ = Command::new("git")
                    .args(&["config", "user.name", "Test User"])
                    .current_dir(temp_dir.path())
                    .status()
                    .await;
                let _ = Command::new("git")
                    .args(&["config", "user.email", "test@example.com"])
                    .current_dir(temp_dir.path())
                    .status()
                    .await;

                // Create a test file
                let file_path = temp_dir.path().join("test.txt");
                tokio::fs::write(&file_path, "hello world").await.unwrap();

                let status = read_working_tree_status(temp_dir.path())
                    .await
                    .expect("read_working_tree_status success");

                assert!(!status.is_clean());
                assert_eq!(status.untracked().len(), 1);
                assert_eq!(status.untracked()[0].path, Path::new("test.txt"));
            }
        }
    }
}
