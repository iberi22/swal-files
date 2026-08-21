#![deny(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Represents a Hyprland window rule configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HyprlandWindowRule {
    /// Class pattern matcher for the window (e.g., "swal-files")
    pub class_pattern: String,
    /// Rule effect (e.g., "float", "tile", "center", "size 1000 650")
    pub rule_type: String,
}

impl HyprlandWindowRule {
    /// Creates a new Hyprland window rule.
    pub fn new(class_pattern: impl Into<String>, rule_type: impl Into<String>) -> Self {
        Self {
            class_pattern: class_pattern.into(),
            rule_type: rule_type.into(),
        }
    }

    /// Formats the rule into Hyprland config line syntax (`windowrulev2 = <rule_type>, class:^({class_pattern})$`).
    pub fn to_config_line(&self) -> String {
        format!("windowrulev2 = {}, class:^({})$", self.rule_type, self.class_pattern)
    }
}

/// Integrator for XDG desktop entry files, MIME associations, and Hyprland window rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopIntegration {
    /// Application display name
    pub app_name: String,
    /// Binary executable command line
    pub exec_cmd: String,
    /// Icon identifier name
    pub icon_name: String,
    /// Associated MIME types
    pub mime_types: Vec<String>,
}

impl Default for DesktopIntegration {
    fn default() -> Self {
        Self {
            app_name: "SWAL Files".to_string(),
            exec_cmd: "swal-files %U".to_string(),
            icon_name: "folder-open".to_string(),
            mime_types: vec![
                "inode/directory".to_string(),
                "x-scheme-handler/file".to_string(),
            ],
        }
    }
}

impl DesktopIntegration {
    /// Creates a new `DesktopIntegration` instance with specified application name and executable command.
    pub fn new(app_name: impl Into<String>, exec_cmd: impl Into<String>) -> Self {
        Self {
            app_name: app_name.into(),
            exec_cmd: exec_cmd.into(),
            ..Default::default()
        }
    }

    /// Generates XDG `.desktop` file content using the integration config.
    pub fn generate_desktop_entry(&self) -> String {
        generate_desktop_file(&self.app_name, &self.exec_cmd, &self.icon_name, &self.mime_types)
    }

    /// Generates Hyprland window rules presets for floating or tiling modes.
    pub fn generate_hyprland_rules(&self, floating: bool) -> Vec<HyprlandWindowRule> {
        let class = "swal-files";
        if floating {
            vec![
                HyprlandWindowRule::new(class, "float"),
                HyprlandWindowRule::new(class, "center"),
                HyprlandWindowRule::new(class, "size 1000 650"),
            ]
        } else {
            vec![HyprlandWindowRule::new(class, "tile")]
        }
    }

    /// Generates `mimeapps.list` default application mapping section.
    pub fn generate_mime_associations(&self) -> String {
        let mut out = String::from("[Default Applications]\n");
        for mime in &self.mime_types {
            out.push_str(&format!("{}=swal-files.desktop\n", mime));
        }
        out
    }

    /// Writes the `.desktop` file to the specified target directory.
    pub fn install_desktop_entry<P: AsRef<Path>>(&self, target_dir: P) -> io::Result<PathBuf> {
        let dir = target_dir.as_ref();
        fs::create_dir_all(dir)?;
        let desktop_path = dir.join("swal-files.desktop");
        let content = self.generate_desktop_entry();
        fs::write(&desktop_path, content)?;
        Ok(desktop_path)
    }
}

/// Generates XDG Desktop Entry file contents compliant with Freedesktop specification.
pub fn generate_desktop_file(
    app_name: &str,
    exec_cmd: &str,
    icon_name: &str,
    mime_types: &[String],
) -> String {
    let mime_str = mime_types.join(";");
    let mime_line = if mime_str.is_empty() {
        String::new()
    } else {
        format!("MimeType={};\n", mime_str)
    };

    format!(
        "[Desktop Entry]\n\
        Type=Application\n\
        Name={}\n\
        Comment=Fluent Mica Wayland File Manager\n\
        Exec={}\n\
        Icon={}\n\
        Terminal=false\n\
        Categories=System;FileTools;FileManager;Utility;\n\
        Keywords=file;manager;explorer;browser;folder;\n\
        StartupWMClass=swal-files\n\
        {}",
        app_name, exec_cmd, icon_name, mime_line
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_hyprland_window_rule_formatting() {
        let rule = HyprlandWindowRule::new("swal-files", "float");
        assert_eq!(rule.to_config_line(), "windowrulev2 = float, class:^(swal-files)$");
    }

    #[test]
    fn test_generate_desktop_file() {
        let mimes = vec!["inode/directory".to_string(), "x-scheme-handler/file".to_string()];
        let content = generate_desktop_file("SWAL Files", "swal-files %U", "folder-open", &mimes);

        assert!(content.contains("[Desktop Entry]"));
        assert!(content.contains("Name=SWAL Files"));
        assert!(content.contains("Exec=swal-files %U"));
        assert!(content.contains("MimeType=inode/directory;x-scheme-handler/file;\n"));
        assert!(content.contains("StartupWMClass=swal-files"));
    }

    #[test]
    fn test_desktop_integration_hyprland_presets() {
        let integration = DesktopIntegration::default();
        let floating_rules = integration.generate_hyprland_rules(true);
        assert_eq!(floating_rules.len(), 3);
        assert_eq!(floating_rules[0].to_config_line(), "windowrulev2 = float, class:^(swal-files)$");

        let tiling_rules = integration.generate_hyprland_rules(false);
        assert_eq!(tiling_rules.len(), 1);
        assert_eq!(tiling_rules[0].to_config_line(), "windowrulev2 = tile, class:^(swal-files)$");
    }

    #[test]
    fn test_generate_mime_associations() {
        let integration = DesktopIntegration::default();
        let assoc = integration.generate_mime_associations();
        assert!(assoc.contains("[Default Applications]"));
        assert!(assoc.contains("inode/directory=swal-files.desktop"));
        assert!(assoc.contains("x-scheme-handler/file=swal-files.desktop"));
    }

    #[test]
    fn test_install_desktop_entry() {
        let temp = tempdir().unwrap();
        let integration = DesktopIntegration::default();
        let installed = integration.install_desktop_entry(temp.path()).unwrap();

        assert!(installed.exists());
        assert_eq!(installed.file_name().unwrap(), "swal-files.desktop");

        let content = fs::read_to_string(installed).unwrap();
        assert!(content.contains("Name=SWAL Files"));
    }
}
