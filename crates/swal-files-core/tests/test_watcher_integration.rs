#![deny(unsafe_code)]

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::sleep;

use swal_files_core::watcher::{FileWatchEvent, FileWatcher, WatcherConfig, WatcherError};

/// Test harness helper for real-time inotify watcher integration testing.
pub struct WatcherIntegrationTest {
    pub temp_dir: TempDir,
    pub watcher: FileWatcher,
}

impl WatcherIntegrationTest {
    /// Creates a new test harness with a temporary directory and watcher watching it.
    pub fn new() -> Result<Self, WatcherError> {
        let temp_dir = TempDir::new()?;
        let mut watcher = FileWatcher::new()?;
        watcher.watch(temp_dir.path())?;
        Ok(Self { temp_dir, watcher })
    }

    /// Creates a new test harness with specified watcher configuration.
    pub fn with_config(config: WatcherConfig) -> Result<Self, WatcherError> {
        let temp_dir = TempDir::new()?;
        let mut watcher = FileWatcher::with_config(config)?;
        watcher.watch(temp_dir.path())?;
        Ok(Self { temp_dir, watcher })
    }

    /// Returns the path to the root temporary directory.
    pub fn root_path(&self) -> &Path {
        self.temp_dir.path()
    }

    /// Creates a file with contents within the temporary directory.
    pub fn create_file(&self, relative_name: &str, content: &[u8]) -> std::io::Result<PathBuf> {
        let path = self.root_path().join(relative_name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = File::create(&path)?;
        file.write_all(content)?;
        file.sync_all()?;
        Ok(path)
    }

    /// Modifies an existing file within the temporary directory.
    pub fn append_file(&self, relative_name: &str, content: &[u8]) -> std::io::Result<PathBuf> {
        let path = self.root_path().join(relative_name);
        let mut file = fs::OpenOptions::new().append(true).open(&path)?;
        file.write_all(content)?;
        file.sync_all()?;
        Ok(path)
    }

    /// Deletes a file within the temporary directory.
    pub fn delete_file(&self, relative_name: &str) -> std::io::Result<()> {
        let path = self.root_path().join(relative_name);
        fs::remove_file(path)
    }

    /// Drains all currently pending events from the channel queue.
    pub fn drain_events(&mut self) {
        while self.watcher.try_recv().is_ok() {}
    }

    /// Collects incoming events matching the given target filename within timeout.
    pub async fn wait_for_file_event(
        &mut self,
        target_name: &str,
        timeout: Duration,
    ) -> Vec<FileWatchEvent> {
        let start = std::time::Instant::now();
        let mut events = Vec::new();
        while start.elapsed() < timeout {
            while let Ok(ev) = self.watcher.try_recv() {
                if ev.path().file_name().and_then(|s| s.to_str()) == Some(target_name) {
                    events.push(ev);
                }
            }
            if !events.is_empty() {
                break;
            }
            sleep(Duration::from_millis(20)).await;
        }
        events
    }

    /// Waits for an event matching a predicate on a target filename within timeout.
    pub async fn wait_for_event_matching<F>(
        &mut self,
        target_name: &str,
        timeout: Duration,
        predicate: F,
    ) -> Option<FileWatchEvent>
    where
        F: Fn(&FileWatchEvent) -> bool,
    {
        let start = std::time::Instant::now();
        while start.elapsed() < timeout {
            while let Ok(ev) = self.watcher.try_recv() {
                if ev.path().file_name().and_then(|s| s.to_str()) == Some(target_name)
                    && predicate(&ev)
                {
                    return Some(ev);
                }
            }
            sleep(Duration::from_millis(20)).await;
        }
        None
    }
}

#[tokio::test]
async fn test_realtime_create_delete_stream() {
    let mut harness = WatcherIntegrationTest::new().expect("Failed to initialize test harness");
    let file_name = "stream_test.txt";

    // 1. Create file and verify creation event in stream
    let created_path = harness
        .create_file(file_name, b"Initial stream data")
        .expect("Failed to create file");
    assert!(created_path.exists());

    let create_event = harness
        .wait_for_event_matching(file_name, Duration::from_secs(2), |e| {
            matches!(e, FileWatchEvent::Created(_)) || matches!(e, FileWatchEvent::Modified(_))
        })
        .await;
    assert!(
        create_event.is_some(),
        "Expected Created or Modified event on file creation"
    );

    // Drain queued creation/modification events before triggering deletion
    harness.drain_events();

    // 2. Delete file and verify deletion event stream
    harness
        .delete_file(file_name)
        .expect("Failed to delete file");
    assert!(!created_path.exists());

    let delete_event = harness
        .wait_for_event_matching(file_name, Duration::from_secs(2), |e| {
            matches!(e, FileWatchEvent::Deleted(_))
        })
        .await;
    assert!(
        delete_event.is_some(),
        "Expected FileWatchEvent::Deleted event upon file removal"
    );
}

#[tokio::test]
async fn test_watcher_debounce_channels() {
    let mut harness = WatcherIntegrationTest::new().expect("Failed to initialize test harness");
    let file_name = "debounce_test.txt";

    // Perform rapid writes to test event channel streaming stability
    harness
        .create_file(file_name, b"v1")
        .expect("Failed to create file");

    for i in 2..=5 {
        sleep(Duration::from_millis(10)).await;
        let _ = harness.append_file(file_name, format!(" v{i}").as_bytes());
    }

    let events = harness
        .wait_for_file_event(file_name, Duration::from_secs(2))
        .await;
    assert!(
        !events.is_empty(),
        "Expected events from channel during rapid writes"
    );
}

#[tokio::test]
async fn test_nested_directory_recursive_watch() {
    let mut harness = WatcherIntegrationTest::new().expect("Failed to initialize test harness");
    let sub_file = "subdir/nested_test.txt";

    harness
        .create_file(sub_file, b"nested content")
        .expect("Failed to create nested file");

    let events = harness
        .wait_for_file_event("nested_test.txt", Duration::from_secs(2))
        .await;
    assert!(
        !events.is_empty(),
        "Expected recursive event for nested file"
    );
}

#[tokio::test]
async fn test_watcher_unwatch_lifecycle() {
    let mut harness = WatcherIntegrationTest::new().expect("Failed to initialize test harness");
    let root = harness.root_path().to_path_buf();

    assert!(harness.watcher.is_watching(&root));

    harness
        .watcher
        .unwatch(&root)
        .expect("Failed to unwatch path");
    assert!(!harness.watcher.is_watching(&root));
    assert!(harness.watcher.watched_paths().is_empty());
}
