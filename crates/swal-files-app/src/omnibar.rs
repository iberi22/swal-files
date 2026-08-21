#![deny(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Intention parsed from the Omnibar input string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OmnibarIntent {
    /// Direct directory or file path navigation
    Navigation(PathBuf),
    /// Fuzzy search query (prefixed with `?`)
    Search(String),
    /// Shell command execution (prefixed with `>`)
    ShellCommand(String),
    /// Agent natural language query (prefixed with `@`)
    AgentQuery(String),
    /// Empty input
    Empty,
}

impl OmnibarIntent {
    /// Parses an input string into an [`OmnibarIntent`].
    ///
    /// - `?query` -> `OmnibarIntent::Search("query")`
    /// - `>cmd`   -> `OmnibarIntent::ShellCommand("cmd")`
    /// - `@prompt` -> `OmnibarIntent::AgentQuery("prompt")`
    /// - `path`   -> `OmnibarIntent::Navigation(expand_path(path))`
    pub fn parse(input: &str) -> Self {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return OmnibarIntent::Empty;
        }

        if let Some(query) = trimmed.strip_prefix('?') {
            OmnibarIntent::Search(query.trim().to_string())
        } else if let Some(cmd) = trimmed.strip_prefix('>') {
            OmnibarIntent::ShellCommand(cmd.trim().to_string())
        } else if let Some(prompt) = trimmed.strip_prefix('@') {
            OmnibarIntent::AgentQuery(prompt.trim().to_string())
        } else {
            OmnibarIntent::Navigation(expand_path(trimmed))
        }
    }

    /// Returns `true` if the intent is [`OmnibarIntent::Navigation`].
    pub fn is_navigation(&self) -> bool {
        matches!(self, OmnibarIntent::Navigation(_))
    }

    /// Returns `true` if the intent is [`OmnibarIntent::Search`].
    pub fn is_search(&self) -> bool {
        matches!(self, OmnibarIntent::Search(_))
    }

    /// Returns `true` if the intent is [`OmnibarIntent::ShellCommand`].
    pub fn is_shell_command(&self) -> bool {
        matches!(self, OmnibarIntent::ShellCommand(_))
    }

    /// Returns `true` if the intent is [`OmnibarIntent::AgentQuery`].
    pub fn is_agent_query(&self) -> bool {
        matches!(self, OmnibarIntent::AgentQuery(_))
    }

    /// Returns `true` if the intent is [`OmnibarIntent::Empty`].
    pub fn is_empty(&self) -> bool {
        matches!(self, OmnibarIntent::Empty)
    }
}

/// Represents an interactive segment in a directory path breadcrumb trail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Breadcrumb {
    /// The display name for this path segment
    pub name: String,
    /// Absolute or relative path associated with this segment
    pub path: PathBuf,
    /// Whether this segment represents the root directory or root prefix
    pub is_root: bool,
    /// Whether this segment is the terminal segment in the breadcrumb path
    pub is_last: bool,
}

impl Breadcrumb {
    /// Constructs a vector of interactive breadcrumbs from a given path.
    pub fn parse<P: AsRef<Path>>(path: P) -> Vec<Breadcrumb> {
        let path = path.as_ref();
        let mut breadcrumbs = Vec::new();
        let mut current = PathBuf::new();

        for component in path.components() {
            match component {
                std::path::Component::RootDir | std::path::Component::Prefix(_) => {
                    current.push(component);
                    let name = component.as_os_str().to_string_lossy().to_string();
                    let name = if name.is_empty() { "/".to_string() } else { name };
                    breadcrumbs.push(Breadcrumb {
                        name,
                        path: current.clone(),
                        is_root: true,
                        is_last: false,
                    });
                }
                std::path::Component::Normal(os_str) => {
                    current.push(os_str);
                    let name = os_str.to_string_lossy().to_string();
                    breadcrumbs.push(Breadcrumb {
                        name,
                        path: current.clone(),
                        is_root: false,
                        is_last: false,
                    });
                }
                std::path::Component::CurDir | std::path::Component::ParentDir => {
                    current.push(component);
                    let name = component.as_os_str().to_string_lossy().to_string();
                    breadcrumbs.push(Breadcrumb {
                        name,
                        path: current.clone(),
                        is_root: false,
                        is_last: false,
                    });
                }
            }
        }

        if let Some(last) = breadcrumbs.last_mut() {
            last.is_last = true;
        }

        breadcrumbs
    }
}

/// Represents the output of a shell command executed via Omnibar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShellOutput {
    /// Captured standard output
    pub stdout: String,
    /// Captured standard error
    pub stderr: String,
    /// Process exit status code (if available)
    pub exit_code: Option<i32>,
    /// Whether the command terminated successfully (exit status 0)
    pub success: bool,
}

/// State container for the Hybrid Omnibar component.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Omnibar {
    /// Active text input in the omnibar field
    pub input: String,
    /// Current path of the active panel/tab
    pub current_path: PathBuf,
    /// History of executed commands or navigations
    pub history: Vec<String>,
    /// Index pointer when traversing history (`None` when typing new input)
    pub history_index: Option<usize>,
    /// Whether the omnibar input box is currently focused
    pub is_focused: bool,
    /// Parsed intent from `input`
    pub active_intent: OmnibarIntent,
}

impl Omnibar {
    /// Creates a new Omnibar initialized with the given working path.
    pub fn new<P: AsRef<Path>>(initial_path: P) -> Self {
        let path = initial_path.as_ref().to_path_buf();
        Self {
            input: String::new(),
            current_path: path,
            history: Vec::new(),
            history_index: None,
            is_focused: false,
            active_intent: OmnibarIntent::Empty,
        }
    }

    /// Sets the text input in the Omnibar and updates `active_intent`.
    pub fn set_input(&mut self, input: impl Into<String>) {
        let text = input.into();
        self.active_intent = OmnibarIntent::parse(&text);
        self.input = text;
        self.history_index = None;
    }

    /// Sets the current working path for breadcrumb generation.
    pub fn set_current_path<P: AsRef<Path>>(&mut self, path: P) {
        self.current_path = path.as_ref().to_path_buf();
    }

    /// Generates breadcrumbs for the current path or target navigation path.
    pub fn breadcrumbs(&self) -> Vec<Breadcrumb> {
        if let OmnibarIntent::Navigation(ref nav_path) = self.active_intent {
            Breadcrumb::parse(nav_path)
        } else {
            Breadcrumb::parse(&self.current_path)
        }
    }

    /// Focuses the omnibar.
    pub fn focus(&mut self) {
        self.is_focused = true;
    }

    /// Unfocuses the omnibar.
    pub fn unfocus(&mut self) {
        self.is_focused = false;
    }

    /// Toggles the focus state of the omnibar.
    pub fn toggle_focus(&mut self) {
        self.is_focused = !self.is_focused;
    }

    /// Clears the input field and resets `active_intent`.
    pub fn clear(&mut self) {
        self.input.clear();
        self.active_intent = OmnibarIntent::Empty;
        self.history_index = None;
    }

    /// Submits the current input, pushing it to history if non-empty,
    /// and returns the parsed [`OmnibarIntent`].
    pub fn submit(&mut self) -> OmnibarIntent {
        let intent = self.active_intent.clone();
        let trimmed = self.input.trim().to_string();

        if !trimmed.is_empty() {
            if self.history.last() != Some(&trimmed) {
                self.history.push(trimmed);
            }
        }

        self.history_index = None;
        if let OmnibarIntent::Navigation(ref target) = intent {
            self.current_path = target.clone();
            self.clear();
        }

        intent
    }

    /// Navigates to the previous item in command history.
    pub fn history_prev(&mut self) {
        if self.history.is_empty() {
            return;
        }

        let new_index = match self.history_index {
            None => self.history.len().saturating_sub(1),
            Some(idx) => idx.saturating_sub(1),
        };

        self.history_index = Some(new_index);
        if let Some(entry) = self.history.get(new_index).cloned() {
            self.set_input(entry);
            self.history_index = Some(new_index);
        }
    }

    /// Navigates to the next item in command history.
    pub fn history_next(&mut self) {
        if self.history.is_empty() {
            return;
        }

        if let Some(idx) = self.history_index {
            if idx + 1 < self.history.len() {
                let new_index = idx + 1;
                self.history_index = Some(new_index);
                if let Some(entry) = self.history.get(new_index).cloned() {
                    self.set_input(entry);
                    self.history_index = Some(new_index);
                }
            } else {
                self.clear();
            }
        }
    }

    /// Executes a shell command asynchronously and returns its stdout, stderr, and status.
    pub async fn execute_shell(cmd: &str) -> std::io::Result<ShellOutput> {
        Self::execute_shell_in_dir(cmd, Path::new(".")).await
    }

    /// Executes a shell command asynchronously in a specific working directory.
    pub async fn execute_shell_in_dir(cmd: &str, working_dir: &Path) -> std::io::Result<ShellOutput> {
        let output = if cfg!(target_os = "windows") {
            tokio::process::Command::new("cmd")
                .args(["/C", cmd])
                .current_dir(working_dir)
                .output()
                .await?
        } else {
            tokio::process::Command::new("sh")
                .arg("-c")
                .arg(cmd)
                .current_dir(working_dir)
                .output()
                .await?
        };

        Ok(ShellOutput {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code(),
            success: output.status.success(),
        })
    }
}

/// Expands leading `~` in path strings to the current user's home directory.
pub fn expand_path(input: &str) -> PathBuf {
    if input == "~" {
        dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"))
    } else if let Some(stripped) = input.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            home.join(stripped)
        } else {
            PathBuf::from(input)
        }
    } else if let Some(stripped) = input.strip_prefix("~\\") {
        if let Some(home) = dirs::home_dir() {
            home.join(stripped)
        } else {
            PathBuf::from(input)
        }
    } else {
        PathBuf::from(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intent_parse_empty() {
        assert_eq!(OmnibarIntent::parse(""), OmnibarIntent::Empty);
        assert_eq!(OmnibarIntent::parse("   "), OmnibarIntent::Empty);
        assert!(OmnibarIntent::Empty.is_empty());
    }

    #[test]
    fn test_intent_parse_search() {
        let intent = OmnibarIntent::parse("? rust files");
        assert_eq!(intent, OmnibarIntent::Search("rust files".to_string()));
        assert!(intent.is_search());
    }

    #[test]
    fn test_intent_parse_shell_command() {
        let intent = OmnibarIntent::parse("> cargo build");
        assert_eq!(intent, OmnibarIntent::ShellCommand("cargo build".to_string()));
        assert!(intent.is_shell_command());
    }

    #[test]
    fn test_intent_parse_agent_query() {
        let intent = OmnibarIntent::parse("@ explain changes");
        assert_eq!(intent, OmnibarIntent::AgentQuery("explain changes".to_string()));
        assert!(intent.is_agent_query());
    }

    #[test]
    fn test_intent_parse_navigation() {
        let intent = OmnibarIntent::parse("/usr/local/bin");
        assert_eq!(intent, OmnibarIntent::Navigation(PathBuf::from("/usr/local/bin")));
        assert!(intent.is_navigation());
    }

    #[test]
    fn test_expand_path_tilde() {
        if let Some(home) = dirs::home_dir() {
            assert_eq!(expand_path("~"), home);
            assert_eq!(expand_path("~/documents"), home.join("documents"));
        }
    }

    #[test]
    fn test_breadcrumb_parse_absolute() {
        let crumbs = Breadcrumb::parse("/home/user/docs");
        assert_eq!(crumbs.len(), 4);
        assert!(crumbs[0].is_root);
        assert!(!crumbs[0].is_last);
        assert_eq!(crumbs[0].name, "/");

        assert_eq!(crumbs[1].name, "home");
        assert_eq!(crumbs[2].name, "user");
        assert_eq!(crumbs[3].name, "docs");
        assert!(crumbs[3].is_last);
        assert_eq!(crumbs[3].path, PathBuf::from("/home/user/docs"));
    }

    #[test]
    fn test_omnibar_state_and_history() {
        let mut omnibar = Omnibar::new("/home/user");
        assert_eq!(omnibar.current_path, PathBuf::from("/home/user"));
        assert!(!omnibar.is_focused);

        omnibar.focus();
        assert!(omnibar.is_focused);
        omnibar.toggle_focus();
        assert!(!omnibar.is_focused);

        omnibar.set_input("> cargo check");
        assert_eq!(omnibar.active_intent, OmnibarIntent::ShellCommand("cargo check".to_string()));

        let intent = omnibar.submit();
        assert_eq!(intent, OmnibarIntent::ShellCommand("cargo check".to_string()));
        assert_eq!(omnibar.history, vec!["> cargo check".to_string()]);

        omnibar.set_input("? search_term");
        omnibar.submit();

        assert_eq!(omnibar.history.len(), 2);

        // History traversal
        omnibar.history_prev();
        assert_eq!(omnibar.input, "? search_term");
        omnibar.history_prev();
        assert_eq!(omnibar.input, "> cargo check");

        omnibar.history_next();
        assert_eq!(omnibar.input, "? search_term");
        omnibar.history_next();
        assert_eq!(omnibar.input, "");
    }

    #[tokio::test]
    async fn test_execute_shell() {
        let res = Omnibar::execute_shell("echo hello_omnibar").await;
        assert!(res.is_ok());
        let output = res.unwrap();
        assert!(output.success);
        assert!(output.stdout.contains("hello_omnibar"));
    }
}
