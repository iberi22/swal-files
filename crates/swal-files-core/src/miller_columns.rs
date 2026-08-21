use crate::types::FileEntry;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Selection state within a single column level.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ColumnSelectionState {
    pub selected_index: Option<usize>,
    pub selected_path: Option<PathBuf>,
    pub is_dir: bool,
}

impl ColumnSelectionState {
    pub fn none() -> Self { Self::default() }
    pub fn select(index: usize, path: PathBuf, is_dir: bool) -> Self {
        Self { selected_index: Some(index), selected_path: Some(path), is_dir }
    }
    pub fn clear(&mut self) { *self = Self::default(); }
    pub fn is_selected(&self) -> bool { self.selected_index.is_some() }
}

/// Represents a single column (hierarchy level) in Miller Columns view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColumnLevel {
    pub depth: usize,
    pub path: PathBuf,
    pub entries: Vec<FileEntry>,
    pub selection: ColumnSelectionState,
}

impl ColumnLevel {
    pub fn new(depth: usize, path: PathBuf, entries: Vec<FileEntry>) -> Self {
        Self { depth, path, entries, selection: ColumnSelectionState::none() }
    }

    pub fn select_index(&mut self, index: usize) -> Option<&FileEntry> {
        if index < self.entries.len() {
            let entry = &self.entries[index];
            self.selection = ColumnSelectionState::select(index, entry.path.clone(), entry.is_dir());
            Some(entry)
        } else {
            None
        }
    }

    pub fn selected_entry(&self) -> Option<&FileEntry> {
        self.selection.selected_index.and_then(|i| self.entries.get(i))
    }

    pub fn clear_selection(&mut self) { self.selection.clear(); }
    pub fn len(&self) -> usize { self.entries.len() }
    pub fn is_empty(&self) -> bool { self.entries.is_empty() }
}

/// Multi-column hierarchical directory tree state coordinator (macOS Finder style).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MillerColumnsTree {
    pub root_path: PathBuf,
    columns: Vec<ColumnLevel>,
    active_column_index: usize,
    max_visible_columns: usize,
}

impl MillerColumnsTree {
    pub fn new<P: AsRef<Path>>(root_path: P) -> Self {
        let root = root_path.as_ref().to_path_buf();
        Self {
            columns: vec![ColumnLevel::new(0, root.clone(), Vec::new())],
            root_path: root,
            active_column_index: 0,
            max_visible_columns: 4,
        }
    }

    pub fn columns(&self) -> &[ColumnLevel] { &self.columns }
    pub fn active_column_index(&self) -> usize { self.active_column_index }
    pub fn active_column(&self) -> Option<&ColumnLevel> { self.columns.get(self.active_column_index) }
    pub fn active_column_mut(&mut self) -> Option<&mut ColumnLevel> { self.columns.get_mut(self.active_column_index) }

    pub fn set_root_entries(&mut self, entries: Vec<FileEntry>) {
        if let Some(col) = self.columns.get_mut(0) { col.entries = entries; }
    }

    pub fn set_child_entries(&mut self, depth: usize, entries: Vec<FileEntry>) -> bool {
        if let Some(col) = self.columns.get_mut(depth) {
            col.entries = entries;
            true
        } else {
            false
        }
    }

    pub fn select_item(&mut self, column_depth: usize, item_index: usize) -> Option<&FileEntry> {
        if column_depth >= self.columns.len() { return None; }
        self.columns.truncate(column_depth + 1);
        self.active_column_index = column_depth;
        let entry = self.columns[column_depth].select_index(item_index)?.clone();
        if entry.is_dir() {
            self.columns.push(ColumnLevel::new(column_depth + 1, entry.path.clone(), Vec::new()));
        }
        self.columns.get(column_depth)?.selected_entry()
    }

    pub fn navigate_left(&mut self) -> bool {
        if self.active_column_index > 0 { self.active_column_index -= 1; true } else { false }
    }

    pub fn navigate_right(&mut self) -> bool {
        if self.active_column_index + 1 < self.columns.len() { self.active_column_index += 1; true } else { false }
    }

    pub fn navigate_up(&mut self) -> bool {
        let (idx, len) = match self.active_column() {
            Some(col) => (col.selection.selected_index, col.len()),
            None => return false,
        };
        if len == 0 { return false; }
        let new_idx = idx.map_or(len - 1, |i| i.saturating_sub(1));
        self.select_item(self.active_column_index, new_idx).is_some()
    }

    pub fn navigate_down(&mut self) -> bool {
        let (idx, len) = match self.active_column() {
            Some(col) => (col.selection.selected_index, col.len()),
            None => return false,
        };
        if len == 0 { return false; }
        let new_idx = idx.map_or(0, |i| (i + 1).min(len - 1));
        self.select_item(self.active_column_index, new_idx).is_some()
    }

    pub fn active_path(&self) -> PathBuf {
        self.active_column()
            .and_then(|col| col.selection.selected_path.clone().or_else(|| Some(col.path.clone())))
            .unwrap_or_else(|| self.root_path.clone())
    }

    pub fn depth(&self) -> usize { self.columns.len() }

    pub fn visible_column_range(&self) -> (usize, usize) {
        if self.columns.len() <= self.max_visible_columns {
            (0, self.columns.len())
        } else if self.active_column_index < self.max_visible_columns {
            (0, self.max_visible_columns)
        } else {
            (self.active_column_index + 1 - self.max_visible_columns, self.active_column_index + 1)
        }
    }

    pub fn set_max_visible_columns(&mut self, count: usize) {
        if count > 0 { self.max_visible_columns = count; }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_miller_columns_tree() {
        let dir = tempdir().unwrap();
        let sub = dir.path().join("sub");
        let file = dir.path().join("file.txt");
        std::fs::create_dir(&sub).unwrap();
        std::fs::File::create(&file).unwrap();

        let mut tree = MillerColumnsTree::new(dir.path());
        assert_eq!(tree.depth(), 1);
        tree.set_root_entries(vec![FileEntry::from_path(&sub).unwrap(), FileEntry::from_path(&file).unwrap()]);

        assert!(tree.select_item(0, 0).is_some());
        assert_eq!(tree.depth(), 2);
        assert_eq!(tree.active_path(), sub);

        let inner = sub.join("inner.rs");
        std::fs::File::create(&inner).unwrap();
        assert!(tree.set_child_entries(1, vec![FileEntry::from_path(&inner).unwrap()]));

        assert!(tree.navigate_right());
        assert_eq!(tree.active_column_index(), 1);
        assert!(tree.navigate_down());
        assert_eq!(tree.active_path(), inner);

        assert!(tree.navigate_left());
        assert_eq!(tree.active_column_index(), 0);

        let json = serde_json::to_string(&tree).unwrap();
        let de: MillerColumnsTree = serde_json::from_str(&json).unwrap();
        assert_eq!(de.depth(), tree.depth());
    }

    #[test]
    fn test_navigation_bounds_and_range() {
        let mut tree = MillerColumnsTree::new("/root");
        tree.set_max_visible_columns(2);
        assert_eq!(tree.visible_column_range(), (0, 1));
        assert!(!tree.navigate_up());
        assert!(!tree.navigate_down());
        assert!(!tree.navigate_left());
        assert!(!tree.navigate_right());
    }
}
