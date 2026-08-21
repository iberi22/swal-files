#![deny(unsafe_code)]

use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;

/// Event emitted by the filesystem watcher.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileWatchEvent {
    /// A file or directory was created.
    Created(PathBuf),
    /// A file or directory was modified.
    Modified(PathBuf),
    /// A file or directory was deleted.
    Deleted(PathBuf),
    /// A file or directory was renamed.
    Renamed { from: PathBuf, to: PathBuf },
    /// Other filesystem event (access, metadata change, etc.).
    Other(PathBuf),
}

impl FileWatchEvent {
    /// Returns the primary path associated with this event.
    pub fn path(&self) -> &Path {
        match self {
            FileWatchEvent::Created(p) => p,
            FileWatchEvent::Modified(p) => p,
            FileWatchEvent::Deleted(p) => p,
            FileWatchEvent::Renamed { to, .. } => to,
            FileWatchEvent::Other(p) => p,
        }
    }
}

/// Configuration options for `FileWatcher`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WatcherConfig {
    /// Whether to watch directories recursively by default.
    pub recursive: bool,
    /// Optional poll interval in milliseconds (if polling backend is used).
    pub poll_interval_ms: Option<u64>,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            recursive: true,
            poll_interval_ms: None,
        }
    }
}

/// Errors that can occur within `FileWatcher`.
#[derive(Debug)]
pub enum WatcherError {
    /// Underlying notify crate error.
    Notify(notify::Error),
    /// IO error.
    Io(std::io::Error),
    /// Path does not exist or is invalid.
    PathNotFound(PathBuf),
    /// Event channel was closed.
    ChannelClosed,
    /// Generic watcher error message.
    Custom(String),
}

impl fmt::Display for WatcherError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WatcherError::Notify(err) => write!(f, "Notify error: {err}"),
            WatcherError::Io(err) => write!(f, "IO error: {err}"),
            WatcherError::PathNotFound(path) => write!(f, "Path not found: {}", path.display()),
            WatcherError::ChannelClosed => write!(f, "Watcher channel closed"),
            WatcherError::Custom(msg) => write!(f, "Watcher error: {msg}"),
        }
    }
}

impl std::error::Error for WatcherError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            WatcherError::Notify(err) => Some(err),
            WatcherError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<notify::Error> for WatcherError {
    fn from(err: notify::Error) -> Self {
        WatcherError::Notify(err)
    }
}

impl From<std::io::Error> for WatcherError {
    fn from(err: std::io::Error) -> Self {
        WatcherError::Io(err)
    }
}

/// Translates raw `notify::Event` into zero or more high-level `FileWatchEvent` instances.
pub fn parse_notify_event(event: Event) -> Vec<FileWatchEvent> {
    use notify::event::ModifyKind;

    let mut events = Vec::new();
    match event.kind {
        EventKind::Create(_) => {
            for path in event.paths {
                events.push(FileWatchEvent::Created(path));
            }
        }
        EventKind::Remove(_) => {
            for path in event.paths {
                events.push(FileWatchEvent::Deleted(path));
            }
        }
        EventKind::Modify(ModifyKind::Name(_)) => {
            if event.paths.len() >= 2 {
                events.push(FileWatchEvent::Renamed {
                    from: event.paths[0].clone(),
                    to: event.paths[1].clone(),
                });
            } else {
                for path in event.paths {
                    events.push(FileWatchEvent::Modified(path));
                }
            }
        }
        EventKind::Modify(_) => {
            for path in event.paths {
                events.push(FileWatchEvent::Modified(path));
            }
        }
        _ => {
            for path in event.paths {
                events.push(FileWatchEvent::Other(path));
            }
        }
    }
    events
}

/// Real-time inotify-backed filesystem watcher.
pub struct FileWatcher {
    watcher: RecommendedWatcher,
    receiver: mpsc::UnboundedReceiver<FileWatchEvent>,
    watched_paths: HashSet<PathBuf>,
    config: WatcherConfig,
}

impl FileWatcher {
    /// Creates a new `FileWatcher` with default configuration.
    pub fn new() -> Result<Self, WatcherError> {
        Self::with_config(WatcherConfig::default())
    }

    /// Creates a new `FileWatcher` with specified configuration.
    pub fn with_config(config: WatcherConfig) -> Result<Self, WatcherError> {
        let (tx, rx) = mpsc::unbounded_channel();

        let notify_config = Config::default();
        let watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    let watch_events = parse_notify_event(event);
                    for ev in watch_events {
                        let _ = tx.send(ev);
                    }
                }
            },
            notify_config,
        )?;

        Ok(Self {
            watcher,
            receiver: rx,
            watched_paths: HashSet::new(),
            config,
        })
    }

    /// Returns the current configuration of the watcher.
    pub fn config(&self) -> &WatcherConfig {
        &self.config
    }

    /// Start watching a path using default recursive mode setting.
    pub fn watch<P: AsRef<Path>>(&mut self, path: P) -> Result<(), WatcherError> {
        let recursive = self.config.recursive;
        self.watch_recursive(path, recursive)
    }

    /// Start watching a path with explicit recursive mode.
    pub fn watch_recursive<P: AsRef<Path>>(&mut self, path: P, recursive: bool) -> Result<(), WatcherError> {
        let path_ref = path.as_ref();
        if !path_ref.exists() {
            return Err(WatcherError::PathNotFound(path_ref.to_path_buf()));
        }

        let canonical = path_ref.canonicalize().unwrap_or_else(|_| path_ref.to_path_buf());
        let mode = if recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };

        self.watcher.watch(&canonical, mode)?;
        self.watched_paths.insert(canonical);
        Ok(())
    }

    /// Stop watching a specified path.
    pub fn unwatch<P: AsRef<Path>>(&mut self, path: P) -> Result<(), WatcherError> {
        let path_ref = path.as_ref();
        let canonical = path_ref.canonicalize().unwrap_or_else(|_| path_ref.to_path_buf());

        if self.watched_paths.remove(&canonical) || self.watched_paths.remove(path_ref) {
            let _ = self.watcher.unwatch(&canonical);
            let _ = self.watcher.unwatch(path_ref);
            Ok(())
        } else {
            Err(WatcherError::PathNotFound(path_ref.to_path_buf()))
        }
    }

    /// Checks if a path is currently being watched.
    pub fn is_watching<P: AsRef<Path>>(&self, path: P) -> bool {
        let path_ref = path.as_ref();
        if self.watched_paths.contains(path_ref) {
            return true;
        }
        if let Ok(canonical) = path_ref.canonicalize() {
            if self.watched_paths.contains(&canonical) {
                return true;
            }
        }
        false
    }

    /// Returns a list of all paths currently being watched.
    pub fn watched_paths(&self) -> Vec<PathBuf> {
        self.watched_paths.iter().cloned().collect()
    }

    /// Clears all watched paths.
    pub fn clear_watched(&mut self) -> Result<(), WatcherError> {
        let paths: Vec<PathBuf> = self.watched_paths.drain().collect();
        for path in paths {
            let _ = self.watcher.unwatch(&path);
        }
        Ok(())
    }

    /// Receives the next filesystem event asynchronously.
    pub async fn recv(&mut self) -> Option<FileWatchEvent> {
        self.receiver.recv().await
    }

    /// Non-blocking attempt to receive the next event from channel.
    pub fn try_recv(&mut self) -> Result<FileWatchEvent, mpsc::error::TryRecvError> {
        self.receiver.try_recv()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use std::time::Duration;
    use tempfile::tempdir;
    use tokio::time::sleep;

    #[test]
    fn test_watcher_config_default() {
        let cfg = WatcherConfig::default();
        assert!(cfg.recursive);
        assert_eq!(cfg.poll_interval_ms, None);
    }

    #[test]
    fn test_watcher_creation() {
        let watcher = FileWatcher::new();
        assert!(watcher.is_ok());
        let w = watcher.unwrap();
        assert!(w.watched_paths().is_empty());
        assert_eq!(w.config().recursive, true);
    }

    #[test]
    fn test_watch_nonexistent_path() {
        let mut watcher = FileWatcher::new().unwrap();
        let res = watcher.watch("/nonexistent_path_swal_files_test_123456789");
        assert!(res.is_err());
        match res.unwrap_err() {
            WatcherError::PathNotFound(p) => {
                assert!(p.to_string_lossy().contains("nonexistent_path"));
            }
            err => panic!("Unexpected error: {err}"),
        }
    }

    #[test]
    fn test_watch_unwatch_lifecycle() {
        let dir = tempdir().unwrap();
        let dir_path = dir.path();

        let mut watcher = FileWatcher::new().unwrap();
        assert!(!watcher.is_watching(dir_path));

        watcher.watch(dir_path).unwrap();
        assert!(watcher.is_watching(dir_path));
        assert_eq!(watcher.watched_paths().len(), 1);

        watcher.unwatch(dir_path).unwrap();
        assert!(!watcher.is_watching(dir_path));
        assert!(watcher.watched_paths().is_empty());
    }

    #[test]
    fn test_unwatch_non_watched_path() {
        let dir = tempdir().unwrap();
        let mut watcher = FileWatcher::new().unwrap();
        let res = watcher.unwatch(dir.path());
        assert!(res.is_err());
    }

    #[test]
    fn test_clear_watched() {
        let dir1 = tempdir().unwrap();
        let dir2 = tempdir().unwrap();

        let mut watcher = FileWatcher::new().unwrap();
        watcher.watch(dir1.path()).unwrap();
        watcher.watch(dir2.path()).unwrap();
        assert_eq!(watcher.watched_paths().len(), 2);

        watcher.clear_watched().unwrap();
        assert!(watcher.watched_paths().is_empty());
    }

    #[test]
    fn test_parse_notify_event_create() {
        use notify::event::{CreateKind, EventKind};

        let path = PathBuf::from("/tmp/test.txt");
        let event = Event {
            kind: EventKind::Create(CreateKind::File),
            paths: vec![path.clone()],
            attrs: Default::default(),
        };

        let parsed = parse_notify_event(event);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0], FileWatchEvent::Created(path.clone()));
        assert_eq!(parsed[0].path(), path.as_path());
    }

    #[test]
    fn test_parse_notify_event_remove() {
        use notify::event::{EventKind, RemoveKind};

        let path = PathBuf::from("/tmp/test.txt");
        let event = Event {
            kind: EventKind::Remove(RemoveKind::File),
            paths: vec![path.clone()],
            attrs: Default::default(),
        };

        let parsed = parse_notify_event(event);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0], FileWatchEvent::Deleted(path));
    }

    #[test]
    fn test_parse_notify_event_rename() {
        use notify::event::{EventKind, ModifyKind, RenameMode};

        let path_from = PathBuf::from("/tmp/old.txt");
        let path_to = PathBuf::from("/tmp/new.txt");
        let event = Event {
            kind: EventKind::Modify(ModifyKind::Name(RenameMode::Both)),
            paths: vec![path_from.clone(), path_to.clone()],
            attrs: Default::default(),
        };

        let parsed = parse_notify_event(event);
        assert_eq!(parsed.len(), 1);
        assert_eq!(
            parsed[0],
            FileWatchEvent::Renamed {
                from: path_from,
                to: path_to.clone()
            }
        );
        assert_eq!(parsed[0].path(), path_to.as_path());
    }

    #[test]
    fn test_error_display() {
        let err = WatcherError::Custom("test error".to_string());
        assert_eq!(format!("{err}"), "Watcher error: test error");

        let err_path = WatcherError::PathNotFound(PathBuf::from("/test"));
        assert_eq!(format!("{err_path}"), "Path not found: /test");

        let err_closed = WatcherError::ChannelClosed;
        assert_eq!(format!("{err_closed}"), "Watcher channel closed");
    }

    #[tokio::test]
    async fn test_realtime_filesystem_event_detection() {
        let dir = tempdir().unwrap();
        let dir_path = dir.path();

        let mut watcher = FileWatcher::new().unwrap();
        watcher.watch(dir_path).unwrap();

        // Create a file in watched directory
        let file_path = dir_path.join("created_file.txt");
        {
            let mut f = File::create(&file_path).unwrap();
            f.write_all(b"hello world").unwrap();
            f.sync_all().unwrap();
        }

        // Wait for event with timeout
        let mut event_found = false;
        for _ in 0..20 {
            sleep(Duration::from_millis(50)).await;
            while let Ok(ev) = watcher.try_recv() {
                if ev.path().file_name() == Some(std::ffi::OsStr::new("created_file.txt")) {
                    event_found = true;
                    break;
                }
            }
            if event_found {
                break;
            }
        }
        assert!(event_found, "File creation event was not received");

        // Delete file
        fs::remove_file(&file_path).unwrap();

        let mut delete_found = false;
        for _ in 0..20 {
            sleep(Duration::from_millis(50)).await;
            while let Ok(ev) = watcher.try_recv() {
                if ev.path().file_name() == Some(std::ffi::OsStr::new("created_file.txt")) {
                    if matches!(ev, FileWatchEvent::Deleted(_)) {
                        delete_found = true;
                        break;
                    }
                }
            }
            if delete_found {
                break;
            }
        }
        assert!(delete_found, "File deletion event was not received");
    }
}
