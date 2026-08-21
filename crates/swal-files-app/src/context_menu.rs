#![deny(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Kinds of actions that can be triggered from context menus.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MenuActionKind {
    Open,
    Cut,
    Copy,
    Paste,
    Delete,
    Rename,
    CopyPath,
    XavierPrompt(String),
    GitStage,
    GitUnstage,
    NewFile,
    NewFolder,
    Refresh,
    Properties,
}

/// Represents a visual menu separator line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MenuSeparator;

/// Represents an item in a context menu (actionable menu item or separator).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MenuItem {
    Action {
        label: String,
        action: MenuActionKind,
        shortcut: Option<String>,
        enabled: bool,
    },
    Separator(MenuSeparator),
}

impl MenuItem {
    pub fn action(label: impl Into<String>, action: MenuActionKind) -> Self {
        Self::Action {
            label: label.into(),
            action,
            shortcut: None,
            enabled: true,
        }
    }

    pub fn with_shortcut(mut self, shortcut_str: impl Into<String>) -> Self {
        if let Self::Action { ref mut shortcut, .. } = self {
            *shortcut = Some(shortcut_str.into());
        }
        self
    }

    pub fn disabled(mut self) -> Self {
        if let Self::Action { ref mut enabled, .. } = self {
            *enabled = false;
        }
        self
    }

    pub fn separator() -> Self {
        Self::Separator(MenuSeparator)
    }

    pub fn is_separator(&self) -> bool {
        matches!(self, Self::Separator(_))
    }

    pub fn label(&self) -> Option<&str> {
        match self {
            Self::Action { label, .. } => Some(label),
            Self::Separator(_) => None,
        }
    }

    pub fn is_enabled(&self) -> bool {
        match self {
            Self::Action { enabled, .. } => *enabled,
            Self::Separator(_) => false,
        }
    }
}

/// Selection context for generating dynamic context menus.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SelectionContext {
    pub selected_paths: Vec<PathBuf>,
    pub is_directory: bool,
    pub has_clipboard: bool,
    pub is_git_tracked: bool,
    pub is_git_staged: bool,
    pub parent_dir: PathBuf,
}

impl SelectionContext {
    pub fn empty_space(parent_dir: impl AsRef<Path>, has_clipboard: bool) -> Self {
        Self {
            selected_paths: Vec::new(),
            is_directory: true,
            has_clipboard,
            is_git_tracked: false,
            is_git_staged: false,
            parent_dir: parent_dir.as_ref().to_path_buf(),
        }
    }

    pub fn single_file(path: impl AsRef<Path>, parent_dir: impl AsRef<Path>) -> Self {
        Self {
            selected_paths: vec![path.as_ref().to_path_buf()],
            is_directory: false,
            has_clipboard: false,
            is_git_tracked: false,
            is_git_staged: false,
            parent_dir: parent_dir.as_ref().to_path_buf(),
        }
    }

    pub fn single_dir(path: impl AsRef<Path>, parent_dir: impl AsRef<Path>) -> Self {
        Self {
            selected_paths: vec![path.as_ref().to_path_buf()],
            is_directory: true,
            has_clipboard: false,
            is_git_tracked: false,
            is_git_staged: false,
            parent_dir: parent_dir.as_ref().to_path_buf(),
        }
    }

    pub fn is_empty_space(&self) -> bool {
        self.selected_paths.is_empty()
    }
}

/// Builder for constructing context menus dynamically based on selection and workspace state.
#[derive(Debug, Clone, Default)]
pub struct ContextMenuBuilder {
    items: Vec<MenuItem>,
}

impl ContextMenuBuilder {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn add_item(mut self, item: MenuItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn add_separator(mut self) -> Self {
        self.items.push(MenuItem::separator());
        self
    }

    /// Builds a dynamic context menu tailored to the given [`SelectionContext`].
    pub fn build_for_selection(context: &SelectionContext) -> Vec<MenuItem> {
        let mut builder = ContextMenuBuilder::new();

        if context.is_empty_space() {
            builder = builder
                .add_item(MenuItem::action("New File", MenuActionKind::NewFile).with_shortcut("Ctrl+N"))
                .add_item(MenuItem::action("New Folder", MenuActionKind::NewFolder).with_shortcut("Ctrl+Shift+N"))
                .add_separator();

            let paste_item = MenuItem::action("Paste", MenuActionKind::Paste).with_shortcut("Ctrl+V");
            let paste_item = if context.has_clipboard { paste_item } else { paste_item.disabled() };
            builder = builder.add_item(paste_item).add_separator();

            builder = builder
                .add_item(MenuItem::action("Ask Xavier AI", MenuActionKind::XavierPrompt("Explain directory contents".into())).with_shortcut("Ctrl+I"))
                .add_separator()
                .add_item(MenuItem::action("Refresh", MenuActionKind::Refresh).with_shortcut("F5"))
                .add_item(MenuItem::action("Properties", MenuActionKind::Properties).with_shortcut("Alt+Enter"));
        } else if context.selected_paths.len() == 1 {
            let open_label = if context.is_directory { "Open Directory" } else { "Open File" };
            builder = builder
                .add_item(MenuItem::action(open_label, MenuActionKind::Open).with_shortcut("Enter"))
                .add_separator()
                .add_item(MenuItem::action("Cut", MenuActionKind::Cut).with_shortcut("Ctrl+X"))
                .add_item(MenuItem::action("Copy", MenuActionKind::Copy).with_shortcut("Ctrl+C"));

            if context.is_directory {
                let paste_item = MenuItem::action("Paste Into", MenuActionKind::Paste).with_shortcut("Ctrl+V");
                let paste_item = if context.has_clipboard { paste_item } else { paste_item.disabled() };
                builder = builder.add_item(paste_item);
            }

            builder = builder
                .add_item(MenuItem::action("Copy Path", MenuActionKind::CopyPath).with_shortcut("Ctrl+Shift+C"))
                .add_separator()
                .add_item(MenuItem::action("Rename", MenuActionKind::Rename).with_shortcut("F2"))
                .add_item(MenuItem::action("Delete", MenuActionKind::Delete).with_shortcut("Delete"))
                .add_separator();

            if context.is_git_tracked {
                if context.is_git_staged {
                    builder = builder.add_item(MenuItem::action("Unstage Git Change", MenuActionKind::GitUnstage));
                } else {
                    builder = builder.add_item(MenuItem::action("Stage Git Change", MenuActionKind::GitStage));
                }
                builder = builder.add_separator();
            }

            let target_name = context.selected_paths[0]
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("selection");
            let xavier_prompt = format!("Analyze {}", target_name);
            builder = builder
                .add_item(MenuItem::action("Ask Xavier AI", MenuActionKind::XavierPrompt(xavier_prompt)).with_shortcut("Ctrl+I"))
                .add_separator()
                .add_item(MenuItem::action("Properties", MenuActionKind::Properties).with_shortcut("Alt+Enter"));
        } else {
            let count = context.selected_paths.len();
            builder = builder
                .add_item(MenuItem::action(format!("Open ({})", count), MenuActionKind::Open))
                .add_separator()
                .add_item(MenuItem::action("Cut", MenuActionKind::Cut).with_shortcut("Ctrl+X"))
                .add_item(MenuItem::action("Copy", MenuActionKind::Copy).with_shortcut("Ctrl+C"))
                .add_item(MenuItem::action("Delete", MenuActionKind::Delete).with_shortcut("Delete"))
                .add_separator();

            if context.is_git_tracked {
                if context.is_git_staged {
                    builder = builder.add_item(MenuItem::action("Unstage All Selected", MenuActionKind::GitUnstage));
                } else {
                    builder = builder.add_item(MenuItem::action("Stage All Selected", MenuActionKind::GitStage));
                }
                builder = builder.add_separator();
            }

            let xavier_prompt = format!("Analyze {} selected items", count);
            builder = builder.add_item(MenuItem::action("Ask Xavier AI", MenuActionKind::XavierPrompt(xavier_prompt)).with_shortcut("Ctrl+I"));
        }

        builder.build()
    }

    pub fn build(self) -> Vec<MenuItem> {
        self.items
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_menu_item_constructors() {
        let item = MenuItem::action("Cut", MenuActionKind::Cut).with_shortcut("Ctrl+X");
        assert_eq!(item.label(), Some("Cut"));
        assert!(item.is_enabled());
        assert!(!item.is_separator());

        let disabled_item = item.disabled();
        assert!(!disabled_item.is_enabled());

        let sep = MenuItem::separator();
        assert!(sep.is_separator());
        assert_eq!(sep.label(), None);
    }

    #[test]
    fn test_empty_space_context_menu() {
        let ctx = SelectionContext::empty_space("/tmp", false);
        let menu = ContextMenuBuilder::build_for_selection(&ctx);

        assert!(!menu.is_empty());
        let paste = menu.iter().find(|m| m.label() == Some("Paste"));
        assert!(paste.is_some());
        assert!(!paste.unwrap().is_enabled());

        let ctx_with_clip = SelectionContext::empty_space("/tmp", true);
        let menu2 = ContextMenuBuilder::build_for_selection(&ctx_with_clip);
        let paste2 = menu2.iter().find(|m| m.label() == Some("Paste"));
        assert!(paste2.unwrap().is_enabled());
    }

    #[test]
    fn test_single_file_context_menu() {
        let mut ctx = SelectionContext::single_file("/tmp/doc.txt", "/tmp");
        ctx.is_git_tracked = true;
        ctx.is_git_staged = false;

        let menu = ContextMenuBuilder::build_for_selection(&ctx);
        assert!(menu.iter().any(|m| m.label() == Some("Open File")));
        assert!(menu.iter().any(|m| m.label() == Some("Stage Git Change")));
        assert!(menu.iter().any(|m| matches!(m, MenuItem::Action { action: MenuActionKind::XavierPrompt(p), .. } if p.contains("doc.txt"))));
    }

    #[test]
    fn test_single_dir_context_menu() {
        let ctx = SelectionContext::single_dir("/tmp/subfolder", "/tmp");
        let menu = ContextMenuBuilder::build_for_selection(&ctx);
        assert!(menu.iter().any(|m| m.label() == Some("Open Directory")));
        assert!(menu.iter().any(|m| m.label() == Some("Paste Into")));
    }

    #[test]
    fn test_multi_selection_context_menu() {
        let ctx = SelectionContext {
            selected_paths: vec![PathBuf::from("/tmp/a.txt"), PathBuf::from("/tmp/b.txt")],
            is_directory: false,
            has_clipboard: false,
            is_git_tracked: true,
            is_git_staged: true,
            parent_dir: PathBuf::from("/tmp"),
        };

        let menu = ContextMenuBuilder::build_for_selection(&ctx);
        assert!(menu.iter().any(|m| m.label() == Some("Open (2)")));
        assert!(menu.iter().any(|m| m.label() == Some("Unstage All Selected")));
    }
}
