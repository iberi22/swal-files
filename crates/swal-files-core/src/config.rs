use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use tokio::fs as tokio_fs;

/// Custom error type for configuration operations.
#[derive(Debug)]
pub enum ConfigError {
    /// I/O error occurred while reading or writing configuration.
    Io(io::Error),
    /// Error parsing or serializing JSON data.
    Json(serde_json::Error),
    /// Could not determine user configuration directory.
    ConfigDirNotFound,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::Io(err) => write!(f, "I/O error: {}", err),
            ConfigError::Json(err) => write!(f, "JSON error: {}", err),
            ConfigError::ConfigDirNotFound => {
                write!(f, "Could not determine user configuration directory")
            }
        }
    }
}

impl Error for ConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ConfigError::Io(err) => Some(err),
            ConfigError::Json(err) => Some(err),
            ConfigError::ConfigDirNotFound => None,
        }
    }
}

impl From<io::Error> for ConfigError {
    fn from(err: io::Error) -> Self {
        ConfigError::Io(err)
    }
}

impl From<serde_json::Error> for ConfigError {
    fn from(err: serde_json::Error) -> Self {
        ConfigError::Json(err)
    }
}

/// Theme options conforming to `@swal/ui` design system specs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThemeConfig {
    /// Name of the theme (e.g. "swal-dark", "swal-light").
    pub theme_name: String,
    /// Enables Wayland Mica / Acrylic glassmorphism translucent backgrounds.
    pub enable_mica: bool,
    /// Window background opacity (0.0 to 1.0).
    pub opacity: f32,
    /// Accent color hex string (e.g. "#0078D4").
    pub accent_color: String,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            theme_name: "swal-dark".to_string(),
            enable_mica: true,
            opacity: 0.85,
            accent_color: "#0078D4".to_string(),
        }
    }
}

/// View options for SWAL Files explorer views.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ViewConfig {
    /// Whether to display hidden files (starting with '.').
    pub show_hidden_files: bool,
    /// Default view mode: "details", "grid", or "columns".
    pub default_view_mode: String,
    /// Field to sort by: "name", "size", "modified", "type".
    pub sort_by: String,
    /// Sort orientation: true for ascending, false for descending.
    pub sort_ascending: bool,
    /// Dual-pane view mode enabled by default (F6 toggle).
    pub dual_pane_enabled: bool,
}

impl Default for ViewConfig {
    fn default() -> Self {
        Self {
            show_hidden_files: false,
            default_view_mode: "columns".to_string(),
            sort_by: "name".to_string(),
            sort_ascending: true,
            dual_pane_enabled: false,
        }
    }
}

/// Application behavior configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BehaviorConfig {
    /// Prompt confirmation before deleting files.
    pub confirm_delete: bool,
    /// Command line pattern for launching external terminal.
    pub open_in_terminal_cmd: String,
    /// Single click to open items instead of double click.
    pub single_click_to_open: bool,
    /// Maximum search/scan depth for file indexer.
    pub max_scan_depth: usize,
}

impl Default for BehaviorConfig {
    fn default() -> Self {
        Self {
            confirm_delete: true,
            open_in_terminal_cmd: "alacritty --working-directory %p".to_string(),
            single_click_to_open: false,
            max_scan_depth: 20,
        }
    }
}

/// Root configuration structure for SWAL Files (`~/.config/swal/files/config.json`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct AppConfig {
    /// UI Theme configuration.
    pub theme: ThemeConfig,
    /// File view configuration.
    pub view: ViewConfig,
    /// Application behavior configuration.
    pub behavior: BehaviorConfig,
}

impl AppConfig {
    /// Resolves the default configuration file path: `~/.config/swal/files/config.json`.
    pub fn get_config_path() -> Result<PathBuf, ConfigError> {
        let mut config_dir = dirs::config_dir().ok_or(ConfigError::ConfigDirNotFound)?;
        config_dir.push("swal");
        config_dir.push("files");
        config_dir.push("config.json");
        Ok(config_dir)
    }

    /// Loads configuration from default location (`~/.config/swal/files/config.json`).
    /// If file does not exist, writes and returns default configuration.
    pub fn load() -> Result<Self, ConfigError> {
        let path = Self::get_config_path()?;
        Self::load_from_path(&path)
    }

    /// Loads configuration from a specified file path.
    /// Creates and writes default configuration if file does not exist.
    pub fn load_from_path<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        if !path.exists() {
            let config = Self::default();
            config.save_to_path(path)?;
            return Ok(config);
        }

        let content = fs::read_to_string(path)?;
        let config: Self = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Asynchronously loads configuration from default location.
    pub async fn load_async() -> Result<Self, ConfigError> {
        let path = Self::get_config_path()?;
        Self::load_from_path_async(&path).await
    }

    /// Asynchronously loads configuration from a specified file path.
    pub async fn load_from_path_async<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        if !path.exists() {
            let config = Self::default();
            config.save_to_path_async(path).await?;
            return Ok(config);
        }

        let content = tokio_fs::read_to_string(path).await?;
        let config: Self = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Saves configuration to default path (`~/.config/swal/files/config.json`).
    pub fn save(&self) -> Result<(), ConfigError> {
        let path = Self::get_config_path()?;
        self.save_to_path(&path)
    }

    /// Saves configuration to specified path, creating parent directories if necessary.
    pub fn save_to_path<P: AsRef<Path>>(&self, path: P) -> Result<(), ConfigError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json_data = serde_json::to_string_pretty(self)?;
        fs::write(path, json_data)?;
        Ok(())
    }

    /// Asynchronously saves configuration to default path.
    pub async fn save_async(&self) -> Result<(), ConfigError> {
        let path = Self::get_config_path()?;
        self.save_to_path_async(&path).await
    }

    /// Asynchronously saves configuration to specified path.
    pub async fn save_to_path_async<P: AsRef<Path>>(&self, path: P) -> Result<(), ConfigError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            tokio_fs::create_dir_all(parent).await?;
        }
        let json_data = serde_json::to_string_pretty(self)?;
        tokio_fs::write(path, json_data).await?;
        Ok(())
    }

    /// Mutates configuration using provided closure and saves changes synchronously to default path.
    pub fn update<F>(&mut self, updater: F) -> Result<(), ConfigError>
    where
        F: FnOnce(&mut AppConfig),
    {
        updater(self);
        self.save()
    }

    /// Mutates configuration using provided closure and saves changes synchronously to specified path.
    pub fn update_at<F, P: AsRef<Path>>(&mut self, path: P, updater: F) -> Result<(), ConfigError>
    where
        F: FnOnce(&mut AppConfig),
    {
        updater(self);
        self.save_to_path(path)
    }

    /// Mutates configuration using provided closure and saves changes asynchronously to specified path.
    pub fn update_at_async<'a, F, P: AsRef<Path> + 'a>(
        &'a mut self,
        path: P,
        updater: F,
    ) -> impl std::future::Future<Output = Result<(), ConfigError>> + 'a
    where
        F: FnOnce(&mut AppConfig) + 'a,
    {
        updater(self);
        self.save_to_path_async(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_default_config_values() {
        let config = AppConfig::default();
        assert_eq!(config.theme.theme_name, "swal-dark");
        assert!(config.theme.enable_mica);
        assert_eq!(config.theme.opacity, 0.85);
        assert_eq!(config.theme.accent_color, "#0078D4");

        assert!(!config.view.show_hidden_files);
        assert_eq!(config.view.default_view_mode, "columns");
        assert_eq!(config.view.sort_by, "name");
        assert!(config.view.sort_ascending);
        assert!(!config.view.dual_pane_enabled);

        assert!(config.behavior.confirm_delete);
        assert_eq!(
            config.behavior.open_in_terminal_cmd,
            "alacritty --working-directory %p"
        );
        assert!(!config.behavior.single_click_to_open);
        assert_eq!(config.behavior.max_scan_depth, 20);
    }

    #[test]
    fn test_json_serialization_deserialization() {
        let original = AppConfig::default();
        let json_str = serde_json::to_string(&original).expect("Serialization failed");
        let deserialized: AppConfig =
            serde_json::from_str(&json_str).expect("Deserialization failed");
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_save_and_load_sync() {
        let dir = tempdir().expect("Failed to create temp dir");
        let config_file = dir.path().join("sub/dir/config.json");

        let mut config = AppConfig::default();
        config.theme.theme_name = "swal-light".to_string();
        config.view.show_hidden_files = true;

        config
            .save_to_path(&config_file)
            .expect("Save to path failed");
        assert!(config_file.exists());

        let loaded = AppConfig::load_from_path(&config_file).expect("Load from path failed");
        assert_eq!(loaded, config);
    }

    #[test]
    fn test_load_non_existent_file_creates_default() {
        let dir = tempdir().expect("Failed to create temp dir");
        let config_file = dir.path().join("non_existent_config.json");

        assert!(!config_file.exists());
        let loaded = AppConfig::load_from_path(&config_file).expect("Load should auto-create file");
        assert!(config_file.exists());
        assert_eq!(loaded, AppConfig::default());
    }

    #[test]
    fn test_update_at() {
        let dir = tempdir().expect("Failed to create temp dir");
        let config_file = dir.path().join("config.json");

        let mut config = AppConfig::default();
        config
            .save_to_path(&config_file)
            .expect("Initial save failed");

        config
            .update_at(&config_file, |c| {
                c.theme.accent_color = "#FF0000".to_string();
                c.view.dual_pane_enabled = true;
            })
            .expect("Update failed");

        let loaded = AppConfig::load_from_path(&config_file).expect("Load failed");
        assert_eq!(loaded.theme.accent_color, "#FF0000");
        assert!(loaded.view.dual_pane_enabled);
    }

    #[tokio::test]
    async fn test_async_save_load_update() {
        let dir = tempdir().expect("Failed to create temp dir");
        let config_file = dir.path().join("async_config.json");

        let mut config = AppConfig::default();
        config.theme.opacity = 0.95;

        config
            .save_to_path_async(&config_file)
            .await
            .expect("Async save failed");
        assert!(config_file.exists());

        let loaded = AppConfig::load_from_path_async(&config_file)
            .await
            .expect("Async load failed");
        assert_eq!(loaded, config);

        config
            .update_at_async(&config_file, |c| {
                c.behavior.single_click_to_open = true;
            })
            .await
            .expect("Async update failed");

        let reloaded = AppConfig::load_from_path_async(&config_file)
            .await
            .expect("Reload failed");
        assert!(reloaded.behavior.single_click_to_open);
    }

    #[test]
    fn test_invalid_json_returns_error() {
        let dir = tempdir().expect("Failed to create temp dir");
        let config_file = dir.path().join("invalid.json");

        fs::write(&config_file, "{ invalid json }").expect("Write invalid file failed");

        let result = AppConfig::load_from_path(&config_file);
        assert!(result.is_err());
        if let Err(ConfigError::Json(_)) = result {
            // Success
        } else {
            panic!("Expected ConfigError::Json");
        }
    }

    #[test]
    fn test_get_config_path() {
        let path_res = AppConfig::get_config_path();
        assert!(path_res.is_ok());
        let path = path_res.unwrap();
        assert!(path.ends_with("swal/files/config.json"));
    }

    #[test]
    fn test_config_error_display_and_source() {
        let io_err = ConfigError::Io(io::Error::new(io::ErrorKind::NotFound, "file not found"));
        assert!(io_err.to_string().contains("I/O error"));
        assert!(io_err.source().is_some());

        let dir_err = ConfigError::ConfigDirNotFound;
        assert!(dir_err.to_string().contains("Could not determine user configuration directory"));
        assert!(dir_err.source().is_none());
    }
}
