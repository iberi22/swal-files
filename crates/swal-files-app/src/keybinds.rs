#![deny(unsafe_code)]

use serde::{Deserialize, Serialize};

/// Modifier key flags for keyboard shortcuts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct KeyModifier {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
}

impl KeyModifier {
    pub const NONE: Self = Self { ctrl: false, alt: false, shift: false, meta: false };
    pub const CTRL: Self = Self { ctrl: true, alt: false, shift: false, meta: false };
    pub const ALT: Self = Self { ctrl: false, alt: true, shift: false, meta: false };
    pub const SHIFT: Self = Self { ctrl: false, alt: false, shift: true, meta: false };
    pub const META: Self = Self { ctrl: false, alt: false, shift: false, meta: true };

    /// Creates a new [`KeyModifier`] set.
    pub fn new(ctrl: bool, alt: bool, shift: bool, meta: bool) -> Self {
        Self { ctrl, alt, shift, meta }
    }
}

/// System and navigation actions triggered by keyboard shortcuts.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KeyAction {
    NewTab,
    CloseTab,
    SwitchPane,
    ToggleSelection,
    FocusOmnibar,
    NavigateParent,
    OpenItem,
    Cancel,
    Custom(String),
}

/// A binding connecting a key and modifier combination to a [`KeyAction`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ShortcutBinding {
    pub key: String,
    pub modifier: KeyModifier,
    pub action: KeyAction,
}

impl ShortcutBinding {
    /// Constructs a new [`ShortcutBinding`].
    pub fn new(key: impl Into<String>, modifier: KeyModifier, action: KeyAction) -> Self {
        Self {
            key: key.into().to_lowercase(),
            modifier,
            action,
        }
    }

    /// Returns `true` if the given key and modifier match this binding.
    pub fn matches(&self, key: &str, modifier: &KeyModifier) -> bool {
        self.key.eq_ignore_ascii_case(key) && self.modifier == *modifier
    }
}

/// Dispatcher and manager for application keyboard shortcuts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeybindManager {
    bindings: Vec<ShortcutBinding>,
}

impl KeybindManager {
    /// Creates a new [`KeybindManager`] initialized with default keybindings.
    pub fn new() -> Self {
        let mut manager = Self { bindings: Vec::new() };
        manager.register_defaults();
        manager
    }

    /// Registers standard dual-pane keybindings (Ctrl+T, Ctrl+W, F6, Space, Ctrl+L, Backspace, Enter, Esc).
    pub fn register_defaults(&mut self) {
        self.bindings = vec![
            ShortcutBinding::new("t", KeyModifier::CTRL, KeyAction::NewTab),
            ShortcutBinding::new("w", KeyModifier::CTRL, KeyAction::CloseTab),
            ShortcutBinding::new("f6", KeyModifier::NONE, KeyAction::SwitchPane),
            ShortcutBinding::new("space", KeyModifier::NONE, KeyAction::ToggleSelection),
            ShortcutBinding::new("l", KeyModifier::CTRL, KeyAction::FocusOmnibar),
            ShortcutBinding::new("backspace", KeyModifier::NONE, KeyAction::NavigateParent),
            ShortcutBinding::new("enter", KeyModifier::NONE, KeyAction::OpenItem),
            ShortcutBinding::new("esc", KeyModifier::NONE, KeyAction::Cancel),
        ];
    }

    /// Registers or overrides a [`ShortcutBinding`].
    pub fn register(&mut self, binding: ShortcutBinding) {
        self.bindings
            .retain(|b| !(b.key.eq_ignore_ascii_case(&binding.key) && b.modifier == binding.modifier));
        self.bindings.push(binding);
    }

    /// Unregisters a binding matching the key and modifier. Returns `true` if removed.
    pub fn unregister(&mut self, key: &str, modifier: &KeyModifier) -> bool {
        let initial_len = self.bindings.len();
        self.bindings
            .retain(|b| !(b.key.eq_ignore_ascii_case(key) && b.modifier == *modifier));
        self.bindings.len() < initial_len
    }

    /// Dispatches a key press and returns the associated [`KeyAction`], if any.
    pub fn dispatch(&self, key: &str, modifier: &KeyModifier) -> Option<KeyAction> {
        self.bindings
            .iter()
            .find(|b| b.matches(key, modifier))
            .map(|b| b.action.clone())
    }

    /// Returns a slice of active bindings.
    pub fn bindings(&self) -> &[ShortcutBinding] {
        &self.bindings
    }

    /// Removes all registered bindings.
    pub fn clear(&mut self) {
        self.bindings.clear();
    }
}

impl Default for KeybindManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_keybindings_dispatch() {
        let manager = KeybindManager::new();

        assert_eq!(manager.dispatch("t", &KeyModifier::CTRL), Some(KeyAction::NewTab));
        assert_eq!(manager.dispatch("w", &KeyModifier::CTRL), Some(KeyAction::CloseTab));
        assert_eq!(manager.dispatch("f6", &KeyModifier::NONE), Some(KeyAction::SwitchPane));
        assert_eq!(manager.dispatch("space", &KeyModifier::NONE), Some(KeyAction::ToggleSelection));
        assert_eq!(manager.dispatch("l", &KeyModifier::CTRL), Some(KeyAction::FocusOmnibar));
        assert_eq!(manager.dispatch("backspace", &KeyModifier::NONE), Some(KeyAction::NavigateParent));
        assert_eq!(manager.dispatch("enter", &KeyModifier::NONE), Some(KeyAction::OpenItem));
        assert_eq!(manager.dispatch("esc", &KeyModifier::NONE), Some(KeyAction::Cancel));
    }

    #[test]
    fn test_case_insensitivity_and_unmatched() {
        let manager = KeybindManager::new();

        assert_eq!(manager.dispatch("T", &KeyModifier::CTRL), Some(KeyAction::NewTab));
        assert_eq!(manager.dispatch("W", &KeyModifier::CTRL), Some(KeyAction::CloseTab));
        assert_eq!(manager.dispatch("x", &KeyModifier::CTRL), None);
        assert_eq!(manager.dispatch("t", &KeyModifier::NONE), None);
    }

    #[test]
    fn test_custom_binding_register_and_unregister() {
        let mut manager = KeybindManager::new();
        let custom = ShortcutBinding::new("g", KeyModifier::CTRL, KeyAction::Custom("GitStatus".to_string()));

        manager.register(custom);
        assert_eq!(
            manager.dispatch("g", &KeyModifier::CTRL),
            Some(KeyAction::Custom("GitStatus".to_string()))
        );

        assert!(manager.unregister("g", &KeyModifier::CTRL));
        assert_eq!(manager.dispatch("g", &KeyModifier::CTRL), None);
        assert!(!manager.unregister("g", &KeyModifier::CTRL));
    }

    #[test]
    fn test_override_existing_binding() {
        let mut manager = KeybindManager::new();
        let override_binding = ShortcutBinding::new("t", KeyModifier::CTRL, KeyAction::Custom("CustomNewTab".to_string()));

        manager.register(override_binding);
        assert_eq!(
            manager.dispatch("t", &KeyModifier::CTRL),
            Some(KeyAction::Custom("CustomNewTab".to_string()))
        );
    }

    #[test]
    fn test_clear_bindings() {
        let mut manager = KeybindManager::new();
        assert!(!manager.bindings().is_empty());
        manager.clear();
        assert!(manager.bindings().is_empty());
        assert_eq!(manager.dispatch("t", &KeyModifier::CTRL), None);
    }
}
