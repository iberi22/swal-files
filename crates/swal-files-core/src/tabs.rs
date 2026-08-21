use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::sync::mpsc::UnboundedSender;

/// Unique identifier for a tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TabId(pub u64);

/// Classification of tab content type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TabKind {
    /// Standard file browsing tab.
    Standard,
    /// Git changes and diff viewer tab.
    Git,
    /// QuickLook preview tab.
    Preview,
    /// Search results tab.
    Search,
    /// Custom extension tab.
    Custom(String),
}

impl Default for TabKind {
    fn default() -> Self {
        Self::Standard
    }
}

/// Identifies the active pane in a dual-pane layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PaneId {
    Left,
    Right,
}

impl Default for PaneId {
    fn default() -> Self {
        Self::Left
    }
}

/// Defines the layout mode of the dual-pane system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DualPaneLayout {
    /// Single pane mode (only primary/left pane visible).
    Single,
    /// Dual pane split horizontally (side-by-side).
    SplitHorizontal,
    /// Dual pane split vertically (top and bottom).
    SplitVertical,
}

impl Default for DualPaneLayout {
    fn default() -> Self {
        Self::Single
    }
}

/// Events emitted during tab and pane state operations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TabEvent {
    Opened {
        pane_id: PaneId,
        tab_id: TabId,
        path: PathBuf,
    },
    Closed {
        pane_id: PaneId,
        tab_id: TabId,
    },
    Switched {
        pane_id: PaneId,
        active_tab_id: TabId,
    },
    Navigated {
        pane_id: PaneId,
        tab_id: TabId,
        path: PathBuf,
    },
    PinnedChanged {
        pane_id: PaneId,
        tab_id: TabId,
        is_pinned: bool,
    },
    PaneLayoutChanged {
        layout: DualPaneLayout,
    },
    ActivePaneChanged {
        active_pane: PaneId,
    },
    PanesSwapped,
}

/// Represents an individual tab with navigation history and metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Tab {
    pub id: TabId,
    pub title: String,
    pub path: PathBuf,
    pub history: Vec<PathBuf>,
    pub history_index: usize,
    pub is_pinned: bool,
    pub icon: Option<String>,
    pub kind: TabKind,
}

impl Tab {
    /// Creates a new tab at the given path.
    pub fn new<P: AsRef<Path>>(id: TabId, path: P) -> Self {
        Self::with_kind(id, path, TabKind::Standard)
    }

    /// Creates a new tab with a specific `TabKind`.
    pub fn with_kind<P: AsRef<Path>>(id: TabId, path: P, kind: TabKind) -> Self {
        let path_buf = path.as_ref().to_path_buf();
        let title = Self::derive_title_from_path(&path_buf);
        Self {
            id,
            title,
            path: path_buf.clone(),
            history: vec![path_buf],
            history_index: 0,
            is_pinned: false,
            icon: None,
            kind,
        }
    }

    /// Derives a display title from a path.
    pub fn derive_title_from_path(path: &Path) -> String {
        path.file_name()
            .and_then(|s| s.to_str())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| path.to_str().unwrap_or("/"))
            .to_string()
    }

    /// Navigates the tab to a new path, truncating forward history.
    pub fn navigate_to<P: AsRef<Path>>(&mut self, new_path: P) {
        let path_buf = new_path.as_ref().to_path_buf();
        if self.path == path_buf {
            return;
        }

        // Truncate any forward history past current index
        if self.history_index + 1 < self.history.len() {
            self.history.truncate(self.history_index + 1);
        }

        self.history.push(path_buf.clone());
        self.history_index = self.history.len() - 1;
        self.path = path_buf.clone();
        self.title = Self::derive_title_from_path(&path_buf);
    }

    /// Returns `true` if back navigation is possible.
    pub fn can_go_back(&self) -> bool {
        self.history_index > 0
    }

    /// Navigates back in history if possible, returning the new path.
    pub fn go_back(&mut self) -> Option<&PathBuf> {
        if self.can_go_back() {
            self.history_index -= 1;
            self.path = self.history[self.history_index].clone();
            self.title = Self::derive_title_from_path(&self.path);
            Some(&self.path)
        } else {
            None
        }
    }

    /// Returns `true` if forward navigation is possible.
    pub fn can_go_forward(&self) -> bool {
        self.history_index + 1 < self.history.len()
    }

    /// Navigates forward in history if possible, returning the new path.
    pub fn go_forward(&mut self) -> Option<&PathBuf> {
        if self.can_go_forward() {
            self.history_index += 1;
            self.path = self.history[self.history_index].clone();
            self.title = Self::derive_title_from_path(&self.path);
            Some(&self.path)
        } else {
            None
        }
    }

    /// Sets a custom title for the tab.
    pub fn set_title<S: Into<String>>(&mut self, title: S) {
        self.title = title.into();
    }
}

/// Tab manager responsible for tab ordering, switching, and operations within a single pane.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabManager {
    pub pane_id: PaneId,
    tabs: Vec<Tab>,
    active_index: usize,
    next_id: u64,
    #[serde(skip)]
    event_sender: Option<UnboundedSender<TabEvent>>,
}

impl TabManager {
    /// Creates a new `TabManager` initialized with one tab at `initial_path`.
    pub fn new<P: AsRef<Path>>(pane_id: PaneId, initial_path: P) -> Self {
        let mut manager = Self {
            pane_id,
            tabs: Vec::new(),
            active_index: 0,
            next_id: 1,
            event_sender: None,
        };
        let tab_id = manager.alloc_id();
        let first_tab = Tab::new(tab_id, initial_path);
        manager.tabs.push(first_tab);
        manager
    }

    /// Sets an event channel sender for tab state mutations.
    pub fn set_event_sender(&mut self, sender: UnboundedSender<TabEvent>) {
        self.event_sender = Some(sender);
    }

    fn alloc_id(&mut self) -> TabId {
        let id = TabId(self.next_id);
        self.next_id += 1;
        id
    }

    fn notify(&self, event: TabEvent) {
        if let Some(ref sender) = self.event_sender {
            let _ = sender.send(event);
        }
    }

    /// Returns a slice of all tabs.
    pub fn tabs(&self) -> &[Tab] {
        &self.tabs
    }

    /// Returns the index of the active tab.
    pub fn active_index(&self) -> usize {
        self.active_index
    }

    /// Returns the number of tabs.
    pub fn len(&self) -> usize {
        self.tabs.len()
    }

    /// Returns `true` if there are no tabs.
    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }

    /// Returns a reference to the active tab.
    pub fn active_tab(&self) -> Option<&Tab> {
        self.tabs.get(self.active_index)
    }

    /// Returns a mutable reference to the active tab.
    pub fn active_tab_mut(&mut self) -> Option<&mut Tab> {
        self.tabs.get_mut(self.active_index)
    }

    /// Opens a new tab at the specified path and activates it.
    pub fn open_tab<P: AsRef<Path>>(&mut self, path: P) -> TabId {
        let insert_index = if self.tabs.is_empty() {
            0
        } else {
            self.active_index + 1
        };
        self.open_tab_at(path, insert_index)
    }

    /// Opens a new tab at a specific index.
    pub fn open_tab_at<P: AsRef<Path>>(&mut self, path: P, index: usize) -> TabId {
        let id = self.alloc_id();
        let path_buf = path.as_ref().to_path_buf();
        let new_tab = Tab::new(id, &path_buf);

        let clamped_index = index.min(self.tabs.len());
        self.tabs.insert(clamped_index, new_tab);
        self.active_index = clamped_index;

        self.notify(TabEvent::Opened {
            pane_id: self.pane_id,
            tab_id: id,
            path: path_buf,
        });
        self.notify(TabEvent::Switched {
            pane_id: self.pane_id,
            active_tab_id: id,
        });

        id
    }

    /// Closes the tab with the given `TabId`. Returns the closed tab if found.
    pub fn close_tab(&mut self, id: TabId) -> Option<Tab> {
        if let Some(pos) = self.tabs.iter().position(|t| t.id == id) {
            self.close_tab_at(pos)
        } else {
            None
        }
    }

    /// Closes the tab at `index`. Adjusts active index as necessary.
    pub fn close_tab_at(&mut self, index: usize) -> Option<Tab> {
        if index >= self.tabs.len() {
            return None;
        }

        let closed_tab = self.tabs.remove(index);
        let closed_id = closed_tab.id;

        self.notify(TabEvent::Closed {
            pane_id: self.pane_id,
            tab_id: closed_id,
        });

        if self.tabs.is_empty() {
            self.active_index = 0;
        } else {
            if self.active_index > index || self.active_index >= self.tabs.len() {
                self.active_index = self.active_index.saturating_sub(1);
            }
            if let Some(active_tab) = self.active_tab() {
                let active_id = active_tab.id;
                self.notify(TabEvent::Switched {
                    pane_id: self.pane_id,
                    active_tab_id: active_id,
                });
            }
        }

        Some(closed_tab)
    }

    /// Closes all tabs except the specified one.
    pub fn close_other_tabs(&mut self, keep_id: TabId) {
        if let Some(keep_tab) = self.tabs.iter().find(|t| t.id == keep_id).cloned() {
            for tab in &self.tabs {
                if tab.id != keep_id {
                    self.notify(TabEvent::Closed {
                        pane_id: self.pane_id,
                        tab_id: tab.id,
                    });
                }
            }
            self.tabs = vec![keep_tab];
            self.active_index = 0;
            self.notify(TabEvent::Switched {
                pane_id: self.pane_id,
                active_tab_id: keep_id,
            });
        }
    }

    /// Closes all tabs to the right of the specified tab.
    pub fn close_tabs_to_right(&mut self, id: TabId) {
        if let Some(pos) = self.tabs.iter().position(|t| t.id == id) {
            let to_remove: Vec<TabId> = self.tabs[pos + 1..].iter().map(|t| t.id).collect();
            for tab_id in to_remove {
                self.close_tab(tab_id);
            }
        }
    }

    /// Selects tab by ID.
    pub fn select_tab(&mut self, id: TabId) -> bool {
        if let Some(pos) = self.tabs.iter().position(|t| t.id == id) {
            self.select_tab_at(pos)
        } else {
            false
        }
    }

    /// Selects tab by index.
    pub fn select_tab_at(&mut self, index: usize) -> bool {
        if index < self.tabs.len() {
            self.active_index = index;
            let active_id = self.tabs[index].id;
            self.notify(TabEvent::Switched {
                pane_id: self.pane_id,
                active_tab_id: active_id,
            });
            true
        } else {
            false
        }
    }

    /// Switches to the next tab, cycling to start if at the end.
    pub fn select_next_tab(&mut self) {
        if !self.tabs.is_empty() {
            let next_index = (self.active_index + 1) % self.tabs.len();
            self.select_tab_at(next_index);
        }
    }

    /// Switches to the previous tab, cycling to end if at start.
    pub fn select_previous_tab(&mut self) {
        if !self.tabs.is_empty() {
            let prev_index = if self.active_index == 0 {
                self.tabs.len() - 1
            } else {
                self.active_index - 1
            };
            self.select_tab_at(prev_index);
        }
    }

    /// Moves a tab from `from_index` to `to_index`.
    pub fn move_tab(&mut self, from_index: usize, to_index: usize) -> bool {
        if from_index >= self.tabs.len() || to_index >= self.tabs.len() || from_index == to_index {
            return false;
        }

        let active_id = self.active_tab().map(|t| t.id);
        let tab = self.tabs.remove(from_index);
        self.tabs.insert(to_index, tab);

        if let Some(id) = active_id {
            if let Some(new_active_pos) = self.tabs.iter().position(|t| t.id == id) {
                self.active_index = new_active_pos;
            }
        }
        true
    }

    /// Pins a tab by ID.
    pub fn pin_tab(&mut self, id: TabId) -> bool {
        if let Some(pos) = self.tabs.iter().position(|t| t.id == id) {
            if self.tabs[pos].is_pinned {
                return true;
            }
            self.tabs[pos].is_pinned = true;
            self.notify(TabEvent::PinnedChanged {
                pane_id: self.pane_id,
                tab_id: id,
                is_pinned: true,
            });
            true
        } else {
            false
        }
    }

    /// Unpins a tab by ID.
    pub fn unpin_tab(&mut self, id: TabId) -> bool {
        if let Some(pos) = self.tabs.iter().position(|t| t.id == id) {
            if !self.tabs[pos].is_pinned {
                return true;
            }
            self.tabs[pos].is_pinned = false;
            self.notify(TabEvent::PinnedChanged {
                pane_id: self.pane_id,
                tab_id: id,
                is_pinned: false,
            });
            true
        } else {
            false
        }
    }

    /// Duplicates the tab with given ID and inserts it next to original.
    pub fn duplicate_tab(&mut self, id: TabId) -> Option<TabId> {
        let (path, pos, kind) = {
            let tab = self.tabs.iter().find(|t| t.id == id)?;
            (tab.path.clone(), self.tabs.iter().position(|t| t.id == id)?, tab.kind.clone())
        };

        let new_id = self.alloc_id();
        let mut dup_tab = Tab::with_kind(new_id, &path, kind);
        dup_tab.title = format!("{} (copy)", self.tabs[pos].title);

        let insert_index = pos + 1;
        self.tabs.insert(insert_index, dup_tab);
        self.active_index = insert_index;

        self.notify(TabEvent::Opened {
            pane_id: self.pane_id,
            tab_id: new_id,
            path,
        });
        self.notify(TabEvent::Switched {
            pane_id: self.pane_id,
            active_tab_id: new_id,
        });

        Some(new_id)
    }

    /// Navigates the active tab to `path`.
    pub fn navigate_active<P: AsRef<Path>>(&mut self, path: P) -> bool {
        let pane_id = self.pane_id;
        if let Some(tab) = self.active_tab_mut() {
            tab.navigate_to(path.as_ref());
            let tab_id = tab.id;
            let path_buf = path.as_ref().to_path_buf();
            self.notify(TabEvent::Navigated {
                pane_id,
                tab_id,
                path: path_buf,
            });
            true
        } else {
            false
        }
    }
}

/// Dual-pane coordinator managing two pane tab managers and split layout configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DualPaneCoordinator {
    pub left_pane: TabManager,
    pub right_pane: TabManager,
    pub active_pane: PaneId,
    pub layout: DualPaneLayout,
    #[serde(skip)]
    event_sender: Option<UnboundedSender<TabEvent>>,
}

impl DualPaneCoordinator {
    /// Creates a new dual-pane coordinator with `initial_path` on the left pane.
    pub fn new<P: AsRef<Path>>(initial_path: P) -> Self {
        let path = initial_path.as_ref();
        let left_pane = TabManager::new(PaneId::Left, path);
        let right_pane = TabManager::new(PaneId::Right, path);

        Self {
            left_pane,
            right_pane,
            active_pane: PaneId::Left,
            layout: DualPaneLayout::Single,
            event_sender: None,
        }
    }

    /// Attaches an event channel sender to both panes and the coordinator.
    pub fn set_event_channel(&mut self, sender: UnboundedSender<TabEvent>) {
        self.left_pane.set_event_sender(sender.clone());
        self.right_pane.set_event_sender(sender.clone());
        self.event_sender = Some(sender);
    }

    fn notify(&self, event: TabEvent) {
        if let Some(ref sender) = self.event_sender {
            let _ = sender.send(event);
        }
    }

    /// Returns current dual-pane layout mode.
    pub fn layout(&self) -> DualPaneLayout {
        self.layout
    }

    /// Toggles between single pane and horizontal dual pane mode.
    pub fn toggle_dual_pane(&mut self) -> DualPaneLayout {
        let new_layout = match self.layout {
            DualPaneLayout::Single => DualPaneLayout::SplitHorizontal,
            DualPaneLayout::SplitHorizontal | DualPaneLayout::SplitVertical => DualPaneLayout::Single,
        };
        self.set_layout(new_layout);
        new_layout
    }

    /// Sets dual pane layout explicitly.
    pub fn set_layout(&mut self, layout: DualPaneLayout) {
        self.layout = layout;
        if layout == DualPaneLayout::Single {
            self.active_pane = PaneId::Left;
        }
        self.notify(TabEvent::PaneLayoutChanged { layout });
    }

    /// Returns `true` if dual-pane mode is active.
    pub fn is_dual_pane_active(&self) -> bool {
        self.layout != DualPaneLayout::Single
    }

    /// Switches keyboard focus to the other pane (if dual pane is active).
    pub fn switch_active_pane(&mut self) {
        if self.is_dual_pane_active() {
            let next_pane = match self.active_pane {
                PaneId::Left => PaneId::Right,
                PaneId::Right => PaneId::Left,
            };
            self.set_active_pane(next_pane);
        }
    }

    /// Sets active pane ID.
    pub fn set_active_pane(&mut self, pane_id: PaneId) {
        self.active_pane = pane_id;
        self.notify(TabEvent::ActivePaneChanged {
            active_pane: pane_id,
        });
    }

    /// Returns current active pane ID.
    pub fn active_pane_id(&self) -> PaneId {
        self.active_pane
    }

    /// Returns reference to active pane tab manager.
    pub fn active_tab_manager(&self) -> &TabManager {
        match self.active_pane {
            PaneId::Left => &self.left_pane,
            PaneId::Right => &self.right_pane,
        }
    }

    /// Returns mutable reference to active pane tab manager.
    pub fn active_tab_manager_mut(&mut self) -> &mut TabManager {
        match self.active_pane {
            PaneId::Left => &mut self.left_pane,
            PaneId::Right => &mut self.right_pane,
        }
    }

    /// Returns reference to inactive pane tab manager.
    pub fn inactive_tab_manager(&self) -> &TabManager {
        match self.active_pane {
            PaneId::Left => &self.right_pane,
            PaneId::Right => &self.left_pane,
        }
    }

    /// Returns mutable reference to inactive pane tab manager.
    pub fn inactive_tab_manager_mut(&mut self) -> &mut TabManager {
        match self.active_pane {
            PaneId::Left => &mut self.right_pane,
            PaneId::Right => &mut self.left_pane,
        }
    }

    /// Returns reference to specific pane tab manager.
    pub fn get_pane(&self, pane_id: PaneId) -> &TabManager {
        match pane_id {
            PaneId::Left => &self.left_pane,
            PaneId::Right => &self.right_pane,
        }
    }

    /// Returns mutable reference to specific pane tab manager.
    pub fn get_pane_mut(&mut self, pane_id: PaneId) -> &mut TabManager {
        match pane_id {
            PaneId::Left => &mut self.left_pane,
            PaneId::Right => &mut self.right_pane,
        }
    }

    /// Swaps tabs between left and right panes.
    pub fn swap_panes(&mut self) {
        std::mem::swap(&mut self.left_pane, &mut self.right_pane);
        self.left_pane.pane_id = PaneId::Left;
        self.right_pane.pane_id = PaneId::Right;
        self.notify(TabEvent::PanesSwapped);
    }

    /// Opens a path in the inactive pane (and activates dual-pane mode if inactive).
    pub fn open_in_other_pane<P: AsRef<Path>>(&mut self, path: P) -> TabId {
        if !self.is_dual_pane_active() {
            self.set_layout(DualPaneLayout::SplitHorizontal);
        }
        let inactive_pane = match self.active_pane {
            PaneId::Left => PaneId::Right,
            PaneId::Right => PaneId::Left,
        };
        let tab_id = self.get_pane_mut(inactive_pane).open_tab(path);
        tab_id
    }

    /// Synchronizes inactive pane's active path to match the active pane's active path.
    pub fn sync_other_pane_to_active_path(&mut self) -> Option<TabId> {
        let active_path = self.active_tab_manager().active_tab()?.path.clone();
        Some(self.open_in_other_pane(active_path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tab_creation_and_title() {
        let path = PathBuf::from("/home/user/documents");
        let tab = Tab::new(TabId(1), &path);

        assert_eq!(tab.id, TabId(1));
        assert_eq!(tab.path, path);
        assert_eq!(tab.title, "documents");
        assert_eq!(tab.history, vec![path.clone()]);
        assert_eq!(tab.history_index, 0);
        assert!(!tab.can_go_back());
        assert!(!tab.can_go_forward());
        assert_eq!(tab.kind, TabKind::Standard);
    }

    #[test]
    fn test_tab_navigation_history() {
        let path1 = PathBuf::from("/home/user/folder1");
        let path2 = PathBuf::from("/home/user/folder2");
        let path3 = PathBuf::from("/home/user/folder3");

        let mut tab = Tab::new(TabId(1), &path1);

        tab.navigate_to(&path2);
        assert_eq!(tab.path, path2);
        assert_eq!(tab.history_index, 1);
        assert!(tab.can_go_back());
        assert!(!tab.can_go_forward());

        tab.navigate_to(&path3);
        assert_eq!(tab.path, path3);
        assert_eq!(tab.history_index, 2);

        // Test go_back
        assert_eq!(tab.go_back(), Some(&path2));
        assert_eq!(tab.path, path2);
        assert_eq!(tab.history_index, 1);
        assert!(tab.can_go_forward());

        assert_eq!(tab.go_back(), Some(&path1));
        assert_eq!(tab.path, path1);
        assert_eq!(tab.history_index, 0);
        assert!(!tab.can_go_back());

        // Attempt extra go_back
        assert_eq!(tab.go_back(), None);

        // Test go_forward
        assert_eq!(tab.go_forward(), Some(&path2));
        assert_eq!(tab.path, path2);
        assert_eq!(tab.history_index, 1);

        // Truncate history test: navigate from path2 to path4
        let path4 = PathBuf::from("/home/user/folder4");
        tab.navigate_to(&path4);

        assert_eq!(tab.history, vec![path1.clone(), path2.clone(), path4.clone()]);
        assert_eq!(tab.history_index, 2);
        assert!(!tab.can_go_forward());
    }

    #[test]
    fn test_tab_manager_open_and_select() {
        let root = PathBuf::from("/home/user");
        let mut manager = TabManager::new(PaneId::Left, &root);

        assert_eq!(manager.len(), 1);
        assert_eq!(manager.active_index(), 0);

        let p1 = PathBuf::from("/home/user/p1");
        let t1_id = manager.open_tab(&p1);

        assert_eq!(manager.len(), 2);
        assert_eq!(manager.active_index(), 1);
        assert_eq!(manager.active_tab().unwrap().id, t1_id);

        let p2 = PathBuf::from("/home/user/p2");
        let t2_id = manager.open_tab(&p2);

        assert_eq!(manager.len(), 3);
        assert_eq!(manager.active_index(), 2);
        assert_eq!(manager.active_tab().unwrap().id, t2_id);

        // Select previous
        manager.select_previous_tab();
        assert_eq!(manager.active_index(), 1);
        assert_eq!(manager.active_tab().unwrap().id, t1_id);

        // Select next
        manager.select_next_tab();
        assert_eq!(manager.active_index(), 2);

        // Select next wraps to 0
        manager.select_next_tab();
        assert_eq!(manager.active_index(), 0);

        // Select by ID
        assert!(manager.select_tab(t1_id));
        assert_eq!(manager.active_index(), 1);
    }

    #[test]
    fn test_tab_manager_close_and_reorder() {
        let p0 = PathBuf::from("/home/user/p0");
        let p1 = PathBuf::from("/home/user/p1");
        let p2 = PathBuf::from("/home/user/p2");

        let mut manager = TabManager::new(PaneId::Left, &p0);
        let id0 = manager.active_tab().unwrap().id;
        let id1 = manager.open_tab(&p1);
        let id2 = manager.open_tab(&p2);

        assert_eq!(manager.len(), 3);
        assert_eq!(manager.active_index(), 2);

        // Move tab 2 to 0
        assert!(manager.move_tab(2, 0));
        assert_eq!(manager.tabs()[0].id, id2);
        assert_eq!(manager.tabs()[1].id, id0);
        assert_eq!(manager.tabs()[2].id, id1);
        assert_eq!(manager.active_index(), 0); // active tab id2 remains active at pos 0

        // Close tab 0
        let closed = manager.close_tab(id2);
        assert!(closed.is_some());
        assert_eq!(manager.len(), 2);
        assert_eq!(manager.active_index(), 0);

        // Close other tabs
        manager.close_other_tabs(id1);
        assert_eq!(manager.len(), 1);
        assert_eq!(manager.active_tab().unwrap().id, id1);
    }

    #[test]
    fn test_pin_and_duplicate_tab() {
        let p0 = PathBuf::from("/home/user/p0");
        let mut manager = TabManager::new(PaneId::Left, &p0);
        let id0 = manager.active_tab().unwrap().id;

        assert!(manager.pin_tab(id0));
        assert!(manager.tabs()[0].is_pinned);

        assert!(manager.unpin_tab(id0));
        assert!(!manager.tabs()[0].is_pinned);

        let dup_id = manager.duplicate_tab(id0);
        assert!(dup_id.is_some());
        assert_eq!(manager.len(), 2);
        assert_eq!(manager.active_index(), 1);
        assert!(manager.active_tab().unwrap().title.contains("(copy)"));
    }

    #[test]
    fn test_dual_pane_coordinator() {
        let root = PathBuf::from("/home/user/root");
        let mut coord = DualPaneCoordinator::new(&root);

        assert_eq!(coord.layout(), DualPaneLayout::Single);
        assert!(!coord.is_dual_pane_active());
        assert_eq!(coord.active_pane_id(), PaneId::Left);

        // Toggle layout
        coord.toggle_dual_pane();
        assert_eq!(coord.layout(), DualPaneLayout::SplitHorizontal);
        assert!(coord.is_dual_pane_active());

        // Switch pane
        coord.switch_active_pane();
        assert_eq!(coord.active_pane_id(), PaneId::Right);

        // Open path in other pane
        let other_path = PathBuf::from("/home/user/other");
        let new_tab_id = coord.open_in_other_pane(&other_path);
        assert_eq!(coord.left_pane.active_tab().unwrap().id, new_tab_id);

        // Swap panes
        coord.swap_panes();
        assert_eq!(coord.left_pane.pane_id, PaneId::Left);
        assert_eq!(coord.right_pane.pane_id, PaneId::Right);

        // Sync path
        let synced_tab = coord.sync_other_pane_to_active_path();
        assert!(synced_tab.is_some());
    }

    #[tokio::test]
    async fn test_async_event_broadcasting() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let root = PathBuf::from("/home/user/events");
        let mut coord = DualPaneCoordinator::new(&root);
        coord.set_event_channel(tx);

        coord.toggle_dual_pane();
        let ev1 = rx.recv().await;
        assert_eq!(ev1, Some(TabEvent::PaneLayoutChanged { layout: DualPaneLayout::SplitHorizontal }));

        coord.active_tab_manager_mut().open_tab("/home/user/new");
        let ev2 = rx.recv().await;
        assert!(matches!(ev2, Some(TabEvent::Opened { pane_id: PaneId::Left, .. })));
    }

    #[test]
    fn test_serde_serialization() {
        let root = PathBuf::from("/home/user/serde");
        let coord = DualPaneCoordinator::new(&root);

        let json = serde_json::to_string(&coord).expect("serialization failed");
        let deserialized: DualPaneCoordinator = serde_json::from_str(&json).expect("deserialization failed");

        assert_eq!(deserialized.active_pane, coord.active_pane);
        assert_eq!(deserialized.layout, coord.layout);
        assert_eq!(deserialized.left_pane.tabs().len(), coord.left_pane.tabs().len());
    }
}
